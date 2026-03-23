import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  ExecutionLogEntry,
  TaskExecutionState,
  ExecutionQueue,
  CreateExecutionLogInput,
  SaveTaskExecutionResultInput,
  TaskExecutionResultRecord,
  PlanExecutionProgress
} from '@/types/taskExecution'
import type { ToolCall } from '@/stores/message'
import type { StreamEvent, McpServerConfig } from '@/services/conversation/strategies/types'
import type { AIFormRequest, UpdatePlanInput, UpdateTaskInput } from '@/types/plan'
import { useTaskStore } from '@/stores/task'
import { usePlanStore } from '@/stores/plan'
import { useProjectStore } from '@/stores/project'
import { useAgentStore } from '@/stores/agent'
import { agentExecutor } from '@/services/conversation/AgentExecutor'
import type { ConversationContext } from '@/services/conversation/strategies/types'
import { extractFirstFormRequest } from '@/utils/structuredContent'
import {
  buildExecutionPrompt,
  buildResumeExecutionContext,
  compactExecutionSummary,
  parseExecutionResult
} from '@/utils/taskExecutionText'
import { resolvePlanTaskAgentSelection } from '@/utils/planExecutionProgress'
import {
  createExecutionLogEntry,
  createExecutionQueue,
  createExecutionState
} from './taskExecutionShared'
import type { PendingLogBuffer } from './taskExecutionShared'
import {
  clearPendingBuffer as clearPendingLogBuffer,
  clearPlanExecutionResultsFromBackend,
  clearTaskLogsFromBackend,
  flushPendingLogs as flushTaskPendingLogs,
  addStreamLogToBuffer as addBufferedStreamLog,
  getPlanExecutionProgressFromBackend,
  listRecentPlanResultsFromBackend,
  loadTaskLogsFromBackend,
  persistExecutionLog,
  saveTaskExecutionResultToBackend,
} from './taskExecutionPersistence'
import {
  clearPlanExecutionRuntime,
  computePlanRuntimeUpdate,
  findNextExecutableTask,
  getPlanTasks,
  resetExecutionStateRuntime,
  shouldQueueReadyTask
} from './taskExecutionPlanRuntime'
import {
  buildTaskInputRequest,
  canHydrateTaskLogs,
  clearTaskStopRequested,
  hydrateTaskLogs,
  isTaskStopRequested,
  markTaskStopRequested,
  shouldKeepInMemoryLogs
} from './taskExecutionTaskRuntime'
import { loadAgentMcpServers } from '@/utils/mcpServerConfig'
import { mergeToolInputArguments } from '@/utils/toolInput'
import { getErrorMessage } from '@/utils/api'

function finalizeRunningToolCalls(toolCalls: ToolCall[]): void {
  for (const toolCall of toolCalls) {
    if (toolCall.status === 'running') {
      toolCall.status = 'success'
    }
  }
}

const TASK_EXECUTION_STOPPED_ERROR = '__TASK_EXECUTION_STOPPED__'

function isMissingRecordError(error: unknown): boolean {
  return /query returned no rows/i.test(getErrorMessage(error))
}

/**
 *
 * 功能�?
 */

export const useTaskExecutionStore = defineStore('taskExecution', () => {
  // ==================== State ====================

  const executionStates = ref<Map<string, TaskExecutionState>>(new Map())

  const executionQueues = ref<Map<string, ExecutionQueue>>(new Map())

  const currentViewingTaskId = ref<string | null>(null)

  const stopRequestedTaskIds = ref<Set<string>>(new Set())

  // 待持久化的日志缓冲区 (taskId -> PendingLogBuffer)
  const pendingLogBuffers = ref<Map<string, PendingLogBuffer>>(new Map())

  // ==================== Getters ====================

  /**
   */
  const getExecutionState = computed(() => {
    return (taskId: string): TaskExecutionState | undefined => {
      return executionStates.value.get(taskId)
    }
  })

  /**
   */
  const isTaskExecuting = computed(() => {
    return (taskId: string): boolean => {
      const state = executionStates.value.get(taskId)
      if (!state) return false
      return state.status === 'running' || state.status === 'queued'
    }
  })

  /**
   */
  const isTaskRunning = computed(() => {
    return (taskId: string): boolean => {
      const state = executionStates.value.get(taskId)
      return state?.status === 'running'
    }
  })

  const isTaskStopped = computed(() => {
    return (taskId: string): boolean => {
      const state = executionStates.value.get(taskId)
      return state?.status === 'stopped'
    }
  })

  /**
   */
  const getExecutionQueue = computed(() => {
    return (planId: string): ExecutionQueue | undefined => {
      return executionQueues.value.get(planId)
    }
  })

  /**
   */
  const getCurrentRunningTaskId = computed(() => {
    return (planId: string): string | null => {
      const queue = executionQueues.value.get(planId)
      return queue?.currentTaskId ?? null
    }
  })

  /**
   */
  const getQueuePosition = computed(() => {
    return (taskId: string): number => {
      const taskStore = useTaskStore()
      const task = taskStore.tasks.find(t => t.id === taskId)
      if (!task) return -1

      const queue = executionQueues.value.get(task.planId)
      if (!queue) return -1

      if (queue.currentTaskId === taskId) return 0
      const index = queue.pendingTaskIds.indexOf(taskId)
      return index === -1 ? -1 : index + 1
    }
  })

  /**
   */
  const getTaskLogs = computed(() => {
    return (taskId: string): ExecutionLogEntry[] => {
      const state = executionStates.value.get(taskId)
      return state?.logs ?? []
    }
  })

  // ==================== Actions ====================

  /**
   */
  function initExecutionState(taskId: string): TaskExecutionState {
    let state = executionStates.value.get(taskId)
    if (!state) {
      state = createExecutionState(taskId)
      executionStates.value.set(taskId, state)
    }
    return state
  }

  function clearPendingBuffer(taskId: string): void {
    clearPendingLogBuffer(taskId, pendingLogBuffers.value)
  }

  async function flushPendingLogs(taskId: string): Promise<void> {
    await flushTaskPendingLogs(taskId, pendingLogBuffers.value)
  }

  function addStreamLogToBuffer(taskId: string, type: 'content' | 'thinking', content: string): void {
    addBufferedStreamLog(taskId, type, content, executionStates.value, pendingLogBuffers.value)
  }

  function resetExecutionRuntime(taskId: string): void {
    const state = executionStates.value.get(taskId)
    resetExecutionStateRuntime(state)
    clearPendingBuffer(taskId)
  }

  async function updateTaskSafely(taskId: string, updates: UpdateTaskInput): Promise<void> {
    const taskStore = useTaskStore()
    if (!taskStore.tasks.some(task => task.id === taskId)) {
      return
    }

    try {
      await taskStore.updateTask(taskId, updates)
    } catch (error) {
      if (isMissingRecordError(error) || !taskStore.tasks.some(task => task.id === taskId)) {
        console.warn('[TaskExecution] Ignoring stale task update for removed task:', taskId)
        return
      }
      throw error
    }
  }

  async function updatePlanSafely(planId: string, updates: UpdatePlanInput): Promise<void> {
    const planStore = usePlanStore()
    if (!planStore.plans.some(plan => plan.id === planId)) {
      return
    }

    try {
      await planStore.updatePlan(planId, updates)
    } catch (error) {
      if (isMissingRecordError(error) || !planStore.plans.some(plan => plan.id === planId)) {
        console.warn('[TaskExecution] Ignoring stale plan update for removed plan:', planId)
        return
      }
      throw error
    }
  }

  function updateTaskTokenUsage(
    taskId: string,
    usage: Pick<StreamEvent, 'inputTokens' | 'outputTokens' | 'model'>
  ): void {
    const state = executionStates.value.get(taskId)
    if (!state) return

    const current = state.tokenUsage
    const nextInputTokens = typeof usage.inputTokens === 'number'
      ? usage.inputTokens
      : current.inputTokens
    const nextOutputTokens = typeof usage.outputTokens === 'number'
      ? usage.outputTokens
      : current.outputTokens
    const nextModel = usage.model || current.model
    const didReset = (
      typeof usage.inputTokens === 'number'
      || typeof usage.outputTokens === 'number'
    ) && (
      nextInputTokens < current.inputTokens
      || nextOutputTokens < current.outputTokens
      || (nextInputTokens + nextOutputTokens) < (current.inputTokens + current.outputTokens)
    )

    state.tokenUsage = {
      inputTokens: nextInputTokens,
      outputTokens: nextOutputTokens,
      model: nextModel,
      resetCount: didReset ? current.resetCount + 1 : current.resetCount,
      lastUpdatedAt: new Date().toISOString()
    }
  }

  async function persistTaskResult(
    taskId: string,
    status: 'success' | 'failed',
    fallbackSummary: string,
    failReason?: string
  ): Promise<void> {
    const parsedResult = parseExecutionResult(executionStates.value.get(taskId)?.accumulatedContent ?? '')
    const summary = parsedResult.summary === '任务已执行完成（无详细输出）'
      ? fallbackSummary
      : parsedResult.summary

    await saveTaskExecutionResult({
      task_id: taskId,
      result_status: status,
      result_summary: compactExecutionSummary(summary),
      result_files: parsedResult.files,
      fail_reason: failReason
    })
  }

  async function markTaskInProgress(taskId: string): Promise<void> {
    const taskStore = useTaskStore()
    const task = taskStore.tasks.find(item => item.id === taskId)
    if (!task || task.status === 'in_progress') return

    await updateTaskSafely(taskId, {
      status: 'in_progress',
      errorMessage: undefined,
      blockReason: undefined
    })
  }

  function getOrCreateQueue(planId: string): ExecutionQueue {
    let queue = executionQueues.value.get(planId)
    if (!queue) {
      queue = createExecutionQueue(planId)
      executionQueues.value.set(planId, queue)
    }
    return queue
  }

  function removeTaskFromQueue(queue: ExecutionQueue, taskId: string): void {
    if (queue.currentTaskId === taskId) {
      queue.currentTaskId = null
    }

    const pendingIndex = queue.pendingTaskIds.indexOf(taskId)
    if (pendingIndex >= 0) {
      queue.pendingTaskIds.splice(pendingIndex, 1)
    }
  }

  async function markTaskStopped(taskId: string, options: { persistLog?: boolean } = {}): Promise<void> {
    const state = executionStates.value.get(taskId)
    if (!state) {
      return
    }

    finalizeRunningToolCalls(state.toolCalls)
    state.status = 'stopped'
    state.completedAt = new Date().toISOString()
    await flushPendingLogs(taskId)

    if (options.persistLog !== false) {
      const lastLog = state.logs[state.logs.length - 1]
      if (lastLog?.type !== 'system' || lastLog.content !== '???????') {
        await addExecutionLog({
          taskId,
          type: 'system',
          content: '???????'
        })
      }
    }
  }

  async function syncPlanRuntimeState(planId: string): Promise<void> {
    const planStore = usePlanStore()
    const plan = planStore.plans.find(item => item.id === planId)
    if (!plan) return

    const tasksInPlan = getPlanTasks(useTaskStore().tasks, planId)
    const queue = executionQueues.value.get(planId)
    const nextPlanState = computePlanRuntimeUpdate(plan, tasksInPlan, queue, executionStates.value)

    if (nextPlanState) {
      await updatePlanSafely(planId, nextPlanState)
    }
  }

  async function seedReadyPendingTasks(planId: string): Promise<void> {
    const taskStore = useTaskStore()
    let queue = executionQueues.value.get(planId)
    if (!queue) {
      queue = createExecutionQueue(planId)
      executionQueues.value.set(planId, queue)
    }

    const readyTasks = taskStore.getReadyTasks(planId)

    for (const task of readyTasks) {
      const state = executionStates.value.get(task.id)
      if (!shouldQueueReadyTask(task.id, queue, state)) continue

      await markTaskInProgress(task.id)
      initExecutionState(task.id).status = 'queued'
      queue.pendingTaskIds.push(task.id)
    }
  }

  /**
   */
  async function enqueueTask(planId: string, taskId: string): Promise<void> {
    await markTaskInProgress(taskId)

    const queue = getOrCreateQueue(planId)
    const state = initExecutionState(taskId)
    const isResumingStoppedTask = state.status === 'stopped'

    if (queue.currentTaskId === taskId && state.status === 'running') {
      await syncPlanRuntimeState(planId)
      return
    }

    if (queue.pendingTaskIds.includes(taskId) && state.status === 'queued') {
      await syncPlanRuntimeState(planId)
      return
    }

    removeTaskFromQueue(queue, taskId)
    if (queue.lastInterruptedTaskId === taskId) {
      queue.lastInterruptedTaskId = null
    }

    if (queue.currentTaskId) {
      if (!queue.pendingTaskIds.includes(taskId)) {
        queue.pendingTaskIds.push(taskId)
        state.status = 'queued'
      }
      await syncPlanRuntimeState(planId)
      return
    }

    if (queue.isPaused) {
      if (!queue.pendingTaskIds.includes(taskId)) {
        queue.pendingTaskIds.push(taskId)
        state.status = 'queued'
      }
      await syncPlanRuntimeState(planId)
      return
    }

    queue.currentTaskId = taskId
    await syncPlanRuntimeState(planId)
    await executeTask(planId, taskId, { resume: isResumingStoppedTask })
  }

  /**
   */
  async function executeTask(planId: string, taskId: string, options: { resume?: boolean } = {}): Promise<void> {
    const taskStore = useTaskStore()
    const planStore = usePlanStore()
    const projectStore = useProjectStore()
    const agentStore = useAgentStore()

    const task = taskStore.tasks.find(t => t.id === taskId)
    if (!task) {
      console.error('[TaskExecution] Task not found:', taskId)
      return
    }

    const plan = planStore.plans.find(p => p.id === planId)
    if (!plan) {
      console.error('[TaskExecution] Plan not found:', planId)
      return
    }

    // 获取项目路径
    const project = projectStore.projects.find(p => p.id === plan.projectId)
    const workingDirectory = project?.path

    const maxRetries = plan.maxRetryCount ?? 3

    const state = initExecutionState(taskId)
    const isResume = options.resume === true
    const resumeContext = isResume ? buildResumeExecutionContext(state) : ''
    clearTaskStopRequested(taskId, stopRequestedTaskIds.value)
    state.status = 'running'
    state.startedAt = isResume ? (state.startedAt ?? new Date().toISOString()) : new Date().toISOString()
    state.completedAt = null

    if (!isResume) {
      state.accumulatedContent = ''
      state.accumulatedThinking = ''
      state.toolCalls = []
      state.logs = []
      resetExecutionRuntime(taskId)
    }

    await addExecutionLog({
      taskId,
      type: 'system',
      content: isResume
        ? `继续执行任务: ${task.title}`
        : `??????: ${task.title}${task.retryCount > 0 ? ` (??? ${task.retryCount} ?)` : ''}`
    })
    await syncPlanRuntimeState(planId)

    let skipQueueAdvance = false

    try {
      const selection = resolvePlanTaskAgentSelection(
        {
          agent_id: task.agentId ?? null,
          model_id: task.modelId ?? null
        },
        plan
      )

      const baseAgent = (selection.agentId
        ? agentStore.agents.find(agent => agent.id === selection.agentId)
        : null)
        || (plan.splitAgentId ? agentStore.agents.find(agent => agent.id === plan.splitAgentId) : null)
        || agentStore.agents[0]

      if (!baseAgent) {
        throw new Error('?????????')
      }

      const agent = {
        ...baseAgent,
        modelId: selection.modelId || baseAgent.modelId
      }

      if (!agentExecutor.isSupported(agent)) {
        throw new Error(`?????????: ${agent.type}`)
      }

      const recentResults = await listRecentPlanResults(planId, 5)
      const planProgress = await getPlanExecutionProgress(planId)

      const prompt = buildExecutionPrompt(task, recentResults, planProgress, resumeContext || undefined)

      const mcpServers = await loadAgentMcpServers(agent).catch((error) => {
        console.warn('[TaskExecution] Failed to load MCP servers:', error)
        return [] as McpServerConfig[]
      })

      // 构建对话上下�?
      const context: ConversationContext = {
        sessionId: `task-${taskId}`,
        agent,
        messages: [{
          id: `task-prompt-${taskId}`,
          sessionId: `task-${taskId}`,
          role: 'user',
          content: prompt,
          status: 'completed',
          createdAt: new Date().toISOString()
        }],
        workingDirectory,
        mcpServers: mcpServers.length > 0 ? mcpServers : undefined,
        executionMode: 'task_execution',
        responseMode: 'stream_text'
      }

      await agentExecutor.execute(context, (event: StreamEvent) => {
        handleStreamEvent(taskId, event)
      })

      if (isTaskStopRequested(taskId, stopRequestedTaskIds.value)) {
        throw new Error(TASK_EXECUTION_STOPPED_ERROR)
      }

      const fatalErrorLog = [...state.logs]
        .reverse()
        .find(log => log.type === 'error' && log.content.trim().length > 0)

      if (fatalErrorLog) {
        throw new Error(fatalErrorLog.content.trim())
      }

      finalizeRunningToolCalls(state.toolCalls)
      state.status = 'completed'
      state.completedAt = new Date().toISOString()

      const formRequest = parseFormRequest(state.accumulatedContent)
      if (formRequest) {
        await blockTaskForInput(taskId, formRequest)
        return
      }

      await addExecutionLog({
        taskId,
        type: 'system',
        content: '任务执行完成'
      })

      await persistTaskResult(taskId, 'success', '任务执行完成')

      try {
        await updateTaskSafely(taskId, { status: 'completed' })
      } catch (statusError) {
        console.warn('[TaskExecution] Failed to update task status to completed:', statusError)
      }

    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)

      const wasStopped = isTaskStopRequested(taskId, stopRequestedTaskIds.value)

      if (wasStopped) {
        await markTaskStopped(taskId)
      } else {
        const currentRetryCount = task.retryCount + 1

        if (currentRetryCount < maxRetries) {
          await addExecutionLog({
            taskId,
            type: 'system',
            content: `??????: ${errorMessage}???? ${currentRetryCount + 1} ???...`
          })

          // 更新重试次数
          try {
            await updateTaskSafely(taskId, {
              retryCount: currentRetryCount,
              errorMessage
            })
          } catch (statusError) {
            console.warn('[TaskExecution] Failed to update task retry count:', statusError)
          }

          await persistTaskResult(taskId, 'failed', `任务执行失败: ${errorMessage}`, errorMessage)

          finalizeRunningToolCalls(state.toolCalls)
          state.status = 'queued'
          state.completedAt = new Date().toISOString()
          skipQueueAdvance = true

          // 使用 setTimeout 延迟重试，避免立即重�?
          setTimeout(() => {
            void executeTask(planId, taskId)
          }, 1000)

          return
        } else {
          finalizeRunningToolCalls(state.toolCalls)
          state.status = 'failed'
          state.completedAt = new Date().toISOString()

          await addExecutionLog({
            taskId,
            type: 'error',
            content: `任务执行失败（已重试 ${currentRetryCount} 次）: ${errorMessage}`
          })

          try {
            await updateTaskSafely(taskId, {
              status: 'failed',
              retryCount: currentRetryCount,
              errorMessage
            })
          } catch (statusError) {
            console.warn('[TaskExecution] Failed to update task status to failed:', statusError)
          }

          await persistTaskResult(taskId, 'failed', `任务执行失败: ${errorMessage}`, errorMessage)
        }
      }
    } finally {
      clearTaskStopRequested(taskId, stopRequestedTaskIds.value)
      await flushPendingLogs(taskId)
      if (!skipQueueAdvance) {
        await processNextInQueue(planId)
      }
    }
  }

  /**
   */
  function handleStreamEvent(taskId: string, event: StreamEvent): void {
    const state = executionStates.value.get(taskId)
    if (!state) return

    if (event.inputTokens !== undefined || event.outputTokens !== undefined || event.model) {
      updateTaskTokenUsage(taskId, event)
    }

    switch (event.type) {
      case 'content':
        if (event.content) {
          state.accumulatedContent += event.content
          addStreamLogToBuffer(taskId, 'content', event.content)
        }
        break

      case 'thinking_start':
        void addExecutionLog({
          taskId,
          type: 'thinking_start',
          content: ''
        })
        break

      case 'thinking':
        if (event.content) {
          state.accumulatedThinking += event.content
          addStreamLogToBuffer(taskId, 'thinking', event.content)
        }
        break

      case 'tool_use':
        if (event.toolName && event.toolCallId) {
          const toolCall: ToolCall = {
            id: event.toolCallId,
            name: event.toolName,
            arguments: event.toolInput || {},
            status: 'running'
          }

          const existingIndex = state.toolCalls.findIndex(tc => tc.id === toolCall.id)
          if (existingIndex >= 0) {
            state.toolCalls[existingIndex] = toolCall
          } else {
            state.toolCalls.push(toolCall)
          }

          addExecutionLog({
            taskId,
            type: 'tool_use',
            content: JSON.stringify(event.toolInput, null, 2),
            metadata: {
              toolName: event.toolName,
              toolCallId: event.toolCallId,
              toolInput: JSON.stringify(event.toolInput ?? {})
            }
          })
        }
        break

      case 'tool_input_delta': {
        const targetToolCall = (event.toolCallId
          ? state.toolCalls.find(tool => tool.id === event.toolCallId)
          : null) || [...state.toolCalls].reverse().find(tool => tool.status === 'running')

        if (targetToolCall) {
          targetToolCall.arguments = mergeToolInputArguments(targetToolCall.arguments, event.toolInput)
        }

        if (event.toolInput) {
          void addExecutionLog({
            taskId,
            type: 'tool_input_delta',
            content: typeof event.toolInput.raw === 'string'
              ? event.toolInput.raw
              : JSON.stringify(event.toolInput),
            metadata: {
              toolCallId: event.toolCallId,
              toolInput: typeof event.toolInput.raw === 'string'
                ? event.toolInput.raw
                : JSON.stringify(event.toolInput)
            }
          })
        }
        break
      }

      case 'tool_result':
        if (event.toolCallId) {
          const result = typeof event.toolResult === 'string'
            ? event.toolResult
            : JSON.stringify(event.toolResult, null, 2)
          const isError = false

          const tc = state.toolCalls.find(t => t.id === event.toolCallId)
          if (tc) {
            tc.result = result
            tc.status = isError ? 'error' : 'success'
            if (isError) {
              tc.errorMessage = result
            }
          }

          addExecutionLog({
            taskId,
            type: 'tool_result',
            content: result,
            metadata: {
              toolCallId: event.toolCallId,
              toolResult: result,
              isError
            }
          })
        }
        break

      case 'error':
        if (event.error) {
          addExecutionLog({
            taskId,
            type: 'error',
            content: event.error
          })
        }
        break

      case 'usage':
      case 'done':
      case 'system':
      case 'file_edit':
        break
    }
  }

  /**
   */
  async function addExecutionLog(input: CreateExecutionLogInput, persist: boolean = true): Promise<void> {
    const state = executionStates.value.get(input.taskId)
    if (!state) return

    const entry = persist
      ? await persistExecutionLog(input)
      : createExecutionLogEntry(input)

    state.logs.push(entry)
  }

  async function saveTaskExecutionResult(input: SaveTaskExecutionResultInput): Promise<void> {
    await saveTaskExecutionResultToBackend(input)
  }

  async function listRecentPlanResults(planId: string, limit: number = 5): Promise<TaskExecutionResultRecord[]> {
    return listRecentPlanResultsFromBackend(planId, limit)
  }

  async function getPlanExecutionProgress(planId: string): Promise<PlanExecutionProgress | null> {
    return getPlanExecutionProgressFromBackend(planId)
  }

  /**
   */
  async function clearPlanExecutionResults(planId: string): Promise<number> {
    try {
      const deletedCount = await clearPlanExecutionResultsFromBackend(planId)

      const taskStore = useTaskStore()
      const planTasks = taskStore.tasks.filter(t => t.planId === planId)
      for (const task of planTasks) {
        const state = executionStates.value.get(task.id)
        if (state) {
          state.logs = []
          state.accumulatedContent = ''
          state.accumulatedThinking = ''
          state.toolCalls = []
        }

        clearPendingBuffer(task.id)
      }

      return deletedCount
    } catch (error) {
      console.warn('[TaskExecution] Failed to clear plan execution results:', error)
      throw error
    }
  }

  /**
   */
  async function submitTaskInput(taskId: string, values: Record<string, unknown>): Promise<void> {
    const taskStore = useTaskStore()
    const task = taskStore.tasks.find(t => t.id === taskId)
    if (!task || task.status !== 'blocked') {
      console.warn('[TaskExecution] Cannot submit input: task not blocked')
      return
    }

    // 保存用户输入
    await updateTaskSafely(taskId, {
      inputResponse: values,
      status: 'pending',
      blockReason: undefined,
      inputRequest: undefined
    })

    // 添加日志
    await addExecutionLog({
      taskId,
      type: 'system',
      content: `???????: ${JSON.stringify(values)}`
    })

    await enqueueTask(task.planId, taskId)
  }

  /**
   * 跳过阻塞的任�?
   */
  async function skipBlockedTask(taskId: string): Promise<void> {
    const taskStore = useTaskStore()
    const task = taskStore.tasks.find(t => t.id === taskId)
    if (!task || task.status !== 'blocked') {
      console.warn('[TaskExecution] Cannot skip: task not blocked')
      return
    }

    await updateTaskSafely(taskId, {
      status: 'pending',
      blockReason: undefined,
      inputRequest: undefined
    })

    // 添加日志
    await addExecutionLog({
      taskId,
      type: 'system',
      content: '用户跳过了此任务'
    })

    await syncPlanRuntimeState(task.planId)
  }

  /**
   */
  async function blockTaskForInput(taskId: string, request: AIFormRequest): Promise<void> {
    const taskStore = useTaskStore()
    const state = executionStates.value.get(taskId)
    const task = taskStore.tasks.find(item => item.id === taskId)
    const inputRequest = buildTaskInputRequest(request)

    await updateTaskSafely(taskId, {
      status: 'blocked',
      blockReason: 'waiting_input',
      inputRequest
    })

    if (state) {
      state.status = 'waiting_input'
      state.completedAt = new Date().toISOString()
    }

    // 添加日志
    await addExecutionLog({
      taskId,
      type: 'system',
      content: `任务等待用户输入: ${request.question || inputRequest.formSchema.title}`
    })

    if (task) {
      await syncPlanRuntimeState(task.planId)
    }
  }

  /**
   */
  async function processNextInQueue(planId: string): Promise<void> {
    const taskStore = useTaskStore()
    const queue = executionQueues.value.get(planId)
    if (!queue) return

    queue.currentTaskId = null

    if (queue.isPaused) {
      await syncPlanRuntimeState(planId)
      return
    }

    await seedReadyPendingTasks(planId)

    const nextTaskId = findNextExecutableTask(taskStore.tasks, queue.pendingTaskIds, taskStore.areDependenciesMet)

    if (nextTaskId) {
      removeTaskFromQueue(queue, nextTaskId)
      queue.currentTaskId = nextTaskId
      queue.lastInterruptedTaskId = null

      const state = executionStates.value.get(nextTaskId)
      if (state && state.status === 'queued') {
        state.status = 'running'
      }

      await executeTask(planId, nextTaskId)
      return
    }

    await syncPlanRuntimeState(planId)
  }

  /**
   */
  /**
   */
  async function stopTaskExecution(
    taskId: string,
    options: { pauseQueue?: boolean; autoAdvance?: boolean } = {}
  ): Promise<void> {
    const state = initExecutionState(taskId)

    const taskStore = useTaskStore()
    const task = taskStore.tasks.find(t => t.id === taskId)
    if (!task) return

    const queue = getOrCreateQueue(task.planId)
    const pauseQueue = options.pauseQueue === true
    const autoAdvance = options.autoAdvance !== false
    const isCurrentTask = queue.currentTaskId === taskId
    const isRunningTask = state.status === 'running' && isCurrentTask

    if (pauseQueue) {
      queue.isPaused = true
      queue.lastInterruptedTaskId = taskId
    }

    const sessionId = `task-${taskId}`
    markTaskStopRequested(taskId, stopRequestedTaskIds.value)

    if (isRunningTask) {
      state.status = 'stopped'
      state.completedAt = new Date().toISOString()
      finalizeRunningToolCalls(state.toolCalls)
      agentExecutor.abort(sessionId)
      await syncPlanRuntimeState(task.planId)
      return
    }

    agentExecutor.abort(sessionId)
    removeTaskFromQueue(queue, taskId)
    if (!pauseQueue && queue.lastInterruptedTaskId === taskId) {
      queue.lastInterruptedTaskId = null
    }
    await markTaskStopped(taskId)

    if (autoAdvance && !pauseQueue) {
      await processNextInQueue(task.planId)
      return
    }

    await syncPlanRuntimeState(task.planId)
  }

  async function pausePlanExecutionFlow(planId: string): Promise<void> {
    const queue = getOrCreateQueue(planId)
    queue.isPaused = true

    if (queue.currentTaskId) {
      queue.lastInterruptedTaskId = queue.currentTaskId
      await stopTaskExecution(queue.currentTaskId, {
        pauseQueue: true,
        autoAdvance: false
      })
      return
    }

    await syncPlanRuntimeState(planId)
  }

  async function resumePlanExecutionFlow(planId: string): Promise<void> {
    const queue = getOrCreateQueue(planId)
    const taskStore = useTaskStore()
    const planStore = usePlanStore()
    queue.isPaused = false

    const plan = planStore.plans.find(item => item.id === planId)
    const resumeTaskId = queue.lastInterruptedTaskId
      ?? plan?.currentTaskId
      ?? taskStore.tasks
        .filter(item => item.planId === planId && item.status === 'in_progress')
        .sort((a, b) => a.order - b.order)[0]?.id

    if (resumeTaskId) {
      const task = taskStore.tasks.find(item => item.id === resumeTaskId)
      const prepared = await prepareInterruptedTaskForResume(resumeTaskId)
      if (prepared && task?.status === 'in_progress') {
        removeTaskFromQueue(queue, resumeTaskId)
        queue.currentTaskId = resumeTaskId
        queue.lastInterruptedTaskId = null
        await syncPlanRuntimeState(planId)
        await executeTask(planId, resumeTaskId, { resume: true })
        return
      }
      queue.lastInterruptedTaskId = null
    }

    const resumableTasks = taskStore.tasks
      .filter((item) => item.planId === planId && item.status === 'in_progress')
      .sort((a, b) => a.order - b.order)

    if (resumableTasks.length > 0) {
      for (const task of resumableTasks) {
        await enqueueTask(planId, task.id)
      }
      return
    }

    await processNextInQueue(planId)
  }

  async function resumeTaskExecution(taskId: string): Promise<void> {
    const taskStore = useTaskStore()
    const task = taskStore.tasks.find(item => item.id === taskId)
    if (!task) return

    const queue = getOrCreateQueue(task.planId)
    if (queue.isPaused) {
      if (queue.lastInterruptedTaskId === taskId) {
        await resumePlanExecutionFlow(task.planId)
        return
      }

      queue.isPaused = false
      queue.lastInterruptedTaskId = null
      await syncPlanRuntimeState(task.planId)
    }

    await enqueueTask(task.planId, taskId)
  }

  /**
   */
  function setCurrentViewingTask(taskId: string | null): void {
    currentViewingTaskId.value = taskId
  }

  /**
   */
  async function loadTaskLogs(taskId: string): Promise<void> {
    try {
      const existingState = executionStates.value.get(taskId)
      if (!canHydrateTaskLogs(existingState)) {
        return
      }

      const rustLogs = await loadTaskLogsFromBackend(taskId)

      const state = initExecutionState(taskId)

      // 如果已有日志且数量大于等于后端返回的日志数量，不覆盖（可能内存中更新�?
      if (shouldKeepInMemoryLogs(existingState, rustLogs.length)) {
        return
      }

      const task = useTaskStore().tasks.find(item => item.id === taskId)
      hydrateTaskLogs(state, rustLogs, task)

    } catch (error) {
      console.warn('[TaskExecution] Failed to load logs:', error)
    }
  }

  async function prepareInterruptedTaskForResume(taskId: string): Promise<boolean> {
    const taskStore = useTaskStore()
    const task = taskStore.tasks.find(item => item.id === taskId)
    if (!task || task.status !== 'in_progress') {
      return false
    }

    const state = initExecutionState(taskId)
    if (state.logs.length === 0) {
      const rustLogs = await loadTaskLogsFromBackend(taskId)
      hydrateTaskLogs(state, rustLogs, task)
    }

    if (state.status === 'running' || state.status === 'queued' || state.status === 'idle') {
      state.status = 'stopped'
    }

    if (!state.startedAt) {
      state.startedAt = state.logs[0]?.timestamp ?? task.updatedAt ?? new Date().toISOString()
    }
    state.completedAt = null

    return state.logs.length > 0
  }

  /**
   */
  async function clearTaskLogs(taskId: string): Promise<void> {
    try {
      await clearTaskLogsFromBackend(taskId)

      resetExecutionRuntime(taskId)
    } catch (error) {
      console.warn('[TaskExecution] Failed to clear logs:', error)
    }
  }

  /**
   */
  function clearPlanExecution(planId: string): void {
    const queue = executionQueues.value.get(planId)
    const taskIds = [
      queue?.currentTaskId ?? null,
      ...(queue?.pendingTaskIds ?? [])
    ].filter((taskId): taskId is string => Boolean(taskId))

    taskIds.forEach((taskId) => {
      markTaskStopRequested(taskId, stopRequestedTaskIds.value)
      clearPendingBuffer(taskId)
      agentExecutor.abort(`task-${taskId}`)
    })

    clearPlanExecutionRuntime(planId, executionQueues.value, executionStates.value)

    if (currentViewingTaskId.value) {
      const taskStore = useTaskStore()
      const currentTask = taskStore.tasks.find(task => task.id === currentViewingTaskId.value)
      if (!currentTask || currentTask.planId === planId) {
        currentViewingTaskId.value = null
      }
    }
  }

  function clearPlansExecution(planIds: string[]): void {
    planIds.forEach((planId) => {
      clearPlanExecution(planId)
    })

    if (currentViewingTaskId.value) {
      const taskStore = useTaskStore()
      const currentTask = taskStore.tasks.find(task => task.id === currentViewingTaskId.value)
      if (!currentTask || planIds.includes(currentTask.planId)) {
        currentViewingTaskId.value = null
      }
    }
  }

  return {
    // State
    executionStates,
    executionQueues,
    currentViewingTaskId,

    // Getters
    getExecutionState,
    isTaskExecuting,
    isTaskRunning,
    isTaskStopped,
    getExecutionQueue,
    getCurrentRunningTaskId,
    getQueuePosition,
    getTaskLogs,

    // Actions
    initExecutionState,
    enqueueTask,
    executeTask,
    stopTaskExecution,
    pausePlanExecutionFlow,
    resumePlanExecutionFlow,
    resumeTaskExecution,
    submitTaskInput,
    skipBlockedTask,
    setCurrentViewingTask,
    loadTaskLogs,
    clearTaskLogs,
    clearPlanExecution,
    clearPlansExecution,
    listRecentPlanResults,
    getPlanExecutionProgress,
    clearPlanExecutionResults
  }
})

/**
 * 解析 AI 输出中的表单请求
 */
function parseFormRequest(content: string): AIFormRequest | null {
  return extractFirstFormRequest(content)
}
