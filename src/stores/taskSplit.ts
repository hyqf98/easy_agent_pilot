import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { inferAgentProvider, useAgentStore, type AgentConfig } from './agent'
import { useAgentTeamsStore } from './agentTeams'
import { usePlanStore } from './plan'
import { useSettingsStore } from './settings'
import type { MessageAttachment } from './message'
import { logger } from '@/utils/logger'
import { normalizeFormSchemaForRendering, normalizeFormSchemasForRendering } from '@/utils/formSchema'
import {
  extractFirstFormRequestFromContents,
  extractTaskSplitResultFromContents
} from '@/utils/structuredContent'
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
import { buildContextStrategyNotice } from '@/utils/runtimeNotice'
import { loadAgentMcpServers } from '@/utils/mcpServerConfig'
import {
  classifyCliFailureFragments,
  createCliFailureFragment
} from '@/utils/cliFailureMonitor'
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

interface ContinueSessionWithInstructionOptions {
  attachments?: MessageAttachment[]
  displayContent?: string
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
const PLAN_SPLIT_AUTO_RETRY_DELAY_MS = 10_000
const PLAN_SPLIT_AUTO_RETRY_GROUP = 'cli_failure_retry'

interface RustPlanSplitLogRecord {
  id: string
  planId: string
  sessionId: string
  logType?: PlanSplitLogRecord['type']
  type?: PlanSplitLogRecord['type']
  content: string
  metadata?: string | null
  createdAt: string
}

function measurePlanSplit(label: string, startedAt: number, detail: Record<string, unknown> = {}) {
  const durationMs = Math.round((performance.now() - startedAt) * 10) / 10
  console.info(`[PlanSplitPerf] ${label}`, { durationMs, ...detail })
}

function measurePlanSplitPoint(label: string, detail: Record<string, unknown> = {}) {
  console.info(`[PlanSplitPerf] ${label}`, detail)
}

function sumPlanSplitLogBytes(items: RustPlanSplitLogRecord[] | PlanSplitLogRecord[]) {
  return items.reduce((total, item) =>
    total + (item.content?.length ?? 0) + (item.metadata?.length ?? 0),
  0)
}

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

function readMetadataNumericValue(
  metadata: Record<string, unknown> | null,
  ...keys: string[]
): number | undefined {
  for (const key of keys) {
    const value = metadata?.[key]
    if (typeof value === 'number' && Number.isFinite(value)) {
      return value
    }
    if (typeof value === 'string' && value.trim()) {
      const numeric = Number(value)
      if (Number.isFinite(numeric)) {
        return numeric
      }
    }
  }

  return undefined
}

function readMetadataString(
  metadata: Record<string, unknown> | null,
  key: string,
  fallbackKey?: string
): string | null {
  const target = metadata?.[key] ?? (fallbackKey ? metadata?.[fallbackKey] : undefined)
  return typeof target === 'string' && target.trim()
    ? target.trim()
    : null
}

function isStructuredOutputToolName(toolName: string | null): boolean {
  return toolName === 'StructuredOutput' || toolName === 'structured_output'
}

function collectStructuredOutputCandidatesFromLogs(splitLogs: PlanSplitLogRecord[]): string[] {
  const sortedLogs = [...splitLogs]
    .sort((left, right) => new Date(left.createdAt).getTime() - new Date(right.createdAt).getTime())
  const structuredToolCallIds = new Set<string>()
  const structuredToolCallOrder: string[] = []
  const structuredToolInputChunks = new Map<string, string[]>()
  const candidates: string[] = []

  sortedLogs.forEach((log) => {
    const metadata = parseStreamPayloadMetadata(log.metadata)
    const toolName = readMetadataString(metadata, 'toolName', 'tool_name')
    const toolCallId = readMetadataString(metadata, 'toolCallId', 'tool_call_id')

    if (log.type === 'tool_use' && isStructuredOutputToolName(toolName)) {
      if (toolCallId && !structuredToolCallIds.has(toolCallId)) {
        structuredToolCallIds.add(toolCallId)
        structuredToolCallOrder.push(toolCallId)
      }

      const toolInput = readMetadataString(metadata, 'toolInput', 'tool_input')
      const candidate = toolInput ?? (log.content?.trim() ? log.content.trim() : '')
      if (candidate) {
        candidates.push(candidate)
      }
      return
    }

    if (!toolCallId || !structuredToolCallIds.has(toolCallId)) {
      return
    }

    if (log.type === 'tool_input_delta') {
      const chunk = log.content?.trim()
        ? log.content
        : readMetadataString(metadata, 'toolInput', 'tool_input') ?? ''
      if (!chunk.trim()) {
        return
      }

      const chunks = structuredToolInputChunks.get(toolCallId) ?? []
      chunks.push(chunk)
      structuredToolInputChunks.set(toolCallId, chunks)
      return
    }

    if (log.type === 'tool_result') {
      const toolResult = readMetadataString(metadata, 'toolResult', 'tool_result')
      const candidate = toolResult ?? (log.content?.trim() ? log.content.trim() : '')
      if (candidate) {
        candidates.push(candidate)
      }
    }
  })

  structuredToolCallOrder.forEach((toolCallId) => {
    const chunks = structuredToolInputChunks.get(toolCallId)
    if (!chunks?.length) {
      return
    }

    const candidate = chunks.join('').trim()
    if (candidate) {
      candidates.push(candidate)
    }
  })

  return candidates
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
  const rawInputTokens = typeof payload.rawInputTokens === 'number'
    ? payload.rawInputTokens
    : readMetadataNumericValue(parsedMetadata, 'rawInputTokens', 'raw_input_tokens')
  const rawOutputTokens = typeof payload.rawOutputTokens === 'number'
    ? payload.rawOutputTokens
    : readMetadataNumericValue(parsedMetadata, 'rawOutputTokens', 'raw_output_tokens')
  const cacheReadInputTokens = typeof payload.cacheReadInputTokens === 'number'
    ? payload.cacheReadInputTokens
    : readMetadataNumericValue(parsedMetadata, 'cacheReadInputTokens', 'cache_read_input_tokens')
  const cacheCreationInputTokens = typeof payload.cacheCreationInputTokens === 'number'
    ? payload.cacheCreationInputTokens
    : readMetadataNumericValue(parsedMetadata, 'cacheCreationInputTokens', 'cache_creation_input_tokens')
  const externalSessionId = typeof payload.externalSessionId === 'string' && payload.externalSessionId.trim()
    ? payload.externalSessionId.trim()
    : typeof parsedMetadata?.externalSessionId === 'string' && parsedMetadata.externalSessionId.trim()
      ? parsedMetadata.externalSessionId.trim()
      : undefined

  return JSON.stringify({
    model,
    inputTokens,
    outputTokens,
    rawInputTokens,
    rawOutputTokens,
    cacheReadInputTokens,
    cacheCreationInputTokens,
    externalSessionId,
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
  mcpServers: ExecutionRequest['mcpServers'],
  resumeSessionId?: string
): ExecutionRequest {
  const normalizedProvider = (agent.provider || 'claude').toLowerCase()
  const provider = normalizedProvider === 'codex'
    ? 'codex'
    : normalizedProvider === 'claude'
      ? 'claude'
      : 'generic'
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
    responseMode: 'stream_text',
    resumeSessionId
  })
}

function toSplitMessages(raw?: string | null): SplitMessage[] {
  const parsed = parseJson<Array<{
    id: string
    role: 'user' | 'assistant'
    content: string
    attachments?: SplitMessage['attachments']
    timestamp: string
  }>>(raw, [])
  return parsed.map(message => ({
    id: message.id,
    role: message.role,
    content: message.content,
    attachments: message.attachments,
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

function toPlanSplitLogs(logs: RustPlanSplitLogRecord[]): PlanSplitLogRecord[] {
  return logs
    .map(log => ({
      ...log,
      type: log.type ?? log.logType ?? 'system'
    }))
    .sort((left, right) => new Date(left.createdAt).getTime() - new Date(right.createdAt).getTime())
}

function collectStructuredRecoveryCandidates(
  snapshot: PlanSplitSessionRecord,
  splitLogs: PlanSplitLogRecord[]
): string[] {
  const messageContents = toSplitMessages(snapshot.messagesJson)
    .filter(message => message.role === 'assistant')
    .map(message => message.content)
  const logContents = [...splitLogs]
    .sort((left, right) => new Date(left.createdAt).getTime() - new Date(right.createdAt).getTime())
    .map(log => log.content)
  const structuredOutputCandidates = collectStructuredOutputCandidatesFromLogs(splitLogs)

  return [
    ...messageContents,
    ...logContents,
    ...structuredOutputCandidates,
    snapshot.rawContent ?? null
  ].filter((item): item is string => typeof item === 'string' && item.trim().length > 0)
}

function recoverSplitResultFromSnapshot(
  snapshot: PlanSplitSessionRecord,
  splitLogs: PlanSplitLogRecord[]
): AITaskItem[] | null {
  const persisted = toSplitResult(snapshot.resultJson)
  if (persisted?.length) {
    return persisted
  }

  const recovered = extractTaskSplitResultFromContents(
    collectStructuredRecoveryCandidates(snapshot, splitLogs)
  )
  return recovered?.tasks ?? null
}

function recoverFormQueueFromSnapshot(
  snapshot: PlanSplitSessionRecord,
  splitLogs: PlanSplitLogRecord[]
): DynamicFormSchema[] {
  const persisted = toFormQueue(snapshot.formQueueJson)
  if (persisted.length > 0) {
    return persisted
  }

  const recovered = extractFirstFormRequestFromContents(
    collectStructuredRecoveryCandidates(snapshot, splitLogs)
  )
  return normalizeFormSchemasForRendering(recovered?.forms ?? [])
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

function trimPlanSplitLogsBySessionId(
  items: PlanSplitLogRecord[],
  sessionId?: string | null
): PlanSplitLogRecord[] {
  const normalizedSessionId = sessionId?.trim()
  if (!normalizedSessionId) {
    return [...items]
  }

  return items.filter(log => log.sessionId !== normalizedSessionId)
}

function trimPlanSplitLogsAfterTimestamp(
  items: PlanSplitLogRecord[],
  boundaryTimestamp?: string | null
): PlanSplitLogRecord[] {
  if (!boundaryTimestamp) {
    return [...items]
  }

  const boundaryMs = new Date(boundaryTimestamp).getTime()
  if (!Number.isFinite(boundaryMs)) {
    return [...items]
  }

  return items.filter((log) => {
    const logMs = new Date(log.createdAt).getTime()
    return !Number.isFinite(logMs) || logMs < boundaryMs
  })
}

function isAutoRetryProgressLog(log: Pick<PlanSplitLogRecord, 'type' | 'metadata'>): boolean {
  if (log.type !== 'system' || typeof log.metadata !== 'string') {
    return false
  }

  return log.metadata.includes(`"retryGroup":"${PLAN_SPLIT_AUTO_RETRY_GROUP}"`)
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
  const isLoadingLogs = ref(false)
  const renderVersion = ref(0)
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
  const autoRetryCount = ref(0)
  const autoRetryTimer = ref<ReturnType<typeof setTimeout> | null>(null)
  const autoRetryScheduled = ref(false)
  const autoRetryNextRunAt = ref<number | null>(null)

  let initVersion = 0
  let logLoadingTimer: ReturnType<typeof setTimeout> | null = null

  function bumpRenderVersion() {
    renderVersion.value += 1
  }

  interface PlanSplitCache {
    messages: SplitMessage[]
    logs: PlanSplitLogRecord[]
    splitResult: AITaskItem[] | null
    submittedForms: SubmittedFormSnapshot[]
    currentFormId: string | null
    formQueue: DynamicFormSchema[]
    currentFormIndex: number
    session: PlanSplitSessionRecord | null
    context: TaskSplitContext | null
    refinementState: TaskSplitRefinementState | null
    runtimeNotices: RuntimeNotice[]
    usageModelHint: string | null
    recordedUsageSessionIds: Set<string>
    autoRetryCount: number
  }

  const planStateCache = new Map<string, PlanSplitCache>()

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
    cancelAutoRetry()
    messages.value = []
    logs.value = []
    isLoadingLogs.value = false
    bumpRenderVersion()
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
    autoRetryCount.value = 0
    autoRetryNextRunAt.value = null
  }

  function saveCurrentPlanState() {
    const planId = context.value?.planId
    if (!planId) return

    cancelAutoRetry()

    planStateCache.set(planId, {
      messages: [...messages.value],
      logs: [...logs.value],
      splitResult: splitResult.value,
      submittedForms: [...submittedForms.value],
      currentFormId: currentFormId.value,
      formQueue: [...formQueue.value],
      currentFormIndex: currentFormIndex.value,
      session: session.value,
      context: context.value,
      refinementState: refinementState.value,
      runtimeNotices: [...runtimeNotices.value],
      usageModelHint: usageModelHint.value,
      recordedUsageSessionIds: new Set(recordedUsageSessionIds.value),
      autoRetryCount: autoRetryCount.value
    })
  }

  function restorePlanState(planId: string): boolean {
    const cached = planStateCache.get(planId)
    if (!cached) return false

    messages.value = [...cached.messages]
    logs.value = [...cached.logs]
    splitResult.value = cached.splitResult
    submittedForms.value = [...cached.submittedForms]
    currentFormId.value = cached.currentFormId
    formQueue.value = [...cached.formQueue]
    currentFormIndex.value = cached.currentFormIndex
    session.value = cached.session
    context.value = cached.context
    refinementState.value = cached.refinementState
    runtimeNotices.value = [...cached.runtimeNotices]
    usageModelHint.value = cached.usageModelHint
    recordedUsageSessionIds.value = new Set(cached.recordedUsageSessionIds)
    autoRetryCount.value = cached.autoRetryCount
    isProcessing.value = cached.session?.status === 'running'
    runtimeMetrics.value = null
    autoRetryScheduled.value = false
    autoRetryNextRunAt.value = null
    autoRetryTimer.value = null
    bumpRenderVersion()

    return true
  }

  function clearPlanStateCache(planId?: string) {
    if (planId) {
      planStateCache.delete(planId)
    } else {
      planStateCache.clear()
    }
  }

  function cancelAutoRetry() {
    if (autoRetryTimer.value) {
      clearTimeout(autoRetryTimer.value)
      autoRetryTimer.value = null
    }
    autoRetryScheduled.value = false
    autoRetryNextRunAt.value = null
  }

  function startLogLoadingDelay() {
    stopLogLoadingDelay()
    logLoadingTimer = setTimeout(() => {
      isLoadingLogs.value = true
      logLoadingTimer = null
    }, 150)
  }

  function stopLogLoadingDelay() {
    if (logLoadingTimer) {
      clearTimeout(logLoadingTimer)
      logLoadingTimer = null
    }
  }

  function isCurrentPlan(planId?: string | null): boolean {
    return Boolean(planId && context.value?.planId === planId)
  }

  function upsertAutoRetryProgressLog(retryNumber: number, maxRetries: number, delaySeconds: number) {
    if (!context.value || !session.value) {
      return
    }

    const metadata = JSON.stringify({
      retryGroup: PLAN_SPLIT_AUTO_RETRY_GROUP,
      retryCount: retryNumber,
      maxRetries,
      delaySeconds,
      nextRunAt: autoRetryNextRunAt.value
    })
    const content = `自动重试准备中... 第 ${retryNumber}/${maxRetries} 次，${delaySeconds} 秒后重试`
    const existingLog = [...logs.value].reverse().find(isAutoRetryProgressLog)

    if (existingLog) {
      existingLog.content = content
      existingLog.metadata = metadata
      void invoke('update_plan_split_log', {
        id: existingLog.id,
        content,
        metadata
      }).catch((error) => {
        logger.warn('[TaskSplit] Failed to update auto-retry progress log:', error)
      })
      return
    }

    const tempLog: PlanSplitLogRecord = {
      id: `auto-retry-${Date.now()}`,
      planId: context.value.planId,
      sessionId: session.value.executionSessionId || '',
      type: 'system',
      content,
      metadata,
      createdAt: new Date().toISOString()
    }
    logs.value.push(tempLog)

    void invoke<RustPlanSplitLogRecord>('create_plan_split_log', {
      planId: context.value.planId,
      sessionId: tempLog.sessionId,
      logType: tempLog.type,
      content,
      metadata
    }).then((persistedLog) => {
      const index = logs.value.findIndex(log => log.id === tempLog.id)
      if (index < 0) {
        return
      }
      logs.value[index] = toPlanSplitLogs([persistedLog])[0]
    }).catch((error) => {
      logger.warn('[TaskSplit] Failed to persist auto-retry progress log:', error)
    })
  }

  function scheduleAutoRetry() {
    if (!context.value || !session.value || session.value.status !== 'failed') {
      return
    }

    const retryableFailure = classifyCliFailureFragments('Plan Split CLI', [
      createCliFailureFragment('error', session.value.errorMessage),
      createCliFailureFragment('error', session.value.parseError),
      createCliFailureFragment('content', session.value.rawContent)
    ].filter((item): item is NonNullable<typeof item> => Boolean(item)))
    if (!retryableFailure || retryableFailure.kind !== 'retryable') {
      autoRetryScheduled.value = false
      autoRetryNextRunAt.value = null
      return
    }

    const settingsStore = useSettingsStore()
    const maxRetries = settingsStore.settings.cliFailureMaxRetries ?? 5

    if (autoRetryCount.value >= maxRetries) {
      autoRetryScheduled.value = false
      return
    }

    autoRetryScheduled.value = true
    autoRetryCount.value += 1
    const retryNumber = autoRetryCount.value
    const delaySeconds = Math.ceil(PLAN_SPLIT_AUTO_RETRY_DELAY_MS / 1000)
    autoRetryNextRunAt.value = Date.now() + PLAN_SPLIT_AUTO_RETRY_DELAY_MS

    upsertAutoRetryProgressLog(retryNumber, maxRetries, delaySeconds)

    autoRetryTimer.value = setTimeout(async () => {
      autoRetryScheduled.value = false
      autoRetryTimer.value = null
      autoRetryNextRunAt.value = null
      if (!context.value || !session.value || session.value.status !== 'failed') {
        return
      }
      try {
        await restartFromPersistedContext(session.value, {
          preserveAutoRetryState: true
        })
      } catch (error) {
        logger.error('[TaskSplit] Auto-retry failed:', error)
      }
    }, PLAN_SPLIT_AUTO_RETRY_DELAY_MS)
  }

  function applySessionSnapshot(snapshot: PlanSplitSessionRecord | null, preserveLogs: boolean = true) {
    if (snapshot && !isCurrentPlan(snapshot.planId)) {
      return
    }

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
    const parsedResult = recoverSplitResultFromSnapshot(snapshot, logs.value)
    if (parsedResult) {
      splitResult.value = parsedResult
    } else if (!refinementState.value) {
      splitResult.value = null
    }
    const shouldKeepFormQueue = snapshot.status === 'waiting_input'
    const nextFormQueue = shouldKeepFormQueue
      ? recoverFormQueueFromSnapshot(snapshot, logs.value)
      : []
    formQueue.value = nextFormQueue
    currentFormIndex.value = nextFormQueue.length > 0
      ? Math.min(snapshot.currentFormIndex ?? 0, nextFormQueue.length - 1)
      : 0
    currentFormId.value = formQueue.value[currentFormIndex.value]?.formId ?? null
    isProcessing.value = snapshot.status === 'running'

    if (['completed', 'failed', 'stopped'].includes(snapshot.status)) {
      recordPlanSplitUsage(snapshot)
    }

    if (snapshot.status === 'completed' || snapshot.status === 'stopped') {
      cancelAutoRetry()
      autoRetryCount.value = 0
    }

    if (snapshot.status === 'failed' && context.value && !autoRetryScheduled.value) {
      scheduleAutoRetry()
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
    const loadStartedAt = performance.now()
    startLogLoadingDelay()
    measurePlanSplitPoint('loadSession:start', { planId })
    const logsInvokeStartedAt = performance.now()
    const splitLogsPromise = invoke<RustPlanSplitLogRecord[]>('list_plan_split_logs', { planId })
      .then((splitLogs) => {
        measurePlanSplit('ipc:list_plan_split_logs', logsInvokeStartedAt, {
          planId,
          count: splitLogs.length,
          bytes: sumPlanSplitLogBytes(splitLogs)
        })
        return splitLogs
      })
      .catch((error) => {
        measurePlanSplit('ipc:list_plan_split_logs:error', logsInvokeStartedAt, {
          planId,
          error: String(error)
        })
        return []
      })
    const sessionInvokeStartedAt = performance.now()
    const sessionPromise = invoke<PlanSplitSessionRecord | null>('get_plan_split_session', { planId })
    const [snapshot, splitLogs] = await Promise.all([sessionPromise, splitLogsPromise])
    measurePlanSplit('ipc:get_plan_split_session', sessionInvokeStartedAt, {
      planId,
      status: snapshot?.status ?? null,
      messageBytes: snapshot?.messagesJson?.length ?? 0,
      rawBytes: snapshot?.rawContent?.length ?? 0,
      resultBytes: snapshot?.resultJson?.length ?? 0
    })

    if (!isCurrentPlan(planId)) {
      stopLogLoadingDelay()
      isLoadingLogs.value = false
      measurePlanSplit('loadSession:stale', loadStartedAt, { planId })
      return
    }

    const applySnapshotStartedAt = performance.now()
      logs.value = toPlanSplitLogs(splitLogs)
      stopLogLoadingDelay()
      isLoadingLogs.value = false
      applySessionSnapshot(snapshot, true)
      bumpRenderVersion()
    measurePlanSplit('applySessionSnapshot:initial', applySnapshotStartedAt, {
      planId,
      messages: messages.value.length,
      logs: logs.value.length,
      hasResult: Boolean(splitResult.value?.length)
    })
    measurePlanSplit('loadSession:done', loadStartedAt, {
      planId,
      logs: splitLogs.length,
      bytes: sumPlanSplitLogBytes(splitLogs),
      messages: messages.value.length,
      recoveredTasks: splitResult.value?.length ?? 0,
      isLoadingLogs: isLoadingLogs.value
    })
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
      if (!isCurrentPlan(payload.planId)) {
        return
      }

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
      preserveAutoRetryState?: boolean
      resumeSessionId?: string
      preserveResult?: boolean
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
    cancelAutoRetry()
    if (!options?.preserveAutoRetryState) {
      autoRetryCount.value = 0
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
      mcpServers.length > 0 ? mcpServers : undefined,
      options?.resumeSessionId
    )
    const snapshot = await invoke<PlanSplitSessionRecord>('start_plan_split', {
      input: {
        planId: nextContext.planId,
        granularity: nextContext.granularity,
        taskCountMode: nextContext.taskCountMode ?? 'min',
        executionRequest,
        llmMessages,
        messages: uiMessages,
        preserveResult: options?.preserveResult ?? false
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
    const initStartedAt = performance.now()
    const version = ++initVersion
    const previousPlanId = context.value?.planId
    measurePlanSplitPoint('initSession:start', {
      planId: nextContext.planId,
      previousPlanId: previousPlanId ?? null
    })

    if (previousPlanId && previousPlanId !== nextContext.planId) {
      saveCurrentPlanState()
    }

    detachStream()
    cancelAutoRetry()

    const planChanged = previousPlanId !== nextContext.planId
    const restoreStartedAt = performance.now()
    const restoredFromCache = planChanged && restorePlanState(nextContext.planId)
    measurePlanSplit('initSession:restore_cache', restoreStartedAt, {
      planId: nextContext.planId,
      planChanged,
      restoredFromCache
    })

    if (planChanged) {
      const restored = restoredFromCache
      if (!restored) {
        resetState()
      }
    }

    context.value = nextContext
    runtimeNotices.value = []
    usageModelHint.value = null
    const selectedAgent = useAgentStore().agents.find(agent => agent.id === nextContext.agentId)
    if (selectedAgent) {
      const selectedExpert = resolveExpertById(nextContext.expertId, useAgentTeamsStore().experts)
      const contextNotice = buildContextStrategyNotice({
        strategy: 'Plan Split Prompt Context',
        runtime: inferAgentProvider(selectedAgent)?.toUpperCase() || selectedAgent.type,
        model: nextContext.modelId || selectedAgent.modelId || usageModelHint.value || undefined,
        expert: selectedExpert?.name || nextContext.expertId,
        systemMessageCount: 1,
        userMessageCount: 1,
        assistantMessageCount: 0,
        historyMessageCount: 0
      })
      runtimeNotices.value = contextNotice ? [contextNotice] : []

      void resolveUsageModelHint(selectedAgent)
        .catch(() => undefined)
        .then((modelHint) => {
        if (version !== initVersion || !isCurrentPlan(nextContext.planId)) return
        usageModelHint.value = modelHint ?? null
        runtimeNotices.value = contextNotice ? [contextNotice] : []
      })
    }
    if (version !== initVersion) return

    await subscribeToPlan(nextContext.planId)
    measurePlanSplit('initSession:subscribe_done', initStartedAt, { planId: nextContext.planId })
    if (version !== initVersion) return

    const sessionLoad = loadSession(nextContext.planId)
    if (restoredFromCache) {
      void sessionLoad
    } else {
      await sessionLoad
    }
    measurePlanSplit('initSession:loadSession_returned', initStartedAt, {
      planId: nextContext.planId,
      restoredFromCache,
      messages: messages.value.length,
      logs: logs.value.length,
      isLoadingLogs: isLoadingLogs.value
    })
    if (version !== initVersion) return

    await recoverStaleRunningSession(nextContext.planId)
    measurePlanSplit('initSession:recover_done', initStartedAt, { planId: nextContext.planId })
    if (version !== initVersion) return

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

    if (version !== initVersion) return
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
   * 基于当前会话上下文重新发起最新一轮拆分，同时保留历史轮次的消息与日志渲染。
   */
  async function restartFromPersistedContext(
    snapshot: PlanSplitSessionRecord,
    options?: {
      preserveAutoRetryState?: boolean
      extraUserPrompt?: string
      extraUserAttachments?: MessageAttachment[]
      displayContent?: string
      trimLatestTurn?: boolean
    }
  ) {
    if (!context.value) {
      return
    }

    if (!options?.preserveAutoRetryState) {
      cancelAutoRetry()
      autoRetryCount.value = 0
    }

    const resumeSessionId = parseJson<ExecutionRequest | null>(
      snapshot.executionRequestJson ?? null,
      null
    )?.resumeSessionId?.trim() || undefined

    let llmMessages = trimTrailingAssistantMessages(
      parseJson<MessageInput[]>(snapshot.llmMessagesJson, [])
    )
    let uiMessages = toSplitMessages(snapshot.messagesJson)
    const latestExecutionSessionId = snapshot.executionSessionId?.trim() || null
    const latestUserTimestamp = [...uiMessages]
      .filter(message => message.role === 'user')
      .map(message => message.timestamp)
      .filter(Boolean)
      .slice(-1)[0] ?? null
    if (llmMessages.length === 0) {
      throw new Error('当前会话缺少可重试的上下文。')
    }

    const extraUserPrompt = options?.extraUserPrompt?.trim()
    const extraUserAttachments = options?.extraUserAttachments?.length
      ? options.extraUserAttachments
      : undefined
    const trimLatestTurn = options?.trimLatestTurn ?? !(extraUserPrompt || extraUserAttachments?.length)

    if (trimLatestTurn) {
      uiMessages = trimTrailingAssistantMessages(uiMessages)
      logs.value = trimPlanSplitLogsAfterTimestamp(
        trimPlanSplitLogsBySessionId(logs.value, latestExecutionSessionId),
        latestUserTimestamp
      )
    }

    if (extraUserPrompt || extraUserAttachments?.length) {
      llmMessages = [
        ...llmMessages,
        {
          role: 'user',
          content: extraUserPrompt || '请结合附件继续调整当前任务拆分结果。',
          attachments: extraUserAttachments
        }
      ]
      uiMessages = [
        ...uiMessages,
        {
          id: crypto.randomUUID(),
          role: 'user',
          content: options?.displayContent?.trim() || extraUserPrompt || '',
          attachments: extraUserAttachments,
          timestamp: new Date().toISOString()
        }
      ]
    }

    const restartedAt = new Date().toISOString()
    const resetSnapshot = await invoke<PlanSplitSessionRecord>('reset_plan_split_turn_for_restart', {
      planId: context.value.planId,
      options: {
        trimLatestTurn,
        preserveResult: Boolean(splitResult.value?.length)
      }
    })

    const splitLogs = await invoke<RustPlanSplitLogRecord[]>('list_plan_split_logs', {
      planId: context.value.planId
    }).catch(() => null)
    if (splitLogs) {
      const nextLogs = toPlanSplitLogs(splitLogs)
      logs.value = trimLatestTurn
        ? trimPlanSplitLogsAfterTimestamp(
            trimPlanSplitLogsBySessionId(nextLogs, latestExecutionSessionId),
            latestUserTimestamp
          )
        : nextLogs
    }

    applySessionSnapshot({
      ...resetSnapshot,
      startedAt: restartedAt,
      updatedAt: restartedAt,
      messagesJson: JSON.stringify(uiMessages),
      llmMessagesJson: JSON.stringify(llmMessages)
    }, true)

    await startBackgroundSession(context.value, llmMessages, uiMessages, {
      preserveLogs: true,
      preserveAutoRetryState: options?.preserveAutoRetryState,
      resumeSessionId,
      preserveResult: Boolean(splitResult.value?.length)
    })
  }

  async function retry() {
    if (!session.value) {
      return
    }

    await restartFromPersistedContext(session.value, {
      trimLatestTurn: true
    })
  }

  async function continueSession() {
    if (!session.value || session.value.status !== 'stopped') {
      return
    }

    await restartFromPersistedContext(session.value, {
      trimLatestTurn: true
    })
  }

  /**
   * 在保留上一轮上下文的前提下追加一条用户纠偏指令，并继续当前拆分会话。
   */
  async function continueSessionWithInstruction(
    text: string,
    options: ContinueSessionWithInstructionOptions = {}
  ) {
    if (!session.value || (session.value.status !== 'stopped' && session.value.status !== 'failed')) {
      return
    }

    const normalizedText = text.trim()
    const attachments = options.attachments?.length ? options.attachments : undefined
    if (!normalizedText && !attachments?.length) {
      return
    }

    await restartFromPersistedContext(session.value, {
      extraUserPrompt: normalizedText,
      extraUserAttachments: attachments,
      displayContent: options.displayContent ?? normalizedText,
      trimLatestTurn: false
    })
  }

  async function stop() {
    if (!context.value) return
    const snapshot = await invoke<PlanSplitSessionRecord | null>('stop_plan_split', {
      planId: context.value.planId
    })
    applySessionSnapshot(snapshot, true)
    const splitLogs = await invoke<RustPlanSplitLogRecord[]>('list_plan_split_logs', {
      planId: context.value.planId
    }).catch(() => null)
    if (splitLogs) {
      logs.value = toPlanSplitLogs(splitLogs)
    }
  }

  async function clearAllSplitData(planId: string) {
    await invoke('clear_plan_split_session', { planId })
    planStateCache.delete(planId)
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
        }),
        attachments: config.attachments
      }
    ]

    const preservedMessages = messages.value.filter(m => m.content.trim())
    const displayContent = config.displayContent?.trim()
    const uiMessages: SplitMessage[] = [
      ...preservedMessages,
      {
        id: crypto.randomUUID(),
        role: 'user',
        content: displayContent || [
          `继续拆分任务：${targetTask.title}`,
          `原任务描述：${targetTask.description || '（无）'}`,
          `拆分颗粒度：至少 ${config.granularity} 个子任务`,
          config.customPrompt ? `额外要求：${config.customPrompt}` : ''
        ].filter(Boolean).join('\n'),
        attachments: config.attachments,
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

    await startBackgroundSession(nextContext, llmMessages, uiMessages, {
      preserveLogs: true,
      preserveResult: true
    })
  }

  /**
   * 对当前整份任务列表发起整体优化，保留任务数量不变。
   */
  async function startListOptimize(config: TaskListOptimizeConfig) {
    if (!context.value || !splitResult.value || splitResult.value.length === 0) return

    const originalTasks = JSON.parse(JSON.stringify(splitResult.value)) as AITaskItem[]
    const taskCountMode = config.taskCountMode ?? 'exact'
    const minTaskCount = taskCountMode === 'exact'
      ? originalTasks.length
      : Math.max(1, config.minTaskCount ?? 1)
    refinementState.value = {
      mode: 'list_optimize',
      targetIndex: null,
      originalTasks,
      config
    }

    const nextContext: TaskSplitContext = {
      ...context.value,
      granularity: minTaskCount,
      expertId: config.expertId || context.value.expertId,
      agentId: config.agentId || context.value.agentId,
      modelId: config.modelId || context.value.modelId,
      taskCountMode
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
          targetIndex: config.targetIndex,
          taskCountMode,
          minTaskCount
        }),
        attachments: config.attachments
      }
    ]

    const preservedMessages = messages.value.filter(m => m.content.trim())
    const displayContent = config.displayContent?.trim()
    const targetLabel = config.targetIndex !== undefined && config.targetIndex >= 0 && config.targetIndex < originalTasks.length
      ? `优化任务 ${config.targetIndex + 1}《${originalTasks[config.targetIndex].title}》`
      : `整体优化任务列表：共 ${originalTasks.length} 个任务`
    const uiMessages: SplitMessage[] = [
      ...preservedMessages,
      {
        id: crypto.randomUUID(),
        role: 'user',
        content: displayContent || [
          targetLabel,
          taskCountMode === 'exact'
            ? '约束：保持任务数量不变'
            : `约束：允许调整任务数量，至少保留 ${minTaskCount} 个任务`,
          config.customPrompt ? `额外要求：${config.customPrompt}` : ''
        ].filter(Boolean).join('\n'),
        attachments: config.attachments,
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

    await startBackgroundSession(nextContext, llmMessages, uiMessages, {
      preserveLogs: true,
      preserveResult: true
    })
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
    planStateCache.clear()
  }

  function detach() {
    saveCurrentPlanState()
    detachStream()
    isProcessing.value = false
  }

  function attachToPlan(planId: string) {
    if (context.value?.planId === planId) return
    saveCurrentPlanState()
    detachStream()
    cancelAutoRetry()
    if (!restorePlanState(planId)) {
      resetState()
    }
  }

  return {
    messages,
    logs,
    isLoadingLogs,
    renderVersion,
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
    continueSessionWithInstruction,
    stop,
    autoRetryCount,
    autoRetryScheduled,
    autoRetryNextRunAt,
    cancelAutoRetry,
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
    detachStream,
    attachToPlan,
    saveCurrentPlanState,
    clearPlanStateCache,
    loadSession
  }
})
