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
import type { AIFormRequest } from '@/types/plan'
import { useTaskStore } from '@/stores/task'
import { usePlanStore } from '@/stores/plan'
import { useProjectStore } from '@/stores/project'
import { useAgentStore } from '@/stores/agent'
import { agentExecutor } from '@/services/conversation/AgentExecutor'
import type { ConversationContext } from '@/services/conversation/strategies/types'
import { extractFirstFormRequest } from '@/utils/structuredContent'
import { buildExecutionPrompt, compactExecutionSummary, parseExecutionResult } from '@/utils/taskExecutionText'
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

/**
 * 任务执行状态管理 Store
 *
 * 功能：
 * - 执行队列管理：同时只能执行一个任务
 * - 流式事件处理：记录 AI 执行日志
 * - 日志持久化：保存到后端数据库
 */

export const useTaskExecutionStore = defineStore('taskExecution', () => {
  // ==================== State ====================

  // 任务执行状态映射 (taskId -> TaskExecutionState)
  const executionStates = ref<Map<string, TaskExecutionState>>(new Map())

  // 执行队列映射 (planId -> ExecutionQueue)
  const executionQueues = ref<Map<string, ExecutionQueue>>(new Map())

  // 当前查看的任务 ID（用于右侧面板显示日志）
  const currentViewingTaskId = ref<string | null>(null)

  // 按任务记录用户主动停止请求，避免不同计划/任务串状态
  const stopRequestedTaskIds = ref<Set<string>>(new Set())

  // 待持久化的日志缓冲区 (taskId -> PendingLogBuffer)
  const pendingLogBuffers = ref<Map<string, PendingLogBuffer>>(new Map())

  // ==================== Getters ====================

  /**
   * 获取任务的执行状态
   */
  const getExecutionState = computed(() => {
    return (taskId: string): TaskExecutionState | undefined => {
      return executionStates.value.get(taskId)
    }
  })

  /**
   * 判断任务是否正在执行（包括排队中）
   */
  const isTaskExecuting = computed(() => {
    return (taskId: string): boolean => {
      const state = executionStates.value.get(taskId)
      if (!state) return false
      return state.status === 'running' || state.status === 'queued'
    }
  })

  /**
   * 判断任务是否正在运行（不包括排队中）
   */
  const isTaskRunning = computed(() => {
    return (taskId: string): boolean => {
      const state = executionStates.value.get(taskId)
      return state?.status === 'running'
    }
  })

  /**
   * 获取计划的执行队列
   */
  const getExecutionQueue = computed(() => {
    return (planId: string): ExecutionQueue | undefined => {
      return executionQueues.value.get(planId)
    }
  })

  /**
   * 获取计划中正在执行的任务 ID
   */
  const getCurrentRunningTaskId = computed(() => {
    return (planId: string): string | null => {
      const queue = executionQueues.value.get(planId)
      return queue?.currentTaskId ?? null
    }
  })

  /**
   * 获取任务的排队位置（0 表示正在执行，-1 表示不在队列中）
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
   * 获取任务的执行日志
   */
  const getTaskLogs = computed(() => {
    return (taskId: string): ExecutionLogEntry[] => {
      const state = executionStates.value.get(taskId)
      return state?.logs ?? []
    }
  })

  // ==================== Actions ====================

  /**
   * 初始化任务执行状态
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

    await taskStore.updateTask(taskId, {
      status: 'in_progress',
      errorMessage: undefined,
      blockReason: undefined
    })
  }

  async function syncPlanRuntimeState(planId: string): Promise<void> {
    const planStore = usePlanStore()
    const plan = planStore.plans.find(item => item.id === planId)
    if (!plan) return

    const tasksInPlan = getPlanTasks(useTaskStore().tasks, planId)
    const queue = executionQueues.value.get(planId)
    const nextPlanState = computePlanRuntimeUpdate(plan, tasksInPlan, queue)

    if (nextPlanState) {
      await planStore.updatePlan(planId, nextPlanState)
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
   * 将任务加入执行队列
   */
  async function enqueueTask(planId: string, taskId: string): Promise<void> {
    await markTaskInProgress(taskId)

    // 确保队列存在
    let queue = executionQueues.value.get(planId)
    if (!queue) {
      queue = createExecutionQueue(planId)
      executionQueues.value.set(planId, queue)
    }

    // 初始化执行状态
    const state = initExecutionState(taskId)

    // 如果当前有任务在执行，加入排队
    if (queue.currentTaskId) {
      if (!queue.pendingTaskIds.includes(taskId)) {
        queue.pendingTaskIds.push(taskId)
        state.status = 'queued'
      }
      await syncPlanRuntimeState(planId)
      return
    }

    // 开始执行
    queue.currentTaskId = taskId
    await syncPlanRuntimeState(planId)
    await executeTask(planId, taskId)
  }

  /**
   * 执行任务
   */
  async function executeTask(planId: string, taskId: string): Promise<void> {
    const taskStore = useTaskStore()
    const planStore = usePlanStore()
    const projectStore = useProjectStore()
    const agentStore = useAgentStore()

    // 获取任务信息
    const task = taskStore.tasks.find(t => t.id === taskId)
    if (!task) {
      console.error('[TaskExecution] Task not found:', taskId)
      return
    }

    // 获取计划信息
    const plan = planStore.plans.find(p => p.id === planId)
    if (!plan) {
      console.error('[TaskExecution] Plan not found:', planId)
      return
    }

    // 获取项目路径
    const project = projectStore.projects.find(p => p.id === plan.projectId)
    const workingDirectory = project?.path

    // 获取最大重试次数（从计划配置读取）
    const maxRetries = plan.maxRetryCount ?? 3

    // 初始化执行状态
    const state = initExecutionState(taskId)
    clearTaskStopRequested(taskId, stopRequestedTaskIds.value)
    state.status = 'running'
    state.startedAt = new Date().toISOString()
    state.accumulatedContent = ''
    state.accumulatedThinking = ''
    state.toolCalls = []
    state.logs = []

    resetExecutionRuntime(taskId)

    // 添加系统日志
    await addExecutionLog({
      taskId,
      type: 'system',
      content: `开始执行任务: ${task.title}${task.retryCount > 0 ? ` (重试第 ${task.retryCount} 次)` : ''}`
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
        throw new Error('未找到可用的智能体')
      }

      const agent = {
        ...baseAgent,
        modelId: selection.modelId || baseAgent.modelId
      }

      // 检查策略支持
      if (!agentExecutor.isSupported(agent)) {
        throw new Error(`不支持的智能体类型: ${agent.type}`)
      }

      // 读取同计划最近任务结果，作为上下文注入
      const recentResults = await listRecentPlanResults(planId, 5)
      const planProgress = await getPlanExecutionProgress(planId)

      // 构建执行提示
      const prompt = buildExecutionPrompt(task, recentResults, planProgress)

      // 获取 MCP 配置（暂时使用空配置）
      const mcpServers: McpServerConfig[] = []

      // 构建对话上下文
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
        executionMode: 'chat',
        responseMode: 'stream_text'
      }

      // 执行对话
      await agentExecutor.execute(context, (event: StreamEvent) => {
        handleStreamEvent(taskId, event)
      })

      const fatalErrorLog = [...state.logs]
        .reverse()
        .find(log => log.type === 'error' && log.content.trim().length > 0)

      if (fatalErrorLog) {
        throw new Error(fatalErrorLog.content.trim())
      }

      // 执行完成
      state.status = 'completed'
      state.completedAt = new Date().toISOString()

      // 检查是否包含表单请求（AI 需要用户输入）
      const formRequest = parseFormRequest(state.accumulatedContent)
      if (formRequest) {
        // 任务需要等待用户输入
        await blockTaskForInput(taskId, formRequest)
        return
      }

      await addExecutionLog({
        taskId,
        type: 'system',
        content: '任务执行完成'
      })

      await persistTaskResult(taskId, 'success', '任务执行完成')

      // 更新任务状态为完成（状态更新失败不影响主流程）
      try {
        await taskStore.updateTask(taskId, { status: 'completed' })
      } catch (statusError) {
        console.warn('[TaskExecution] Failed to update task status to completed:', statusError)
      }

    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)

      // 检查是否是用户主动停止
      const wasStopped = isTaskStopRequested(taskId, stopRequestedTaskIds.value)

      if (wasStopped) {
        // 用户主动停止
        state.status = 'stopped'
        state.completedAt = new Date().toISOString()
        await addExecutionLog({
          taskId,
          type: 'system',
          content: '任务执行已停止'
        })
        // 更新任务状态为 pending
        try {
          await taskStore.updateTask(taskId, { status: 'pending' })
        } catch (statusError) {
          console.warn('[TaskExecution] Failed to update task status to pending:', statusError)
        }
      } else {
        // 执行失败 - 检查是否需要重试
        const currentRetryCount = task.retryCount + 1

        if (currentRetryCount < maxRetries) {
          // 还可以重试
          await addExecutionLog({
            taskId,
            type: 'system',
            content: `任务执行失败: ${errorMessage}，准备第 ${currentRetryCount + 1} 次重试...`
          })

          // 更新重试次数
          try {
            await taskStore.updateTask(taskId, {
              retryCount: currentRetryCount,
              errorMessage
            })
          } catch (statusError) {
            console.warn('[TaskExecution] Failed to update task retry count:', statusError)
          }

          await persistTaskResult(taskId, 'failed', `任务执行失败: ${errorMessage}`, errorMessage)

          // 重新加入队列执行（延迟一小段时间）
          state.status = 'queued'
          state.completedAt = new Date().toISOString()
          skipQueueAdvance = true

          // 使用 setTimeout 延迟重试，避免立即重试
          setTimeout(() => {
            void executeTask(planId, taskId)
          }, 1000)

          // 不处理队列中的下一个任务，因为当前任务会重试
          return
        } else {
          // 超过最大重试次数，标记为执行失败
          state.status = 'failed'
          state.completedAt = new Date().toISOString()

          await addExecutionLog({
            taskId,
            type: 'error',
            content: `任务执行失败（已重试 ${currentRetryCount} 次）: ${errorMessage}`
          })

          // 更新任务状态为 failed（执行失败）
          try {
            await taskStore.updateTask(taskId, {
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
      // 确保所有缓冲的日志都被持久化
      await flushPendingLogs(taskId)
      // 处理队列中下一个任务
      if (!skipQueueAdvance) {
        await processNextInQueue(planId)
      }
    }
  }

  /**
   * 处理流式事件
   */
  function handleStreamEvent(taskId: string, event: StreamEvent): void {
    const state = executionStates.value.get(taskId)
    if (!state) return

    switch (event.type) {
      case 'content':
        if (event.content) {
          state.accumulatedContent += event.content
          // 使用批量持久化机制
          addStreamLogToBuffer(taskId, 'content', event.content)
        }
        break

      case 'thinking':
        if (event.content) {
          state.accumulatedThinking += event.content
          // 使用批量持久化机制
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
              toolCallId: event.toolCallId
            }
          })
        }
        break

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
    }
  }

  /**
   * 初始化待持久化缓冲区
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
   * 清除计划的执行结果（同时清除关联任务的日志）
   */
  async function clearPlanExecutionResults(planId: string): Promise<number> {
    try {
      const deletedCount = await clearPlanExecutionResultsFromBackend(planId)

      // 清除内存中相关任务的执行状态
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

        // 清除缓冲区
        clearPendingBuffer(task.id)
      }

      return deletedCount
    } catch (error) {
      console.warn('[TaskExecution] Failed to clear plan execution results:', error)
      throw error
    }
  }

  /**
   * 提交任务输入并恢复执行
   */
  async function submitTaskInput(taskId: string, values: Record<string, unknown>): Promise<void> {
    const taskStore = useTaskStore()
    const task = taskStore.tasks.find(t => t.id === taskId)
    if (!task || task.status !== 'blocked') {
      console.warn('[TaskExecution] Cannot submit input: task not blocked')
      return
    }

    // 保存用户输入
    await taskStore.updateTask(taskId, {
      inputResponse: values,
      status: 'pending',
      blockReason: undefined,
      inputRequest: undefined
    })

    // 添加日志
    await addExecutionLog({
      taskId,
      type: 'system',
      content: `用户已提交输入: ${JSON.stringify(values)}`
    })

    // 重新加入执行队列
    await enqueueTask(task.planId, taskId)
  }

  /**
   * 跳过阻塞的任务
   */
  async function skipBlockedTask(taskId: string): Promise<void> {
    const taskStore = useTaskStore()
    const task = taskStore.tasks.find(t => t.id === taskId)
    if (!task || task.status !== 'blocked') {
      console.warn('[TaskExecution] Cannot skip: task not blocked')
      return
    }

    // 更新任务状态为 pending
    await taskStore.updateTask(taskId, {
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
   * 阻塞任务以等待用户输入
   */
  async function blockTaskForInput(taskId: string, request: AIFormRequest): Promise<void> {
    const taskStore = useTaskStore()
    const state = executionStates.value.get(taskId)
    const task = taskStore.tasks.find(item => item.id === taskId)
    const inputRequest = buildTaskInputRequest(request)

    // 更新任务状态
    await taskStore.updateTask(taskId, {
      status: 'blocked',
      blockReason: 'waiting_input',
      inputRequest
    })

    // 更新执行状态
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
   * 处理队列中下一个任务
   */
  async function processNextInQueue(planId: string): Promise<void> {
    const taskStore = useTaskStore()
    const queue = executionQueues.value.get(planId)
    if (!queue) return

    // 清除当前任务
    queue.currentTaskId = null

    // 自动补充依赖已满足的待办任务
    await seedReadyPendingTasks(planId)

    // 查找下一个可执行任务
    const nextTaskId = findNextExecutableTask(taskStore.tasks, queue.pendingTaskIds, taskStore.areDependenciesMet)

    if (nextTaskId) {
      // 从队列中移除
      const index = queue.pendingTaskIds.indexOf(nextTaskId)
      if (index >= 0) {
        queue.pendingTaskIds.splice(index, 1)
      }

      queue.currentTaskId = nextTaskId

      // 更新排队状态
      const state = executionStates.value.get(nextTaskId)
      if (state && state.status === 'queued') {
        state.status = 'running'
      }

      // 执行下一个任务
      await executeTask(planId, nextTaskId)
      return
    }

    await syncPlanRuntimeState(planId)
  }

  /**
   * 查找下一个可执行任务
   * 跳过阻塞任务，检查依赖关系
   */
  /**
   * 停止任务执行
   */
  async function stopTaskExecution(taskId: string): Promise<void> {
    const state = executionStates.value.get(taskId)
    if (!state) return

    // 使用正确的 sessionId 格式调用 agentExecutor.abort
    // 这会同时中止前端状态和通知后端停止执行
    const sessionId = `task-${taskId}`
    markTaskStopRequested(taskId, stopRequestedTaskIds.value)
    agentExecutor.abort(sessionId)

    // 更新状态
    state.status = 'stopped'
    state.completedAt = new Date().toISOString()

    // 先刷新缓冲区中的日志
    await flushPendingLogs(taskId)

    // 添加停止日志
    await addExecutionLog({
      taskId,
      type: 'system',
      content: '任务执行已停止'
    })

    // 更新任务状态
    const taskStore = useTaskStore()
    try {
      await taskStore.updateTask(taskId, { status: 'pending' })
    } catch (statusError) {
      console.warn('[TaskExecution] Failed to update task status while stopping:', statusError)
    }

    // 从队列中移除
    const task = taskStore.tasks.find(t => t.id === taskId)
    if (task) {
      const queue = executionQueues.value.get(task.planId)
      if (queue) {
        if (queue.currentTaskId === taskId) {
          queue.currentTaskId = null
          // 处理下一个任务
          await processNextInQueue(task.planId)
        } else {
          // 从排队中移除
          const index = queue.pendingTaskIds.indexOf(taskId)
          if (index >= 0) {
            queue.pendingTaskIds.splice(index, 1)
          }
        }
      }

      await syncPlanRuntimeState(task.planId)
    }
  }

  /**
   * 设置当前查看的任务
   */
  function setCurrentViewingTask(taskId: string | null): void {
    currentViewingTaskId.value = taskId
  }

  /**
   * 加载任务历史日志
   * 如果任务正在执行中，不会覆盖内存中的日志（因为内存中的日志是最新的）
   */
  async function loadTaskLogs(taskId: string): Promise<void> {
    try {
      // 检查任务是否正在执行中
      const existingState = executionStates.value.get(taskId)
      if (!canHydrateTaskLogs(existingState)) {
        // 任务正在执行中，内存中的日志是最新的，不需要从后端加载
        return
      }

      const rustLogs = await loadTaskLogsFromBackend(taskId)

      const state = initExecutionState(taskId)

      // 如果已有日志且数量大于等于后端返回的日志数量，不覆盖（可能内存中更新）
      if (shouldKeepInMemoryLogs(existingState, rustLogs.length)) {
        return
      }

      const task = useTaskStore().tasks.find(item => item.id === taskId)
      hydrateTaskLogs(state, rustLogs, task)

    } catch (error) {
      console.warn('[TaskExecution] Failed to load logs:', error)
    }
  }

  /**
   * 清除任务的执行日志
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
   * 清除计划的执行状态
   */
  function clearPlanExecution(planId: string): void {
    clearPlanExecutionRuntime(planId, executionQueues.value, executionStates.value)
  }

  function clearPlansExecution(planIds: string[]): void {
    planIds.forEach((planId) => {
      clearPlanExecutionRuntime(planId, executionQueues.value, executionStates.value)
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
    getExecutionQueue,
    getCurrentRunningTaskId,
    getQueuePosition,
    getTaskLogs,

    // Actions
    initExecutionState,
    enqueueTask,
    executeTask,
    stopTaskExecution,
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
