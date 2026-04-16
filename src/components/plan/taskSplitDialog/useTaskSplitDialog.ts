import { ref, computed, watch, nextTick, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import { usePlanStore } from '@/stores/plan'
import { useTaskSplitStore } from '@/stores/taskSplit'
import { useTaskStore } from '@/stores/task'
import { useProjectStore } from '@/stores/project'
import { useThemeStore } from '@/stores/theme'
import { useAgentStore } from '@/stores/agent'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import { useOverlayDismiss } from '@/composables/useOverlayDismiss'
import type {
  AITaskItem,
  DynamicFormSchema,
  PlanSplitLogRecord,
  SplitMessage
} from '@/types/plan'
import type { TimelineEntry } from '@/types/timeline'
import { buildToolCallMapFromLogs, extractDynamicFormSchemas } from '@/utils/toolCallLog'
import {
  buildRuntimeNoticeFromSystemContent,
  buildUsageNotice,
  isContextRuntimeNotice
} from '@/utils/runtimeNotice'
import { extractFirstFormRequest, extractTaskSplitResult } from '@/utils/structuredContent'
import { DEFAULT_SPLIT_GRANULARITY } from '@/constants/plan'
import { logger } from '@/utils/logger'
import {
  materializeTaskMentions,
  normalizeTaskInstructionInput,
  parseTaskInstruction
} from '@/utils/taskInstructionParser'
import { resolveRecordedModelId } from '@/services/usage/agentCliUsageRecorder'
import { resolveExpertById, resolveExpertRuntime } from '@/services/agentTeams/runtime'

export function useTaskSplitDialog() {
  const planStore = usePlanStore()
  const { t } = useI18n()
  const taskSplitStore = useTaskSplitStore()
  const taskStore = useTaskStore()
  const projectStore = useProjectStore()
  const themeStore = useThemeStore()
  const agentStore = useAgentStore()
  const agentTeamsStore = useAgentTeamsStore()
  const isDarkTheme = computed(() => themeStore.isDark)

  const isConfirming = ref(false)
  const messagesContainerRef = ref<HTMLElement | null>(null)
  const userInstruction = ref('')
  const instructionInputRef = ref<HTMLTextAreaElement | null>(null)
  const instructionSelectionStart = ref(0)
  const selectedMentionOptionIndex = ref(0)
  const mentionDismissKey = ref('')


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

  // 是否显示预览
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
    if (!isSubSplitActive.value || taskSplitStore.subSplitTargetIndex === null) return ''
    const originalTasks = taskSplitStore.refinementState?.originalTasks
    const idx = taskSplitStore.subSplitTargetIndex
    return originalTasks?.[idx]?.title ?? t('taskBoard.emptyNoTasks')
  })
  const previewActionsDisabled = computed(() =>
    isSessionRunning.value || isConfirming.value
  )
  const canApplyRefinement = computed(() =>
    hasPendingRefinement.value && taskSplitStore.session?.status === 'completed'
  )

  const activeFormSchema = computed(() => taskSplitStore.activeFormSchema)
  const isSessionRunning = computed(() => taskSplitStore.session?.status === 'running')
  const showStopButton = computed(() => taskSplitStore.session?.status === 'running')
  const showLoadingIndicator = computed(() => taskSplitStore.session?.status === 'running')
  const canRetrySplit = computed(() => taskSplitStore.session?.status === 'failed')
  const canContinueSplit = computed(() =>
    taskSplitStore.session?.status === 'stopped'
    && !activeFormSchema.value
    && (!showPreview.value || hasPendingRefinement.value)
  )
  const mentionState = computed<MentionState | null>(() => {
    const tasks = taskSplitStore.splitResult
    if (!tasks?.length) return null

    const caret = instructionSelectionStart.value
    const beforeCaret = userInstruction.value.slice(0, caret)
    const mentionStart = beforeCaret.lastIndexOf('@')
    if (mentionStart < 0) return null

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
    if (isSessionRunning.value || isConfirming.value) return false
    if (!mentionState.value || mentionSuggestions.value.length === 0) return false
    return mentionDismissKey.value !== mentionState.value.key
  })
  const retryActionLabel = computed(() => {
    const hasUserMessage = taskSplitStore.messages.some(message =>
      message.role === 'user' && message.content.trim()
    )
    return hasUserMessage ? t('taskSplit.resendLabel') : t('taskSplit.retryLabel')
  })
  const retryNow = ref(Date.now())
  let retryCountdownTimer: ReturnType<typeof setInterval> | null = null
  const splitErrorMessage = computed(() =>
    taskSplitStore.session?.errorMessage?.trim()
    || taskSplitStore.session?.parseError?.trim()
    || ''
  )
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
      : retryActionLabel.value
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
      const hasArchivedForms = taskSplitStore.logs.some(log =>
        log.type === 'content' && log.content.includes('"type": "form_request"')
      )
      return hasArchivedForms
        ? t('taskSplit.hintSessionStoppedWithHistory')
        : t('taskSplit.hintSessionStopped')
    }

    if (taskSplitStore.session?.status === 'running') {
      return t('taskSplit.hintSessionRunning')
    }

    if (taskSplitStore.session?.status === 'waiting_input' && activeFormSchema.value) {
      return t('taskSplit.hintWaitingFormInput')
    }

    if (!activeFormSchema.value && !showPreview.value) {
      return t('taskSplit.hintNoFormData')
    }

    return t('taskSplit.hintWaitingFormInput')
  })

  const sortedSplitLogs = computed(() => [...taskSplitStore.logs].sort((left, right) =>
    compareTimestamp(left.createdAt, right.createdAt)
  ))

  const toolCallMap = computed(() => buildToolCallMapFromLogs(sortedSplitLogs.value, {
    fallbackStatus: isSessionRunning.value
      ? 'running'
      : taskSplitStore.session?.status === 'failed' || taskSplitStore.session?.status === 'stopped'
        ? 'error'
        : 'success'
  }))

  const latestRuntimeLog = computed(() => {
    if (sortedSplitLogs.value.length === 0) return null
    return sortedSplitLogs.value[sortedSplitLogs.value.length - 1] ?? null
  })

  const runtimeLogStatusTextResolvers: Partial<Record<PlanSplitLogRecord['type'], (log: PlanSplitLogRecord) => string>> = {
    thinking: () => t('taskSplit.runtimeThinking'),
    thinking_start: () => t('taskSplit.runtimeThinkingStart'),
    system: () => t('taskSplit.runtimeLoadingExtensions'),
    tool_use: (log) => {
      const toolCall = toolCallMap.value.get(log.id)
      return toolCall ? t('taskSplit.runtimeCallingTool', { name: toolCall.name }) : t('taskSplit.runtimeCallingToolFallback')
    },
    tool_input_delta: () => t('taskSplit.runtimeToolInputDelta'),
    content: () => t('taskSplit.runtimeGenerating'),
    tool_result: () => t('taskSplit.runtimeToolResult'),
    error: () => t('taskSplit.runtimeError')
  }

  const runningStatusText = computed(() => {
    if (!isSessionRunning.value) {
      return ''
    }

    const latestLog = latestRuntimeLog.value
    if (!latestLog) {
      return t('taskSplit.runtimeWaitingFirstOutput')
    }

    const resolver = runtimeLogStatusTextResolvers[latestLog.type]
    if (resolver) {
      return resolver(latestLog)
    }

    return t('taskSplit.runtimeProcessing')
  })

  function scrollMessagesToBottom() {
    const container = messagesContainerRef.value
    if (!container) return

    container.scrollTop = container.scrollHeight
  }

  function compareTimestamp(left?: string, right?: string) {
    return new Date(left || 0).getTime() - new Date(right || 0).getTime()
  }

  function toTimestampMs(value?: string | null) {
    if (!value) {
      return null
    }

    const timestampMs = new Date(value).getTime()
    return Number.isFinite(timestampMs) ? timestampMs : null
  }

  function formatElapsedLabel(durationMs: number | null) {
    if (durationMs === null || durationMs < 250) {
      return null
    }

    if (durationMs < 1_000) {
      return `${Math.round(durationMs)}ms`
    }

    if (durationMs < 60_000) {
      return `${(durationMs / 1_000).toFixed(durationMs >= 10_000 ? 0 : 1)}s`
    }

    const minutes = Math.floor(durationMs / 60_000)
    const seconds = Math.round((durationMs % 60_000) / 1_000)
    return `${minutes}m ${seconds}s`
  }

  function parseLogMetadata(log: PlanSplitLogRecord): Record<string, unknown> | null {
    if (!log.metadata) return null
    if (typeof log.metadata === 'string') {
      try {
        const parsed = JSON.parse(log.metadata) as Record<string, unknown>
        const rawMetadata = typeof parsed.rawMetadata === 'string'
          ? (() => {
              try {
                const nested = JSON.parse(parsed.rawMetadata) as unknown
                return nested && typeof nested === 'object' && !Array.isArray(nested)
                  ? nested as Record<string, unknown>
                  : null
              } catch {
                return null
              }
            })()
          : null

        return rawMetadata
          ? {
              ...rawMetadata,
              ...parsed
            }
          : parsed
      } catch {
        return null
      }
    }
    return log.metadata as unknown as Record<string, unknown>
  }

  function readMetadataNumber(metadata: Record<string, unknown> | null, key: string): number | undefined {
    const value = metadata?.[key]
    if (typeof value === 'number' && Number.isFinite(value)) {
      return value
    }
    if (typeof value === 'string') {
      const numeric = Number(value)
      if (Number.isFinite(numeric)) {
        return numeric
      }
    }
    return undefined
  }

  function resolveUsageState(logs: PlanSplitLogRecord[]) {
    const requestedModelId = taskSplitStore.context?.modelId
      || taskSplitStore.usageModelHint
      || undefined
    const usageState: { model?: string, inputTokens?: number, outputTokens?: number } = {
      model: requestedModelId
    }

    for (const log of logs) {
      const metadata = parseLogMetadata(log)
      const reportedModel = typeof metadata?.model === 'string' && metadata.model.trim()
        ? metadata.model.trim()
        : undefined
      const inputTokens = readMetadataNumber(metadata, 'inputTokens')
      const outputTokens = readMetadataNumber(metadata, 'outputTokens')

      if (reportedModel) {
        usageState.model = resolveRecordedModelId({
          reportedModelId: reportedModel,
          requestedModelId
        }) ?? reportedModel
      }
      if (inputTokens !== undefined && (usageState.inputTokens === undefined || inputTokens >= usageState.inputTokens)) {
        usageState.inputTokens = inputTokens
      }
      if (outputTokens !== undefined && (usageState.outputTokens === undefined || outputTokens >= usageState.outputTokens)) {
        usageState.outputTokens = outputTokens
      }
    }

    return usageState
  }

  function buildSummaryValueMap(schema: DynamicFormSchema, content: string): Record<string, string> {
    const labelSet = new Set(schema.fields.map(field => field.label))
    const valueMap: Record<string, string> = {}
    let currentLabel = ''
    let currentValue = ''

    const flushCurrentValue = () => {
      if (!currentLabel) return
      valueMap[currentLabel] = currentValue.trim()
    }

    for (const rawLine of content.split('\n')) {
      const line = rawLine.trimEnd()
      const fullWidthSeparatorIndex = line.indexOf('：')
      const asciiSeparatorIndex = line.indexOf(':')
      const separatorIndex = fullWidthSeparatorIndex >= 0 ? fullWidthSeparatorIndex : asciiSeparatorIndex
      const candidateLabel = separatorIndex >= 0 ? line.slice(0, separatorIndex).trim() : ''

      if (candidateLabel && labelSet.has(candidateLabel)) {
        flushCurrentValue()
        currentLabel = candidateLabel
        currentValue = line.slice(separatorIndex + 1).trim()
        continue
      }

      if (currentLabel) {
        currentValue = [currentValue, line.trim()].filter(Boolean).join('\n')
      }
    }

    flushCurrentValue()
    return valueMap
  }

  function parseFieldSummaryValue(
    field: DynamicFormSchema['fields'][number],
    rawValue?: string
  ): unknown {
    const normalizedValue = rawValue?.trim()
    if (!normalizedValue || normalizedValue === '-') {
      if (field.type === 'multiselect') return []
      if (field.type === 'checkbox') return false
      return ''
    }

    const parseOptionValue = (value: string) => {
      const matchedOption = field.options?.find(option =>
        String(option.label).trim() === value || String(option.value).trim() === value
      )
      return matchedOption ? matchedOption.value : value
    }

    switch (field.type) {
      case 'select':
      case 'radio':
        return parseOptionValue(normalizedValue)
      case 'multiselect':
        return normalizedValue
          .split(/[、,，]/)
          .map(item => item.trim())
          .filter(Boolean)
          .map(parseOptionValue)
      case 'number':
      case 'slider': {
        const numericValue = Number(normalizedValue)
        return Number.isFinite(numericValue) ? numericValue : normalizedValue
      }
      case 'checkbox':
        return ['true', '1', 'yes', 'on', 'checked', '是'].includes(normalizedValue.toLowerCase())
      default:
        return normalizedValue
    }
  }

  function formatFieldSummaryValue(
    field: DynamicFormSchema['fields'][number],
    value: unknown
  ): string {
    if (value === undefined || value === null || value === '') return '-'
    if (Array.isArray(value)) {
      if (value.length === 0) return '-'
      return value.map(item => formatFieldSummaryValue(field, item)).join('、')
    }

    const matchedOption = field.options?.find(option => option.value === value)
    return matchedOption ? matchedOption.label : String(value)
  }

  function summarizeFormValues(
    schema: DynamicFormSchema,
    values: Record<string, unknown>
  ): string {
    return schema.fields
      .map(field => `${field.label}：${formatFieldSummaryValue(field, values[field.name])}`)
      .join('\n')
  }

  function normalizeMultilineText(value?: string | null): string {
    return (value ?? '')
      .split('\n')
      .map(line => line.trim())
      .join('\n')
      .trim()
  }

  function resolveMessageTimestampDistance(left?: string, right?: string): number {
    const leftMs = toTimestampMs(left)
    const rightMs = toTimestampMs(right)
    if (leftMs === null || rightMs === null) {
      return Number.MAX_SAFE_INTEGER
    }

    return Math.abs(leftMs - rightMs)
  }

  function buildFormValuesFromMessage(schema: DynamicFormSchema, messageContent: string): Record<string, unknown> {
    const summaryValueMap = buildSummaryValueMap(schema, messageContent)
    const values: Record<string, unknown> = {}

    for (const field of schema.fields) {
      values[field.name] = parseFieldSummaryValue(field, summaryValueMap[field.label])
    }

    return values
  }

  interface FormRequestSnapshot {
    formId: string
    schema: DynamicFormSchema
    promptText?: string
    requestedAt: string
  }

  interface FormRequestTurn {
    requestedAt: string
    promptText?: string
    forms: FormRequestSnapshot[]
  }

  function sortTimelineEntries(entries: TimelineEntry[]) {
    entries.sort((left, right) => {
      const timeDiff = compareTimestamp(left.timestamp, right.timestamp)
      if (timeDiff !== 0) {
        return timeDiff
      }

      const leftSequence = left.sequence ?? Number.MAX_SAFE_INTEGER
      const rightSequence = right.sequence ?? Number.MAX_SAFE_INTEGER
      if (leftSequence !== rightSequence) {
        return leftSequence - rightSequence
      }

      return left.id.localeCompare(right.id)
    })
  }

  function extractFormRequestTurnFromLog(log: PlanSplitLogRecord): FormRequestTurn | null {
    const toTurn = (
      payload: Record<string, unknown>,
      requestedAt: string
    ): FormRequestTurn | null => {
      const schemas = extractDynamicFormSchemas(payload)
      if (schemas.length === 0) {
        return null
      }

      const promptText = typeof payload.question === 'string' ? payload.question.trim() : undefined
      return {
        requestedAt,
        promptText,
        forms: schemas.map(schema => ({
          formId: schema.formId,
          schema,
          promptText,
          requestedAt
        }))
      }
    }

    if (log.type === 'content') {
      try {
        return toTurn(JSON.parse(log.content) as Record<string, unknown>, log.createdAt)
      } catch {
        return null
      }
    }

    const toolCall = toolCallMap.value.get(log.id)
    if (!toolCall) {
      return null
    }

    const isStructuredOutput = toolCall.name.toLowerCase() === 'structuredoutput'
      || toolCall.name.toLowerCase() === 'structured_output'
    if (!isStructuredOutput) {
      return null
    }

    return toTurn(toolCall.arguments, log.createdAt)
  }

  const formRequestTurns = computed<FormRequestTurn[]>(() => {
    const turns: FormRequestTurn[] = []

    for (const log of sortedSplitLogs.value) {
      const turn = extractFormRequestTurnFromLog(log)
      if (turn) {
        turns.push(turn)
      }
    }

    return turns
  })

  const formRequestHistory = computed<FormRequestSnapshot[]>(() =>
    formRequestTurns.value.flatMap(turn => turn.forms)
  )

  const activeFormRequestedAt = computed(() => {
    const activeFormId = activeFormSchema.value?.formId
    if (!activeFormId) {
      return null
    }

    for (let index = formRequestTurns.value.length - 1; index >= 0; index -= 1) {
      const turn = formRequestTurns.value[index]
      if (turn.forms.some(form => form.formId === activeFormId)) {
        return turn.requestedAt
      }
    }

    return taskSplitStore.session?.updatedAt || new Date().toISOString()
  })

  function shouldSuppressStructuredContentLog(log: PlanSplitLogRecord) {
    if (log.type !== 'content' || !log.content.trim()) {
      return false
    }

    if (extractFirstFormRequest(log.content)) {
      return true
    }

    return showPreview.value && Boolean(extractTaskSplitResult(log.content))
  }

  function isRuntimeSystemLog(log: PlanSplitLogRecord) {
    if (log.type !== 'system' || !log.content.trim()) {
      return false
    }

    return Boolean(buildRuntimeNoticeFromSystemContent(log.content))
  }

  function shouldSuppressSystemProgressLog(log: PlanSplitLogRecord) {
    if (log.type !== 'system') {
      return false
    }

    const normalizedContent = log.content.trim()
    if (!normalizedContent) {
      return true
    }

    if (isRuntimeSystemLog(log)) {
      return true
    }

    return [
      '工具返回结果',
      '正在继续处理',
      '正在加载运行扩展',
      '后台正在执行拆分'
    ].some(keyword => normalizedContent.includes(keyword))
  }

  const visibleRuntimeNotices = computed(() =>
    taskSplitStore.runtimeNotices.filter(notice => !isContextRuntimeNotice(notice))
  )

  const hasRuntimeSystemLog = computed(() =>
    taskSplitStore.logs.some(log =>
      log.type === 'system'
      && log.content.trim()
      && !isRuntimeSystemLog(log)
    )
  )

  const usageNoticeEntry = computed<TimelineEntry | null>(() => {
    const usageLogs = sortedSplitLogs.value.filter(log => {
      const metadata = parseLogMetadata(log)
      return log.type === 'usage'
        || log.type === 'message_start'
        || typeof metadata?.model === 'string'
        || readMetadataNumber(metadata, 'inputTokens') !== undefined
        || readMetadataNumber(metadata, 'outputTokens') !== undefined
    })

    const usageState = resolveUsageState(sortedSplitLogs.value)

  const notice = buildUsageNotice(usageState)
  if (!notice) return null

  const latestUsageLog = usageLogs[usageLogs.length - 1]
  return {
    id: `usage-${latestUsageLog?.id || 'runtime'}`,
    type: 'system',
    content: `### ${notice.title}\n${notice.content}`,
    timestamp: latestUsageLog?.createdAt || taskSplitStore.session?.updatedAt,
    runtimeFallbackUsage: usageState
  }
})

const sessionElapsedLabel = computed(() => {
  const sessionStartMs = toTimestampMs(taskSplitStore.session?.startedAt || taskSplitStore.session?.createdAt)
  const sessionEndMs = toTimestampMs(
    taskSplitStore.session?.completedAt
      || taskSplitStore.session?.stoppedAt
      || (!isSessionRunning.value ? taskSplitStore.session?.updatedAt : null)
  )

  if (sessionStartMs === null || sessionEndMs === null) {
    const runtimeMetrics = taskSplitStore.runtimeMetrics
    if (runtimeMetrics?.doneAt !== undefined) {
      return formatElapsedLabel(Math.max(0, runtimeMetrics.doneAt - runtimeMetrics.startedAt))
    }
    return null
  }

  return formatElapsedLabel(Math.max(0, sessionEndMs - sessionStartMs))
})

function attachSessionElapsedLabel(entries: TimelineEntry[]) {
  const elapsedLabel = sessionElapsedLabel.value
  if (!elapsedLabel) {
    return
  }

  const activeFormEntry = [...entries].reverse().find(entry =>
    entry.type === 'form'
    && entry.formVariant === 'active'
    && entry.formSchema?.formId === activeFormSchema.value?.formId
  )
  if (activeFormEntry) {
    activeFormEntry.metaLabel = elapsedLabel
    return
  }

  const lastRenderableEntry = [...entries].reverse().find(entry => entry.role !== 'user')
  if (lastRenderableEntry) {
    lastRenderableEntry.metaLabel = elapsedLabel
  }
}

const historicalSubmittedForms = computed(() => {
  const derivedSnapshots: Array<{
    formId: string
    schema: DynamicFormSchema
    promptText?: string
    requestedAt: string
    values: Record<string, unknown>
    submittedAt: string
    sourceMessageId?: string
  }> = []
  const sortedMessages = [...taskSplitStore.messages].sort((left, right) =>
    compareTimestamp(left.timestamp, right.timestamp)
  )
  let formQueueIndex = 0
  let latestAssistantPrompt = ''

  for (const message of sortedMessages) {
    if (message.role === 'assistant') {
      latestAssistantPrompt = message.content.trim()
      continue
    }
    if (message.role !== 'user') continue
    const pendingForm = formRequestHistory.value[formQueueIndex]
    if (!pendingForm) break
    if (compareTimestamp(message.timestamp, pendingForm.requestedAt) < 0) {
      continue
    }

    derivedSnapshots.push({
      formId: pendingForm.schema.formId,
      schema: pendingForm.schema,
      promptText: latestAssistantPrompt || undefined,
      requestedAt: pendingForm.requestedAt,
      values: buildFormValuesFromMessage(pendingForm.schema, message.content),
      submittedAt: message.timestamp,
      sourceMessageId: message.id
    })
    formQueueIndex += 1
  }

  const mergedSnapshots = new Map<string, typeof derivedSnapshots[number]>()

  for (const snapshot of derivedSnapshots) {
    mergedSnapshots.set(snapshot.formId, snapshot)
  }

  for (const snapshot of taskSplitStore.submittedForms) {
    const derivedSnapshot = mergedSnapshots.get(snapshot.formId)
    const normalizedSummary = normalizeMultilineText(summarizeFormValues(snapshot.schema, snapshot.values))
    const matchedMessage = sortedMessages
      .filter(message => message.role === 'user' && normalizeMultilineText(message.content) === normalizedSummary)
      .sort((left, right) =>
        resolveMessageTimestampDistance(left.timestamp, snapshot.submittedAt)
        - resolveMessageTimestampDistance(right.timestamp, snapshot.submittedAt)
      )[0]

    mergedSnapshots.set(snapshot.formId, {
      formId: snapshot.formId,
      schema: snapshot.schema,
      promptText: snapshot.promptText,
      requestedAt: derivedSnapshot?.requestedAt || snapshot.submittedAt,
      values: snapshot.values,
      submittedAt: snapshot.submittedAt,
      sourceMessageId: derivedSnapshot?.sourceMessageId || matchedMessage?.id
    })
  }

  return Array.from(mergedSnapshots.values()).sort((left, right) =>
    compareTimestamp(left.submittedAt, right.submittedAt)
  )
})

const timelineEntries = computed<TimelineEntry[]>(() => {
  const entries: TimelineEntry[] = []
  const activeFormId = activeFormSchema.value?.formId ?? null
  const sortedMessages = [...taskSplitStore.messages].sort((left, right) =>
    compareTimestamp(left.timestamp, right.timestamp)
  )
  const submittedMessageIds = new Set(
    historicalSubmittedForms.value
      .map(item => item.sourceMessageId)
      .filter((messageId): messageId is string => Boolean(messageId))
  )
  const activeFormPrompt = [...sortedMessages]
    .slice()
    .reverse()
    .find(message => message.role === 'assistant' && message.content.trim())
    ?.content
    ?.trim()
  const animateLiveEntries = isSessionRunning.value
  let lastThinkingEntry: TimelineEntry | null = null
  let lastAssistantContentEntry: TimelineEntry | null = null
  let nextSequence = 0

  const pushEntry = (entry: TimelineEntry) => {
    entries.push({
      ...entry,
      sequence: nextSequence++
    })
  }

  for (const message of sortedMessages) {
    if (!message.content.trim() || submittedMessageIds.has(message.id)) {
      continue
    }
    if (message.role === 'user') {
      pushEntry({
        id: `message-${message.id}`,
        type: 'message',
        role: message.role,
        content: message.content,
        timestamp: message.timestamp
      })
    } else if (message.role === 'assistant' && !message.formSchema) {
      pushEntry({
        id: `message-${message.id}`,
        type: 'message',
        role: message.role,
        content: message.content,
        timestamp: message.timestamp
      })
    }
  }

  if (!hasRuntimeSystemLog.value) {
    for (const notice of visibleRuntimeNotices.value) {
      pushEntry({
        id: `runtime-notice-${notice.id}`,
        type: 'system',
        content: `### ${notice.title}\n${notice.content}`,
        timestamp: taskSplitStore.session?.startedAt || taskSplitStore.session?.createdAt
      })
    }
  }

  if (usageNoticeEntry.value) {
    pushEntry(usageNoticeEntry.value)
  }

  for (const log of sortedSplitLogs.value) {
    if (!['content', 'thinking', 'thinking_start', 'tool_use', 'tool_input_delta', 'tool_result', 'usage', 'message_start', 'error', 'system'].includes(log.type)) {
      lastThinkingEntry = null
      lastAssistantContentEntry = null
      continue
    }

    if (log.type === 'content' && log.content) {
      if (shouldSuppressStructuredContentLog(log)) {
        lastThinkingEntry = null
        lastAssistantContentEntry = null
        continue
      }

      if (lastAssistantContentEntry) {
        lastAssistantContentEntry.content = `${lastAssistantContentEntry.content || ''}${log.content}`
        lastAssistantContentEntry.timestamp = log.createdAt
        lastAssistantContentEntry.animate = animateLiveEntries
      } else {
        const contentEntry: TimelineEntry = {
          id: `content-${log.id}`,
          type: 'content',
          role: 'assistant',
          content: log.content,
          timestamp: log.createdAt,
          animate: animateLiveEntries
        }
        pushEntry(contentEntry)
        lastAssistantContentEntry = contentEntry
      }

      lastThinkingEntry = null
      continue
    }

    if (log.type === 'thinking_start') {
      if (!lastThinkingEntry) {
        const thinkingEntry: TimelineEntry = {
          id: `log-${log.id}`,
          type: 'thinking',
          content: '',
          timestamp: log.createdAt,
          animate: animateLiveEntries
        }
        pushEntry(thinkingEntry)
        lastThinkingEntry = thinkingEntry
      }
      continue
    }

    if (log.type === 'thinking') {
      if (lastThinkingEntry) {
        lastThinkingEntry.content = [lastThinkingEntry.content, log.content].filter(Boolean).join('\n')
        lastThinkingEntry.timestamp = log.createdAt
        lastThinkingEntry.animate = animateLiveEntries
        continue
      }

      const thinkingEntry: TimelineEntry = {
        id: `log-${log.id}`,
        type: 'thinking',
        content: log.content,
        timestamp: log.createdAt,
        animate: animateLiveEntries
      }
      pushEntry(thinkingEntry)
      lastThinkingEntry = thinkingEntry
      lastAssistantContentEntry = null
      continue
    }

    lastThinkingEntry = null
    lastAssistantContentEntry = null

    if (log.type === 'system') {
      if (isRuntimeSystemLog(log)) {
        continue
      }

      pushEntry({
        id: `system-${log.id}`,
        type: 'system',
        content: log.content,
        timestamp: log.createdAt
      })
      continue
    }

    if (log.type === 'tool_use') {
      const toolCall = toolCallMap.value.get(log.id)
      if (!toolCall) continue
      const normalizedToolName = toolCall.name.toLowerCase()
      const isStructuredOutput = normalizedToolName === 'structuredoutput'
        || normalizedToolName === 'structured_output'
      const hasFormOutput = Boolean(extractFormRequestTurnFromLog(log))

      if (isStructuredOutput && hasFormOutput) {
        continue
      }

      pushEntry({
        id: `tool-${log.id}`,
        type: 'tool',
        toolCall,
        timestamp: log.createdAt,
        animate: animateLiveEntries,
        toolCompact: isStructuredOutput,
        toolDefaultExpanded: isStructuredOutput ? false : undefined,
        toolDefaultResultExpanded: isStructuredOutput ? false : undefined
      })
      continue
    }

    if (log.type === 'tool_result' || log.type === 'tool_input_delta' || log.type === 'usage' || log.type === 'message_start') {
      continue
    }

    pushEntry({
      id: `log-${log.id}`,
      type: log.type === 'error' ? 'error' : 'system',
      content: log.content,
      timestamp: log.createdAt,
      animate: animateLiveEntries
    })
  }

  for (const submittedForm of historicalSubmittedForms.value) {
    pushEntry({
      id: `frozen-form-${submittedForm.formId}-${submittedForm.requestedAt}`,
      type: 'form',
      formSchema: submittedForm.schema,
      formPrompt: submittedForm.promptText,
      formInitialValues: submittedForm.values,
      formDisabled: true,
      formVariant: 'archived',
      timestamp: submittedForm.requestedAt
    })

    pushEntry({
      id: `submitted-form-${submittedForm.formId}-${submittedForm.submittedAt}`,
      type: 'form',
      role: 'user',
      formSchema: submittedForm.schema,
      formPrompt: submittedForm.promptText,
      formInitialValues: submittedForm.values,
      formDisabled: true,
      formVariant: 'submitted',
      timestamp: submittedForm.submittedAt
    })
  }

  const requestedFormIds = new Set<string>()
  for (const submittedForm of historicalSubmittedForms.value) {
    requestedFormIds.add(submittedForm.formId)
  }
  if (activeFormId) {
    requestedFormIds.add(activeFormId)
  }

  for (const turn of formRequestTurns.value) {
    const pendingForm = turn.forms.find(form => !requestedFormIds.has(form.formId))
    if (!pendingForm) {
      continue
    }

    pushEntry({
      id: `archived-form-${pendingForm.formId}-${pendingForm.requestedAt}`,
      type: 'form',
      formSchema: pendingForm.schema,
      formPrompt: pendingForm.promptText,
      formDisabled: true,
      formVariant: 'archived',
      timestamp: pendingForm.requestedAt
    })
  }

  if (activeFormSchema.value && (!showPreview.value || hasPendingRefinement.value)) {
    pushEntry({
      id: `form-${activeFormSchema.value.formId}`,
      type: 'form',
      formSchema: activeFormSchema.value,
      formPrompt: activeFormPrompt,
      formDisabled: taskSplitStore.isProcessing,
      formVariant: 'active',
      timestamp: activeFormRequestedAt.value || taskSplitStore.session?.updatedAt || new Date().toISOString()
    })
  }

  sortTimelineEntries(entries)
  attachSessionElapsedLabel(entries)

  return entries
})

const messageRenderState = computed(() => {
  const latestEntry = timelineEntries.value[timelineEntries.value.length - 1]
  return [
    planStore.splitDialogVisible,
    timelineEntries.value.length,
    latestEntry?.id ?? '',
    latestEntry?.type ?? '',
    latestEntry?.timestamp ?? '',
    latestEntry?.content?.length ?? 0,
    latestEntry?.toolCall?.status ?? '',
    latestEntry?.toolCall?.result?.length ?? 0,
    latestEntry?.formSchema?.formId ?? '',
    taskSplitStore.isProcessing,
    activeFormSchema.value?.formId ?? '',
    showPreview.value
  ].join('|')
})

async function initializeDialogSession() {
  const dialogContext = planStore.splitDialogContext
  if (!dialogContext) return

  try {
    await agentTeamsStore.loadExperts()
    const existingPlan = planStore.plans.find(p => p.id === dialogContext.planId)
    const plan = existingPlan || await planStore.getPlan(dialogContext.planId)
    if (!plan) return

    const project = projectStore.projects.find(p => p.id === plan.projectId)
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
  }
}

async function restartSplit() {
  const dialogContext = planStore.splitDialogContext
  if (!dialogContext) return

  if (showStopButton.value) {
    await taskSplitStore.stop()
  }
  await taskSplitStore.clearPlanSplitSessions(dialogContext.planId)
  taskSplitStore.reset()

  await initializeDialogSession()
}

async function handleFormSubmit(values: Record<string, any>) {
  if (!activeFormSchema.value) return
  try {
    await taskSplitStore.submitFormResponse(activeFormSchema.value.formId, values)
  } catch (error) {
    logger.error('[TaskSplitDialog] Failed to submit task split form:', error)
  }
}

function handleTimelineFormSubmit(_entryId: string, values: Record<string, unknown>) {
  void handleFormSubmit(values as Record<string, any>)
}

async function confirmSplit() {
  const splitContext = planStore.splitDialogContext
  if (!taskSplitStore.splitResult || !splitContext || isConfirming.value) return

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
    console.error('Failed to confirm split:', error)
  } finally {
    isConfirming.value = false
  }
}

async function closeDialog() {
  if (hasPendingRefinement.value) {
    if (showStopButton.value) {
      await taskSplitStore.stop()
    }
    await taskSplitStore.cancelRefinement({ discardSession: true })
  }
  taskSplitStore.detach()
  planStore.closeSplitDialog()
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

function pushInstructionMessages(userContent: string, assistantContent: string) {
  const now = new Date().toISOString()
  const userMsg: SplitMessage = {
    id: crypto.randomUUID(),
    role: 'user',
    content: userContent,
    timestamp: now
  }
  const assistantMsg: SplitMessage = {
    id: crypto.randomUUID(),
    role: 'assistant',
    content: assistantContent,
    timestamp: new Date(Date.now() + 1).toISOString()
  }
  taskSplitStore.messages.push(userMsg, assistantMsg)
}

function autoResizeInput() {
  const el = instructionInputRef.value
  if (!el) return
  el.style.height = 'auto'
  const lineHeight = parseFloat(getComputedStyle(el).lineHeight) || 20
  const maxH = lineHeight * 4 + 16
  el.style.height = `${Math.min(el.scrollHeight, maxH)}px`
}

function updateInstructionSelection() {
  const input = instructionInputRef.value
  if (!input) return
  instructionSelectionStart.value = input.selectionStart ?? userInstruction.value.length
}

function applyMentionSuggestion(index = selectedMentionOptionIndex.value) {
  const mention = mentionState.value
  if (!mention) return

  const option = mention.options[index]
  if (!option) return

  const replacement = `@[${option.index + 1}:${option.title}] `
  userInstruction.value = [
    userInstruction.value.slice(0, mention.rangeStart),
    replacement,
    userInstruction.value.slice(mention.rangeEnd)
  ].join('')
  mentionDismissKey.value = ''

  nextTick(() => {
    const input = instructionInputRef.value
    if (!input) return
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
  if (!taskSplitStore.splitResult) return text
  return materializeTaskMentions(text, taskSplitStore.splitResult)
}

function pushUpdateInstructionMessage(text: string, targetIndex: number, updates: Partial<AITaskItem>) {
  if (typeof updates.title === 'string') {
    pushInstructionMessages(text, t('taskSplit.instructionTitleUpdated', {
      index: targetIndex + 1,
      title: updates.title
    }))
    return
  }
  if (typeof updates.description === 'string') {
    pushInstructionMessages(text, t('taskSplit.instructionDescUpdated', { index: targetIndex + 1 }))
    return
  }
  if (updates.priority) {
    pushInstructionMessages(text, t('taskSplit.instructionPriorityUpdated', {
      index: targetIndex + 1,
      priority: updates.priority
    }))
  }
}

async function executeParsedInstruction(text: string) {
  const tasks = taskSplitStore.splitResult
  if (!tasks) return false

  const parsed = parseTaskInstruction(text, tasks)
  const normalizedText = normalizeTaskInstructionInput(text)
  const instructionPrompt = buildInstructionPrompt(parsed.customPrompt || normalizedText)

  if (parsed.type === 'delete' && parsed.targetIndex !== undefined) {
    const title = tasks[parsed.targetIndex]?.title || ''
    taskSplitStore.removeSplitTask(parsed.targetIndex)
    pushInstructionMessages(text, t('taskSplit.instructionDeleted', {
      index: parsed.targetIndex + 1,
      title
    }))
    return true
  }

  if (parsed.type === 'add') {
    taskSplitStore.addSplitTask()
    const nextIndex = (taskSplitStore.splitResult?.length ?? 1) - 1
    if (parsed.updates && taskSplitStore.splitResult) {
      taskSplitStore.updateSplitTask(nextIndex, parsed.updates)
    }
    pushInstructionMessages(text, t('taskSplit.instructionAdded'))
    return true
  }

  if (parsed.type === 'update' && parsed.targetIndex !== undefined && parsed.updates) {
    taskSplitStore.updateSplitTask(parsed.targetIndex, parsed.updates)
    pushUpdateInstructionMessage(text, parsed.targetIndex, parsed.updates)
    return true
  }

  if (parsed.type === 'move_up' && parsed.targetIndex !== undefined && parsed.targetIndex > 0) {
    const moved = tasks.splice(parsed.targetIndex, 1)[0]
    tasks.splice(parsed.targetIndex - 1, 0, moved)
    pushInstructionMessages(text, t('taskSplit.instructionMovedUp', {
      index: parsed.targetIndex + 1,
      title: moved.title
    }))
    return true
  }

  if (parsed.type === 'move_down' && parsed.targetIndex !== undefined && parsed.targetIndex < tasks.length - 1) {
    const moved = tasks.splice(parsed.targetIndex, 1)[0]
    tasks.splice(parsed.targetIndex + 1, 0, moved)
    pushInstructionMessages(text, t('taskSplit.instructionMovedDown', {
      index: parsed.targetIndex + 1,
      title: moved.title
    }))
    return true
  }

  if (parsed.type === 'resplit' && parsed.targetIndex !== undefined) {
    await taskSplitStore.startSubSplit(parsed.targetIndex, {
      taskIndex: parsed.targetIndex,
      granularity: taskSplitStore.context?.granularity || DEFAULT_SPLIT_GRANULARITY,
      expertId: taskSplitStore.context?.expertId,
      agentId: taskSplitStore.context?.agentId,
      modelId: taskSplitStore.context?.modelId,
      customPrompt: instructionPrompt
    })
    return true
  }

  if (parsed.type === 'optimize' || parsed.type === 'reorder' || parsed.type === 'unknown') {
    await taskSplitStore.startListOptimize({
      expertId: taskSplitStore.context?.expertId,
      agentId: taskSplitStore.context?.agentId,
      modelId: taskSplitStore.context?.modelId,
      customPrompt: instructionPrompt,
      targetIndex: parsed.targetIndex
    })
    return true
  }

  return false
}

async function handleUserInstruction() {
  const text = userInstruction.value.trim()
  if (!text) return

  let handled = false
  if (taskSplitStore.splitResult) {
    handled = await executeParsedInstruction(text)
  } else if (showRecoveryInput.value) {
    await taskSplitStore.continueSessionWithInstruction(text)
    handled = true
  }

  if (!handled) {
    logger.warn('[TaskSplitDialog] Unrecognized instruction:', text)
  }

  userInstruction.value = ''
  mentionDismissKey.value = ''
  nextTick(autoResizeInput)
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

watch(() => planStore.splitDialogVisible, async (visible) => {
  if (visible) {
    await initializeDialogSession()
    await nextTick()
    scrollMessagesToBottom()
  } else {
    taskSplitStore.detach()
  }
})

watch(messageRenderState, async () => {
  await nextTick()
  scrollMessagesToBottom()
}, { flush: 'post' })

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
})

const { handleOverlayPointerDown, handleOverlayClick } = useOverlayDismiss(closeDialog)

  return {
    planStore,
    taskSplitStore,
    taskStore,
    projectStore,
    themeStore,
    agentStore,
    agentTeamsStore,
    isDarkTheme,
    isConfirming,
    messagesContainerRef,
    userInstruction,
    instructionInputRef,
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
    activeFormSchema,
    isSessionRunning,
    showStopButton,
    showLoadingIndicator,
    canRetrySplit,
    canContinueSplit,
    retryActionLabel,
    retryButtonLabel,
    isAutoRetryPending,
    splitErrorMessage,
    primaryActionLabel,
    footerHint,
    sortedSplitLogs,
    toolCallMap,
    latestRuntimeLog,
    runningStatusText,
    scrollMessagesToBottom,
    compareTimestamp,
    toTimestampMs,
    formatElapsedLabel,
    parseLogMetadata,
    readMetadataNumber,
    resolveUsageState,
    buildSummaryValueMap,
    parseFieldSummaryValue,
    formatFieldSummaryValue,
    summarizeFormValues,
    normalizeMultilineText,
    resolveMessageTimestampDistance,
    buildFormValuesFromMessage,
    sortTimelineEntries,
    extractFormRequestTurnFromLog,
    formRequestTurns,
    formRequestHistory,
    activeFormRequestedAt,
    shouldSuppressStructuredContentLog,
    isRuntimeSystemLog,
    shouldSuppressSystemProgressLog,
    visibleRuntimeNotices,
    hasRuntimeSystemLog,
    usageNoticeEntry,
    sessionElapsedLabel,
    attachSessionElapsedLabel,
    historicalSubmittedForms,
    timelineEntries,
    messageRenderState,
    initializeDialogSession,
    restartSplit,
    handleFormSubmit,
    handleTimelineFormSubmit,
    confirmSplit,
    closeDialog,
    stopSplitTask,
    retrySplitTask,
    continueSplitTask,
    pushInstructionMessages,
    handleUserInstruction,
    handleInstructionInput,
    handleInstructionKeydown,
    handleInstructionCaretChange,
    applyMentionSuggestion,
    autoResizeInput,
    handleOverlayPointerDown,
    handleOverlayClick,
  }
}
