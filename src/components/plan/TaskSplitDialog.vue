<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { usePlanStore } from '@/stores/plan'
import { useTaskSplitStore } from '@/stores/taskSplit'
import { useTaskStore } from '@/stores/task'
import { useProjectStore } from '@/stores/project'
import { useThemeStore } from '@/stores/theme'
import TaskSplitPreview from './TaskSplitPreview.vue'
import TaskResplitModal from './TaskResplitModal.vue'
import ExecutionTimeline from '@/components/message/ExecutionTimeline.vue'
import { useOverlayDismiss } from '@/composables/useOverlayDismiss'
import type { AITaskItem, DynamicFormSchema, PlanSplitLogRecord, TaskResplitConfig } from '@/types/plan'
import type { TimelineEntry } from '@/types/timeline'
import { buildToolCallFromLogs, extractDynamicFormSchemas } from '@/utils/toolCallLog'
import { buildUsageNotice } from '@/utils/runtimeNotice'

const planStore = usePlanStore()
const taskSplitStore = useTaskSplitStore()
const taskStore = useTaskStore()
const projectStore = useProjectStore()
const themeStore = useThemeStore()
const isDarkTheme = computed(() => themeStore.isDark)

const isConfirming = ref(false)
const messagesContainerRef = ref<HTMLElement | null>(null)

// 继续拆分相关状态
const resplitModalVisible = ref(false)
const resplitTargetIndex = ref<number | null>(null)
const resplitTargetTask = ref<AITaskItem | null>(null)

// 是否显示预览
const showPreview = computed(() => taskSplitStore.splitResult !== null)

// 当前表单数据
const activeFormSchema = computed(() => taskSplitStore.activeFormSchema)
const isSessionRunning = computed(() => taskSplitStore.session?.status === 'running')
const showStopButton = computed(() => taskSplitStore.session?.status === 'running')
const showLoadingIndicator = computed(() => taskSplitStore.session?.status === 'running')
const canRetrySplit = computed(() => taskSplitStore.session?.status === 'failed')
const retryActionLabel = computed(() => {
  const hasUserMessage = taskSplitStore.messages.some(message =>
    message.role === 'user' && message.content.trim()
  )
  return hasUserMessage ? '重新发送' : '重试'
})
const splitErrorMessage = computed(() =>
  taskSplitStore.session?.errorMessage?.trim()
  || taskSplitStore.session?.parseError?.trim()
  || ''
)
const footerHint = computed(() => {
  if (canRetrySplit.value) {
    return splitErrorMessage.value || 'AI 响应失败，可点击重试重新发起请求。'
  }

  if (taskSplitStore.session?.status === 'stopped') {
    if (activeFormSchema.value) {
      return '当前会话已停止，可继续填写上方表单后再提交。'
    }
    const hasArchivedForms = taskSplitStore.logs.some(log =>
      log.type === 'content' && log.content.includes('"type": "form_request"')
    )
    return hasArchivedForms
      ? '当前会话已停止，历史表单已固化展示，可回看每一轮的建议与表单内容。'
      : '当前会话已停止，且没有待填写的表单数据，仅保留已有思考记录。'
  }

  if (taskSplitStore.session?.status === 'running') {
    return '后台正在执行拆分，可关闭弹框稍后回来查看'
  }

  if (taskSplitStore.session?.status === 'waiting_input' && activeFormSchema.value) {
    return '请根据上方 AI 动态表单逐步补充需求'
  }

  if (!activeFormSchema.value && !showPreview.value) {
    return '当前会话没有待展示的表单数据。'
  }

  return '请根据上方 AI 动态表单逐步补充需求'
})

const latestRuntimeLog = computed(() => {
  if (taskSplitStore.logs.length === 0) return null
  return [...taskSplitStore.logs].sort((left, right) =>
    compareTimestamp(left.createdAt, right.createdAt)
  )[taskSplitStore.logs.length - 1] ?? null
})

const runningStatusText = computed(() => {
  if (!isSessionRunning.value) {
    return ''
  }

  const latestLog = latestRuntimeLog.value
  if (!latestLog) {
    return '任务已启动，等待首个输出...'
  }

  if (latestLog.type === 'thinking') {
    return '正在思考并拆分任务...'
  }

  if (latestLog.type === 'thinking_start') {
    return '正在进入思考阶段...'
  }

  if (latestLog.type === 'system') {
    return '正在加载运行扩展...'
  }

  if (latestLog.type === 'tool_use') {
    const toolCall = buildToolCallFromLogs(latestLog, taskSplitStore.logs, {
      fallbackStatus: isSessionRunning.value ? 'running' : 'success'
    })
    return toolCall ? `正在调用工具 ${toolCall.name}...` : '正在调用工具...'
  }

  if (latestLog.type === 'tool_input_delta') {
    return '工具参数正在流式更新...'
  }

  if (latestLog.type === 'content') {
    return '正在生成结构化结果...'
  }

  if (latestLog.type === 'tool_result') {
    return '工具返回结果，正在继续处理...'
  }

  if (latestLog.type === 'error') {
    return '收到错误输出，正在等待最终状态...'
  }

  return '正在处理中...'
})

function scrollMessagesToBottom() {
  const container = messagesContainerRef.value
  if (!container) return

  container.scrollTop = container.scrollHeight
}

function compareTimestamp(left?: string, right?: string) {
  return new Date(left || 0).getTime() - new Date(right || 0).getTime()
}

function parseLogMetadata(log: PlanSplitLogRecord): Record<string, unknown> | null {
  if (!log.metadata) return null
  if (typeof log.metadata === 'string') {
    try {
      return JSON.parse(log.metadata) as Record<string, unknown>
    } catch {
      return null
    }
  }
  return log.metadata as unknown as Record<string, unknown>
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
        .split('、')
        .map(item => item.trim())
        .filter(Boolean)
        .map(parseOptionValue)
    case 'number':
    case 'slider': {
      const numericValue = Number(normalizedValue)
      return Number.isFinite(numericValue) ? numericValue : normalizedValue
    }
    case 'checkbox':
      return ['true', '1', 'yes', 'on', '是'].includes(normalizedValue.toLowerCase())
    default:
      return normalizedValue
  }
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

function getTimelineEntryPriority(entry: TimelineEntry) {
  if (entry.type === 'thinking') return 0
  if (entry.type === 'tool') return 1
  if (entry.type === 'content') return 2
  if (entry.type === 'message') {
    return entry.role === 'user' ? 5 : 3
  }
  if (entry.type === 'form') return 4
  if (entry.type === 'system') return 6
  if (entry.type === 'error') return 7
  return 8
}

function sortTimelineEntries(entries: TimelineEntry[]) {
  entries.sort((left, right) => {
    const timeDiff = compareTimestamp(left.timestamp, right.timestamp)
    if (timeDiff !== 0) {
      return timeDiff
    }

    const priorityDiff = getTimelineEntryPriority(left) - getTimelineEntryPriority(right)
    if (priorityDiff !== 0) {
      return priorityDiff
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

  const toolCall = buildToolCallFromLogs(log, taskSplitStore.logs, {
    fallbackStatus: isSessionRunning.value ? 'running' : 'success'
  })
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
  const sortedLogs = [...taskSplitStore.logs].sort((left, right) =>
    compareTimestamp(left.createdAt, right.createdAt)
  )

  for (const log of sortedLogs) {
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

const suppressAssistantContentLogs = computed(() =>
  formRequestTurns.value.length > 0 || showPreview.value
)

const hasRuntimeSystemLog = computed(() =>
  taskSplitStore.logs.some(log => log.type === 'system' && log.content.trim())
)

const usageNoticeEntry = computed<TimelineEntry | null>(() => {
  const usageLogs = [...taskSplitStore.logs]
    .filter(log => log.type === 'usage' || log.type === 'message_start')
    .sort((left, right) => compareTimestamp(left.createdAt, right.createdAt))

  const usageState: { model?: string, inputTokens?: number, outputTokens?: number } = {}
  for (const log of usageLogs) {
    const metadata = parseLogMetadata(log)
    const model = typeof metadata?.model === 'string' ? metadata.model : undefined
    const inputTokens = typeof metadata?.inputTokens === 'number' ? metadata.inputTokens : undefined
    const outputTokens = typeof metadata?.outputTokens === 'number' ? metadata.outputTokens : undefined

    if (model) usageState.model = model
    if (inputTokens !== undefined) usageState.inputTokens = inputTokens
    if (outputTokens !== undefined) usageState.outputTokens = outputTokens
  }

  const notice = buildUsageNotice(usageState)
  if (!notice) return null

  const latestUsageLog = usageLogs[usageLogs.length - 1]
  return {
    id: `usage-${latestUsageLog?.id || 'runtime'}`,
    type: 'system',
    content: `### ${notice.title}\n${notice.content}`,
    timestamp: latestUsageLog?.createdAt || taskSplitStore.session?.updatedAt
  }
})

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
    mergedSnapshots.set(snapshot.formId, {
      formId: snapshot.formId,
      schema: snapshot.schema,
      promptText: snapshot.promptText,
      requestedAt: derivedSnapshot?.requestedAt || snapshot.submittedAt,
      values: snapshot.values,
      submittedAt: snapshot.submittedAt,
      sourceMessageId: derivedSnapshot?.sourceMessageId
    })
  }

  return Array.from(mergedSnapshots.values()).sort((left, right) =>
    compareTimestamp(left.submittedAt, right.submittedAt)
  )
})

const timelineEntries = computed<TimelineEntry[]>(() => {
  const entries: TimelineEntry[] = []
  const activeFormId = activeFormSchema.value?.formId ?? null
  const submittedMessageIds = new Set(
    historicalSubmittedForms.value
      .map(item => item.sourceMessageId)
      .filter((messageId): messageId is string => Boolean(messageId))
  )
  const submittedMessagesById = new Map(
    taskSplitStore.messages
      .filter(message => message.role === 'user' && message.content.trim())
      .map(message => [message.id, message] as const)
  )
  const activeFormPrompt = [...taskSplitStore.messages]
    .slice()
    .reverse()
    .find(message => message.role === 'assistant' && message.content.trim())
    ?.content
    ?.trim()
  const animateLiveEntries = isSessionRunning.value
  let lastThinkingEntry: TimelineEntry | null = null
  let lastAssistantContentEntry: TimelineEntry | null = null

  for (const message of taskSplitStore.messages) {
    if (message.role !== 'user' || !message.content.trim() || submittedMessageIds.has(message.id)) {
      continue
    }

    entries.push({
      id: `message-${message.id}`,
      type: 'message',
      role: message.role,
      content: message.content,
      timestamp: message.timestamp
    })
  }

  if (!hasRuntimeSystemLog.value) {
    for (const notice of taskSplitStore.runtimeNotices) {
      entries.push({
        id: `runtime-notice-${notice.id}`,
        type: 'system',
        content: `### ${notice.title}\n${notice.content}`,
        timestamp: taskSplitStore.session?.startedAt || taskSplitStore.session?.createdAt
      })
    }
  }

  if (usageNoticeEntry.value) {
    entries.push(usageNoticeEntry.value)
  }

  const sortedLogs = [...taskSplitStore.logs].sort((left, right) =>
    compareTimestamp(left.createdAt, right.createdAt)
  )

  for (const log of sortedLogs) {
    if (!['content', 'thinking', 'thinking_start', 'tool_use', 'tool_input_delta', 'tool_result', 'usage', 'message_start', 'error', 'system'].includes(log.type)) {
      lastThinkingEntry = null
      lastAssistantContentEntry = null
      continue
    }

    if (log.type === 'content' && log.content) {
      if (suppressAssistantContentLogs.value) {
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
        entries.push(contentEntry)
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
        entries.push(thinkingEntry)
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
      entries.push(thinkingEntry)
      lastThinkingEntry = thinkingEntry
      lastAssistantContentEntry = null
      continue
    }

    lastThinkingEntry = null
    lastAssistantContentEntry = null

    if (log.type === 'system') {
      entries.push({
        id: `system-${log.id}`,
        type: 'system',
        content: log.content,
        timestamp: log.createdAt
      })
      continue
    }

    if (log.type === 'tool_use') {
      const toolCall = buildToolCallFromLogs(log, taskSplitStore.logs, {
        fallbackStatus: animateLiveEntries ? 'running' : 'success'
      })
      if (!toolCall) continue
      const normalizedToolName = toolCall.name.toLowerCase()
      const isStructuredOutput = normalizedToolName === 'structuredoutput'
        || normalizedToolName === 'structured_output'
      const hasFormOutput = Boolean(extractFormRequestTurnFromLog(log))

      if (isStructuredOutput && hasFormOutput) {
        continue
      }

      entries.push({
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

    if (log.type === 'tool_result') {
      continue
    }

    if (log.type === 'tool_input_delta') {
      continue
    }

    if (log.type === 'usage' || log.type === 'message_start') {
      continue
    }

    entries.push({
      id: `log-${log.id}`,
      type: log.type === 'error' ? 'error' : 'system',
      content: log.content,
      timestamp: log.createdAt,
      animate: animateLiveEntries
    })
  }

  sortTimelineEntries(entries)

  for (const submittedForm of historicalSubmittedForms.value) {
    entries.push({
      id: `submitted-form-${submittedForm.formId}-${submittedForm.submittedAt}`,
      type: 'form',
      formSchema: submittedForm.schema,
      formPrompt: submittedForm.promptText,
      formInitialValues: submittedForm.values,
      formDisabled: true,
      formVariant: 'submitted',
      timestamp: submittedForm.requestedAt
    })

    const submittedMessage = submittedForm.sourceMessageId
      ? submittedMessagesById.get(submittedForm.sourceMessageId)
      : null

    if (submittedMessage?.content.trim()) {
      entries.push({
        id: `message-${submittedMessage.id}`,
        type: 'message',
        role: 'user',
        content: submittedMessage.content,
        timestamp: submittedForm.submittedAt
      })
    }
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

    entries.push({
      id: `archived-form-${pendingForm.formId}-${pendingForm.requestedAt}`,
      type: 'form',
      formSchema: pendingForm.schema,
      formPrompt: pendingForm.promptText,
      formDisabled: true,
      formVariant: 'archived',
      timestamp: pendingForm.requestedAt
    })
  }

  if (activeFormSchema.value && !showPreview.value) {
    entries.push({
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

  return entries
})

const messageRenderState = computed(() => {
  const timelineSignature = timelineEntries.value
    .map(entry => [
      entry.id,
      entry.type,
      entry.timestamp || '',
      entry.content?.length ?? 0,
      entry.toolCall?.status ?? '',
      entry.toolCall?.result?.length ?? 0,
      entry.toolCall ? Object.keys(entry.toolCall.arguments || {}).length : 0,
      entry.formSchema?.formId ?? '',
      entry.animate ? '1' : '0'
    ].join(':'))
    .join('|')
  return [
    planStore.splitDialogVisible,
    timelineEntries.value.length,
    timelineSignature,
    taskSplitStore.isProcessing,
    activeFormSchema.value?.formId ?? '',
    showPreview.value
  ].join('|')
})

async function initializeDialogSession() {
  const dialogContext = planStore.splitDialogContext
  if (!dialogContext) return

  const existingPlan = planStore.plans.find(p => p.id === dialogContext.planId)
  const plan = existingPlan || await planStore.getPlan(dialogContext.planId)
  if (!plan) return

  const project = projectStore.projects.find(p => p.id === plan.projectId)
  await taskSplitStore.initSession({
    planId: plan.id,
    planName: plan.name,
    planDescription: plan.description,
    granularity: plan.granularity,
    agentId: dialogContext.agentId,
    modelId: dialogContext.modelId,
    workingDirectory: project?.path
  })
}

// 重新拆分（清理当前状态，开始新会话）
async function restartSplit() {
  const dialogContext = planStore.splitDialogContext
  if (!dialogContext) return

  if (showStopButton.value) {
    await taskSplitStore.stop()
  }
  await taskSplitStore.clearPlanSplitSessions(dialogContext.planId)
  taskSplitStore.reset()

  // 开始新会话
  await initializeDialogSession()
}

// 处理表单提交
async function handleFormSubmit(values: Record<string, any>) {
  if (!activeFormSchema.value) return
  await taskSplitStore.submitFormResponse(activeFormSchema.value.formId, values)
}

function handleTimelineFormSubmit(_entryId: string, values: Record<string, unknown>) {
  void handleFormSubmit(values as Record<string, any>)
}

// 打开继续拆分弹框
function handleResplit(index: number) {
  const tasks = taskSplitStore.splitResult
  if (!tasks || !tasks[index]) return

  resplitTargetIndex.value = index
  resplitTargetTask.value = tasks[index]
  resplitModalVisible.value = true
}

// 确认继续拆分配置
async function handleResplitConfirm(config: TaskResplitConfig) {
  if (resplitTargetIndex.value === null) return

  const dialogContext = planStore.splitDialogContext
  if (!dialogContext) return

  // 更新配置中的 taskIndex
  config.taskIndex = resplitTargetIndex.value

  // 关闭弹框
  resplitModalVisible.value = false

  // 启动子拆分模式
  await taskSplitStore.startSubSplit(resplitTargetIndex.value, config)
}

// 确认拆分结果
async function confirmSplit() {
  const splitContext = planStore.splitDialogContext
  if (!taskSplitStore.splitResult || !splitContext || isConfirming.value) return

  // 如果是子拆分模式，先合并结果
  if (taskSplitStore.subSplitMode) {
    taskSplitStore.completeSubSplit(taskSplitStore.splitResult)
    return // 合并后继续显示更新后的任务列表，不关闭弹框
  }

  const planId = splitContext.planId
  isConfirming.value = true

  try {
    // 转换为 CreateTaskInput 格式
    const taskInputs = taskSplitStore.splitResult.map((task, index) => ({
      planId,
      title: task.title,
      description: task.description,
      priority: task.priority,
      agentId: task.agentId,
      modelId: task.modelId,
      implementationSteps: task.implementationSteps,
      testSteps: task.testSteps,
      acceptanceCriteria: task.acceptanceCriteria,
      dependsOn: task.dependsOn, // 传递依赖关系（任务标题列表）
      order: index
    }))

    // 批量创建任务
    await taskStore.createTasksFromSplit(planId, taskInputs)

    // 重新加载任务列表，确保 TaskBoard 显示最新数据
    await taskStore.loadTasks(planId)

    // 同步更新计划状态为"已拆分"
    await planStore.markPlanAsReady(planId)
    planStore.setCurrentPlan(planId)

    await taskSplitStore.clearPlanSplitSessions(planId)

    // 重置并关闭对话框
    taskSplitStore.reset()
    planStore.closeSplitDialog()
  } catch (error) {
    console.error('Failed to confirm split:', error)
  } finally {
    isConfirming.value = false
  }
}

// 关闭对话框（保存状态以便下次恢复）
async function closeDialog() {
  taskSplitStore.detach()
  planStore.closeSplitDialog()
}

async function stopSplitTask() {
  await taskSplitStore.stop()
}

async function retrySplitTask() {
  await taskSplitStore.retry()
}

// 监听对话框打开
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

const { handleOverlayPointerDown, handleOverlayClick } = useOverlayDismiss(closeDialog)
</script>

<template>
  <Teleport to="body">
    <div
      v-if="planStore.splitDialogVisible"
      class="split-dialog-overlay"
      :class="{ 'split-dialog-overlay--dark': isDarkTheme }"
      @pointerdown.capture="handleOverlayPointerDown"
      @click.self="handleOverlayClick"
    >
      <div
        class="split-dialog"
        :class="{ 'split-dialog--dark': isDarkTheme }"
      >
        <div class="dialog-header">
          <h4>
            <span class="dialog-icon">✂️</span>
            任务拆分
          </h4>
          <button
            class="btn-close"
            @click="closeDialog"
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="dialog-body">
          <div class="split-content">
            <div class="conversation-pane">
              <div
                ref="messagesContainerRef"
                class="messages-container"
              >
                <ExecutionTimeline
                  :entries="timelineEntries"
                  @form-submit="handleTimelineFormSubmit"
                  @form-cancel="closeDialog"
                  @message-form-submit="(_formId, values) => handleTimelineFormSubmit('', values)"
                />

                <div
                  v-if="showLoadingIndicator"
                  class="message assistant"
                >
                  <div class="message-content loading">
                    <span class="dot" />
                    <span class="dot" />
                    <span class="dot" />
                  </div>
                  <div class="message-loading-status">
                    {{ runningStatusText }}
                  </div>
                </div>
              </div>
            </div>

            <div
              v-if="showPreview"
              class="preview-pane"
            >
              <TaskSplitPreview
                :tasks="taskSplitStore.splitResult!"
                @update="taskSplitStore.updateSplitTask"
                @remove="taskSplitStore.removeSplitTask"
                @add="taskSplitStore.addSplitTask"
                @resplit="handleResplit"
              />
            </div>
          </div>
        </div>

        <div class="dialog-footer">
          <!-- 无预览时通过动态表单引导，不展示自由输入 -->
          <div
            v-if="!showPreview"
            class="footer-bar"
          >
            <span
              class="idle-hint"
              :class="{ 'idle-hint--error': canRetrySplit }"
            >
              {{ footerHint }}
            </span>
            <div class="footer-actions">
              <button
                v-if="canRetrySplit"
                class="btn btn-secondary btn-retry"
                @click="retrySplitTask"
              >
                {{ retryActionLabel }}
              </button>
              <button
                v-if="showStopButton"
                class="btn btn-danger"
                @click="stopSplitTask"
              >
                停止任务
              </button>
              <button
                class="btn btn-secondary"
                @click="closeDialog"
              >
                取消
              </button>
            </div>
          </div>

          <!-- 确认按钮 - 仅在有预览时显示 -->
          <div
            v-else
            class="footer-actions footer-actions--confirm"
          >
            <button
              v-if="showStopButton"
              class="btn btn-danger"
              @click="stopSplitTask"
            >
              停止任务
            </button>
            <button
              class="btn btn-secondary"
              :disabled="isConfirming"
              @click="closeDialog"
            >
              关闭
            </button>
            <button
              class="btn btn-secondary"
              @click="restartSplit"
            >
              重新拆分
            </button>
            <button
              class="btn btn-primary"
              :disabled="isConfirming"
              @click="confirmSplit"
            >
              {{ isConfirming ? '创建中...' : '确认并创建任务' }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 继续拆分配置弹框 -->
    <TaskResplitModal
      v-model:visible="resplitModalVisible"
      :task="resplitTargetTask"
      :default-granularity="taskSplitStore.context?.granularity || 3"
      :default-agent-id="taskSplitStore.context?.agentId"
      :default-model-id="taskSplitStore.context?.modelId"
      @confirm="handleResplitConfirm"
    />
  </Teleport>
</template>

<style scoped>
.split-dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--color-bg-overlay, rgba(0, 0, 0, 0.5));
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: var(--z-modal-backdrop, 1040);
  backdrop-filter: blur(4px);
}

.split-dialog {
  --split-dialog-bg:
    radial-gradient(circle at 12% 0%, rgba(14, 165, 233, 0.12), transparent 26%),
    radial-gradient(circle at 88% 100%, rgba(99, 102, 241, 0.1), transparent 30%),
    var(--color-surface, #fff);
  --split-dialog-border: rgba(148, 163, 184, 0.2);
  --split-dialog-shadow: var(--shadow-xl, 0 20px 25px -5px rgba(0, 0, 0, 0.1));
  --split-dialog-header-bg-color: rgba(255, 255, 255, 0.92);
  --split-dialog-header-bg: linear-gradient(90deg, rgba(239, 246, 255, 0.92), rgba(238, 242, 255, 0.9));
  --split-dialog-footer-bg: linear-gradient(180deg, #f8fbff, #f1f5ff);
  --split-pane-bg: var(--color-surface, #fff);
  --split-pane-border: rgba(125, 148, 188, 0.22);
  --split-pane-shadow: 0 10px 24px rgba(15, 23, 42, 0.06);
  --split-messages-bg:
    radial-gradient(circle at top left, rgba(59, 130, 246, 0.06), transparent 42%),
    linear-gradient(to bottom, var(--color-bg-secondary, #f8fafc), var(--color-surface, #fff) 35%);
  --split-assistant-message-bg: linear-gradient(180deg, #ffffff, #f8fbff);
  --split-assistant-message-border: rgba(148, 163, 184, 0.26);
  --split-assistant-message-text: var(--color-text-primary, #1e293b);
  --split-loading-text: var(--color-text-secondary, #64748b);
  background-color: var(--color-surface, #fff);
  border-radius: 1.15rem;
  width: min(96vw, 92rem);
  max-width: 92rem;
  height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--split-dialog-shadow);
  animation: dialogIn 0.2s var(--easing-out);
  border: 1px solid var(--split-dialog-border);
  background: var(--split-dialog-bg);
}

@keyframes dialogIn {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(-10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4, 1rem) var(--spacing-5, 1.25rem);
  border-bottom: 1px solid var(--color-border, #e2e8f0);
  flex-shrink: 0;
  background-color: var(--split-dialog-header-bg-color);
  background-image: var(--split-dialog-header-bg);
}

.dialog-header h4 {
  margin: 0;
  font-size: var(--font-size-base, 14px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
}

.dialog-icon {
  font-size: 1.125rem;
  width: 1.9rem;
  height: 1.9rem;
  border-radius: 0.75rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #38bdf8, #6366f1);
  color: #fff;
  box-shadow: 0 8px 18px rgba(79, 70, 229, 0.3);
}

.btn-close {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-1, 0.25rem);
  border: none;
  background: transparent;
  color: var(--color-text-tertiary, #94a3b8);
  cursor: pointer;
  border-radius: var(--radius-md, 8px);
  transition: all var(--transition-fast, 150ms);
}

.btn-close:hover {
  background-color: var(--color-surface-hover, #f8fafc);
  color: var(--color-text-primary, #1e293b);
}

.split-dialog--dark .dialog-header {
  background-color: rgba(17, 24, 39, 0.98) !important;
  background-image: linear-gradient(90deg, rgba(30, 64, 175, 0.2), rgba(49, 46, 129, 0.22)) !important;
  border-bottom-color: rgba(71, 85, 105, 0.68) !important;
}

.split-dialog--dark .dialog-header h4,
.split-dialog--dark .btn-close {
  color: #e2e8f0 !important;
}

.dialog-body {
  flex: 1;
  overflow: hidden;
}

.split-content {
  height: 100%;
  display: flex;
  gap: var(--spacing-3, 0.75rem);
  padding: var(--spacing-3, 0.75rem);
}

.conversation-pane {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--split-pane-border);
  border-radius: 0.95rem;
  overflow: hidden;
  background-color: var(--split-pane-bg);
  box-shadow: var(--split-pane-shadow);
}

.preview-pane {
  min-width: 0;
  width: 46%;
  border: 1px solid var(--split-pane-border);
  border-radius: 0.95rem;
  overflow: hidden;
  background-color: var(--split-pane-bg);
  display: flex;
  flex-direction: column;
  box-shadow: var(--split-pane-shadow);
}

.messages-container {
  --timeline-panel-width: min(100%, 29.5rem);
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-4, 1rem);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2, 0.5rem);
  background: var(--split-messages-bg);
}

.message {
  display: flex;
  width: 100%;
}

.message.user {
  justify-content: flex-end;
}

.message.assistant {
  justify-content: flex-start;
}

.message-content {
  padding: var(--spacing-3, 0.75rem) var(--spacing-4, 1rem);
  border-radius: 1rem;
  font-size: var(--font-size-sm, 13px);
  line-height: 1.6;
  width: fit-content;
  max-width: min(85%, 42rem);
  border: 1px solid transparent;
  box-shadow: 0 8px 16px rgba(15, 23, 42, 0.05);
}

.message.user .message-content {
  background: linear-gradient(135deg, #0ea5e9, #6366f1);
  color: white;
  border-bottom-right-radius: 0.38rem;
  box-shadow: 0 12px 20px rgba(79, 70, 229, 0.25);
}

.message.assistant .message-content {
  background: var(--split-assistant-message-bg);
  color: var(--split-assistant-message-text);
  border-bottom-left-radius: 0.38rem;
  border-color: var(--split-assistant-message-border);
}

.message-content p {
  margin: 0;
  white-space: pre-line;
  word-break: break-word;
}

.message-form {
  margin-top: -0.18rem;
}

.form-message-content {
  width: 100%;
  max-width: min(85%, 44rem);
  background: transparent !important;
  border: none !important;
  padding: 0;
  box-shadow: none;
}

.form-message-content.disabled {
  opacity: 0.72;
  pointer-events: none;
}

.message-content.loading {
  display: flex;
  gap: 4px;
  padding: var(--spacing-4, 1rem);
  box-shadow: none;
}

.message-loading-status {
  margin-top: -0.35rem;
  padding: 0 var(--spacing-4, 1rem) var(--spacing-4, 1rem);
  font-size: 0.78rem;
  color: var(--split-loading-text);
}

.message-content.loading .dot {
  width: 8px;
  height: 8px;
  background-color: var(--color-text-tertiary, #94a3b8);
  border-radius: 50%;
  animation: bounce 1.4s ease-in-out infinite both;
}

.message-content.loading .dot:nth-child(1) {
  animation-delay: -0.32s;
}

.message-content.loading .dot:nth-child(2) {
  animation-delay: -0.16s;
}

@keyframes bounce {
  0%, 80%, 100% {
    transform: scale(0);
  }
  40% {
    transform: scale(1);
  }
}

.submitted-values {
  margin-top: var(--spacing-3, 0.75rem);
  padding: var(--spacing-3, 0.75rem);
  background: rgba(255, 255, 255, 0.3);
  border-radius: 0.7rem;
  border: 1px dashed rgba(255, 255, 255, 0.34);
}

.submitted-value-item {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-2, 0.5rem);
  font-size: var(--font-size-sm, 13px);
}

.submitted-value-item .field-label {
  color: rgba(255, 255, 255, 0.8);
  flex-shrink: 0;
}

.submitted-value-item .field-value {
  color: white;
  font-weight: var(--font-weight-medium, 500);
}

.message.assistant .submitted-values {
  background: linear-gradient(180deg, #f1f5f9, #e7eef8);
  border: 1px dashed rgba(99, 102, 241, 0.25);
}

.message.assistant .submitted-value-item .field-label {
  color: #64748b;
}

.message.assistant .submitted-value-item .field-value {
  color: #0f172a;
}

.dialog-footer {
  padding: var(--spacing-4, 1rem) var(--spacing-5, 1.25rem);
  border-top: 1px solid var(--color-border, #e2e8f0);
  background: var(--split-dialog-footer-bg);
  flex-shrink: 0;
}

.footer-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3, 0.75rem);
  flex-wrap: wrap;
}

.idle-hint {
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-tertiary, #94a3b8);
}

.idle-hint--error {
  color: #dc2626;
}

.footer-actions {
  display: flex;
  gap: var(--spacing-3, 0.75rem);
  align-items: center;
  justify-content: flex-end;
  flex-wrap: wrap;
}

.footer-actions--confirm {
  width: 100%;
}

.btn-retry {
  border-color: rgba(14, 165, 233, 0.36);
  color: #0369a1;
  background: linear-gradient(180deg, #ffffff, #eff8ff);
}

.split-log-panel {
  margin-top: 0.9rem;
  border: 1px solid rgba(148, 163, 184, 0.24);
  border-radius: 0.9rem;
  background: linear-gradient(180deg, rgba(248, 250, 252, 0.96), rgba(241, 245, 249, 0.92));
  overflow: hidden;
}

.split-log-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.65rem 0.85rem;
  border-bottom: 1px solid rgba(148, 163, 184, 0.18);
  font-size: 0.78rem;
  font-weight: 600;
  color: #1e293b;
}

.split-log-panel__status {
  color: #64748b;
  font-size: 0.72rem;
}

.split-log-panel__body {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
  padding: 0.8rem;
}

.split-log-panel__entry {
  padding: 0.7rem 0.8rem;
  border-radius: 0.75rem;
  background: var(--color-surface, #ffffff);
  border: 1px solid var(--color-border, rgba(148, 163, 184, 0.18));
}

.split-log-panel__entry-type {
  margin-bottom: 0.45rem;
  font-size: 0.68rem;
  font-weight: 700;
  color: #6366f1;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.split-log-panel__entry--error .split-log-panel__entry-type {
  color: #dc2626;
}

.btn {
  padding: var(--spacing-2, 0.5rem) var(--spacing-4, 1rem);
  border-radius: 0.72rem;
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-medium, 500);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-primary {
  background: linear-gradient(135deg, #0ea5e9, #6366f1);
  color: white;
  border: none;
  box-shadow: 0 9px 18px rgba(79, 70, 229, 0.24);
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 12px 24px rgba(79, 70, 229, 0.3);
}

.btn-secondary {
  background: var(--color-surface, #ffffff);
  color: var(--color-text-primary, #334155);
  border: 1px solid var(--color-border, rgba(148, 163, 184, 0.42));
}

.btn-secondary:hover {
  background: linear-gradient(180deg, var(--color-surface, #ffffff), var(--color-bg-secondary, #f5f9ff));
  border-color: rgba(99, 102, 241, 0.35);
}

.btn-danger {
  background: linear-gradient(135deg, #ef4444, #dc2626);
  color: #fff;
  border: none;
  box-shadow: 0 9px 18px rgba(220, 38, 38, 0.2);
}

.btn-danger:hover {
  transform: translateY(-1px);
  box-shadow: 0 12px 24px rgba(220, 38, 38, 0.24);
}

.split-dialog--dark .dialog-footer {
  background: linear-gradient(180deg, rgba(17, 24, 39, 0.98), rgba(15, 23, 42, 0.96)) !important;
  border-top-color: rgba(71, 85, 105, 0.68) !important;
}

.split-dialog--dark .footer-bar {
  background: transparent !important;
}

.split-dialog--dark .idle-hint {
  color: #94a3b8 !important;
}

.split-dialog--dark .idle-hint--error {
  color: #fca5a5 !important;
}

.split-dialog--dark .btn-secondary {
  background: rgba(15, 23, 42, 0.92) !important;
  color: #e2e8f0 !important;
  border-color: rgba(71, 85, 105, 0.76) !important;
}

.split-dialog--dark .btn-secondary:hover {
  background: rgba(30, 41, 59, 0.96) !important;
  border-color: rgba(100, 116, 139, 0.82) !important;
}

:global([data-theme='dark']) .split-dialog,
:global(.dark) .split-dialog {
  --split-dialog-bg:
    radial-gradient(circle at 12% 0%, rgba(14, 165, 233, 0.16), transparent 26%),
    radial-gradient(circle at 88% 100%, rgba(99, 102, 241, 0.14), transparent 30%),
    linear-gradient(180deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.96));
  --split-dialog-border: rgba(71, 85, 105, 0.72);
  --split-dialog-shadow: 0 28px 56px rgba(2, 6, 23, 0.48);
  --split-dialog-header-bg-color: rgba(17, 24, 39, 0.98);
  --split-dialog-header-bg: linear-gradient(90deg, rgba(30, 64, 175, 0.2), rgba(49, 46, 129, 0.22));
  --split-dialog-footer-bg: linear-gradient(180deg, rgba(17, 24, 39, 0.98), rgba(15, 23, 42, 0.96));
  --split-pane-bg: rgba(15, 23, 42, 0.86);
  --split-pane-border: rgba(71, 85, 105, 0.76);
  --split-pane-shadow: 0 16px 34px rgba(2, 6, 23, 0.28);
  --split-messages-bg:
    radial-gradient(circle at top left, rgba(59, 130, 246, 0.12), transparent 42%),
    linear-gradient(to bottom, rgba(15, 23, 42, 0.98), rgba(17, 24, 39, 0.94) 35%);
  --split-assistant-message-bg: linear-gradient(180deg, rgba(17, 24, 39, 0.96), rgba(15, 23, 42, 0.92));
  --split-assistant-message-border: rgba(71, 85, 105, 0.62);
  --split-assistant-message-text: #e2e8f0;
  --split-loading-text: #94a3b8;
}

:global([data-theme='dark']) .split-dialog .btn-close:hover,
:global(.dark) .split-dialog .btn-close:hover {
  background-color: rgba(51, 65, 85, 0.72);
  color: #f8fafc;
}

/* 取消状态的消息样式 */
.message.cancelled {
  opacity: 0.65;
}

.message.cancelled .message-content {
  border-style: dashed;
}

.cancelled-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-top: 6px;
  padding: 2px 8px;
  background: rgba(239, 68, 68, 0.1);
  border-radius: 4px;
  font-size: 11px;
  color: #ef4444;
}

.cancelled-badge svg {
  opacity: 0.8;
}

@media (max-width: 1024px) {
  .split-content {
    flex-direction: column;
  }

  .preview-pane {
    width: 100%;
    min-height: 16rem;
  }

  .footer-bar {
    align-items: flex-start;
  }

  .footer-actions,
  .footer-actions--confirm {
    width: 100%;
    justify-content: flex-start;
  }
}
</style>
