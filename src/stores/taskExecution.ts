import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  ExecutionLogEntry,
  ExecutionLogMetadata,
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
import { useAgentStore, type AgentConfig, inferAgentProvider } from '@/stores/agent'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import { useSettingsStore } from '@/stores/settings'
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
import { ACTIVE_EXECUTION_STATUSES } from './taskExecutionShared'
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
  updateExecutionLog,
} from './taskExecutionPersistence'
import {
  clearPlanExecutionRuntime,
  computePlanRuntimeUpdate,
  findNextExecutableTask,
  getPlanTasks,
  resetExecutionStateRuntime
} from './taskExecutionPlanRuntime'
import {
  buildTaskInputRequest,
  clearTaskStopRequested,
  hydrateTaskLogs,
  isTaskStopRequested,
  markTaskStopRequested,
  shouldKeepInMemoryLogs
} from './taskExecutionTaskRuntime'
import { loadAgentMcpServers } from '@/utils/mcpServerConfig'
import { mergeToolInputArguments } from '@/utils/toolInput'
import { getErrorMessage } from '@/utils/api'
import {
  buildContextStrategyNotice,
  buildUsageNotice,
  formatRuntimeNoticeAsSystemContent
} from '@/utils/runtimeNotice'
import {
  isCumulativeUsageRuntime,
  normalizeRuntimeUsage,
  type UsageBaseline
} from '@/utils/runtimeUsage'
import { recordAgentCliUsageInBackground, resolveRecordedModelId } from '@/services/usage/agentCliUsageRecorder'
import {
  buildExpertSystemPrompt,
  resolveExpertById,
  resolveExpertRuntime
} from '@/services/agentTeams/runtime'
import {
  getTaskRuntimeBinding,
  resolveRuntimeBindingKey,
  upsertTaskRuntimeBinding
} from '@/services/conversation/runtimeBindings'
import { loadMountedMemoryPrompt } from '@/services/memory/mountedMemoryPrompt'
import { resolveUsageModelHint } from '@/services/conversation/usageModelHint'
import {
  classifyCliFailureFragments,
  createCliFailureFragment,
  type CliFailureMatch
} from '@/utils/cliFailureMonitor'

function finalizeRunningToolCalls(toolCalls: ToolCall[]): void {
  for (const toolCall of toolCalls) {
    if (toolCall.status === 'running') {
      toolCall.status = 'success'
    }
  }
}

const TASK_EXECUTION_STOPPED_ERROR = '__TASK_EXECUTION_STOPPED__'
const TASK_AUTO_RETRY_DELAY_MS = 10_000
const CLI_FAILURE_RETRY_DELAY_MS = 10_000

function isMissingRecordError(error: unknown): boolean {
  return /query returned no rows/i.test(getErrorMessage(error))
}

function createFallbackToolCallId(taskId: string): string {
  return `tool-${taskId}-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
}

function collectMountedMemoryLibraryIds(
  projectIds: string[] | undefined,
  planIds: string[] | undefined,
  taskIds: string[] | undefined
): string[] {
  return Array.from(
    new Set(
      [...(projectIds ?? []), ...(planIds ?? []), ...(taskIds ?? [])]
        .map((id) => id.trim())
        .filter(Boolean)
    )
  )
}

export const useTaskExecutionStore = defineStore('taskExecution', () => {
  // ==================== State ====================

  const executionStates = ref<Map<string, TaskExecutionState>>(new Map())

  const executionQueues = ref<Map<string, ExecutionQueue>>(new Map())

  const recentPlanResultsCache = ref<Map<string, {
    limit: number
    records: TaskExecutionResultRecord[]
  }>>(new Map())

  const currentViewingTaskId = ref<string | null>(null)

  const stopRequestedTaskIds = ref<Set<string>>(new Set())

  // 待持久化的日志缓冲区 (taskId -> PendingLogBuffer)
  const pendingLogBuffers = ref<Map<string, PendingLogBuffer>>(new Map())
  const pendingLogWrites = new Map<string, Set<Promise<unknown>>>()
  const usageBaselines = new Map<string, UsageBaseline>()

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

  function trackPendingLogWrite<T>(taskId: string, task: Promise<T>): Promise<T> {
    let tasks = pendingLogWrites.get(taskId)
    if (!tasks) {
      tasks = new Set()
      pendingLogWrites.set(taskId, tasks)
    }

    const tracked = task.finally(() => {
      const currentTasks = pendingLogWrites.get(taskId)
      currentTasks?.delete(tracked)
      if (currentTasks && currentTasks.size === 0) {
        pendingLogWrites.delete(taskId)
      }
    })

    tasks.add(tracked)
    return tracked
  }

  async function waitForPendingLogWrites(taskId: string): Promise<void> {
    const tasks = pendingLogWrites.get(taskId)
    if (!tasks || tasks.size === 0) {
      return
    }

    await Promise.allSettled(Array.from(tasks))
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

  function detectTaskCliFailure(
    taskId: string,
    runtimeLabel: string,
    options: {
      errorMessage?: string | null
      logStartIndex?: number
    } = {}
  ): CliFailureMatch | null {
    const state = executionStates.value.get(taskId)
    if (!state) {
      return null
    }

    const logStartIndex = options.logStartIndex ?? 0
    const fragments = [
      createCliFailureFragment('error', options.errorMessage),
      createCliFailureFragment('content', state.accumulatedContent),
      ...state.logs.slice(logStartIndex).flatMap((log) => [
        createCliFailureFragment(
          log.type === 'error' ? 'error' : log.type === 'tool_result' ? 'tool_result' : log.type === 'system' ? 'system' : 'content',
          log.content
        )
      ]),
      ...state.toolCalls.flatMap((toolCall) => [
        createCliFailureFragment('tool_result', typeof toolCall.result === 'string' ? toolCall.result : undefined),
        createCliFailureFragment('error', toolCall.errorMessage)
      ])
    ].filter((item): item is NonNullable<typeof item> => Boolean(item))

    return classifyCliFailureFragments(runtimeLabel, fragments)
  }

  function updateTaskTokenUsage(
    taskId: string,
    usage: Pick<StreamEvent, 'inputTokens' | 'outputTokens' | 'model'>,
    requestedModelId?: string | null,
    didResetUsageWindow: boolean = false
  ): void {
    const state = executionStates.value.get(taskId)
    if (!state) return

    const current = state.tokenUsage
    const nextInputTokens = current.inputTokens + (typeof usage.inputTokens === 'number'
      ? Math.max(0, usage.inputTokens)
      : 0)
    const nextOutputTokens = current.outputTokens + (typeof usage.outputTokens === 'number'
      ? Math.max(0, usage.outputTokens)
      : 0)
    const resolvedModel = resolveRecordedModelId({
      reportedModelId: usage.model || current.model,
      requestedModelId
    }) ?? usage.model ?? current.model

    state.tokenUsage = {
      inputTokens: nextInputTokens,
      outputTokens: nextOutputTokens,
      model: resolvedModel,
      resetCount: didResetUsageWindow ? current.resetCount + 1 : current.resetCount,
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

  function getOrderedInProgressTaskIds(planId: string): string[] {
    const taskStore = useTaskStore()
    return taskStore.tasks
      .filter(task => task.planId === planId && task.status === 'in_progress')
      .sort((a, b) => a.order - b.order)
      .map(task => task.id)
  }

  function getOrderedAutoExecutionTaskIds(planId: string): string[] {
    const taskStore = useTaskStore()
    return taskStore.tasks
      .filter(task => (
        task.planId === planId
        && (task.status === 'pending' || task.status === 'in_progress')
      ))
      .sort((a, b) => a.order - b.order)
      .map(task => task.id)
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

  function resetTaskExecutionState(taskId: string): void {
    const state = executionStates.value.get(taskId)
    if (!state) {
      return
    }

    finalizeRunningToolCalls(state.toolCalls)
    state.status = 'idle'
    state.sessionId = null
    state.completedAt = null
  }

  function syncExecutionStateWithTask(taskId: string): void {
    const taskStore = useTaskStore()
    const task = taskStore.tasks.find(item => item.id === taskId)
    const state = executionStates.value.get(taskId)
    if (!task || !state) {
      return
    }

    if (task.status === 'blocked' && task.blockReason === 'waiting_input') {
      state.status = 'waiting_input'
      return
    }

    if (task.status === 'completed') {
      state.status = 'completed'
      return
    }

    if (task.status === 'failed') {
      state.status = 'failed'
      return
    }

    if (task.status === 'cancelled') {
      state.status = 'stopped'
      return
    }

    if (task.status !== 'in_progress') {
      state.status = 'idle'
      return
    }

    const queue = executionQueues.value.get(task.planId)
    if (queue?.currentTaskId === taskId) {
      state.status = 'running'
      return
    }

    if (queue?.pendingTaskIds.includes(taskId)) {
      state.status = 'queued'
      return
    }

    if (state.status !== 'waiting_input') {
      state.status = 'stopped'
    }
  }

  function normalizePlanExecutionStates(planId: string): void {
    const taskStore = useTaskStore()
    taskStore.tasks
      .filter(task => task.planId === planId)
      .forEach(task => {
        syncExecutionStateWithTask(task.id)
      })
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
      if (lastLog?.type !== 'system' || lastLog.content !== '任务已停止') {
        await addExecutionLog({
          taskId,
          type: 'system',
          content: '任务已停止'
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

  /**
   */
  async function enqueueTask(planId: string, taskId: string): Promise<void> {
    const taskStore = useTaskStore()
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

    if (queue.currentTaskId || queue.pendingTaskIds.length > 0) {
      if (!queue.pendingTaskIds.includes(taskId)) {
        queue.pendingTaskIds.push(taskId)
        state.status = 'queued'
      }
      await syncPlanRuntimeState(planId)
      // 无当前运行任务但有排队任务时，触发队列处理以避免死锁
      if (!queue.currentTaskId) {
        void processNextInQueue(planId)
      }
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

    if (!taskStore.areDependenciesMet(taskId)) {
      queue.pendingTaskIds.push(taskId)
      state.status = 'queued'
      await syncPlanRuntimeState(planId)
      void processNextInQueue(planId)
      return
    }

    queue.currentTaskId = taskId
    state.status = 'running'
    await syncPlanRuntimeState(planId)
    void executeTask(planId, taskId, { resume: isResumingStoppedTask })
  }

  /**
   */
  async function executeTask(planId: string, taskId: string, options: { resume?: boolean } = {}): Promise<void> {
    const taskStore = useTaskStore()
    const planStore = usePlanStore()
    const projectStore = useProjectStore()
    const agentStore = useAgentStore()
    const agentTeamsStore = useAgentTeamsStore()
    const settingsStore = useSettingsStore()

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

    if (!projectStore.projects.some(p => p.id === plan.projectId)) {
      await projectStore.loadProjects()
    }

    // 获取项目路径
    const project = projectStore.projects.find(p => p.id === plan.projectId)
    const workingDirectory = project?.path

    const maxRetries = plan.maxRetryCount ?? 3
    const maxCliRetries = settingsStore.settings.cliFailureMaxRetries ?? 5

    const state = initExecutionState(taskId)
    const isResume = options.resume === true
    const executionSessionId = `task-${taskId}`
    const resumeContext = isResume ? buildResumeExecutionContext(state) : ''
    clearTaskStopRequested(taskId, stopRequestedTaskIds.value)
    state.status = 'running'
    state.sessionId = executionSessionId
    state.executionRunId = isResume && state.executionRunId
      ? state.executionRunId
      : crypto.randomUUID()
    state.startedAt = isResume ? (state.startedAt ?? new Date().toISOString()) : new Date().toISOString()
    state.completedAt = null

    if (!isResume) {
      state.accumulatedContent = ''
      state.accumulatedThinking = ''
      state.toolCalls = []
      state.logs = []
      resetExecutionRuntime(taskId)
      await clearTaskLogsFromBackend(taskId).catch((error) => {
        console.warn('[TaskExecution] Failed to clear previous execution logs:', error)
      })
    }

    await addExecutionLog({
      taskId,
      type: 'system',
      content: isResume
        ? `继续执行任务: ${task.title}`
        : `开始执行任务: ${task.title}${task.retryCount > 0 ? `（第 ${task.retryCount + 1} 次）` : ''}`
    })
    await syncPlanRuntimeState(planId)

    let skipQueueAdvance = false
    let usageRecorded = false
    let currentAgentForUsage: AgentConfig | null = null

    const recordTaskUsageOnce = () => {
      if (usageRecorded || !currentAgentForUsage || !state.executionRunId) {
        return
      }

      usageRecorded = true
      recordAgentCliUsageInBackground(currentAgentForUsage, {
        executionId: state.executionRunId,
        executionMode: 'task_execution',
        modelId: resolveRecordedModelId({
          reportedModelId: state.tokenUsage.model,
          requestedModelId: currentAgentForUsage.modelId
        }),
        projectId: plan.projectId,
        sessionId: null,
        taskId,
        inputTokens: state.tokenUsage.inputTokens,
        outputTokens: state.tokenUsage.outputTokens,
        occurredAt: state.completedAt || new Date().toISOString()
      })
    }

    try {
      await agentTeamsStore.loadExperts()

      const selectedExpert = resolveExpertById(task.expertId, agentTeamsStore.experts)
        || resolveExpertById(plan.splitExpertId, agentTeamsStore.experts)
        || agentTeamsStore.builtinDeveloperExpert
        || agentTeamsStore.enabledExperts.find(expert => expert.category === 'developer')
        || null

      const selection = resolvePlanTaskAgentSelection(
        {
          expert_id: task.expertId ?? null,
          agent_id: task.agentId ?? null,
          model_id: task.modelId ?? null
        },
        plan
      )

      const runtime = resolveExpertRuntime(
        selectedExpert,
        agentStore.agents,
        task.modelId || plan.splitModelId || undefined
      )

      const explicitlySelectedAgent = selection.agentId
        ? agentStore.agents.find(agent => agent.id === selection.agentId)
        : null

      const baseAgent = explicitlySelectedAgent
        || runtime?.agent
        || (plan.splitAgentId ? agentStore.agents.find(agent => agent.id === plan.splitAgentId) : null)
        || agentStore.agents[0]

      if (!baseAgent) {
        throw new Error('未找到可用执行运行时')
      }

      const usageModelHint = !baseAgent.modelId?.trim()
        ? await resolveUsageModelHint(baseAgent).catch((error) => {
            console.warn('[TaskExecution] Failed to resolve usage model hint:', error)
            return undefined
          })
        : undefined

      const agent = {
        ...baseAgent,
        modelId: runtime?.modelId || selection.modelId || baseAgent.modelId || usageModelHint
      }
      currentAgentForUsage = agent
      if (agent.modelId?.trim()) {
        updateTaskTokenUsage(taskId, { model: agent.modelId }, agent.modelId)
      }
      const agentProvider = inferAgentProvider(agent)
      const runtimeKey = resolveRuntimeBindingKey(agent)
      const runtimeBinding = isResume && runtimeKey
        ? await getTaskRuntimeBinding(taskId, runtimeKey).catch((error) => {
            console.warn('[TaskExecution] Failed to load task runtime binding:', error)
            return null
          })
        : null
      const resumableExternalSessionId = isResume
        ? runtimeBinding?.externalSessionId?.trim()
          || (
            agentProvider
            && (!task.cliSessionProvider || task.cliSessionProvider === agentProvider)
              ? task.sessionId?.trim()
              : ''
          )
          || undefined
        : undefined

      if (!agentExecutor.isSupported(agent)) {
        throw new Error(`当前不支持该智能体类型: ${agent.type}`)
      }

      const recentResults = await listRecentPlanResults(planId, 5)
      const planProgress = await getPlanExecutionProgress(planId)
      const mountedMemoryPrompt = await loadMountedMemoryPrompt(
        collectMountedMemoryLibraryIds(
          project?.memoryLibraryIds,
          plan.memoryLibraryIds,
          task.memoryLibraryIds
        )
      )

      const prompt = resumableExternalSessionId
        ? '继续'
        : buildExecutionPrompt(task, recentResults, planProgress, resumeContext || undefined)

      const mcpServers = await loadAgentMcpServers(agent).catch((error) => {
        console.warn('[TaskExecution] Failed to load MCP servers:', error)
        return [] as McpServerConfig[]
      })

      // 构建对话上下文
      const context: ConversationContext = {
        sessionId: executionSessionId,
        agent,
        messages: [
          ...(!resumableExternalSessionId && mountedMemoryPrompt
            ? [{
                id: `task-memory-${taskId}`,
                sessionId: executionSessionId,
                role: 'system' as const,
                content: mountedMemoryPrompt,
                status: 'completed' as const,
                createdAt: new Date().toISOString()
              }]
            : []),
          ...(!resumableExternalSessionId && selectedExpert?.prompt
            ? [{
                id: `task-system-${taskId}`,
                sessionId: executionSessionId,
                role: 'system' as const,
                content: buildExpertSystemPrompt(selectedExpert.prompt),
                status: 'completed' as const,
                createdAt: new Date().toISOString()
              }]
            : []),
          {
            id: `task-prompt-${taskId}`,
            sessionId: executionSessionId,
            role: 'user',
            content: prompt,
            status: 'completed',
            createdAt: new Date().toISOString()
          }
        ],
        workingDirectory,
        mcpServers: mcpServers.length > 0 ? mcpServers : undefined,
        executionMode: 'task_execution',
        responseMode: 'stream_text',
        resumeSessionId: resumableExternalSessionId
      }

      if (!resumableExternalSessionId || !isCumulativeUsageRuntime(agentProvider)) {
        usageBaselines.delete(taskId)
      }

      const contextNotice = buildContextStrategyNotice({
        strategy: resumableExternalSessionId ? 'Task Resume Prompt' : 'Task Execution Prompt',
        runtime: inferAgentProvider(agent)?.toUpperCase() || agent.type,
        model: agent.modelId,
        expert: selectedExpert?.name || task.expertId || plan.splitExpertId,
        systemMessageCount: context.messages.filter(message => message.role === 'system').length,
        userMessageCount: context.messages.filter(message => message.role === 'user').length,
        assistantMessageCount: context.messages.filter(message => message.role === 'assistant').length,
        historyMessageCount: isResume ? Math.min(state.logs.length, 50) : 0,
        resumeSessionId: resumableExternalSessionId
      })

      if (contextNotice) {
        await addExecutionLog({
          taskId,
          type: 'system',
          content: formatRuntimeNoticeAsSystemContent(contextNotice),
          metadata: {
            strategy: resumableExternalSessionId ? 'task_resume' : 'task_execution',
            runtime: inferAgentProvider(agent)?.toUpperCase() || agent.type,
            model: agent.modelId,
            expert: selectedExpert?.name || task.expertId || plan.splitExpertId,
            externalSessionId: resumableExternalSessionId,
            systemMessageCount: context.messages.filter(message => message.role === 'system').length,
            userMessageCount: context.messages.filter(message => message.role === 'user').length,
            assistantMessageCount: context.messages.filter(message => message.role === 'assistant').length,
            historyMessageCount: isResume ? Math.min(state.logs.length, 50) : 0
          }
        })
      }

      if (selectedExpert && (task.expertId !== selectedExpert.id || task.agentId !== agent.id || task.modelId !== agent.modelId)) {
        await updateTaskSafely(taskId, {
          expertId: selectedExpert.id,
          agentId: agent.id,
          modelId: agent.modelId || undefined
        })
      }

      let cliRetryCount = 0
      let cliRetryLogId: string | null = null
      const runtimeLabel = agentProvider?.toUpperCase() || agent.type
      let latestAttemptLogStartIndex = state.logs.length

      while (true) {
        const logStartIndex = state.logs.length
        latestAttemptLogStartIndex = logStartIndex
        state.accumulatedContent = ''
        state.accumulatedThinking = ''
        state.toolCalls = []

        try {
          await agentExecutor.execute(context, (event: StreamEvent) => {
            const externalSessionId = event.externalSessionId?.trim()
            if (externalSessionId && agentProvider) {
              void (async () => {
                try {
                  if (runtimeKey) {
                    await upsertTaskRuntimeBinding(taskId, runtimeKey, externalSessionId)
                  }
                  await updateTaskSafely(taskId, {
                    sessionId: externalSessionId,
                    cliSessionProvider: agentProvider
                  })
                } catch (error) {
                  console.warn('[TaskExecution] Failed to persist external session binding:', error)
                }
              })()
            }
            handleStreamEvent(taskId, event, agent.modelId, agentProvider)
          })

          await flushPendingLogs(taskId)
          await waitForPendingLogWrites(taskId)

          const abnormalCompletion = detectTaskCliFailure(taskId, runtimeLabel, { logStartIndex })
          if (!abnormalCompletion) {
            break
          }

          throw new Error(abnormalCompletion.message)
        } catch (error) {
          await flushPendingLogs(taskId)
          await waitForPendingLogWrites(taskId)

          const errorMessage = error instanceof Error ? error.message : String(error)
          const classifiedFailure = detectTaskCliFailure(taskId, runtimeLabel, {
            errorMessage,
            logStartIndex
          })

          if (errorMessage === TASK_EXECUTION_STOPPED_ERROR || isTaskStopRequested(taskId, stopRequestedTaskIds.value)) {
            throw error
          }

          if (!classifiedFailure || classifiedFailure.kind !== 'retryable' || cliRetryCount >= maxCliRetries) {
            throw error
          }

          cliRetryCount += 1
          cliRetryLogId = await upsertCliRetryProgressLog(
            taskId,
            cliRetryLogId,
            runtimeLabel,
            cliRetryCount,
            maxCliRetries
          )
          await sleep(CLI_FAILURE_RETRY_DELAY_MS)
        }
      }

      if (isTaskStopRequested(taskId, stopRequestedTaskIds.value)) {
        throw new Error(TASK_EXECUTION_STOPPED_ERROR)
      }

      const fatalErrorLog = [...state.logs]
        .slice(latestAttemptLogStartIndex)
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
        await appendExecutionUsageNotice(taskId)
        await blockTaskForInput(taskId, formRequest)
        recordTaskUsageOnce()
        return
      }

      await appendExecutionUsageNotice(taskId)
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
      recordTaskUsageOnce()

    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)

      const wasStopped = isTaskStopRequested(taskId, stopRequestedTaskIds.value)

      if (wasStopped) {
        await appendExecutionUsageNotice(taskId)
        await markTaskStopped(taskId)
        recordTaskUsageOnce()
      } else {
        const currentRetryCount = task.retryCount + 1

        if (currentRetryCount < maxRetries) {
          await appendExecutionUsageNotice(taskId)
          await addExecutionLog({
            taskId,
            type: 'system',
            content: `任务执行失败: ${errorMessage}，10 秒后准备第 ${currentRetryCount + 1} 次重试...`
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
          recordTaskUsageOnce()
          skipQueueAdvance = true

          // 使用 setTimeout 延迟重试，避免立即重入
          setTimeout(() => {
            void executeTask(planId, taskId)
          }, TASK_AUTO_RETRY_DELAY_MS)

          return
        } else {
          finalizeRunningToolCalls(state.toolCalls)
          state.status = 'failed'
          state.completedAt = new Date().toISOString()

          await appendExecutionUsageNotice(taskId)
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
          recordTaskUsageOnce()
        }
      }
    } finally {
      clearTaskStopRequested(taskId, stopRequestedTaskIds.value)
      await flushPendingLogs(taskId)
      await waitForPendingLogWrites(taskId)
      if (!skipQueueAdvance) {
        await processNextInQueue(planId)
      }
    }
  }

  /**
   */
  function handleStreamEvent(
    taskId: string,
    event: StreamEvent,
    requestedModelId?: string | null,
    runtimeProvider?: string | null
  ): void {
    const state = executionStates.value.get(taskId)
    if (!state) return
    const normalizedUsage = normalizeRuntimeUsage({
      provider: runtimeProvider,
      inputTokens: event.inputTokens,
      outputTokens: event.outputTokens,
      rawInputTokens: event.rawInputTokens,
      rawOutputTokens: event.rawOutputTokens,
      cacheReadInputTokens: event.cacheReadInputTokens,
      cacheCreationInputTokens: event.cacheCreationInputTokens,
      baseline: usageBaselines.get(taskId) ?? null
    })
    const normalizedEvent: StreamEvent = {
      ...event,
      inputTokens: normalizedUsage.inputTokens,
      outputTokens: normalizedUsage.outputTokens
    }

    if (normalizedUsage.nextBaseline) {
      usageBaselines.set(taskId, normalizedUsage.nextBaseline)
    } else {
      usageBaselines.delete(taskId)
    }

    if (normalizedEvent.inputTokens !== undefined || normalizedEvent.outputTokens !== undefined || normalizedEvent.model) {
      updateTaskTokenUsage(taskId, normalizedEvent, requestedModelId, normalizedUsage.didReset)
    }

    switch (normalizedEvent.type) {
      case 'content':
        if (normalizedEvent.content) {
          state.accumulatedContent += normalizedEvent.content
          addStreamLogToBuffer(taskId, 'content', normalizedEvent.content)
        }
        break

      case 'thinking_start':
        trackPendingLogWrite(taskId, addExecutionLog({
          taskId,
          type: 'thinking_start',
          content: ''
        }))
        break

      case 'thinking':
        if (normalizedEvent.content) {
          state.accumulatedThinking += normalizedEvent.content
          addStreamLogToBuffer(taskId, 'thinking', normalizedEvent.content)
        }
        break

      case 'tool_use':
        if (normalizedEvent.toolName) {
          const toolCallId = normalizedEvent.toolCallId || createFallbackToolCallId(taskId)
          const toolCall: ToolCall = {
            id: toolCallId,
            name: normalizedEvent.toolName,
            arguments: normalizedEvent.toolInput || {},
            status: 'running'
          }

          const existingIndex = state.toolCalls.findIndex(tc => tc.id === toolCall.id)
          if (existingIndex >= 0) {
            state.toolCalls[existingIndex] = toolCall
          } else {
            state.toolCalls.push(toolCall)
          }

          trackPendingLogWrite(taskId, addExecutionLog({
            taskId,
            type: 'tool_use',
            content: JSON.stringify(normalizedEvent.toolInput, null, 2),
            metadata: {
              toolName: normalizedEvent.toolName,
              toolCallId,
              toolInput: JSON.stringify(normalizedEvent.toolInput ?? {})
            }
          }))
        }
        break

      case 'tool_input_delta': {
        const targetToolCall = (normalizedEvent.toolCallId
          ? state.toolCalls.find(tool => tool.id === normalizedEvent.toolCallId)
          : null) || [...state.toolCalls].reverse().find(tool => tool.status === 'running')

        if (targetToolCall) {
          targetToolCall.arguments = mergeToolInputArguments(targetToolCall.arguments, normalizedEvent.toolInput)
        }

        if (normalizedEvent.toolInput) {
          trackPendingLogWrite(taskId, addExecutionLog({
            taskId,
            type: 'tool_input_delta',
            content: typeof normalizedEvent.toolInput.raw === 'string'
              ? normalizedEvent.toolInput.raw
              : JSON.stringify(normalizedEvent.toolInput),
            metadata: {
              toolCallId: normalizedEvent.toolCallId,
              toolInput: typeof normalizedEvent.toolInput.raw === 'string'
                ? normalizedEvent.toolInput.raw
                : JSON.stringify(normalizedEvent.toolInput)
            }
          }))
        }
        break
      }

      case 'tool_result':
        {
          const targetToolCall = (normalizedEvent.toolCallId
            ? state.toolCalls.find(tool => tool.id === normalizedEvent.toolCallId)
            : null) || [...state.toolCalls].reverse().find(tool => tool.status === 'running')
          const resolvedToolCallId = targetToolCall?.id || normalizedEvent.toolCallId
          if (!resolvedToolCallId) {
            break
          }
          const result = typeof normalizedEvent.toolResult === 'string'
            ? normalizedEvent.toolResult
            : JSON.stringify(normalizedEvent.toolResult, null, 2)
          const isError = false

          const tc = state.toolCalls.find(t => t.id === resolvedToolCallId)
          if (tc) {
            tc.result = result
            tc.status = isError ? 'error' : 'success'
            if (isError) {
              tc.errorMessage = result
            }
          }

          trackPendingLogWrite(taskId, addExecutionLog({
            taskId,
            type: 'tool_result',
            content: result,
            metadata: {
              toolCallId: resolvedToolCallId,
              toolResult: result,
              isError
            }
          }))
        }
        break

      case 'error':
        if (normalizedEvent.error) {
          trackPendingLogWrite(taskId, addExecutionLog({
            taskId,
            type: 'error',
            content: normalizedEvent.error
          }))
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
  async function addExecutionLog(input: CreateExecutionLogInput, persist: boolean = true): Promise<ExecutionLogEntry | null> {
    const state = executionStates.value.get(input.taskId)
    if (!state) return null

    const entry = persist
      ? await persistExecutionLog(input)
      : createExecutionLogEntry(input)

    state.logs.push(entry)
    return entry
  }

  async function upsertCliRetryProgressLog(
    taskId: string,
    existingLogId: string | null,
    runtimeLabel: string,
    retryCount: number,
    maxRetries: number
  ): Promise<string | null> {
    const state = executionStates.value.get(taskId)
    if (!state) {
      return existingLogId
    }

    const metadata: ExecutionLogMetadata = {
      retryGroup: 'cli_failure_retry',
      retryCount,
      maxRetries,
      retryDelaySeconds: CLI_FAILURE_RETRY_DELAY_MS / 1000,
      runtime: runtimeLabel
    }
    const content = `检测到可恢复的 ${runtimeLabel} CLI 异常，10 秒后开始第 ${retryCount}/${maxRetries} 次底层重试...`

    if (existingLogId) {
      const existingEntry = state.logs.find((log) => log.id === existingLogId)
      if (existingEntry) {
        existingEntry.content = content
        existingEntry.metadata = {
          ...existingEntry.metadata,
          ...metadata
        }
        await updateExecutionLog(existingLogId, content, existingEntry.metadata)
        return existingLogId
      }
    }

    const entry = await addExecutionLog({
      taskId,
      type: 'system',
      content,
      metadata
    })
    return entry?.id ?? existingLogId
  }

  async function appendExecutionUsageNotice(taskId: string): Promise<void> {
    const state = executionStates.value.get(taskId)
    if (!state) return

    const usageNotice = buildUsageNotice({
      model: state.tokenUsage.model,
      inputTokens: state.tokenUsage.inputTokens,
      outputTokens: state.tokenUsage.outputTokens
    })
    const content = formatRuntimeNoticeAsSystemContent(usageNotice)
    if (!content) {
      return
    }

    const lastLog = state.logs[state.logs.length - 1]
    if (lastLog?.type === 'system' && lastLog.content === content) {
      return
    }

    await addExecutionLog({
      taskId,
      type: 'system',
      content,
      metadata: {
        model: state.tokenUsage.model,
        inputTokens: state.tokenUsage.inputTokens,
        outputTokens: state.tokenUsage.outputTokens
      }
    })
  }

  async function saveTaskExecutionResult(input: SaveTaskExecutionResultInput): Promise<void> {
    await saveTaskExecutionResultToBackend(input)
    const taskStore = useTaskStore()
    const task = taskStore.tasks.find(item => item.id === input.task_id)
    if (task) {
      recentPlanResultsCache.value.delete(task.planId)
    }
  }

  async function listRecentPlanResults(planId: string, limit: number = 5): Promise<TaskExecutionResultRecord[]> {
    const cached = recentPlanResultsCache.value.get(planId)
    if (cached && cached.limit >= limit) {
      return cached.records.slice(0, limit)
    }

    const records = await listRecentPlanResultsFromBackend(planId, limit)
    recentPlanResultsCache.value.set(planId, { limit, records })
    return records
  }

  async function getPlanExecutionProgress(planId: string): Promise<PlanExecutionProgress | null> {
    return getPlanExecutionProgressFromBackend(planId)
  }

  /**
   */
  async function clearPlanExecutionResults(planId: string): Promise<number> {
    try {
      const deletedCount = await clearPlanExecutionResultsFromBackend(planId)
      recentPlanResultsCache.value.delete(planId)

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
      content: `用户已提交输入: ${JSON.stringify(values)}`
    })

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
      normalizePlanExecutionStates(planId)
      await syncPlanRuntimeState(planId)
      return
    }

    const nextTaskId = findNextExecutableTask(taskStore.tasks, queue.pendingTaskIds, taskStore.areDependenciesMet)

    if (nextTaskId) {
      removeTaskFromQueue(queue, nextTaskId)
      await markTaskInProgress(nextTaskId)
      queue.currentTaskId = nextTaskId
      queue.lastInterruptedTaskId = null

      const state = executionStates.value.get(nextTaskId)
      if (state && state.status === 'queued') {
        state.status = 'running'
      }

      await syncPlanRuntimeState(planId)
      void executeTask(planId, nextTaskId)
      return
    }

    normalizePlanExecutionStates(planId)
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
      removeTaskFromQueue(queue, taskId)
      normalizePlanExecutionStates(task.planId)

      if (autoAdvance && !pauseQueue) {
        await processNextInQueue(task.planId)
        return
      }

      await syncPlanRuntimeState(task.planId)
      return
    }

    agentExecutor.abort(sessionId)
    removeTaskFromQueue(queue, taskId)
    if (!pauseQueue && queue.lastInterruptedTaskId === taskId) {
      queue.lastInterruptedTaskId = null
    }
    await markTaskStopped(taskId)
    normalizePlanExecutionStates(task.planId)

    if (autoAdvance && !pauseQueue) {
      await processNextInQueue(task.planId)
      return
    }

    await syncPlanRuntimeState(task.planId)
  }

  async function pausePlanExecutionFlow(planId: string): Promise<void> {
    const queue = getOrCreateQueue(planId)
    queue.isPaused = true

    // 1. 停止当前运行中的任务
    if (queue.currentTaskId) {
      queue.lastInterruptedTaskId = queue.currentTaskId
      await stopTaskExecution(queue.currentTaskId, {
        pauseQueue: true,
        autoAdvance: false
      })
    }

    // 2. 将排队中的任务状态改回 pending，并清空队列
    const queuedTaskIds = [...queue.pendingTaskIds]
    for (const taskId of queuedTaskIds) {
      await updateTaskSafely(taskId, { status: 'pending' })
      const state = executionStates.value.get(taskId)
      if (state) {
        state.status = 'idle'
        state.completedAt = null
      }
    }
    queue.pendingTaskIds = []

    await syncPlanRuntimeState(planId)
  }

  async function resumePlanExecutionFlow(planId: string): Promise<void> {
    const queue = getOrCreateQueue(planId)
    const currentTaskId = queue.currentTaskId
    const currentState = currentTaskId
      ? executionStates.value.get(currentTaskId)
      : undefined

    if (currentTaskId && currentState?.status === 'running') {
      await syncPlanRuntimeState(planId)
      return
    }

    queue.isPaused = false
    queue.currentTaskId = null
    queue.pendingTaskIds = []
    queue.lastInterruptedTaskId = null

    const candidateTaskIds = getOrderedAutoExecutionTaskIds(planId)
    if (candidateTaskIds.length === 0) {
      normalizePlanExecutionStates(planId)
      await syncPlanRuntimeState(planId)
      return
    }

    normalizePlanExecutionStates(planId)

    for (const taskId of candidateTaskIds) {
      await enqueueTask(planId, taskId)
    }
  }

  async function detachTaskFromExecution(taskId: string): Promise<void> {
    const taskStore = useTaskStore()
    const task = taskStore.tasks.find(item => item.id === taskId)
    if (!task) {
      return
    }

    const queue = executionQueues.value.get(task.planId)
    if (queue) {
      removeTaskFromQueue(queue, taskId)
      if (queue.lastInterruptedTaskId === taskId) {
        queue.lastInterruptedTaskId = null
      }
    }

    clearTaskStopRequested(taskId, stopRequestedTaskIds.value)
    resetTaskExecutionState(taskId)

    normalizePlanExecutionStates(task.planId)
    await syncPlanRuntimeState(task.planId)
  }

  async function synchronizePlanExecutionQueue(planId: string): Promise<void> {
    const queue = getOrCreateQueue(planId)
    const orderedTaskIds = getOrderedInProgressTaskIds(planId)
    const orderedTaskIdSet = new Set(orderedTaskIds)

    if (queue.currentTaskId && !orderedTaskIdSet.has(queue.currentTaskId)) {
      queue.currentTaskId = null
    }

    queue.pendingTaskIds = orderedTaskIds.filter(taskId => taskId !== queue.currentTaskId)

    if (queue.lastInterruptedTaskId && !orderedTaskIdSet.has(queue.lastInterruptedTaskId)) {
      queue.lastInterruptedTaskId = null
    }

    queue.pendingTaskIds.forEach((taskId) => {
      const state = initExecutionState(taskId)
      if (state.status !== 'running') {
        state.status = 'queued'
      }
    })

    normalizePlanExecutionStates(planId)
    await syncPlanRuntimeState(planId)
  }

  async function startTaskExecution(taskId: string): Promise<void> {
    const taskStore = useTaskStore()
    const planStore = usePlanStore()
    const task = taskStore.tasks.find(item => item.id === taskId)
    if (!task) {
      return
    }

    if (!taskStore.areDependenciesMet(taskId)) {
      const dependencyNames = taskStore.getUnmetDependencyTitles(taskId)
      const detail = dependencyNames.length > 0
        ? `依赖任务未完成: ${dependencyNames.join('、')}`
        : '依赖任务未完成'
      throw new Error(detail)
    }

    if (task.status === 'blocked' && task.blockReason === 'waiting_input') {
      throw new Error('当前任务正在等待输入，无法直接开始')
    }

    const queue = getOrCreateQueue(task.planId)
    queue.isPaused = false
    if (queue.lastInterruptedTaskId === taskId) {
      queue.lastInterruptedTaskId = null
    }

    await planStore.startPlanExecution(task.planId)

    if (task.status === 'in_progress' && isTaskStopped.value(taskId)) {
      await resumeTaskExecution(taskId)
    } else {
      await enqueueTask(task.planId, taskId)
    }

    await synchronizePlanExecutionQueue(task.planId)
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
      // 运行中/排队中的任务且内存中已有日志时，跳过后端加载（避免覆盖实时数据）
      // 但如果内存中日志为空（例如视图切换后回显场景），仍从后端加载
      if (existingState && ACTIVE_EXECUTION_STATUSES.has(existingState.status) && existingState.logs.length > 0) {
        return
      }

      const rustLogs = await loadTaskLogsFromBackend(taskId)

      const state = initExecutionState(taskId)

      // 如果已有日志且数量大于等于后端返回的日志数量，不覆盖（可能内存中已更新）
      if (shouldKeepInMemoryLogs(existingState, rustLogs.length)) {
        return
      }

      const task = useTaskStore().tasks.find(item => item.id === taskId)
      hydrateTaskLogs(state, rustLogs, task)
      if (task) {
        syncExecutionStateWithTask(task.id)
      }

    } catch (error) {
      console.warn('[TaskExecution] Failed to load logs:', error)
    }
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
    startTaskExecution,
    pausePlanExecutionFlow,
    resumePlanExecutionFlow,
    resumeTaskExecution,
    detachTaskFromExecution,
    synchronizePlanExecutionQueue,
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
