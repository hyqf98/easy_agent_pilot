import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { inferAgentProvider, useAgentStore, type AgentConfig } from './agent'
import { useAgentTeamsStore } from './agentTeams'
import { usePlanStore } from './plan'
import { logger } from '@/utils/logger'
import { normalizeFormSchemaForRendering, normalizeFormSchemasForRendering } from '@/utils/formSchema'
import {
  buildExpertCatalogPrompt,
  buildExpertSystemPrompt,
  resolveExpertById
} from '@/services/agentTeams/runtime'
import {
  buildPlanSplitJsonSchema,
  buildPlanSplitKickoffPrompt,
  buildPlanSplitSystemPrompt,
  buildTaskListOptimizeKickoffPrompt,
  buildTaskResplitKickoffPrompt
} from '@/services/plan'
import { resolveUsageModelHint } from '@/services/conversation/usageModelHint'
import type { ExecutionRequest, MessageInput } from '@/services/conversation/strategies/types'
import { buildAgentExecutionRequest } from '@/services/conversation/runtimeProfiles'
import type { RuntimeNotice } from '@/utils/runtimeNotice'
import { buildCliEnvironmentNotice, buildContextStrategyNotice } from '@/utils/runtimeNotice'
import { loadAgentMcpServers } from '@/utils/mcpServerConfig'
import {
  findLatestUsageSnapshot,
  recordAgentCliUsageInBackground,
  resolveRecordedModelId
} from '@/services/usage/agentCliUsageRecorder'
import type {
  AITaskItem,
  DynamicFormSchema,
  PlanSplitLogRecord,
  PlanSplitSessionRecord,
  PlanSplitStreamPayload,
  SplitMessage,
  TaskCountMode,
  TaskListOptimizeConfig,
  TaskSplitRefinementMode,
  TaskResplitConfig
} from '@/types/plan'

interface TaskSplitContext {
  planId: string
  planName: string
  planDescription?: string
  granularity: number
  expertId?: string
  agentId: string
  modelId: string
  taskCountMode?: TaskCountMode
  workingDirectory?: string
}

interface TaskSplitRefinementState {
  mode: TaskSplitRefinementMode
  originalTasks: AITaskItem[]
  targetIndex: number | null
  config: TaskResplitConfig | TaskListOptimizeConfig
}

interface SubmittedFormSnapshot {
  formId: string
  schema: DynamicFormSchema
  promptText?: string
  values: Record<string, unknown>
  submittedAt: string
}

interface PlanSplitRuntimeMetrics {
  startedAt: number
  firstEventAt?: number
  firstRenderableAt?: number
  doneAt?: number
}

const STALE_PLAN_SPLIT_SESSION_TIMEOUT_MS = 2_000

function formatOptionValue(field: DynamicFormSchema['fields'][number], value: unknown): string {
  if (value === undefined || value === null || value === '') return '-'
  if (Array.isArray(value)) {
    if (value.length === 0) return '-'
    return value.map(item => formatOptionValue(field, item)).join('、')
  }
  const matched = field.options?.find(option => option.value === value)
  if (matched) return matched.label
  return String(value)
}

function summarizeFormValues(schema: DynamicFormSchema, values: Record<string, unknown>): string {
  return schema.fields
    .map(field => `${field.label}：${formatOptionValue(field, values[field.name])}`)
    .join('\n')
}

function parseJson<T>(raw?: string | null, fallback?: T): T {
  if (!raw) return fallback as T
  try {
    return JSON.parse(raw) as T
  } catch (error) {
    logger.warn('[TaskSplit] JSON parse failed:', error)
    return fallback as T
  }
}

function parseStreamPayloadMetadata(raw?: string | null): Record<string, unknown> | null {
  if (!raw) {
    return null
  }

  try {
    const parsed = JSON.parse(raw) as unknown
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
      ? parsed as Record<string, unknown>
      : null
  } catch {
    return null
  }
}

function buildPersistedLogMetadata(
  payload: PlanSplitStreamPayload,
  parsedMetadata: Record<string, unknown> | null
): string {
  const model = typeof payload.model === 'string' && payload.model.trim()
    ? payload.model.trim()
    : typeof parsedMetadata?.model === 'string' && parsedMetadata.model.trim()
      ? parsedMetadata.model.trim()
      : undefined
  const inputTokens = typeof payload.inputTokens === 'number'
    ? payload.inputTokens
    : typeof parsedMetadata?.inputTokens === 'number'
      ? parsedMetadata.inputTokens
      : undefined
  const outputTokens = typeof payload.outputTokens === 'number'
    ? payload.outputTokens
    : typeof parsedMetadata?.outputTokens === 'number'
      ? parsedMetadata.outputTokens
      : undefined

  return JSON.stringify({
    model,
    inputTokens,
    outputTokens,
    ...(payload.metadata ? { rawMetadata: payload.metadata } : {}),
    toolName: payload.toolName,
    toolCallId: payload.toolCallId,
    toolInput: payload.toolInput,
    toolResult: payload.toolResult
  })
}

function buildExecutionRequest(
  context: TaskSplitContext,
  agent: AgentConfig,
  llmMessages: MessageInput[],
  mcpServers: ExecutionRequest['mcpServers']
): ExecutionRequest {
  const provider = (agent.provider || 'claude').toLowerCase() === 'codex' ? 'codex' : 'claude'
  return buildAgentExecutionRequest({
    sessionId: crypto.randomUUID(),
    planId: context.planId,
    agent,
    messages: llmMessages,
    modelId: context.modelId || undefined,
    workingDirectory: context.workingDirectory,
    systemPrompt: llmMessages.find(message => message.role === 'system')?.content,
    mcpServers,
    cliOutputFormat: 'stream-json',
    jsonSchema: agent.type === 'cli'
      ? buildPlanSplitJsonSchema(context.granularity, provider, context.taskCountMode ?? 'min')
      : undefined,
    executionMode: 'task_split',
    responseMode: 'stream_text'
  })
}

function toSplitMessages(raw?: string | null): SplitMessage[] {
  const parsed = parseJson<Array<{ id: string; role: 'user' | 'assistant'; content: string; timestamp: string }>>(raw, [])
  return parsed.map(message => ({
    id: message.id,
    role: message.role,
    content: message.content,
    timestamp: message.timestamp
  }))
}

function toFormQueue(raw?: string | null): DynamicFormSchema[] {
  const forms = parseJson<DynamicFormSchema[]>(raw, [])
  return normalizeFormSchemasForRendering(forms)
}

function toSplitResult(raw?: string | null): AITaskItem[] | null {
  if (!raw) return null
  const parsed = parseJson<{ tasks?: AITaskItem[] }>(raw, {})
  return parsed.tasks ?? null
}

function toPlanSplitLogs(logs: PlanSplitLogRecord[]): PlanSplitLogRecord[] {
  return logs.map(log => ({
    ...log,
    type: log.type
  }))
}

function trimTrailingAssistantMessages<T extends { role: string }>(items: T[]): T[] {
  const lastUserIndex = [...items].map(item => item.role).lastIndexOf('user')
  if (lastUserIndex < 0) {
    return [...items]
  }

  let endIndex = items.length
  while (endIndex > lastUserIndex + 1 && items[endIndex - 1]?.role === 'assistant') {
    endIndex -= 1
  }

  return items.slice(0, endIndex)
}

async function buildSplitSystemPrompt(expertId?: string): Promise<string> {
  const agentStore = useAgentStore()
  const agentTeamsStore = useAgentTeamsStore()
  await agentTeamsStore.loadExperts()

  const expert = resolveExpertById(expertId, agentTeamsStore.experts)
    || agentTeamsStore.builtinPlannerExpert
    || agentTeamsStore.enabledExperts[0]
    || null

  return buildExpertSystemPrompt(expert?.prompt, [
    buildPlanSplitSystemPrompt(),
    buildExpertCatalogPrompt(agentTeamsStore.enabledExperts, agentStore.agents)
  ])
}

export const useTaskSplitStore = defineStore('taskSplit', () => {
  const messages = ref<SplitMessage[]>([])
  const logs = ref<PlanSplitLogRecord[]>([])
  const isProcessing = ref(false)
  const splitResult = ref<AITaskItem[] | null>(null)
  const submittedForms = ref<SubmittedFormSnapshot[]>([])
  const currentFormId = ref<string | null>(null)
  const formQueue = ref<DynamicFormSchema[]>([])
  const currentFormIndex = ref<number>(0)
  const session = ref<PlanSplitSessionRecord | null>(null)
  const context = ref<TaskSplitContext | null>(null)
  const streamUnlisten = ref<UnlistenFn | null>(null)
  const runtimeNotices = ref<RuntimeNotice[]>([])
  const usageModelHint = ref<string | null>(null)
  const runtimeMetrics = ref<PlanSplitRuntimeMetrics | null>(null)
  const recordedUsageSessionIds = ref<Set<string>>(new Set())
  const refinementState = ref<TaskSplitRefinementState | null>(null)

  const subSplitMode = computed(() => refinementState.value?.mode === 'task_resplit')
  const subSplitTargetIndex = computed(() => refinementState.value?.targetIndex ?? null)
  const subSplitOriginalTasks = computed(() => refinementState.value?.originalTasks ?? [])
  const subSplitConfig = computed(() =>
    refinementState.value?.mode === 'task_resplit'
      ? refinementState.value.config as TaskResplitConfig
      : null
  )
  const refinementMode = computed(() => refinementState.value?.mode ?? null)

  const activeFormSchema = computed(() => {
    if (!formQueue.value.length) return null
    return formQueue.value[currentFormIndex.value] ?? null
  })

  function detachStream() {
    if (streamUnlisten.value) {
      streamUnlisten.value()
      streamUnlisten.value = null
    }
  }

  function resetState() {
    messages.value = []
    logs.value = []
    isProcessing.value = false
    splitResult.value = null
    submittedForms.value = []
    currentFormId.value = null
    formQueue.value = []
    currentFormIndex.value = 0
    session.value = null
    context.value = null
    runtimeNotices.value = []
    usageModelHint.value = null
    runtimeMetrics.value = null
    recordedUsageSessionIds.value = new Set()
    refinementState.value = null
  }

  function applySessionSnapshot(snapshot: PlanSplitSessionRecord | null, preserveLogs: boolean = true) {
    session.value = snapshot
    if (!snapshot) {
      messages.value = []
      splitResult.value = null
      formQueue.value = []
      currentFormIndex.value = 0
      currentFormId.value = null
      isProcessing.value = false
      if (!preserveLogs) {
        logs.value = []
      }
      return
    }

    messages.value = toSplitMessages(snapshot.messagesJson)
    const parsedResult = toSplitResult(snapshot.resultJson)
    if (parsedResult) {
      splitResult.value = parsedResult
    } else if (!refinementState.value) {
      splitResult.value = null
    }
    formQueue.value = toFormQueue(snapshot.formQueueJson)
    currentFormIndex.value = snapshot.currentFormIndex ?? 0
    currentFormId.value = formQueue.value[currentFormIndex.value]?.formId ?? null
    isProcessing.value = snapshot.status === 'running'

    if (['completed', 'failed', 'stopped'].includes(snapshot.status)) {
      recordPlanSplitUsage(snapshot)
    }
  }

  function recordPlanSplitUsage(snapshot: PlanSplitSessionRecord) {
    if (recordedUsageSessionIds.value.has(snapshot.id) || !context.value) {
      return
    }

    const agent = useAgentStore().agents.find(item => item.id === context.value?.agentId)
    if (!agent) {
      return
    }

    const latestUsage = findLatestUsageSnapshot(logs.value)
    const projectId = usePlanStore().plans.find(plan => plan.id === context.value?.planId)?.projectId ?? null

    recordedUsageSessionIds.value.add(snapshot.id)
    recordAgentCliUsageInBackground(agent, {
      executionId: `plan-split-${snapshot.id}`,
      executionMode: 'task_split',
      modelId: resolveRecordedModelId({
        reportedModelId: latestUsage.modelId,
        requestedModelId: context.value.modelId || usageModelHint.value || null
      }),
      projectId,
      sessionId: null,
      inputTokens: latestUsage.inputTokens,
      outputTokens: latestUsage.outputTokens,
      occurredAt: snapshot.completedAt || snapshot.stoppedAt || snapshot.updatedAt
    })
  }

  async function loadSession(planId: string) {
    const [snapshot, splitLogs] = await Promise.all([
      invoke<PlanSplitSessionRecord | null>('get_plan_split_session', { planId }),
      invoke<PlanSplitLogRecord[]>('list_plan_split_logs', { planId }).catch(() => [])
    ])

    logs.value = toPlanSplitLogs(splitLogs)
    applySessionSnapshot(snapshot, true)
  }

  async function isExecutionSessionActive(sessionId?: string | null): Promise<boolean> {
    const normalizedSessionId = sessionId?.trim()
    if (!normalizedSessionId) {
      return false
    }

    try {
      return await invoke<boolean>('is_execution_session_active', { sessionId: normalizedSessionId })
    } catch (error) {
      logger.warn('[TaskSplit] Failed to query execution session state:', error)
      return false
    }
  }

  async function recoverStaleRunningSession(planId: string): Promise<boolean> {
    const snapshot = session.value
    if (!snapshot || snapshot.status !== 'running') {
      return false
    }

    const lastUpdatedAt = new Date(snapshot.updatedAt || snapshot.startedAt || snapshot.createdAt).getTime()
    if (!Number.isFinite(lastUpdatedAt)) {
      return false
    }

    const inactiveTooLong = Date.now() - lastUpdatedAt >= STALE_PLAN_SPLIT_SESSION_TIMEOUT_MS
    if (!inactiveTooLong) {
      return false
    }

    const isActive = await isExecutionSessionActive(snapshot.executionSessionId)
    if (isActive) {
      return false
    }

    logger.warn('[TaskSplit] Recovering stale running session', {
      planId,
      executionSessionId: snapshot.executionSessionId,
      updatedAt: snapshot.updatedAt
    })
    await invoke('clear_plan_split_session', { planId })
    logs.value = []
    applySessionSnapshot(null, false)
    return true
  }

  async function subscribeToPlan(planId: string) {
    detachStream()
    streamUnlisten.value = await listen<PlanSplitStreamPayload>(`plan-split-stream-${planId}`, async (event) => {
      const payload = event.payload
      if (payload.type === 'session_updated' && payload.session) {
        if (
          runtimeMetrics.value
          && !runtimeMetrics.value.doneAt
          && ['completed', 'failed', 'stopped'].includes(payload.session.status)
        ) {
          runtimeMetrics.value = {
            ...runtimeMetrics.value,
            doneAt: performance.now()
          }
          ;(globalThis as { __EASY_AGENT_LAST_PLAN_SPLIT_METRICS?: PlanSplitRuntimeMetrics | null }).__EASY_AGENT_LAST_PLAN_SPLIT_METRICS = runtimeMetrics.value
        }
        applySessionSnapshot(payload.session, true)
        return
      }

      if (payload.type === 'done') {
        if (runtimeMetrics.value && !runtimeMetrics.value.doneAt) {
          runtimeMetrics.value = {
            ...runtimeMetrics.value,
            doneAt: performance.now()
          }
          ;(globalThis as { __EASY_AGENT_LAST_PLAN_SPLIT_METRICS?: PlanSplitRuntimeMetrics | null }).__EASY_AGENT_LAST_PLAN_SPLIT_METRICS = runtimeMetrics.value
        }
        return
      }

      if (!runtimeMetrics.value) {
        runtimeMetrics.value = {
          startedAt: performance.now()
        }
      }

      if (!runtimeMetrics.value.firstEventAt) {
        runtimeMetrics.value = {
          ...runtimeMetrics.value,
          firstEventAt: performance.now()
        }
      }

      if (!runtimeMetrics.value.firstRenderableAt && ['content', 'thinking', 'thinking_start', 'tool_use', 'tool_input_delta', 'tool_result', 'system'].includes(payload.type)) {
        runtimeMetrics.value = {
          ...runtimeMetrics.value,
          firstRenderableAt: performance.now()
        }
      }

      ;(globalThis as { __EASY_AGENT_LAST_PLAN_SPLIT_METRICS?: PlanSplitRuntimeMetrics | null }).__EASY_AGENT_LAST_PLAN_SPLIT_METRICS = runtimeMetrics.value

      const content = payload.content ?? payload.error ?? payload.toolResult ?? payload.toolInput ?? ''
      const parsedMetadata = parseStreamPayloadMetadata(payload.metadata)
      logs.value.push({
        id: `${payload.sessionId || 'plan'}-${payload.type}-${payload.createdAt || Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
        planId: payload.planId,
        sessionId: payload.sessionId || '',
        type: payload.type as PlanSplitLogRecord['type'],
        content,
        metadata: buildPersistedLogMetadata(payload, parsedMetadata),
        createdAt: payload.createdAt || new Date().toISOString()
      })
    })
  }

  async function startBackgroundSession(
    nextContext: TaskSplitContext,
    llmMessages: MessageInput[],
    uiMessages: SplitMessage[],
    options?: {
      preserveLogs?: boolean
    }
  ) {
    const agentStore = useAgentStore()
    const agent = agentStore.agents.find(item => item.id === nextContext.agentId)
    if (!agent) {
      throw new Error('未找到选中的智能体，请重新选择后重试。')
    }
    if (agent.type === 'cli' && !agent.cliPath) {
      throw new Error('CLI 路径未配置')
    }
    if (agent.type === 'sdk' && !agent.apiKey) {
      throw new Error('SDK API Key 未配置')
    }

    if (!options?.preserveLogs) {
      logs.value = []
    }
    runtimeMetrics.value = {
      startedAt: performance.now()
    }
    const mcpServers = await loadAgentMcpServers(agent).catch((error) => {
      logger.warn('[TaskSplit] Failed to load MCP servers:', error)
      return []
    })
    const executionRequest = buildExecutionRequest(
      nextContext,
      agent,
      llmMessages,
      mcpServers.length > 0 ? mcpServers : undefined
    )
    const snapshot = await invoke<PlanSplitSessionRecord>('start_plan_split', {
      input: {
        planId: nextContext.planId,
        granularity: nextContext.granularity,
        taskCountMode: nextContext.taskCountMode ?? 'min',
        executionRequest,
        llmMessages,
        messages: uiMessages
      }
    })
    applySessionSnapshot(snapshot, true)
  }

  /**
   * 将当前预览任务列表同步回拆分会话，避免后续会话刷新覆盖已应用的结果。
   */
  async function syncSplitResultToSession(nextTasks: AITaskItem[]) {
    if (!context.value) {
      return
    }

    try {
      const snapshot = await invoke<PlanSplitSessionRecord>('update_plan_split_result', {
        planId: context.value.planId,
        result: nextTasks,
        messages: messages.value
      })
      applySessionSnapshot(snapshot, true)
    } catch (error) {
      logger.warn('[TaskSplit] Failed to sync split result to session:', error)
    }
  }

  function rememberSubmittedForm(
    schema: DynamicFormSchema,
    values: Record<string, unknown>,
    promptText?: string
  ) {
    const submittedAt = new Date().toISOString()
    const snapshot: SubmittedFormSnapshot = {
      formId: schema.formId,
      schema: normalizeFormSchemaForRendering(
        JSON.parse(JSON.stringify(schema)) as DynamicFormSchema
      ),
      promptText,
      values: JSON.parse(JSON.stringify(values)) as Record<string, unknown>,
      submittedAt
    }

    submittedForms.value = [
      ...submittedForms.value.filter(item => item.formId !== schema.formId),
      snapshot
    ]
  }

  async function initSession(nextContext: TaskSplitContext) {
    const previousPlanId = context.value?.planId
    if (previousPlanId !== nextContext.planId) {
      submittedForms.value = []
      messages.value = []
      logs.value = []
      splitResult.value = null
      formQueue.value = []
      currentFormIndex.value = 0
      currentFormId.value = null
      session.value = null
      refinementState.value = null
    }

    context.value = nextContext
    runtimeNotices.value = []
    usageModelHint.value = null
    const selectedAgent = useAgentStore().agents.find(agent => agent.id === nextContext.agentId)
    if (selectedAgent) {
      const [environmentNotice, modelHint] = await Promise.all([
        buildCliEnvironmentNotice(selectedAgent).catch(() => null),
        resolveUsageModelHint(selectedAgent).catch(() => undefined)
      ])
      const selectedExpert = resolveExpertById(nextContext.expertId, useAgentTeamsStore().experts)
      const contextNotice = buildContextStrategyNotice({
        strategy: 'Plan Split Prompt Context',
        runtime: inferAgentProvider(selectedAgent)?.toUpperCase() || selectedAgent.type,
        model: nextContext.modelId || selectedAgent.modelId || modelHint,
        expert: selectedExpert?.name || nextContext.expertId,
        systemMessageCount: 1,
        userMessageCount: 1,
        assistantMessageCount: 0,
        historyMessageCount: 0
      })
      runtimeNotices.value = [environmentNotice, contextNotice].filter((notice): notice is RuntimeNotice => Boolean(notice))
      usageModelHint.value = modelHint ?? null
    }
    await subscribeToPlan(nextContext.planId)
    await loadSession(nextContext.planId)
    await recoverStaleRunningSession(nextContext.planId)

    if (session.value) {
      return
    }

    if (splitResult.value && previousPlanId === nextContext.planId) {
      return
    }

    const llmMessages: MessageInput[] = [
      {
        role: 'system',
        content: await buildSplitSystemPrompt(nextContext.expertId)
      },
      {
        role: 'user',
        content: buildPlanSplitKickoffPrompt({
          planName: nextContext.planName,
          planDescription: nextContext.planDescription,
          minTaskCount: nextContext.granularity
        })
      }
    ]

    const uiMessages: SplitMessage[] = [{
      id: crypto.randomUUID(),
      role: 'user',
      content: [
        `计划标题：${nextContext.planName}`,
        `计划描述：${nextContext.planDescription?.trim() || '（无）'}`,
        `拆分任务数量：至少拆分 ${nextContext.granularity} 个任务`
      ].join('\n'),
      timestamp: new Date().toISOString()
    }]

    await startBackgroundSession(nextContext, llmMessages, uiMessages)
  }

  async function submitFormResponse(formId: string, values: Record<string, unknown>) {
    if (!context.value) return
    const schema = activeFormSchema.value
    if (!schema || schema.formId !== formId) return
    const promptText = [...messages.value]
      .slice()
      .reverse()
      .find(message => message.role === 'assistant' && message.content.trim())
      ?.content
      ?.trim()

    try {
      const snapshot = await invoke<PlanSplitSessionRecord>('submit_plan_split_form', {
        input: {
          planId: context.value.planId,
          formId,
          values,
          displayContent: summarizeFormValues(schema, values)
        }
      })
      applySessionSnapshot(snapshot, true)
      rememberSubmittedForm(schema, values, promptText)
    } catch (error) {
      logger.error('[TaskSplit] Failed to submit form response:', error)
      throw error
    }
  }

  /**
   * 基于当前会话上下文重新发起最新一轮拆分，并清理上一轮失败/停止留下的 AI 渲染痕迹。
   */
  async function restartFromPersistedContext(snapshot: PlanSplitSessionRecord) {
    if (!context.value) {
      return
    }

    const llmMessages = trimTrailingAssistantMessages(
      parseJson<MessageInput[]>(snapshot.llmMessagesJson, [])
    )
    const uiMessages = trimTrailingAssistantMessages(
      toSplitMessages(snapshot.messagesJson)
    )
    if (llmMessages.length === 0) {
      throw new Error('当前会话缺少可重试的上下文。')
    }

    const latestExecutionSessionId = snapshot.executionSessionId?.trim() || null
    const resetSnapshot = await invoke<PlanSplitSessionRecord>('reset_plan_split_turn_for_restart', {
      planId: context.value.planId
    })

    messages.value = uiMessages
    if (latestExecutionSessionId) {
      logs.value = logs.value.filter(log => log.sessionId !== latestExecutionSessionId)
    }
    applySessionSnapshot({
      ...resetSnapshot,
      messagesJson: JSON.stringify(uiMessages),
      llmMessagesJson: JSON.stringify(llmMessages)
    }, true)

    await startBackgroundSession(context.value, llmMessages, uiMessages, {
      preserveLogs: true
    })
  }

  async function retry() {
    if (!session.value) {
      return
    }

    await restartFromPersistedContext(session.value)
  }

  async function continueSession() {
    if (!session.value || session.value.status !== 'stopped') {
      return
    }

    await restartFromPersistedContext(session.value)
  }

  async function stop() {
    if (!context.value) return
    const snapshot = await invoke<PlanSplitSessionRecord | null>('stop_plan_split', {
      planId: context.value.planId
    })
    applySessionSnapshot(snapshot, true)
  }

  async function clearAllSplitData(planId: string) {
    await invoke('clear_plan_split_session', { planId })
    if (context.value?.planId === planId) {
      resetState()
    }
  }

  function clearProjectSplitState(planIds: string[]) {
    if (planIds.length === 0) {
      return
    }

    if (context.value && planIds.includes(context.value.planId)) {
      detachStream()
      resetState()
    }
  }

  function updateSplitTask(index: number, updates: Partial<AITaskItem>) {
    if (!splitResult.value || !splitResult.value[index]) return
    splitResult.value[index] = { ...splitResult.value[index], ...updates }
    void syncSplitResultToSession(splitResult.value)
  }

  function removeSplitTask(index: number) {
    if (!splitResult.value) return
    splitResult.value.splice(index, 1)
    void syncSplitResultToSession(splitResult.value)
  }

  function addSplitTask() {
    const agentTeamsStore = useAgentTeamsStore()
    const defaultExpertId = agentTeamsStore.builtinDeveloperExpert?.id
      || agentTeamsStore.enabledExperts.find(expert => expert.category === 'developer')?.id
      || undefined
    const nextTask: AITaskItem = {
      title: '',
      description: '',
      priority: 'medium',
      expertId: defaultExpertId,
      implementationSteps: [''],
      testSteps: [''],
      acceptanceCriteria: ['']
    }
    if (!splitResult.value) {
      splitResult.value = [nextTask]
      return
    }
    splitResult.value.push(nextTask)
    void syncSplitResultToSession(splitResult.value)
  }

  async function startSubSplit(index: number, config: TaskResplitConfig) {
    if (!context.value || !splitResult.value || !splitResult.value[index]) return

    const targetTask = splitResult.value[index]

    const nextContext: TaskSplitContext = {
      ...context.value,
      granularity: config.granularity,
      expertId: config.expertId || context.value.expertId,
      agentId: config.agentId || context.value.agentId,
      modelId: config.modelId || context.value.modelId,
      taskCountMode: 'min'
    }
    context.value = nextContext
    refinementState.value = {
      mode: 'task_resplit',
      targetIndex: index,
      originalTasks: JSON.parse(JSON.stringify(splitResult.value)) as AITaskItem[],
      config
    }
    const llmMessages: MessageInput[] = [
      {
        role: 'system',
        content: await buildSplitSystemPrompt(nextContext.expertId)
      },
      {
        role: 'user',
        content: buildTaskResplitKickoffPrompt({
          planName: nextContext.planName,
          planDescription: nextContext.planDescription,
          taskTitle: targetTask.title,
          taskDescription: targetTask.description,
          implementationSteps: targetTask.implementationSteps || [],
          testSteps: targetTask.testSteps || [],
          acceptanceCriteria: targetTask.acceptanceCriteria,
          userPrompt: config.customPrompt,
          minTaskCount: config.granularity
        })
      }
    ]

    const preservedMessages = messages.value.filter(m => m.content.trim())
    const uiMessages: SplitMessage[] = [
      ...preservedMessages,
      {
        id: crypto.randomUUID(),
        role: 'user',
        content: [
          `继续拆分任务：${targetTask.title}`,
          `原任务描述：${targetTask.description || '（无）'}`,
          `拆分颗粒度：至少 ${config.granularity} 个子任务`,
          config.customPrompt ? `额外要求：${config.customPrompt}` : ''
        ].filter(Boolean).join('\n'),
        timestamp: new Date().toISOString()
      }
    ]

    const selectedAgent = useAgentStore().agents.find(agent => agent.id === nextContext.agentId)
    if (selectedAgent) {
      const contextNotice = buildContextStrategyNotice({
        strategy: 'Subtask Resplit Context',
        runtime: inferAgentProvider(selectedAgent)?.toUpperCase() || selectedAgent.type,
        model: nextContext.modelId || usageModelHint.value || selectedAgent.modelId,
        expert: resolveExpertById(nextContext.expertId, useAgentTeamsStore().experts)?.name || nextContext.expertId,
        systemMessageCount: llmMessages.filter(message => message.role === 'system').length,
        userMessageCount: llmMessages.filter(message => message.role === 'user').length,
        assistantMessageCount: 0,
        historyMessageCount: uiMessages.length
      })
      runtimeNotices.value = [
        ...runtimeNotices.value.filter(notice => notice.id !== 'context'),
        ...(contextNotice ? [contextNotice] : [])
      ]
    }

    await startBackgroundSession(nextContext, llmMessages, uiMessages)
  }

  /**
   * 对当前整份任务列表发起整体优化，保留任务数量不变。
   */
  async function startListOptimize(config: TaskListOptimizeConfig) {
    if (!context.value || !splitResult.value || splitResult.value.length === 0) return

    const originalTasks = JSON.parse(JSON.stringify(splitResult.value)) as AITaskItem[]
    refinementState.value = {
      mode: 'list_optimize',
      targetIndex: null,
      originalTasks,
      config
    }

    const nextContext: TaskSplitContext = {
      ...context.value,
      granularity: originalTasks.length,
      expertId: config.expertId || context.value.expertId,
      agentId: config.agentId || context.value.agentId,
      modelId: config.modelId || context.value.modelId,
      taskCountMode: 'exact'
    }
    context.value = nextContext

    const llmMessages: MessageInput[] = [
      {
        role: 'system',
        content: await buildSplitSystemPrompt(nextContext.expertId)
      },
      {
        role: 'user',
        content: buildTaskListOptimizeKickoffPrompt({
          planName: nextContext.planName,
          planDescription: nextContext.planDescription,
          tasks: originalTasks,
          userPrompt: config.customPrompt,
          targetIndex: config.targetIndex
        })
      }
    ]

    const preservedMessages = messages.value.filter(m => m.content.trim())
    const targetLabel = config.targetIndex !== undefined && config.targetIndex >= 0 && config.targetIndex < originalTasks.length
      ? `优化任务 ${config.targetIndex + 1}《${originalTasks[config.targetIndex].title}》`
      : `整体优化任务列表：共 ${originalTasks.length} 个任务`
    const uiMessages: SplitMessage[] = [
      ...preservedMessages,
      {
        id: crypto.randomUUID(),
        role: 'user',
        content: [
          targetLabel,
          '约束：保持任务数量不变',
          config.customPrompt ? `额外要求：${config.customPrompt}` : ''
        ].filter(Boolean).join('\n'),
        timestamp: new Date().toISOString()
      }
    ]

    const selectedAgent = useAgentStore().agents.find(agent => agent.id === nextContext.agentId)
    if (selectedAgent) {
      const contextNotice = buildContextStrategyNotice({
        strategy: 'Task List Optimize Context',
        runtime: inferAgentProvider(selectedAgent)?.toUpperCase() || selectedAgent.type,
        model: nextContext.modelId || usageModelHint.value || selectedAgent.modelId,
        expert: resolveExpertById(nextContext.expertId, useAgentTeamsStore().experts)?.name || nextContext.expertId,
        systemMessageCount: llmMessages.filter(message => message.role === 'system').length,
        userMessageCount: llmMessages.filter(message => message.role === 'user').length,
        assistantMessageCount: 0,
        historyMessageCount: uiMessages.length
      })
      runtimeNotices.value = [
        ...runtimeNotices.value.filter(notice => notice.id !== 'context'),
        ...(contextNotice ? [contextNotice] : [])
      ]
    }

    await startBackgroundSession(nextContext, llmMessages, uiMessages)
  }

  function clearRefinementState() {
    refinementState.value = null
  }

  async function completeSubSplit(newTasks: AITaskItem[]) {
    if (!subSplitMode.value || subSplitTargetIndex.value === null || !refinementState.value) {
      return
    }

    const mergedTasks = [...refinementState.value.originalTasks]
    mergedTasks.splice(subSplitTargetIndex.value, 1, ...newTasks)
    splitResult.value = mergedTasks
    clearRefinementState()
    await syncSplitResultToSession(mergedTasks)
  }

  async function completeListOptimize(newTasks: AITaskItem[]) {
    if (refinementState.value?.mode !== 'list_optimize') {
      return
    }

    splitResult.value = [...newTasks]
    clearRefinementState()
    await syncSplitResultToSession(newTasks)
  }

  /**
   * 取消未确认的继续拆分/整体优化结果，并恢复到原始任务列表。
   */
  async function cancelRefinement(options: { discardSession?: boolean } = {}) {
    if (!refinementState.value || !context.value) return

    splitResult.value = [...refinementState.value.originalTasks]
    clearRefinementState()

    if (!options.discardSession) {
      return
    }

    await invoke('clear_plan_split_session', { planId: context.value.planId })
    session.value = null
    messages.value = []
    logs.value = []
    submittedForms.value = []
    currentFormId.value = null
    formQueue.value = []
    currentFormIndex.value = 0
    isProcessing.value = false
    runtimeMetrics.value = null
  }

  function reset() {
    detachStream()
    resetState()
  }

  function detach() {
    detachStream()
    isProcessing.value = false
  }

  return {
    messages,
    logs,
    isProcessing,
    splitResult,
    submittedForms,
    currentFormId,
    currentFormIndex,
    formQueue,
    activeFormSchema,
    session,
    context,
    runtimeNotices,
    usageModelHint,
    runtimeMetrics,
    refinementMode,
    refinementState,
    subSplitMode,
    subSplitTargetIndex,
    subSplitOriginalTasks,
    subSplitConfig,
    initSession,
    submitFormResponse,
    retry,
    continueSession,
    stop,
    updateSplitTask,
    removeSplitTask,
    addSplitTask,
    startSubSplit,
    startListOptimize,
    completeSubSplit,
    completeListOptimize,
    cancelRefinement,
    clearAllSplitData,
    clearPlanSplitSessions: clearAllSplitData,
    clearProjectSplitState,
    reset,
    detach,
    loadSession
  }
})
