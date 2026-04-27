import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import { conversationService } from '@/services/conversation'
import { MANUAL_STOP_ERROR_MARKER, type Message } from '@/stores/message'
import { useMessageStore } from '@/stores/message'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { useTokenStore } from '@/stores/token'
import { FILE_MENTION_PATTERN, getMentionDisplayText } from '@/utils/fileMention'
import {
  isEnvironmentRuntimeNotice,
  getProcessingTimeNoticeSummary,
  isContextRuntimeNotice,
  isProcessingTimeRuntimeNotice
} from '@/utils/runtimeNotice'
import { extractFormResponse, parseStructuredContent } from '@/utils/structuredContent'

export interface MessageBubbleProps {
  message: Message
  sessionId?: string
  hideContextStrategyNotice?: boolean
  sessionMessages?: Message[]
  isCurrentStreamingMessageOverride?: boolean
}

export interface MessageBubbleEmits {
  (event: 'retry', message: Message): void
  (event: 'formSubmit', formId: string, values: Record<string, unknown>, assistantMessageId?: string): void
  (event: 'openEditTrace', messageId: string, traceId: string): void
  (event: 'stop', message: Message): void
}

interface MessagePart {
  type: 'text' | 'file-mention'
  content: string
}

/**
 * 单条消息气泡状态。
 * 负责消息状态推导、结构化内容解析、附件预览与工具调用排序。
 */
export function useMessageBubble(props: MessageBubbleProps, emit: MessageBubbleEmits) {
  const { t, locale } = useI18n()
  const messageStore = useMessageStore()
  const sessionExecutionStore = useSessionExecutionStore()
  const tokenStore = useTokenStore()
  const nowTick = ref(Date.now())
  const areToolCallsExpanded = ref(false)
  let elapsedTimer: ReturnType<typeof setInterval> | null = null

  const isUser = computed(() => props.message.role === 'user')
  const isAssistant = computed(() => props.message.role === 'assistant')
  const isCompression = computed(() => !!props.message.compressionMetadata)
  const isStreaming = computed(() => props.message.status === 'streaming')
  const resolvedSessionMessages = computed(() => {
    if (props.sessionMessages) {
      return props.sessionMessages
    }

    if (!props.sessionId) {
      return []
    }

    return messageStore.messagesBySession(props.sessionId)
  })
  const isCurrentStreamingMessage = computed(() => {
    if (typeof props.isCurrentStreamingMessageOverride === 'boolean') {
      return props.isCurrentStreamingMessageOverride
    }

    if (!props.sessionId || !isStreaming.value) {
      return false
    }

    return sessionExecutionStore.getExecutionState(props.sessionId).currentStreamingMessageId === props.message.id
  })
  const isError = computed(() => props.message.status === 'error')
  const isInterrupted = computed(() => props.message.status === 'interrupted')
  const canRetry = computed(() => isError.value || isInterrupted.value)
  const isManualStopped = computed(() => props.message.errorMessage === MANUAL_STOP_ERROR_MARKER)
  const latestAssistantMessageId = computed(() => {
    const latestAssistant = [...resolvedSessionMessages.value]
      .slice()
      .reverse()
      .find(message => message.role === 'assistant')
    return latestAssistant?.id ?? null
  })
  const latestUserMessageId = computed(() => {
    const latestUser = [...resolvedSessionMessages.value]
      .slice()
      .reverse()
      .find(message => message.role === 'user')
    return latestUser?.id ?? null
  })
  const canRetryCurrentAssistant = computed(() =>
    isAssistant.value
    && !isStreaming.value
    && latestAssistantMessageId.value === props.message.id
    && (canRetry.value || Boolean(props.message.content))
  )
  const canRetryCurrentUser = computed(() =>
    isUser.value
    && canRetry.value
    && latestUserMessageId.value === props.message.id
  )

  watch(
    isStreaming,
    streaming => {
      if (elapsedTimer) {
        clearInterval(elapsedTimer)
        elapsedTimer = null
      }

      if (!streaming) {
        nowTick.value = Date.now()
        return
      }

      nowTick.value = Date.now()
      elapsedTimer = setInterval(() => {
        nowTick.value = Date.now()
      }, 1000)
    },
    { immediate: true }
  )

  onBeforeUnmount(() => {
    if (elapsedTimer) {
      clearInterval(elapsedTimer)
      elapsedTimer = null
    }
  })

  const formattedTime = computed(() => {
    const date = new Date(props.message.createdAt)
    return date.toLocaleTimeString(locale.value, {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false
    })
  })

  const userFormResponse = computed(() => {
    if (!isUser.value) return null
    return extractFormResponse(props.message.content)
  })

  const userFormResponseDisplay = computed(() => {
    const formResponse = userFormResponse.value
    if (!formResponse) return null

    const sessionMessages = resolvedSessionMessages.value
    const currentIndex = sessionMessages.findIndex(message => message.id === props.message.id)
    if (currentIndex < 0) return null

    const fieldLabelMap = new Map<string, string>()
    const fieldOptionsMap = new Map<string, Array<{ label: string; value: unknown }>>()

    for (let index = currentIndex - 1; index >= 0; index -= 1) {
      const candidate = sessionMessages[index]
      if (candidate.role !== 'assistant') continue

      const blocks = parseStructuredContent(candidate.content)
      for (const block of blocks) {
        if (block.type !== 'form' || block.formSchema.formId !== formResponse.formId) continue
        for (const field of block.formSchema.fields) {
          fieldLabelMap.set(field.name, field.label)
          if (field.options) {
            fieldOptionsMap.set(field.name, field.options)
          }
        }
      }
      if (fieldLabelMap.size > 0) break
    }

    const lines: string[] = []
    for (const [key, rawValue] of Object.entries(formResponse.values)) {
      const label = fieldLabelMap.get(key) || key
      const options = fieldOptionsMap.get(key)
      let displayValue = String(rawValue ?? '')

      if (options && options.length > 0) {
        const optionLabels = (Array.isArray(rawValue) ? rawValue : [rawValue])
          .map((value: unknown) => options.find(option => String(option.value) === String(value))?.label || String(value))
        displayValue = optionLabels.join(', ')
      }

      lines.push(`${label}: ${displayValue}`)
    }

    return lines.length > 0 ? lines : null
  })

  const processedUserMessage = computed(() => {
    if (!isUser.value) return []

    if (userFormResponseDisplay.value) {
      return []
    }

    const content = props.message.content
    const parts: MessagePart[] = []
    let lastIndex = 0
    let match: RegExpExecArray | null

    FILE_MENTION_PATTERN.lastIndex = 0

    while ((match = FILE_MENTION_PATTERN.exec(content)) !== null) {
      if (match.index > lastIndex) {
        parts.push({
          type: 'text',
          content: content.slice(lastIndex, match.index)
        })
      }

      parts.push({
        type: 'file-mention',
        content: getMentionDisplayText(match[0], match[1] ?? match[2])
      })

      lastIndex = match.index + match[0].length
    }

    if (lastIndex < content.length) {
      parts.push({
        type: 'text',
        content: content.slice(lastIndex)
      })
    }

    if (parts.length === 0) {
      parts.push({
        type: 'text',
        content
      })
    }

    return parts
  })

  const hasUserText = computed(() =>
    processedUserMessage.value.some(part => part.type === 'file-mention' || part.content.trim().length > 0)
  )

  const statusInfo = computed(() => {
    if (!isUser.value) return null

    switch (props.message.status) {
      case 'pending':
        return { text: t('message.status.userPending'), icon: 'loading', class: 'status--pending' }
      case 'error':
        return { text: t('message.status.userError'), icon: 'error', class: 'status--error' }
      case 'completed':
        return { text: t('message.status.userCompleted'), icon: 'check', class: 'status--completed' }
      default:
        return null
    }
  })

  const assistantStatusInfo = computed(() => {
    if (!isAssistant.value) return null

    switch (props.message.status) {
      case 'streaming':
        return { text: t('message.status.assistantStreaming'), icon: 'loading', class: 'status--streaming' }
      case 'interrupted':
        return {
          text: isManualStopped.value ? t('message.status.assistantStopped') : t('message.status.interrupted'),
          icon: 'square',
          class: 'status--interrupted'
        }
      case 'error':
        return { text: t('message.status.assistantError'), icon: 'error', class: 'status--error' }
      case 'completed':
        return { text: t('message.status.assistantCompleted'), icon: 'check', class: 'status--completed' }
      default:
        return null
    }
  })

  const assistantElapsedLabel = computed(() => {
    if (!isAssistant.value) {
      return ''
    }

    if (!isStreaming.value) {
      const processingTimeNotice = (props.message.runtimeNotices ?? [])
        .find(notice => isProcessingTimeRuntimeNotice(notice))
      return processingTimeNotice
        ? (getProcessingTimeNoticeSummary(processingTimeNotice)?.label || '')
        : ''
    }

    const startedAt = new Date(props.message.createdAt).getTime()
    if (!Number.isFinite(startedAt)) {
      return ''
    }

    const elapsedSeconds = Math.max(0, Math.floor((nowTick.value - startedAt) / 1000))
    const minutes = Math.floor(elapsedSeconds / 60)
    const seconds = elapsedSeconds % 60
    return `${minutes}:${seconds.toString().padStart(2, '0')}`
  })

  const runtimeUsageFallback = computed(() => {
    if (!props.sessionId || !isAssistant.value) {
      return null
    }

    const latestMessageId = messageStore.lastMessage(props.sessionId)?.id
    if (latestMessageId !== props.message.id) {
      return null
    }

    const realtimeUsage = tokenStore.realtimeTokens.get(props.sessionId)
    if (!realtimeUsage) {
      return null
    }

    return {
      model: realtimeUsage.model,
      inputTokens: realtimeUsage.inputTokens,
      outputTokens: realtimeUsage.outputTokens
    }
  })

  const visibleRuntimeNotices = computed(() => {
    const notices = props.message.runtimeNotices ?? []
    return notices.filter(notice =>
      !isContextRuntimeNotice(notice)
      && !isProcessingTimeRuntimeNotice(notice)
      && !isEnvironmentRuntimeNotice(notice)
    )
  })

  const assistantVisibleEditTraces = computed(() => {
    if (props.sessionMessages) {
      return []
    }

    if (!isAssistant.value || !props.message.editTraces?.length) {
      return []
    }

    return messageStore.getVisibleAssistantEditTracesForMessage(props.message.sessionId, props.message.id)
  })

  const errorMessage = computed(() => props.message.errorMessage || t('message.failed'))

  const toolCallCount = computed(() => props.message.toolCalls?.length ?? 0)
  const shouldClampToolCalls = computed(() => toolCallCount.value > 10)

  const sortedToolCalls = computed(() => {
    const toolCalls = props.message.toolCalls ?? []
    return [...toolCalls].sort((left, right) => {
      const leftRunning = left.status === 'running' ? 0 : 1
      const rightRunning = right.status === 'running' ? 0 : 1
      if (leftRunning !== rightRunning) {
        return leftRunning - rightRunning
      }

      if (left.status !== right.status) {
        const weight = (status: string) => {
          switch (status) {
            case 'pending':
              return 0
            case 'running':
              return 1
            case 'error':
              return 2
            default:
              return 3
          }
        }
        return weight(left.status) - weight(right.status)
      }

      return 0
    })
  })

  const assistantStructuredBlocks = computed(() => {
    if (!isAssistant.value) {
      return []
    }

    return parseStructuredContent(props.message.content)
  })

  const assistantFormBlocks = computed(() =>
    assistantStructuredBlocks.value.filter(block => block.type === 'form')
  )

  const isAssistantFormOnly = computed(() => {
    if (!isAssistant.value) {
      return false
    }

    const blocks = assistantStructuredBlocks.value
    return blocks.length > 0 && blocks.every(block => block.type === 'form')
  })

  const assistantFormIds = computed(() =>
    assistantFormBlocks.value.map(block => block.formSchema.formId)
  )

  const resolvedFormResponsesById = computed<Record<string, Record<string, unknown>>>(() => {
    if (assistantFormIds.value.length === 0) {
      return {}
    }

    const sessionMessages = resolvedSessionMessages.value
    const currentIndex = sessionMessages.findIndex(message => message.id === props.message.id)
    if (currentIndex < 0) {
      return {}
    }

    const resolvedById: Record<string, Record<string, unknown>> = {}

    for (let index = currentIndex + 1; index < sessionMessages.length; index += 1) {
      const candidate = sessionMessages[index]
      if (candidate.role !== 'user') {
        continue
      }

      const formResponse = extractFormResponse(candidate.content)
      if (formResponse && assistantFormIds.value.includes(formResponse.formId)) {
        resolvedById[formResponse.formId] = formResponse.values
      }
    }

    return resolvedById
  })

  function handleStop() {
    if (props.sessionMessages) {
      emit('stop', props.message)
      return
    }

    if (props.message.status === 'streaming' && props.sessionId && isCurrentStreamingMessage.value) {
      conversationService.abort(props.sessionId, props.message.id)
      return
    }

    emit('stop', props.message)
  }

  function handleRetry() {
    emit('retry', props.message)
  }

  function handleFormSubmit(formId: string, values: Record<string, unknown>) {
    emit('formSubmit', formId, values, props.message.id)
  }

  function handleOpenEditTrace(traceId: string) {
    if (!traceId) return
    emit('openEditTrace', props.message.id, traceId)
  }

  function formatTraceChangeType(changeType: 'create' | 'modify' | 'delete') {
    switch (changeType) {
      case 'create':
        return '新建'
      case 'delete':
        return '删除'
      default:
        return '修改'
    }
  }

  function getTraceDisplayName(relativePath: string) {
    const segments = relativePath.split(/[\\/]/)
    return segments[segments.length - 1] || relativePath
  }

  function getTraceParentPath(relativePath: string) {
    const segments = relativePath.split(/[\\/]/)
    if (segments.length <= 1) {
      return '项目根目录'
    }
    return segments.slice(0, -1).join('/')
  }

  function getToolCallRenderKey(toolCall: NonNullable<Message['toolCalls']>[number]) {
    return [
      toolCall.id,
      toolCall.status,
      Object.keys(toolCall.arguments ?? {}).length,
      toolCall.result?.length ?? 0,
      toolCall.errorMessage?.length ?? 0
    ].join(':')
  }

  function getTraceChangeIcon(changeType: 'create' | 'modify' | 'delete') {
    switch (changeType) {
      case 'create':
        return 'plus'
      case 'delete':
        return 'trash-2'
      default:
        return 'square-pen'
    }
  }

  function toggleToolCallsExpanded() {
    areToolCallsExpanded.value = !areToolCallsExpanded.value
  }

  return {
    t,
    EaIcon,
    areToolCallsExpanded,
    isUser,
    isAssistant,
    isCompression,
    isStreaming,
    isCurrentStreamingMessage,
    isError,
    isInterrupted,
    canRetry,
    canRetryCurrentAssistant,
    canRetryCurrentUser,
    formattedTime,
    userFormResponseDisplay,
    processedUserMessage,
    hasUserText,
    statusInfo,
    assistantStatusInfo,
    assistantElapsedLabel,
    runtimeUsageFallback,
    visibleRuntimeNotices,
    assistantVisibleEditTraces,
    errorMessage,
    toolCallCount,
    shouldClampToolCalls,
    sortedToolCalls,
    isAssistantFormOnly,
    resolvedFormResponsesById,
    handleStop,
    handleRetry,
    handleFormSubmit,
    handleOpenEditTrace,
    formatTraceChangeType,
    getTraceDisplayName,
    getTraceParentPath,
    getToolCallRenderKey,
    getTraceChangeIcon,
    toggleToolCallsExpanded
  }
}
