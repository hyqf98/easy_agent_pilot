import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useOverlayDismiss } from '@/composables/useOverlayDismiss'
import { DEFAULT_SPLIT_GRANULARITY } from '@/constants/plan'
import { inferAgentProvider, useAgentStore } from '@/stores/agent'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import type { Message, MessageAttachment, MessageStatus, ToolCall } from '@/stores/message'
import { useNotificationStore } from '@/stores/notification'
import { usePlanStore } from '@/stores/plan'
import { useProjectStore } from '@/stores/project'
import { useTaskSplitStore } from '@/stores/taskSplit'
import { useTaskStore } from '@/stores/task'
import { useThemeStore } from '@/stores/theme'
import type {
  DynamicFormSchema,
  PlanSplitLogRecord
} from '@/types/plan'
import { logger } from '@/utils/logger'
import { resolveAttachmentPreviewUrl } from '@/utils/attachmentPreview'
import type { RuntimeNotice } from '@/utils/runtimeNotice'
import {
  buildProcessingTimeNotice,
  buildRuntimeNoticeFromSystemContent,
  buildUsageNotice,
  isEnvironmentRuntimeNotice,
  upsertRuntimeNotice
} from '@/utils/runtimeNotice'
import { normalizeRuntimeUsage } from '@/utils/runtimeUsage'
import { clearMessageListSessionState } from '@/components/message/messageList/useMessageList'
import {
  containsFormSchema,
  extractFormResponse,
  extractFirstFormRequest,
  extractTaskSplitResult
} from '@/utils/structuredContent'
import {
  materializeTaskMentions,
  normalizeTaskInstructionInput,
  parseTaskInstruction
} from '@/utils/taskInstructionParser'
import { resolveExpertById, resolveExpertRuntime } from '@/services/agentTeams/runtime'

type PendingPlanAttachment = MessageAttachment & { previewUrl: string }

interface MentionOption {
  index: number
  title: string
}

interface MentionState {
  query: string
  rangeStart: number
  rangeEnd: number
  options: MentionOption[]
  key: string
}

interface UploadAttachmentInput {
  fileName?: string
  mimeType: string
  bytes: number[]
}

interface UploadAttachmentsResponse {
  attachments: MessageAttachment[]
}

interface SplitTurn {
  userMessage: {
    id: string
    content: string
    attachments?: MessageAttachment[]
    timestamp: string
  }
  assistantSummary?: {
    id: string
    content: string
    timestamp: string
  }
  sessionId?: string
  logs: PlanSplitLogRecord[]
}

function compareTimestamp(left?: string, right?: string) {
  return new Date(left || 0).getTime() - new Date(right || 0).getTime()
}

function parseLogMetadata(log: PlanSplitLogRecord): Record<string, unknown> | null {
  if (!log.metadata) {
    return null
  }

  if (typeof log.metadata !== 'string') {
    return log.metadata as unknown as Record<string, unknown>
  }

  try {
    const parsed = JSON.parse(log.metadata) as unknown
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
      ? parsed as Record<string, unknown>
      : null
  } catch {
    return null
  }
}

function trimContent(value?: string | null) {
  return (value ?? '').trim()
}

function buildResultSummary(content: string) {
  const result = extractTaskSplitResult(content)
  if (!result?.tasks?.length) {
    return null
  }

  return {
    summary: result.summary?.trim() || '',
    count: result.tasks.length,
    tasks: result.tasks
  }
}

function buildAssistantDisplayContent(content: string) {
  const summary = buildResultSummary(content)
  if (summary) {
    return summary.summary || `任务拆分完成，共生成 ${summary.count} 个任务。`
  }

  return content.trim()
}

function estimateTokenCountFromText(content?: string | null): number | undefined {
  const normalized = trimContent(content)?.replace(/\s+/g, ' ')
  if (!normalized) {
    return undefined
  }

  let hanCharCount = 0
  let otherCharCount = 0

  for (const char of normalized) {
    if (/[\u3400-\u9fff]/.test(char)) {
      hanCharCount += 1
    } else {
      otherCharCount += 1
    }
  }

  const estimatedTokens = Math.round(hanCharCount + otherCharCount / 4)
  return estimatedTokens > 0 ? estimatedTokens : undefined
}

function buildFormRequestContent(question: string, forms: DynamicFormSchema[]) {
  return JSON.stringify({
    type: 'form_request',
    question,
    forms
  }, null, 2)
}

function buildFormResponseContent(formId: string, values: Record<string, unknown>) {
  return JSON.stringify({
    type: 'form_response',
    formId,
    values
  }, null, 2)
}

function parseToolArguments(raw: unknown): Record<string, unknown> {
  if (typeof raw !== 'string') {
    return {}
  }

  const content = raw.trim()
  if (!content) {
    return {}
  }

  try {
    const parsed = JSON.parse(content) as unknown
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
      ? parsed as Record<string, unknown>
      : { input: parsed }
  } catch {
    return { input: content }
  }
}

function resolveToolName(metadata: Record<string, unknown> | null) {
  const toolName = metadata?.toolName
  return typeof toolName === 'string' && toolName.trim()
    ? toolName.trim()
    : 'Tool'
}

function resolveToolCallId(log: PlanSplitLogRecord, metadata: Record<string, unknown> | null) {
  const toolCallId = metadata?.toolCallId
  return typeof toolCallId === 'string' && toolCallId.trim()
    ? toolCallId.trim()
    : `tool-${log.id}`
}

function toToolCall(log: PlanSplitLogRecord, metadata: Record<string, unknown> | null): ToolCall {
  const toolInput = typeof metadata?.toolInput === 'string' && metadata.toolInput.trim()
    ? metadata.toolInput.trim()
    : trimContent(log.content)

  return {
    id: resolveToolCallId(log, metadata),
    name: resolveToolName(metadata),
    arguments: parseToolArguments(toolInput),
    status: 'running'
  }
}

function finalizeToolCalls(toolCalls: ToolCall[], status: MessageStatus): ToolCall[] {
  if (status === 'streaming') {
    return toolCalls
  }

  return toolCalls.map((toolCall) => {
    if (toolCall.status !== 'running') {
      return toolCall
    }

    return {
      ...toolCall,
      status: status === 'error' ? 'error' : 'success',
      errorMessage: status === 'error'
        ? toolCall.errorMessage || '消息执行失败'
        : toolCall.errorMessage
    }
  })
}

function resolveMessageStatus(status: MessageStatus, fallback: MessageStatus = 'completed') {
  return status || fallback
}

function readMetadataString(metadata: Record<string, unknown> | null, key: string): string | undefined {
  const value = metadata?.[key]
  return typeof value === 'string' && value.trim()
    ? value.trim()
    : undefined
}

function readMetadataNumber(
  metadata: Record<string, unknown> | null,
  key: string,
  fallbackKey?: string
): number | undefined {
  const value = metadata?.[key] ?? (fallbackKey ? metadata?.[fallbackKey] : undefined)
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value
  }
  if (typeof value === 'string' && value.trim()) {
    const numeric = Number(value)
    return Number.isFinite(numeric) ? numeric : undefined
  }
  return undefined
}

interface TaskSplitUsageSnapshot {
  model?: string
  inputTokens?: number
  outputTokens?: number
}

function buildUsageSnapshotFromMetadata(
  metadata: Record<string, unknown> | null,
  fallbackModel?: string
): TaskSplitUsageSnapshot | null {
  const inputTokens = readMetadataNumber(metadata, 'inputTokens', 'input_tokens')
  const outputTokens = readMetadataNumber(metadata, 'outputTokens', 'output_tokens')
  const rawInputTokens = readMetadataNumber(metadata, 'rawInputTokens', 'raw_input_tokens')
  const rawOutputTokens = readMetadataNumber(metadata, 'rawOutputTokens', 'raw_output_tokens')
  const model = readMetadataString(metadata, 'model') ?? fallbackModel
  const resolvedInputTokens = inputTokens ?? rawInputTokens
  const resolvedOutputTokens = outputTokens ?? rawOutputTokens

  if (resolvedInputTokens === undefined && resolvedOutputTokens === undefined) {
    return null
  }

  return {
    model,
    inputTokens: resolvedInputTokens,
    outputTokens: resolvedOutputTokens
  }
}

function buildAssistantRuntimeNotices(
  logs: PlanSplitLogRecord[],
  fallbackModel?: string,
  runtimeProvider?: string,
  fallbackAssistantContent?: string
): RuntimeNotice[] {
  if (logs.length === 0) {
    return []
  }

  let notices: RuntimeNotice[] | undefined
  let model = fallbackModel?.trim() || undefined
  let totalInputTokens = 0
  let totalOutputTokens = 0
  let hasUsage = false
  let usageBaseline: ReturnType<typeof normalizeRuntimeUsage>['nextBaseline'] = null
  let latestUsageSnapshot: TaskSplitUsageSnapshot | null = null

  const sortedLogs = [...logs].sort((left, right) => compareTimestamp(left.createdAt, right.createdAt))

  for (const log of sortedLogs) {
    const metadata = parseLogMetadata(log)
    const rawContent = trimContent(log.content)
    const metadataModel = readMetadataString(metadata, 'model')
    if (metadataModel) {
      model = metadataModel
    }

    const usageSnapshot = buildUsageSnapshotFromMetadata(metadata, model)
    if (usageSnapshot) {
      latestUsageSnapshot = usageSnapshot
    }

    if (log.type === 'usage' || log.type === 'message_start') {
      const normalizedUsage = normalizeRuntimeUsage({
        provider: runtimeProvider,
        inputTokens: readMetadataNumber(metadata, 'inputTokens', 'input_tokens'),
        outputTokens: readMetadataNumber(metadata, 'outputTokens', 'output_tokens'),
        rawInputTokens: readMetadataNumber(metadata, 'rawInputTokens', 'raw_input_tokens'),
        rawOutputTokens: readMetadataNumber(metadata, 'rawOutputTokens', 'raw_output_tokens'),
        cacheReadInputTokens: readMetadataNumber(metadata, 'cacheReadInputTokens', 'cache_read_input_tokens'),
        cacheCreationInputTokens: readMetadataNumber(metadata, 'cacheCreationInputTokens', 'cache_creation_input_tokens'),
        baseline: usageBaseline
      })

      if (normalizedUsage.nextBaseline) {
        usageBaseline = normalizedUsage.nextBaseline
      } else {
        usageBaseline = null
      }

      if (typeof normalizedUsage.inputTokens === 'number') {
        totalInputTokens += normalizedUsage.inputTokens
        hasUsage = true
      }

      if (typeof normalizedUsage.outputTokens === 'number') {
        totalOutputTokens += normalizedUsage.outputTokens
        hasUsage = true
      }
      continue
    }

    if (log.type === 'system' && rawContent) {
      const runtimeNotice = buildRuntimeNoticeFromSystemContent(rawContent)
      if (runtimeNotice && !isEnvironmentRuntimeNotice(runtimeNotice)) {
        notices = upsertRuntimeNotice(notices, runtimeNotice)
      }
    }
  }

  const fallbackUsageModel = latestUsageSnapshot?.model ?? model
  const fallbackInputTokens = latestUsageSnapshot?.inputTokens
  const fallbackOutputTokens = latestUsageSnapshot?.outputTokens
  const estimatedOutputTokens = estimateTokenCountFromText(fallbackAssistantContent)
  const resolvedInputTokens = hasUsage ? totalInputTokens : fallbackInputTokens
  const resolvedOutputTokens = hasUsage
    ? (totalOutputTokens > 0 ? totalOutputTokens : fallbackOutputTokens || estimatedOutputTokens)
    : (fallbackOutputTokens || estimatedOutputTokens)
  const shouldShowUsageNotice = latestUsageSnapshot !== null
    || resolvedInputTokens !== undefined
    || resolvedOutputTokens !== undefined

  if (shouldShowUsageNotice) {
    notices = upsertRuntimeNotice(notices, buildUsageNotice({
      model: hasUsage ? model : fallbackUsageModel,
      inputTokens: resolvedInputTokens,
      outputTokens: resolvedOutputTokens
    }))
  }

  const startedAt = sortedLogs[0]?.createdAt
  const finishedAt = sortedLogs[sortedLogs.length - 1]?.createdAt
  const durationMs = startedAt && finishedAt
    ? Math.max(0, new Date(finishedAt).getTime() - new Date(startedAt).getTime())
    : null
  notices = upsertRuntimeNotice(notices, buildProcessingTimeNotice(durationMs))

  return notices ?? []
}

function formatFormOptionValue(field: DynamicFormSchema['fields'][number], value: unknown): string {
  if (value === undefined || value === null || value === '') {
    return '-'
  }

  if (Array.isArray(value)) {
    if (value.length === 0) {
      return '-'
    }

    return value.map(item => formatFormOptionValue(field, item)).join('、')
  }

  const matched = field.options?.find(option => option.value === value)
  return matched ? matched.label : String(value)
}

function summarizeFormValues(schema: DynamicFormSchema, values: Record<string, unknown>): string {
  return schema.fields
    .map(field => `${field.label}：${formatFormOptionValue(field, values[field.name])}`)
    .join('\n')
}

function parseSummaryLineMap(content: string): Map<string, string> {
  const lineMap = new Map<string, string>()

  content
    .split('\n')
    .map(line => line.trim())
    .filter(Boolean)
    .forEach((line) => {
      const separatorIndex = line.indexOf('：')
      const fallbackSeparatorIndex = separatorIndex >= 0 ? separatorIndex : line.indexOf(':')
      if (fallbackSeparatorIndex <= 0) {
        return
      }

      const label = line.slice(0, fallbackSeparatorIndex).trim()
      const value = line.slice(fallbackSeparatorIndex + 1).trim()
      if (!label) {
        return
      }

      lineMap.set(label, value)
    })

  return lineMap
}

function parseSummaryValue(
  field: DynamicFormSchema['fields'][number],
  rawValue: string
): unknown {
  const normalizedValue = rawValue.trim()
  const isEmptyValue = normalizedValue === '' || normalizedValue === '-'

  switch (field.type) {
    case 'checkbox':
      return ['true', '1', 'yes', 'on', '是'].includes(normalizedValue.toLowerCase())
    case 'multiselect':
      if (isEmptyValue) {
        return []
      }
      return normalizedValue
        .split('、')
        .map(item => item.trim())
        .filter(Boolean)
        .map((item) => field.options?.find(option => option.label === item)?.value ?? item)
    case 'select':
    case 'radio':
      if (isEmptyValue) {
        return ''
      }
      return field.options?.find(option => option.label === normalizedValue)?.value ?? normalizedValue
    case 'number':
    case 'slider': {
      if (isEmptyValue) {
        return undefined
      }
      const numericValue = Number(normalizedValue)
      return Number.isFinite(numericValue) ? numericValue : undefined
    }
    default:
      return isEmptyValue ? '' : normalizedValue
  }
}

function reconstructSubmittedFormFromSummary(
  content: string,
  schema: DynamicFormSchema
): Record<string, unknown> | null {
  const lineMap = parseSummaryLineMap(content)
  if (lineMap.size === 0) {
    return null
  }

  const values: Record<string, unknown> = {}

  for (const field of schema.fields) {
    if (!lineMap.has(field.label)) {
      return null
    }

    const parsedValue = parseSummaryValue(field, lineMap.get(field.label) ?? '')
    if (field.required) {
      const isMissingRequired = parsedValue === undefined
        || parsedValue === null
        || parsedValue === ''
        || (Array.isArray(parsedValue) && parsedValue.length === 0)
      if (isMissingRequired) {
        return null
      }
    }

    values[field.name] = parsedValue
  }

  return values
}

function findSubmittedFormFromTranscript(
  turns: SplitTurn[],
  turnIndex: number
): { formId: string; values: Record<string, unknown> } | null {
  const currentTurn = turns[turnIndex]
  if (!currentTurn) {
    return null
  }

  if (extractFormResponse(currentTurn.userMessage.content)) {
    return null
  }

  for (let previousIndex = turnIndex - 1; previousIndex >= 0; previousIndex -= 1) {
    const previousAssistantContent = trimContent(turns[previousIndex]?.assistantSummary?.content)
    if (!previousAssistantContent) {
      continue
    }

    const formRequest = extractFirstFormRequest(previousAssistantContent)
    const forms = formRequest?.forms ?? []
    for (const schema of forms) {
      const values = reconstructSubmittedFormFromSummary(currentTurn.userMessage.content, schema)
      if (values) {
        return {
          formId: schema.formId,
          values
        }
      }
    }
  }

  return null
}

function collectTurnSessionIds(logs: PlanSplitLogRecord[]): string[] {
  const uniqueSessionIds: string[] = []
  const seen = new Set<string>()

  logs
    .filter(log => trimContent(log.sessionId))
    .sort((left, right) => compareTimestamp(left.createdAt, right.createdAt))
    .forEach((log) => {
      const sessionId = trimContent(log.sessionId)
      if (!sessionId || seen.has(sessionId)) {
        return
      }

      seen.add(sessionId)
      uniqueSessionIds.push(sessionId)
    })

  return uniqueSessionIds
}

export function useTaskSplitDialog() {
  const planStore = usePlanStore()
  const taskSplitStore = useTaskSplitStore()
  const taskStore = useTaskStore()
  const projectStore = useProjectStore()
  const themeStore = useThemeStore()
  const agentStore = useAgentStore()
  const agentTeamsStore = useAgentTeamsStore()
  const notificationStore = useNotificationStore()
  const { t } = useI18n()

  const isDarkTheme = computed(() => themeStore.isDark)
  const splitDialogVisible = computed(() => planStore.splitDialogVisible)
  const activeSplitPlanId = computed(() => planStore.activeSplitPlanId)
  const splitDialogTabs = computed(() =>
    planStore.splitDialogPlanIds.map(planId => {
      const plan = planStore.plans.find(item => item.id === planId)
      return {
        planId,
        label: plan?.name || planId.slice(0, 8)
      }
    })
  )

  const isConfirming = ref(false)
  const isInitializingSplitSession = ref(false)
  const splitChatScrollToken = ref(0)
  const queuedPlanId = ref<string | null>(null)
  const userInstruction = ref('')
  const instructionInputRef = ref<HTMLTextAreaElement | null>(null)
  const fileInputRef = ref<HTMLInputElement | null>(null)
  const instructionSelectionStart = ref(0)
  const selectedMentionOptionIndex = ref(0)
  const mentionDismissKey = ref('')
  const pendingAttachments = ref<PendingPlanAttachment[]>([])
  const isUploadingAttachments = ref(false)
  const showCloseConfirm = ref(false)

  const showPreview = computed(() => taskSplitStore.splitResult !== null)
  const showRecoveryInput = computed(() =>
    !showPreview.value
    && taskSplitStore.session?.status === 'stopped'
    && Boolean(taskSplitStore.session?.parseError?.trim())
    && Boolean(taskSplitStore.session?.rawContent?.trim())
  )
  const refinementMode = computed(() => taskSplitStore.refinementMode)
  const hasPendingRefinement = computed(() => Boolean(refinementMode.value))
  const isSubSplitActive = computed(() => refinementMode.value === 'task_resplit')
  const subSplitTargetTitle = computed(() => {
    if (!isSubSplitActive.value || taskSplitStore.subSplitTargetIndex === null) {
      return ''
    }

    const originalTasks = taskSplitStore.refinementState?.originalTasks
    const index = taskSplitStore.subSplitTargetIndex
    return originalTasks?.[index]?.title ?? t('taskBoard.emptyNoTasks')
  })
  const isSessionRunning = computed(() => taskSplitStore.session?.status === 'running')
  const showStopButton = computed(() => isSessionRunning.value)
  const activeFormSchema = computed(() => taskSplitStore.activeFormSchema)
  const previewActionsDisabled = computed(() => isConfirming.value || isSessionRunning.value)
  const activePlanLogs = computed(() => taskSplitStore.logs)
  const activePlanMessages = computed(() => taskSplitStore.messages)
  const splitErrorMessage = computed(() =>
    taskSplitStore.session?.errorMessage?.trim()
    || taskSplitStore.session?.parseError?.trim()
    || ''
  )

  const mentionState = computed<MentionState | null>(() => {
    const tasks = taskSplitStore.splitResult
    if (!tasks?.length) {
      return null
    }

    const caret = instructionSelectionStart.value
    const beforeCaret = userInstruction.value.slice(0, caret)
    const mentionStart = beforeCaret.lastIndexOf('@')
    if (mentionStart < 0) {
      return null
    }

    const prefixChar = mentionStart > 0 ? beforeCaret[mentionStart - 1] : ''
    if (prefixChar && !/[\s([{：:，,]/.test(prefixChar)) {
      return null
    }

    const mentionText = beforeCaret.slice(mentionStart + 1)
    if (/\s/.test(mentionText) || mentionText.includes(']')) {
      return null
    }

    const query = mentionText.trim().toLowerCase()
    const options = tasks
      .map((task, index) => ({
        index,
        title: task.title || t('taskBoard.emptyNoTasks')
      }))
      .filter(option => !query
        || `${option.index + 1}`.includes(query)
        || option.title.toLowerCase().includes(query))
      .slice(0, 8)

    return {
      query,
      rangeStart: mentionStart,
      rangeEnd: caret,
      options,
      key: `${mentionStart}:${query}`
    }
  })

  const mentionSuggestions = computed(() => mentionState.value?.options ?? [])
  const showMentionSuggestions = computed(() => {
    if (isConfirming.value) {
      return false
    }

    if (!mentionState.value || mentionSuggestions.value.length === 0) {
      return false
    }

    return mentionDismissKey.value !== mentionState.value.key
  })

  const splitAgentLabel = computed(() => {
    const expert = resolveExpertById(taskSplitStore.context?.expertId, agentTeamsStore.experts)
    if (expert?.name) {
      return expert.name
    }

    const agent = agentStore.agents.find(item => item.id === taskSplitStore.context?.agentId)
    return agent?.name || t('taskSplit.dialogTitle')
  })

  const splitModelLabel = computed(() => {
    const configuredModel = taskSplitStore.context?.modelId?.trim()
    if (configuredModel) {
      return configuredModel
    }

    const selectedAgent = agentStore.agents.find(item => item.id === taskSplitStore.context?.agentId)
    return selectedAgent?.modelId || t('taskDetail.useDefaultModel')
  })

  const instructionDisabled = computed(() => {
    if (isConfirming.value) {
      return true
    }

    if (showPreview.value || showRecoveryInput.value || taskSplitStore.session?.status === 'running') {
      return false
    }

    const status = taskSplitStore.session?.status
    return status !== 'stopped' && status !== 'failed'
  })
  const canApplyRefinement = computed(() =>
    hasPendingRefinement.value && taskSplitStore.session?.status === 'completed'
  )
  const canRetrySplit = computed(() => taskSplitStore.session?.status === 'failed')
  const canContinueSplit = computed(() =>
    taskSplitStore.session?.status === 'stopped'
    && !activeFormSchema.value
    && (!showPreview.value || hasPendingRefinement.value)
  )
  const retryNow = ref(Date.now())
  let retryCountdownTimer: ReturnType<typeof setInterval> | null = null
  const isAutoRetryPending = computed(() =>
    canRetrySplit.value
    && taskSplitStore.autoRetryScheduled
    && typeof taskSplitStore.autoRetryNextRunAt === 'number'
  )
  const autoRetrySecondsRemaining = computed(() => {
    if (!isAutoRetryPending.value || typeof taskSplitStore.autoRetryNextRunAt !== 'number') {
      return 0
    }

    return Math.max(0, Math.ceil((taskSplitStore.autoRetryNextRunAt - retryNow.value) / 1000))
  })
  const retryButtonLabel = computed(() => (
    isAutoRetryPending.value
      ? t('taskSplit.autoRetryButtonPending', { seconds: autoRetrySecondsRemaining.value })
      : t('taskSplit.retryLabel')
  ))
  const primaryActionLabel = computed(() => {
    if (refinementMode.value === 'list_optimize') {
      return canApplyRefinement.value ? t('taskSplit.applyOptimizeResult') : t('taskSplit.waitingOptimizeResult')
    }
    if (refinementMode.value === 'task_resplit') {
      return canApplyRefinement.value ? t('taskSplit.applyResplitResult') : t('taskSplit.waitingResplitResult')
    }
    return isConfirming.value ? t('taskSplit.creating') : t('taskSplit.confirmCreate')
  })
  const footerHint = computed(() => {
    if (isAutoRetryPending.value) {
      return t('taskSplit.autoRetryHint', { seconds: autoRetrySecondsRemaining.value })
    }

    if (canRetrySplit.value) {
      return splitErrorMessage.value || t('taskSplit.hintRetryFailed')
    }

    if (taskSplitStore.session?.status === 'stopped') {
      if (splitErrorMessage.value) {
        return splitErrorMessage.value
      }
      if (activeFormSchema.value) {
        return t('taskSplit.hintSessionStoppedWithForm')
      }
      return t('taskSplit.hintSessionStopped')
    }

    if (taskSplitStore.session?.status === 'running') {
      return t('taskSplit.hintSessionRunning')
    }

    if (taskSplitStore.session?.status === 'waiting_input' && activeFormSchema.value) {
      return t('taskSplit.hintWaitingFormInput')
    }

    return t('taskSplit.instructionPlaceholder')
  })
  const runningStatusText = computed(() => {
    if (taskSplitStore.isLoadingLogs && activePlanLogs.value.length === 0) {
      return '正在加载历史拆分日志...'
    }

    if (!isSessionRunning.value) {
      return ''
    }

    return '正在生成拆分结果...'
  })

  const uploadSessionId = computed(() => `plan-split:${activeSplitPlanId.value || 'dialog'}`)
  const isSplitHistoryLoading = computed(() =>
    taskSplitStore.isLoadingLogs
    && activePlanLogs.value.length === 0
    && activePlanMessages.value.length === 0
  )
  const splitChatSessionId = computed(() =>
    `plan-split:${activeSplitPlanId.value || taskSplitStore.context?.planId || 'dialog'}`
  )

  function clearSplitDialogScrollState(planId?: string | null) {
    const targetPlanId = planId?.trim()
    if (!targetPlanId) {
      return
    }

    clearMessageListSessionState(`plan-split:${targetPlanId}`)
  }

  function requestSplitChatBottomAlign(planId?: string | null) {
    clearSplitDialogScrollState(planId)
    splitChatScrollToken.value += 1
  }

  const activeFormQuestion = computed(() => {
    const activeFormId = activeFormSchema.value?.formId
    if (!activeFormId) {
      return ''
    }

    for (let index = activePlanLogs.value.length - 1; index >= 0; index -= 1) {
      const formRequest = extractFirstFormRequest(activePlanLogs.value[index]?.content ?? '')
      const matchedForm = formRequest?.forms?.find(form => form.formId === activeFormId)
      if (matchedForm) {
        return formRequest?.question ?? ''
      }
    }

    return ''
  })

  const splitChatMessages = computed<Message[]>(() => {
    const sessionId = splitChatSessionId.value
    const sortedLogs = [...activePlanLogs.value].sort((left, right) => compareTimestamp(left.createdAt, right.createdAt))
    const submittedForms = [...taskSplitStore.submittedForms]
      .sort((left, right) => compareTimestamp(left.submittedAt, right.submittedAt))
    const orderedSessionIds = collectTurnSessionIds(sortedLogs)
    const turnMap = new Map<string, SplitTurn>()
    const assistantMessageIndexBySessionId = new Map<string, number>()
    const turns: SplitTurn[] = []
    const messages: Message[] = []
    const usageModelFallback = taskSplitStore.context?.modelId?.trim()
      || taskSplitStore.usageModelHint?.trim()
      || agentStore.agents.find(item => item.id === taskSplitStore.context?.agentId)?.modelId?.trim()
      || undefined
    const runtimeProvider = taskSplitStore.context?.agentId
      ? (inferAgentProvider(agentStore.agents.find(item => item.id === taskSplitStore.context?.agentId)) ?? undefined)
      : undefined

    activePlanMessages.value.forEach((message) => {
      if (message.role === 'user') {
        const content = trimContent(message.content)
        if (!content) {
          return
        }

        const turnIndex = turns.length
        const turn: SplitTurn = {
          userMessage: {
            id: message.id,
            content,
            attachments: message.attachments,
            timestamp: message.timestamp
          },
          sessionId: orderedSessionIds[turnIndex],
          logs: []
        }

        turns.push(turn)
        if (turn.sessionId) {
          turnMap.set(turn.sessionId, turn)
        }

        return
      }

      const latestTurn = turns[turns.length - 1]
      if (!latestTurn || latestTurn.assistantSummary) {
        return
      }

      latestTurn.assistantSummary = {
        id: message.id,
        content: trimContent(message.content),
        timestamp: message.timestamp
      }
    })

    sortedLogs.forEach((log) => {
      const normalizedSessionId = trimContent(log.sessionId)
      const matchedTurn = normalizedSessionId
        ? turnMap.get(normalizedSessionId)
        : turns[turns.length - 1]

      if (!matchedTurn) {
        return
      }

      matchedTurn.logs.push(log)
    })

    turns.forEach((turn, turnIndex) => {
      const matchedFormIndex = submittedForms.findIndex((snapshot) => {
        const summary = summarizeFormValues(snapshot.schema, snapshot.values)
        if (summary === turn.userMessage.content) {
          return true
        }

        const timeDiff = Math.abs(compareTimestamp(snapshot.submittedAt, turn.userMessage.timestamp))
        return Number.isFinite(timeDiff) && timeDiff <= 5_000
      })
      const matchedForm = matchedFormIndex >= 0
        ? submittedForms.splice(matchedFormIndex, 1)[0]
        : null
      const reconstructedForm = matchedForm
        ? null
        : findSubmittedFormFromTranscript(turns, turnIndex)

      messages.push({
        id: `message-${turn.userMessage.id}`,
        sessionId,
        role: 'user',
        content: matchedForm
          ? buildFormResponseContent(matchedForm.formId, matchedForm.values)
          : reconstructedForm
            ? buildFormResponseContent(reconstructedForm.formId, reconstructedForm.values)
          : turn.userMessage.content,
        attachments: turn.userMessage.attachments,
        status: 'completed',
        createdAt: turn.userMessage.timestamp
      })

      const thinkingChunks: string[] = []
      const contentChunks: string[] = []
      const systemChunks: string[] = []
      const toolCalls: ToolCall[] = []
      const toolCallIndexById = new Map<string, number>()
      const runningToolCallIds: string[] = []
      let assistantStatus: MessageStatus = 'completed'
      let assistantErrorMessage = ''
      let assistantCreatedAt = turn.logs[0]?.createdAt || turn.assistantSummary?.timestamp || turn.userMessage.timestamp

      turn.logs.forEach((log) => {
        const rawContent = trimContent(log.content)
        const metadata = parseLogMetadata(log)

        if (!assistantCreatedAt) {
          assistantCreatedAt = log.createdAt
        }

        if (log.type === 'usage' || log.type === 'message_start' || log.type === 'tool_input_delta' || log.type === 'thinking_start') {
          return
        }

        if (log.type === 'content') {
          if (rawContent) {
            contentChunks.push(log.content ?? '')
          }
          return
        }

        if (log.type === 'thinking') {
          if (rawContent) {
            thinkingChunks.push(rawContent)
          }
          return
        }

        if (log.type === 'tool_use') {
          const toolCall = toToolCall(log, metadata)
          toolCallIndexById.set(toolCall.id, toolCalls.length)
          toolCalls.push(toolCall)
          runningToolCallIds.push(toolCall.id)
          return
        }

        if (log.type === 'tool_result') {
          if (!rawContent) {
            return
          }

          const toolCallId = resolveToolCallId(log, metadata)
          const fallbackToolCallId = runningToolCallIds.length > 0
            ? runningToolCallIds[runningToolCallIds.length - 1]
            : undefined
          const matchedToolCallId = toolCallIndexById.has(toolCallId)
            ? toolCallId
            : fallbackToolCallId
          const matchedToolCallIndex = matchedToolCallId
            ? toolCallIndexById.get(matchedToolCallId)
            : undefined

          if (matchedToolCallIndex !== undefined) {
            const matchedToolCall = toolCalls[matchedToolCallIndex]
            if (matchedToolCall) {
              matchedToolCall.status = 'success'
              matchedToolCall.result = rawContent
              const runningIndex = runningToolCallIds.lastIndexOf(matchedToolCall.id)
              if (runningIndex >= 0) {
                runningToolCallIds.splice(runningIndex, 1)
              }
              return
            }
          }

          systemChunks.push(`工具结果\n${rawContent}`)
          return
        }

        if (log.type === 'error') {
          assistantStatus = 'error'
          assistantErrorMessage = rawContent || splitErrorMessage.value || t('message.failed')
          return
        }

        if (rawContent) {
          const runtimeNotice = buildRuntimeNoticeFromSystemContent(rawContent)
          if (runtimeNotice && isEnvironmentRuntimeNotice(runtimeNotice)) {
            return
          }
          systemChunks.push(rawContent)
        }
      })

      const rawAssistantContent = contentChunks.join('')
      const formRequest = rawAssistantContent ? extractFirstFormRequest(rawAssistantContent) : null
      const normalizedAssistantContent = rawAssistantContent
        ? (formRequest
            ? buildFormRequestContent(formRequest.question || '需要补充信息', formRequest.forms ?? [])
            : buildAssistantDisplayContent(rawAssistantContent))
        : ''
      const fallbackContent = turn.assistantSummary?.content || systemChunks.join('\n\n')
      const finalContent = trimContent(normalizedAssistantContent) || trimContent(fallbackContent)
      const finalThinking = thinkingChunks
        .map(item => item.trim())
        .filter(Boolean)
        .join('\n\n')

      const isLatestTurn = turnIndex === turns.length - 1
      const isRunningLatestTurn = isLatestTurn && isSessionRunning.value
      if (isRunningLatestTurn && !assistantErrorMessage) {
        assistantStatus = 'streaming'
      }

      const finalizedToolCalls = finalizeToolCalls(toolCalls, assistantStatus)
      const assistantRuntimeNotices = buildAssistantRuntimeNotices(
        turn.logs,
        usageModelFallback,
        runtimeProvider,
        finalContent
      )
      const hasAssistantPayload = Boolean(
        finalContent
        || finalThinking
        || finalizedToolCalls.length > 0
        || assistantRuntimeNotices.length > 0
        || Boolean(assistantErrorMessage)
      )

      if (!hasAssistantPayload) {
        return
      }

      const assistantMessage: Message = {
        id: `assistant-${turn.assistantSummary?.id || turn.sessionId || turn.userMessage.id}`,
        sessionId,
        role: 'assistant',
        content: finalContent,
        status: assistantStatus,
        errorMessage: assistantErrorMessage || undefined,
        thinking: finalThinking || undefined,
        toolCalls: finalizedToolCalls.length > 0 ? finalizedToolCalls : undefined,
        runtimeNotices: assistantRuntimeNotices.length > 0 ? assistantRuntimeNotices : undefined,
        retryState: isRunningLatestTurn && taskSplitStore.activeRetryState?.current
          ? { ...taskSplitStore.activeRetryState }
          : undefined,
        createdAt: assistantCreatedAt
      }

      messages.push(assistantMessage)
      if (turn.sessionId) {
        assistantMessageIndexBySessionId.set(turn.sessionId, messages.length - 1)
      }
    })

    const activeFormId = activeFormSchema.value?.formId

    if (
      activeFormSchema.value
      && activeFormId
      && !messages.some(message => containsFormSchema(message.content, activeFormId))
    ) {
      const activeFormContent = buildFormRequestContent(activeFormQuestion.value || '需要补充信息', [activeFormSchema.value])
      const activeExecutionSessionId = trimContent(taskSplitStore.session?.executionSessionId)
      const targetAssistantIndex = activeExecutionSessionId
        ? assistantMessageIndexBySessionId.get(activeExecutionSessionId)
        : undefined

      if (targetAssistantIndex !== undefined) {
        const currentAssistantMessage = messages[targetAssistantIndex]
        if (currentAssistantMessage) {
          messages[targetAssistantIndex] = {
            ...currentAssistantMessage,
            content: activeFormContent
          }
        }
      } else {
        const fallbackRuntimeNotices = buildAssistantRuntimeNotices(
          activeExecutionSessionId
            ? sortedLogs.filter(log => trimContent(log.sessionId) === activeExecutionSessionId)
            : sortedLogs,
          usageModelFallback,
          runtimeProvider,
          activeFormContent
        )

        messages.push({
          id: `active-form-${activeFormId}`,
          sessionId,
          role: 'assistant',
          content: activeFormContent,
          status: 'completed',
          runtimeNotices: fallbackRuntimeNotices.length > 0 ? fallbackRuntimeNotices : undefined,
          createdAt: taskSplitStore.session?.updatedAt || new Date().toISOString()
        })
      }
    }

    const hasErrorAssistantMessage = messages.some(message => message.role === 'assistant' && message.status === 'error')

    if (taskSplitStore.session?.status === 'failed' && splitErrorMessage.value && !hasErrorAssistantMessage) {
      messages.push({
        id: `session-error-${taskSplitStore.session.id}`,
        sessionId,
        role: 'assistant',
        content: splitErrorMessage.value,
        status: 'error',
        errorMessage: splitErrorMessage.value,
        createdAt: taskSplitStore.session.updatedAt
      })
    }

    if (isSplitHistoryLoading.value && messages.length === 0) {
      messages.push({
        id: `history-loading-${sessionId}`,
        sessionId,
        role: 'assistant',
        content: '正在加载历史拆分日志...',
        status: 'streaming',
        createdAt: new Date().toISOString()
      })
    } else if (isSessionRunning.value) {
      const lastMessage = messages[messages.length - 1]

      if (lastMessage?.role === 'assistant' && lastMessage.status !== 'error') {
        lastMessage.status = resolveMessageStatus('streaming', lastMessage.status)
        lastMessage.retryState = taskSplitStore.activeRetryState?.current
          ? { ...taskSplitStore.activeRetryState }
          : undefined
      } else {
        const latestUserMessage = [...messages]
          .reverse()
          .find(message => message.role === 'user')

        messages.push({
          id: `streaming-${taskSplitStore.session?.id || latestUserMessage?.id || sessionId}`,
          sessionId,
          role: 'assistant',
          content: runningStatusText.value || '',
          status: 'streaming',
          retryState: taskSplitStore.activeRetryState?.current
            ? { ...taskSplitStore.activeRetryState }
            : undefined,
          createdAt: taskSplitStore.session?.updatedAt || latestUserMessage?.createdAt || new Date().toISOString()
        })
      }
    }

    return messages
  })

  const splitCurrentStreamingMessageId = computed(() => {
    for (let index = splitChatMessages.value.length - 1; index >= 0; index -= 1) {
      const message = splitChatMessages.value[index]
      if (message?.status === 'streaming') {
        return message.id
      }
    }

    return null
  })

  async function toPendingAttachment(attachment: MessageAttachment): Promise<PendingPlanAttachment> {
    return {
      ...attachment,
      previewUrl: await resolveAttachmentPreviewUrl(attachment)
    }
  }

  function autoResizeInput() {
    const element = instructionInputRef.value
    if (!element) {
      return
    }

    const lineHeight = parseFloat(getComputedStyle(element).lineHeight) || 20
    const fixedHeight = lineHeight * 3 + 16
    element.style.height = `${fixedHeight}px`
  }

  function updateInstructionSelection() {
    const input = instructionInputRef.value
    if (!input) {
      return
    }

    instructionSelectionStart.value = input.selectionStart ?? userInstruction.value.length
  }

  async function initializeDialogSession() {
    const dialogContext = planStore.splitDialogContext
    if (!dialogContext) {
      return
    }

    if (isInitializingSplitSession.value) {
      queuedPlanId.value = dialogContext.planId
      return
    }

    isInitializingSplitSession.value = true
    queuedPlanId.value = null

    try {
      if (agentTeamsStore.experts.length === 0) {
        void agentTeamsStore.loadExperts().catch(error => {
          logger.warn('[TaskSplitDialog] Failed to preload experts:', error)
        })
      }

      if (agentStore.agents.length === 0) {
        void agentStore.loadAgents().catch(error => {
          logger.warn('[TaskSplitDialog] Failed to preload agents:', error)
        })
      }

      const existingPlan = planStore.plans.find(item => item.id === dialogContext.planId)
      const plan = existingPlan || await planStore.getPlan(dialogContext.planId)
      if (!plan) {
        return
      }

      const project = projectStore.projects.find(item => item.id === plan.projectId)
      await taskSplitStore.initSession({
        planId: plan.id,
        planName: plan.name,
        planDescription: plan.description,
        granularity: plan.granularity,
        expertId: dialogContext.expertId || plan.splitExpertId,
        agentId: dialogContext.agentId,
        modelId: dialogContext.modelId,
        workingDirectory: project?.path
      })
    } catch (error) {
      logger.error('[TaskSplitDialog] Failed to initialize session:', error)
    } finally {
      isInitializingSplitSession.value = false
      await nextTick()
      autoResizeInput()

      if (splitDialogVisible.value && queuedPlanId.value && queuedPlanId.value !== taskSplitStore.context?.planId) {
        void initializeDialogSession()
      }
    }
  }

  function switchToPlan(planId: string) {
    if (!planStore.splitDialogVisible) {
      return
    }

    planStore.switchSplitDialogTab(planId)
  }

  async function restartSplit() {
    const dialogContext = planStore.splitDialogContext
    if (!dialogContext) {
      return
    }

    requestSplitChatBottomAlign(dialogContext.planId)
    await clearPendingAttachments()
    userInstruction.value = ''
    mentionDismissKey.value = ''
    selectedMentionOptionIndex.value = 0
    await taskSplitStore.clearPlanSplitSessions(dialogContext.planId)
    await initializeDialogSession()
  }

  async function handleActiveFormSubmit(
    formId: string,
    values: Record<string, unknown>,
    assistantMessageId?: string
  ) {
    void assistantMessageId
    try {
      await taskSplitStore.submitFormResponse(formId, values)
    } catch (error) {
      logger.error('[TaskSplitDialog] Failed to submit task split form:', error)
    }
  }

  async function uploadAttachments(files: File[]) {
    const sessionId = uploadSessionId.value
    if (!sessionId || files.length === 0) {
      return
    }

    try {
      isUploadingAttachments.value = true
      const payload: UploadAttachmentInput[] = await Promise.all(files.map(async file => ({
        fileName: file.name,
        mimeType: file.type || 'application/octet-stream',
        bytes: Array.from(new Uint8Array(await file.arrayBuffer()))
      })))

      const result = await invoke<UploadAttachmentsResponse>('upload_session_images', {
        sessionId,
        files: payload
      })

      const uploaded = await Promise.all(result.attachments.map(toPendingAttachment))
      pendingAttachments.value = [...pendingAttachments.value, ...uploaded]
    } catch (error) {
      logger.error('[TaskSplitDialog] Failed to upload attachments:', error)
      notificationStore.smartError(
        t('taskSplit.attachmentUploadAction'),
        error instanceof Error ? error : new Error(String(error))
      )
    } finally {
      isUploadingAttachments.value = false
    }
  }

  function openAttachmentPicker() {
    fileInputRef.value?.click()
  }

  async function handleAttachmentFileChange(event: Event) {
    const target = event.target as HTMLInputElement
    const files = target.files ? Array.from(target.files) : []
    target.value = ''
    await uploadAttachments(files)
  }

  async function handleInstructionPaste(event: ClipboardEvent) {
    const items = Array.from(event.clipboardData?.items ?? [])
    const imageFiles = items
      .filter(item => item.type.startsWith('image/'))
      .map(item => item.getAsFile())
      .filter((file): file is File => file !== null)

    if (imageFiles.length === 0) {
      return
    }

    event.preventDefault()
    await uploadAttachments(imageFiles)
  }

  async function removeAttachment(attachmentId: string) {
    const sessionId = uploadSessionId.value
    const attachment = pendingAttachments.value.find(item => item.id === attachmentId)
    if (!sessionId || !attachment) {
      return
    }

    try {
      await invoke('delete_uploaded_image', {
        sessionId,
        path: attachment.path
      })
      pendingAttachments.value = pendingAttachments.value.filter(item => item.id !== attachmentId)
    } catch (error) {
      logger.error('[TaskSplitDialog] Failed to delete attachment:', error)
      notificationStore.smartError(
        t('taskSplit.attachmentDeleteAction'),
        error instanceof Error ? error : new Error(String(error))
      )
    }
  }

  async function clearPendingAttachments() {
    if (pendingAttachments.value.length === 0) {
      return
    }

    const attachments = [...pendingAttachments.value]
    pendingAttachments.value = []

    await Promise.all(attachments.map(async item => {
      try {
        await invoke('delete_uploaded_image', {
          sessionId: uploadSessionId.value,
          path: item.path
        })
      } catch (error) {
        logger.warn('[TaskSplitDialog] Failed to discard attachment:', error)
      }
    }))
  }

  async function confirmSplit() {
    const splitContext = planStore.splitDialogContext
    if (!taskSplitStore.splitResult || !splitContext || isConfirming.value) {
      return
    }

    if (refinementMode.value === 'task_resplit') {
      await taskSplitStore.completeSubSplit(taskSplitStore.splitResult)
      return
    }

    if (refinementMode.value === 'list_optimize') {
      await taskSplitStore.completeListOptimize(taskSplitStore.splitResult)
      return
    }

    const planId = splitContext.planId
    const currentPlan = planStore.plans.find(plan => plan.id === planId) || null
    isConfirming.value = true

    try {
      await Promise.all([
        agentStore.loadAgents(),
        agentTeamsStore.loadExperts()
      ])

      const fallbackExpert = agentTeamsStore.builtinDeveloperExpert
        || agentTeamsStore.enabledExperts.find(expert => expert.category === 'developer')
        || agentTeamsStore.enabledExperts[0]
        || null

      const taskInputs = taskSplitStore.splitResult.map((task, index) => {
        const selectedExpert = resolveExpertById(task.expertId, agentTeamsStore.experts) || fallbackExpert
        const runtime = resolveExpertRuntime(selectedExpert, agentStore.agents, task.modelId)

        return {
          planId,
          title: task.title,
          description: task.description,
          priority: task.priority,
          expertId: selectedExpert?.id,
          agentId: runtime?.agent.id || task.agentId,
          modelId: runtime?.modelId || task.modelId,
          implementationSteps: task.implementationSteps,
          testSteps: task.testSteps,
          acceptanceCriteria: task.acceptanceCriteria,
          memoryLibraryIds: task.memoryLibraryIds?.length
            ? [...task.memoryLibraryIds]
            : [...(currentPlan?.memoryLibraryIds ?? [])],
          dependsOn: task.dependsOn,
          order: index
        }
      })

      await taskStore.createTasksFromSplit(planId, taskInputs)
      await taskStore.loadTasks(planId)
      await planStore.markPlanAsReady(planId)
      planStore.setCurrentPlan(planId)
      await taskSplitStore.clearPlanSplitSessions(planId)

      taskSplitStore.reset()
      planStore.closeSplitDialog()
    } catch (error) {
      logger.error('[TaskSplitDialog] Failed to confirm split:', error)
    } finally {
      isConfirming.value = false
    }
  }

  async function closeDialog() {
    if (hasPendingRefinement.value) {
      if (isSessionRunning.value) {
        await taskSplitStore.stop()
      }
      await taskSplitStore.cancelRefinement({ discardSession: true })
    }

    await clearPendingAttachments()

    const planId = planStore.activeSplitPlanId
    taskSplitStore.detach()
    if (planId) {
      taskSplitStore.clearPlanStateCache(planId)
      planStore.closeSplitDialogTab(planId)
    }
  }

  async function handleCloseClick() {
    if (isSessionRunning.value) {
      showCloseConfirm.value = true
      return
    }

    await closeDialog()
  }

  function cancelCloseConfirm() {
    showCloseConfirm.value = false
  }

  async function confirmCloseAndStop() {
    showCloseConfirm.value = false
    if (isSessionRunning.value) {
      await taskSplitStore.stop()
    }
    await closeDialog()
  }

  function handleMinimizeClick() {
    taskSplitStore.saveCurrentPlanState()
    taskSplitStore.detachStream()
    planStore.splitDialogVisible = false
  }

  async function stopSplitTask() {
    await taskSplitStore.stop()
  }

  async function retrySplitTask() {
    await taskSplitStore.retry()
  }

  async function continueSplitTask() {
    await taskSplitStore.continueSession()
  }

  function applyMentionSuggestion(index = selectedMentionOptionIndex.value) {
    const mention = mentionState.value
    if (!mention) {
      return
    }

    const option = mention.options[index]
    if (!option) {
      return
    }

    const replacement = `@[${option.index + 1}:${option.title}] `
    userInstruction.value = [
      userInstruction.value.slice(0, mention.rangeStart),
      replacement,
      userInstruction.value.slice(mention.rangeEnd)
    ].join('')
    mentionDismissKey.value = ''

    nextTick(() => {
      const input = instructionInputRef.value
      if (!input) {
        return
      }

      const nextCaret = mention.rangeStart + replacement.length
      input.focus()
      input.setSelectionRange(nextCaret, nextCaret)
      instructionSelectionStart.value = nextCaret
      autoResizeInput()
    })
  }

  function handleInstructionCaretChange() {
    updateInstructionSelection()
  }

  function handleInstructionInput() {
    updateInstructionSelection()
    autoResizeInput()
  }

  function buildInstructionPrompt(text: string) {
    if (!taskSplitStore.splitResult) {
      return text
    }

    return materializeTaskMentions(text, taskSplitStore.splitResult)
  }

  async function executeParsedInstruction(text: string, attachments: MessageAttachment[] = []) {
    const tasks = taskSplitStore.splitResult
    if (!tasks) {
      return false
    }

    const parsed = parseTaskInstruction(text, tasks)
    const normalizedText = normalizeTaskInstructionInput(text)
    const displayContent = text.trim()
    const instructionPrompt = buildInstructionPrompt(
      parsed.customPrompt || normalizedText || t('taskSplit.attachmentRefineFallbackPrompt')
    )

    if (parsed.type === 'delete' && parsed.targetIndex !== undefined) {
      await taskSplitStore.startListOptimize({
        expertId: taskSplitStore.context?.expertId,
        agentId: taskSplitStore.context?.agentId,
        modelId: taskSplitStore.context?.modelId,
        customPrompt: instructionPrompt,
        displayContent,
        attachments,
        targetIndex: parsed.targetIndex,
        taskCountMode: 'min',
        minTaskCount: 1
      })
      return true
    }

    if (parsed.type === 'resplit' && parsed.targetIndex !== undefined) {
      await taskSplitStore.startSubSplit(parsed.targetIndex, {
        taskIndex: parsed.targetIndex,
        granularity: taskSplitStore.context?.granularity || DEFAULT_SPLIT_GRANULARITY,
        expertId: taskSplitStore.context?.expertId,
        agentId: taskSplitStore.context?.agentId,
        modelId: taskSplitStore.context?.modelId,
        customPrompt: instructionPrompt,
        displayContent,
        attachments
      })
      return true
    }

    if (
      parsed.type === 'add'
      || parsed.type === 'update'
      || parsed.type === 'move_up'
      || parsed.type === 'move_down'
      || parsed.type === 'optimize'
      || parsed.type === 'reorder'
      || parsed.type === 'unknown'
    ) {
      await taskSplitStore.startListOptimize({
        expertId: taskSplitStore.context?.expertId,
        agentId: taskSplitStore.context?.agentId,
        modelId: taskSplitStore.context?.modelId,
        customPrompt: instructionPrompt,
        displayContent,
        attachments,
        targetIndex: parsed.targetIndex,
        taskCountMode: parsed.type === 'add' ? 'min' : 'exact',
        minTaskCount: parsed.type === 'add' ? 1 : tasks.length
      })
      return true
    }

    return false
  }

  async function handleUserInstruction() {
    const text = userInstruction.value.trim()
    const attachments = pendingAttachments.value.map(item => ({ ...item }))
    if (!text && attachments.length === 0) {
      return
    }

    if (isSessionRunning.value || isConfirming.value) {
      return
    }

    let handled = false
    if (taskSplitStore.splitResult) {
      handled = await executeParsedInstruction(text, attachments)
    } else if (taskSplitStore.session?.status === 'stopped' || taskSplitStore.session?.status === 'failed') {
      await taskSplitStore.continueSessionWithInstruction(text, {
        attachments,
        displayContent: text
      })
      handled = true
    }

    if (!handled) {
      logger.warn('[TaskSplitDialog] Unrecognized instruction:', text)
      return
    }

    userInstruction.value = ''
    mentionDismissKey.value = ''
    pendingAttachments.value = []
    await nextTick()
    autoResizeInput()
  }

  async function handleInstructionKeydown(event: KeyboardEvent) {
    updateInstructionSelection()

    if (showMentionSuggestions.value) {
      if (event.key === 'ArrowDown') {
        event.preventDefault()
        selectedMentionOptionIndex.value = (selectedMentionOptionIndex.value + 1) % mentionSuggestions.value.length
        return
      }

      if (event.key === 'ArrowUp') {
        event.preventDefault()
        selectedMentionOptionIndex.value = selectedMentionOptionIndex.value <= 0
          ? mentionSuggestions.value.length - 1
          : selectedMentionOptionIndex.value - 1
        return
      }

      if (event.key === 'Enter' && !event.shiftKey) {
        event.preventDefault()
        applyMentionSuggestion()
        return
      }

      if (event.key === 'Escape') {
        event.preventDefault()
        mentionDismissKey.value = mentionState.value?.key || ''
        return
      }
    }

    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      await handleUserInstruction()
    }
  }

  watch(mentionState, current => {
    if (!current) {
      selectedMentionOptionIndex.value = 0
      mentionDismissKey.value = ''
      return
    }

    if (mentionDismissKey.value && mentionDismissKey.value !== current.key) {
      mentionDismissKey.value = ''
    }

    if (selectedMentionOptionIndex.value >= current.options.length) {
      selectedMentionOptionIndex.value = 0
    }
  })

  watch(
    [splitDialogVisible, activeSplitPlanId],
    async ([visible, planId], previousValue) => {
      const prevVisible = previousValue?.[0] ?? false
      if (visible && planId) {
        requestSplitChatBottomAlign(planId)
        await initializeDialogSession()
        return
      }

      if (!visible && prevVisible) {
        await clearPendingAttachments()
        userInstruction.value = ''
        mentionDismissKey.value = ''
        selectedMentionOptionIndex.value = 0
        taskSplitStore.detach()
      }
    },
    { immediate: true }
  )

  watch(isAutoRetryPending, (pending) => {
    if (pending) {
      retryNow.value = Date.now()
      if (!retryCountdownTimer) {
        retryCountdownTimer = setInterval(() => {
          retryNow.value = Date.now()
        }, 1000)
      }
      return
    }

    if (retryCountdownTimer) {
      clearInterval(retryCountdownTimer)
      retryCountdownTimer = null
    }
  }, { immediate: true })

  onBeforeUnmount(() => {
    if (retryCountdownTimer) {
      clearInterval(retryCountdownTimer)
      retryCountdownTimer = null
    }
    pendingAttachments.value = []
  })

  const { handleOverlayPointerDown, handleOverlayClick } = useOverlayDismiss(handleMinimizeClick)

  return {
    planStore,
    splitDialogVisible,
    splitDialogTabs,
    activeSplitPlanId,
    taskSplitStore,
    isDarkTheme,
    isConfirming,
    userInstruction,
    instructionInputRef,
    fileInputRef,
    pendingAttachments,
    isUploadingAttachments,
    showMentionSuggestions,
    mentionSuggestions,
    selectedMentionOptionIndex,
    showPreview,
    showRecoveryInput,
    refinementMode,
    hasPendingRefinement,
    isSubSplitActive,
    subSplitTargetTitle,
    previewActionsDisabled,
    canApplyRefinement,
    isSessionRunning,
    showStopButton,
    canRetrySplit,
    canContinueSplit,
    retryButtonLabel,
    isAutoRetryPending,
    primaryActionLabel,
    footerHint,
    runningStatusText,
    splitAgentLabel,
    splitModelLabel,
    instructionDisabled,
    splitChatSessionId,
    splitChatMessages,
    splitCurrentStreamingMessageId,
    splitChatScrollToken,
    isSplitHistoryLoading,
    activeFormSchema,
    activeFormQuestion,
    handleActiveFormSubmit,
    restartSplit,
    confirmSplit,
    closeDialog,
    handleCloseClick,
    cancelCloseConfirm,
    confirmCloseAndStop,
    handleMinimizeClick,
    switchToPlan,
    showCloseConfirm,
    stopSplitTask,
    retrySplitTask,
    continueSplitTask,
    handleInstructionInput,
    handleInstructionKeydown,
    handleInstructionCaretChange,
    handleAttachmentFileChange,
    handleInstructionPaste,
    openAttachmentPicker,
    removeAttachment,
    applyMentionSuggestion,
    handleOverlayPointerDown,
    handleOverlayClick
  }
}
