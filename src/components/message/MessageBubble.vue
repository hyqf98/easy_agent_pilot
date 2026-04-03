<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Message } from '@/stores/message'
import { useMessageStore } from '@/stores/message'
import { conversationService } from '@/services/conversation'
import { useTokenStore } from '@/stores/token'
import { EaIcon } from '@/components/common'
import { FILE_MENTION_PATTERN, getMentionDisplayText } from '@/utils/fileMention'
import { extractFormResponse, parseStructuredContent } from '@/utils/structuredContent'
import { getAttachmentPreviewUrl, resolveAttachmentPreviewUrl } from '@/utils/attachmentPreview'
import { getProcessingTimeNoticeSummary, isContextRuntimeNotice, isProcessingTimeRuntimeNotice } from '@/utils/runtimeNotice'
import StructuredContentRenderer from './StructuredContentRenderer.vue'
import ToolCallDisplay from './ToolCallDisplay.vue'
import ThinkingDisplay from './ThinkingDisplay.vue'
import CompressionMessageBubble from './CompressionMessageBubble.vue'
import RuntimeNoticeList from './RuntimeNoticeList.vue'

const { t, locale } = useI18n()
const props = withDefaults(defineProps<{
  message: Message
  sessionId?: string
  hideContextStrategyNotice?: boolean
}>(), {
  sessionId: undefined,
  hideContextStrategyNotice: false
})
const emit = defineEmits<{
  retry: [message: Message]
  formSubmit: [formId: string, values: Record<string, unknown>]
  openEditTrace: [messageId: string, traceId: string]
}>()

const isUser = computed(() => props.message.role === 'user')
const isAssistant = computed(() => props.message.role === 'assistant')
const isCompression = computed(() => props.message.role === 'compression')
const isStreaming = computed(() => props.message.status === 'streaming')
const isError = computed(() => props.message.status === 'error')
const messageStore = useMessageStore()
const tokenStore = useTokenStore()
const messageAttachmentPreviews = ref<Array<{ id: string, name: string, previewUrl: string }>>([])
const nowTick = ref(Date.now())
let elapsedTimer: ReturnType<typeof setInterval> | null = null

async function syncMessageAttachmentPreviews(): Promise<void> {
  const attachments = props.message.attachments ?? []
  if (attachments.length === 0) {
    messageAttachmentPreviews.value = []
    return
  }

  messageAttachmentPreviews.value = attachments.map(attachment => ({
    id: attachment.id,
    name: attachment.name,
    previewUrl: getAttachmentPreviewUrl(attachment)
  }))

  const resolvedPreviews = await Promise.all(attachments.map(async attachment => ({
    id: attachment.id,
    name: attachment.name,
    previewUrl: await resolveAttachmentPreviewUrl(attachment)
  })))

  if ((props.message.attachments ?? []).map(attachment => attachment.id).join('|') !== attachments.map(attachment => attachment.id).join('|')) {
    return
  }

  messageAttachmentPreviews.value = resolvedPreviews
}

watch(
  () => props.message.attachments?.map(attachment => `${attachment.id}:${attachment.path}`).join('|') || '',
  () => {
    void syncMessageAttachmentPreviews()
  },
  { immediate: true }
)

watch(isStreaming, (streaming) => {
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
}, { immediate: true })

onBeforeUnmount(() => {
  if (elapsedTimer) {
    clearInterval(elapsedTimer)
    elapsedTimer = null
  }
})

// 停止流式输出
const handleStop = () => {
  if (props.message.status === 'streaming' && props.sessionId) {
    conversationService.abort(props.sessionId, props.message.id)
  }
}

// 格式化时间戳为 HH:MM 格式，并跟随当前界面语言
const formattedTime = computed(() => {
  const date = new Date(props.message.createdAt)
  return date.toLocaleTimeString(locale.value, {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false
  })
})

// 处理用户消息中的文件引用
interface MessagePart {
  type: 'text' | 'file-mention'
  content: string
}

const userFormResponse = computed(() => {
  if (!isUser.value || !props.sessionId) return null
  return extractFormResponse(props.message.content)
})

const userFormResponseDisplay = computed(() => {
  const formResponse = userFormResponse.value
  if (!formResponse || !props.sessionId) return null

  const sessionMessages = messageStore.messagesBySession(props.sessionId)
  const currentIndex = sessionMessages.findIndex(message => message.id === props.message.id)
  if (currentIndex < 0) return null

  // 从前面的 assistant 消息中查找表单 schema，获取字段 label
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
        .map((val: unknown) => options.find(opt => String(opt.value) === String(val))?.label || String(val))
      displayValue = optionLabels.join(', ')
    }

    lines.push(`${label}: ${displayValue}`)
  }

  return lines.length > 0 ? lines : null
})

const processedUserMessage = computed(() => {
  if (!isUser.value) return []

  const content = props.message.content

  // 如果是表单响应，已在 userFormResponseDisplay 中单独渲染
  if (userFormResponseDisplay.value) {
    return []
  }

  const parts: MessagePart[] = []

  // 支持 @path 和 @"path with spaces" 两种文件引用格式
  let lastIndex = 0
  let match: RegExpExecArray | null

  FILE_MENTION_PATTERN.lastIndex = 0

  while ((match = FILE_MENTION_PATTERN.exec(content)) !== null) {
    // 添加 @ 之前的文本
    if (match.index > lastIndex) {
      parts.push({
        type: 'text',
        content: content.slice(lastIndex, match.index)
      })
    }

    // 添加文件引用
    parts.push({
      type: 'file-mention',
      content: getMentionDisplayText(match[0], match[1] ?? match[2])
    })

    lastIndex = match.index + match[0].length
  }

  // 添加剩余的文本
  if (lastIndex < content.length) {
    parts.push({
      type: 'text',
      content: content.slice(lastIndex)
    })
  }

  // 如果没有匹配到任何文件引用，返回整个文本
  if (parts.length === 0) {
    parts.push({
      type: 'text',
      content: content
    })
  }

  return parts
})

const hasUserText = computed(() =>
  processedUserMessage.value.some(part => part.type === 'file-mention' || part.content.trim().length > 0)
)

// 用户消息状态文本和图标
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

// AI 消息状态
const assistantStatusInfo = computed(() => {
  if (!isAssistant.value) return null

  switch (props.message.status) {
    case 'streaming':
      return { text: t('message.status.assistantStreaming'), icon: 'loading', class: 'status--streaming' }
    case 'interrupted':
      return { text: t('message.status.interrupted'), icon: 'square', class: 'status--interrupted' }
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
    !isContextRuntimeNotice(notice) && !isProcessingTimeRuntimeNotice(notice)
  )
})

const assistantVisibleEditTraces = computed(() => {
  if (!isAssistant.value || !props.message.editTraces?.length) {
    return []
  }

  return messageStore.getVisibleAssistantEditTracesForMessage(props.message.sessionId, props.message.id)
})

// 失败原因
const errorMessage = computed(() => props.message.errorMessage || t('message.failed'))

// 处理重试
const handleRetry = () => {
  emit('retry', props.message)
}

const handleFormSubmit = (formId: string, values: Record<string, unknown>) => {
  emit('formSubmit', formId, values)
}

const handleOpenEditTrace = (traceId: string) => {
  if (!traceId) {
    return
  }

  emit('openEditTrace', props.message.id, traceId)
}

const formatTraceChangeType = (changeType: 'create' | 'modify' | 'delete') => {
  switch (changeType) {
    case 'create':
      return '新建'
    case 'delete':
      return '删除'
    default:
      return '修改'
  }
}

const getTraceDisplayName = (relativePath: string) => {
  const segments = relativePath.split(/[\\/]/)
  return segments[segments.length - 1] || relativePath
}

const getTraceParentPath = (relativePath: string) => {
  const segments = relativePath.split(/[\\/]/)
  if (segments.length <= 1) {
    return '项目根目录'
  }
  return segments.slice(0, -1).join('/')
}

const getToolCallRenderKey = (toolCall: NonNullable<Message['toolCalls']>[number]) => {
  return [
    toolCall.id,
    toolCall.status,
    Object.keys(toolCall.arguments ?? {}).length,
    toolCall.result?.length ?? 0,
    toolCall.errorMessage?.length ?? 0
  ].join(':')
}

const getTraceChangeIcon = (changeType: 'create' | 'modify' | 'delete') => {
  switch (changeType) {
    case 'create':
      return 'plus'
    case 'delete':
      return 'trash-2'
    default:
      return 'square-pen'
  }
}

const toolCallCount = computed(() => props.message.toolCalls?.length ?? 0)
const shouldClampToolCalls = computed(() => toolCallCount.value > 10)
const areToolCallsExpanded = ref(false)
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

const toggleToolCallsExpanded = () => {
  areToolCallsExpanded.value = !areToolCallsExpanded.value
}
const assistantStructuredBlocks = computed(() => {
  if (!isAssistant.value) {
    return []
  }

  return parseStructuredContent(props.message.content)
})

const assistantFormBlocks = computed(() => {
  return assistantStructuredBlocks.value
    .filter(block => block.type === 'form')
})

const isAssistantFormOnly = computed(() => {
  if (!isAssistant.value) {
    return false
  }

  const blocks = assistantStructuredBlocks.value
  return blocks.length > 0 && blocks.every(block => block.type === 'form')
})

const assistantFormIds = computed(() => {
  return assistantFormBlocks.value.map(block => block.formSchema.formId)
})

const resolvedFormResponse = computed(() => {
  if (!props.sessionId || assistantFormIds.value.length === 0) {
    return null
  }

  const sessionMessages = messageStore.messagesBySession(props.sessionId)
  const currentIndex = sessionMessages.findIndex(message => message.id === props.message.id)
  if (currentIndex < 0) {
    return null
  }

  for (let index = currentIndex + 1; index < sessionMessages.length; index += 1) {
    const candidate = sessionMessages[index]
    if (candidate.role !== 'user') {
      continue
    }

    const formResponse = extractFormResponse(candidate.content)
    if (formResponse && assistantFormIds.value.includes(formResponse.formId)) {
      return formResponse
    }
  }

  return null
})


</script>

<template>
  <!-- 压缩消息使用专用组件 -->
  <CompressionMessageBubble
    v-if="isCompression"
    :message="message"
  />

  <!-- 普通消息 -->
  <div
    v-else
    :class="[
      'message-bubble',
      {
        'message-bubble--user': isUser,
        'message-bubble--assistant': isAssistant,
        'message-bubble--form-only': isAssistantFormOnly
      }
    ]"
  >
    <!-- AI 头像 -->
    <div
      v-if="isAssistant"
      class="message-bubble__avatar"
    >
      <span class="avatar-icon">🤖</span>
    </div>
    <div class="message-bubble__body">
      <!-- 思考过程显示 -->
      <Transition name="slide-fade">
        <div
          v-if="isAssistant && (message.thinkingActive || message.thinking)"
          class="message-bubble__thinking"
        >
          <ThinkingDisplay
            :thinking="message.thinking || ''"
            :live="isStreaming"
            :default-expanded="false"
          />
        </div>
      </Transition>

      <div
        class="message-bubble__content"
        :class="{ 'message-bubble__content--form-only': isAssistantFormOnly }"
      >
        <StructuredContentRenderer
          v-if="!isUser"
          :content="message.content"
          :interactive-forms="isAssistant && !resolvedFormResponse"
          :form-disabled="Boolean(resolvedFormResponse)"
          :animate="isAssistant && isStreaming"
          :resolved-form-values="resolvedFormResponse?.values ?? null"
          @form-submit="handleFormSubmit"
        />
        <div
          v-else-if="userFormResponseDisplay"
          class="message-bubble__form-response"
        >
          <div
            v-for="(line, index) in userFormResponseDisplay"
            :key="index"
            class="message-bubble__form-response-item"
          >
            <span class="message-bubble__form-response-label">{{ line.split(': ')[0] }}</span>
            <span class="message-bubble__form-response-value">{{ line.split(': ').slice(1).join(': ') }}</span>
          </div>
        </div>
        <div
          v-else-if="hasUserText"
          class="message-bubble__text"
        >
          <template
            v-for="(part, index) in processedUserMessage"
            :key="index"
          >
            <span
              v-if="part.type === 'file-mention'"
              class="file-mention"
            >
              <span class="file-mention__icon">📄</span>
              <span class="file-mention__path">{{ part.content }}</span>
            </span>
            <span v-else>{{ part.content }}</span>
          </template>
        </div>
        <div
          v-if="isUser && messageAttachmentPreviews.length > 0"
          class="message-bubble__attachments"
        >
          <img
            v-for="attachment in messageAttachmentPreviews"
            :key="attachment.id"
            :src="attachment.previewUrl"
            :alt="attachment.name"
            :title="attachment.name"
            class="message-bubble__attachment-image"
          >
        </div>
        <span
          v-if="isStreaming"
          class="message-bubble__cursor"
        />
      </div>

      <div
        v-if="isAssistant && visibleRuntimeNotices.length > 0"
        class="message-bubble__runtime"
      >
        <RuntimeNoticeList
          :notices="visibleRuntimeNotices"
          :fallback-usage="runtimeUsageFallback"
        />
      </div>

      <!-- 工具调用显示 -->
      <div
        v-if="isAssistant && message.toolCalls && message.toolCalls.length > 0"
        class="message-bubble__tool-calls-shell"
        :class="{ 'message-bubble__tool-calls-shell--scrollable': shouldClampToolCalls }"
      >
        <button
          type="button"
          class="message-bubble__tool-calls-head"
          :aria-expanded="areToolCallsExpanded"
          @click="toggleToolCallsExpanded"
        >
          <span class="message-bubble__tool-calls-title">工具调用</span>
          <span class="message-bubble__tool-calls-head-right">
            <span class="message-bubble__tool-calls-count">{{ toolCallCount }}</span>
            <span class="message-bubble__tool-calls-toggle">
              {{ areToolCallsExpanded ? t('message.collapse') : t('message.expand') }}
            </span>
          </span>
        </button>
        <TransitionGroup
          v-if="areToolCallsExpanded"
          name="tool-call"
          tag="div"
          class="message-bubble__tool-calls"
        >
          <ToolCallDisplay
            v-for="toolCall in sortedToolCalls"
            :key="getToolCallRenderKey(toolCall)"
            :tool-call="toolCall"
            :live="isStreaming || toolCall.status === 'running'"
            :default-expanded="false"
            :default-result-expanded="false"
          />
        </TransitionGroup>
        <button
          v-if="areToolCallsExpanded"
          type="button"
          class="message-bubble__tool-calls-footer"
          @click="toggleToolCallsExpanded"
        >
          <span class="message-bubble__tool-calls-toggle">
            {{ t('message.collapse') }}
          </span>
        </button>
      </div>

      <div
        v-if="assistantVisibleEditTraces.length > 0"
        class="message-bubble__trace-rail"
      >
        <div class="message-bubble__trace-rail-head">
          <div class="message-bubble__trace-rail-title">
            <EaIcon
              name="files"
              :size="14"
            />
            <span>文件变更</span>
          </div>
          <span class="message-bubble__trace-rail-count">{{ assistantVisibleEditTraces.length }}</span>
        </div>

        <div class="message-bubble__trace-strip-wrap">
          <div class="message-bubble__trace-strip">
            <button
              v-for="trace in assistantVisibleEditTraces"
              :key="trace.id"
              class="message-bubble__trace-tile"
              :class="`message-bubble__trace-tile--${trace.changeType}`"
              @click="handleOpenEditTrace(trace.id)"
            >
              <div class="message-bubble__trace-tile-top">
                <span class="message-bubble__trace-tile-icon">
                  <EaIcon
                    name="file-code"
                    :size="16"
                  />
                </span>
                <span
                  class="message-bubble__trace-tile-tag"
                  :class="`message-bubble__trace-tile-tag--${trace.changeType}`"
                >
                  <EaIcon
                    :name="getTraceChangeIcon(trace.changeType)"
                    :size="10"
                  />
                  <span>{{ formatTraceChangeType(trace.changeType) }}</span>
                </span>
              </div>
              <div class="message-bubble__trace-tile-name">
                {{ getTraceDisplayName(trace.relativePath) }}
              </div>
              <div class="message-bubble__trace-tile-path">
                {{ getTraceParentPath(trace.relativePath) }}
              </div>
              <div class="message-bubble__trace-tile-meta">
                <span>L{{ trace.range.startLine }}-{{ trace.range.endLine }}</span>
                <EaIcon
                  name="arrow-up-right"
                  :size="12"
                />
              </div>
            </button>
          </div>
        </div>
      </div>

      <!-- 时间戳和状态信息 -->
      <div class="message-bubble__meta">
        <span class="message-bubble__time">{{ formattedTime }}</span>
        <!-- 用户消息状态 -->
        <span
          v-if="statusInfo"
          :class="['message-bubble__status', statusInfo.class]"
        >
          <span
            v-if="statusInfo.icon === 'loading'"
            class="status-icon status-icon--loading"
          >⏳</span>
          <span
            v-else-if="statusInfo.icon === 'error'"
            class="status-icon"
          >⚠️</span>
          <span
            v-else-if="statusInfo.icon === 'check'"
            class="status-icon"
          >✓</span>
          <span class="status-text">{{ statusInfo.text }}</span>
        </span>
        <!-- AI 消息状态 -->
        <span
          v-if="assistantStatusInfo"
          :class="['message-bubble__status', assistantStatusInfo.class]"
        >
          <span
            v-if="assistantStatusInfo.icon === 'loading'"
            class="status-icon status-icon--loading"
          >⏳</span>
          <span
            v-else-if="assistantStatusInfo.icon === 'error'"
            class="status-icon"
          >⚠️</span>
          <span
            v-else-if="assistantStatusInfo.icon === 'check'"
            class="status-icon"
          >✓</span>
          <span
            v-else-if="assistantStatusInfo.icon === 'square'"
            class="status-icon status-icon--interrupted"
          >⏹</span>
          <span class="status-text">{{ assistantStatusInfo.text }}</span>
          <span
            v-if="assistantElapsedLabel"
            class="message-bubble__elapsed"
          >
            {{ assistantElapsedLabel }}
          </span>
        </span>
        <!-- 停止按钮 - 仅在流式输出时显示 -->
        <button
          v-if="isAssistant && isStreaming"
          class="message-bubble__stop"
          :title="t('common.stop')"
          @click="handleStop"
        >
          ⏹
        </button>
        <!-- 重试按钮 - 用户消息失败 -->
        <button
          v-if="isUser && isError"
          class="message-bubble__retry"
          :title="errorMessage"
          @click="handleRetry"
        >
          {{ t('common.retry') }}
        </button>
        <!-- 重试按钮 - AI 消息 -->
        <button
          v-if="isAssistant && !isStreaming && (isError || message.content)"
          class="message-bubble__retry"
          :title="t('message.retry')"
          @click="handleRetry"
        >
          {{ t('message.retry') }}
        </button>
      </div>
      <!-- 错误消息提示 -->
      <div
        v-if="isError"
        class="message-bubble__error"
      >
        {{ errorMessage }}
      </div>
    </div>
    <div
      v-if="isUser"
      class="message-bubble__avatar message-bubble__avatar--user"
    >
      <span class="avatar-icon">🙂</span>
    </div>
  </div>
</template>

<style scoped>
.message-bubble {
  --message-fixed-width: 36rem;
  --message-min-width: var(--message-fixed-width);
  --message-max-width: 50rem;
  --message-compact-max-width: var(--message-fixed-width);
  --message-compact-max-height: 20rem;
  --message-trace-max-width: 46rem;
  --thinking-display-width: var(--message-fixed-width);
  display: flex;
  flex-direction: row;
  width: 100%;
  max-width: 100%;
  gap: var(--spacing-3);
}

.message-bubble--user {
  justify-content: flex-end;
  align-items: flex-start;
}

.message-bubble--assistant {
  justify-content: flex-start;
  align-items: flex-start;
}

.message-bubble--assistant.message-bubble--form-only {
  --message-fixed-width: clamp(17rem, 30vw, 24rem);
  --message-min-width: 0;
  --message-max-width: clamp(18rem, 36vw, 31rem);
  --message-compact-max-width: clamp(17rem, 34vw, 28rem);
  --thinking-display-width: clamp(16rem, 34vw, 26rem);
}

/* AI 头像样式 */
.message-bubble__avatar {
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-full);
  background: linear-gradient(135deg, var(--color-accent-light), var(--color-accent));
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: var(--shadow-sm);
}

.message-bubble__avatar--user {
  background: linear-gradient(135deg, #60a5fa, #2563eb);
}

.avatar-icon {
  font-size: 16px;
}

/* 消息主体 */
.message-bubble__body {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  min-width: 0;
  width: fit-content;
  max-width: min(100%, var(--message-max-width));
  box-sizing: border-box;
}

.message-bubble--assistant .message-bubble__body {
  flex: 0 1 auto;
  align-items: flex-start;
}

.message-bubble--assistant.message-bubble--form-only .message-bubble__body {
  width: min(100%, var(--message-max-width));
  max-width: min(100%, var(--message-max-width));
  min-width: 0;
}

.message-bubble--user .message-bubble__body {
  flex: 0 1 auto;
  align-items: flex-end;
}

/* 思考过程显示 */
.message-bubble__thinking {
  width: var(--message-compact-max-width);
  min-width: min(var(--message-min-width), var(--message-compact-max-width));
  max-width: 100%;
  max-height: var(--message-compact-max-height);
  min-height: fit-content;
  animation: fadeSlideDown 0.3s ease-out;
  overflow: hidden;
  border-radius: var(--radius-lg);
}

/* 消息内容 - 中间区域 */
.message-bubble__content {
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-lg);
  font-size: var(--font-size-sm);
  line-height: 1.6;
  width: fit-content;
  min-width: min(100%, var(--message-fixed-width));
  max-width: min(100%, var(--message-max-width));
  box-sizing: border-box;
  animation: fadeIn 0.2s ease-out;
  overflow: visible;
}

/* AI 消息样式 */
.message-bubble--assistant .message-bubble__content {
  min-width: min(100%, var(--message-fixed-width));
  background-color: var(--color-bg-tertiary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg) var(--radius-lg) var(--radius-lg) var(--radius-sm);
  color: var(--color-text-primary);
}

.message-bubble--assistant .message-bubble__content--form-only {
  width: 100%;
  min-width: 0;
  max-width: 100%;
  max-height: none;
  padding: 0;
  overflow: visible;
  background: transparent;
  border: 0;
  border-radius: 0;
  box-shadow: none;
}

/* 用户消息样式 */
.message-bubble--user .message-bubble__content {
  background-color: var(--color-primary-light);
  color: var(--color-primary-dark);
  border-radius: var(--radius-lg) var(--radius-sm) var(--radius-lg) var(--radius-lg);
}

/* 暗色模式下用户消息 */
:global([data-theme='dark']) .message-bubble--user .message-bubble__content,
:global(.dark) .message-bubble--user .message-bubble__content {
  background-color: rgba(96, 165, 250, 0.15);
  color: var(--color-primary);
}

:global([data-theme='dark']) .message-bubble--assistant .message-bubble__content,
:global(.dark) .message-bubble--assistant .message-bubble__content {
  color: #e5e7eb;
}

.message-bubble__text {
  white-space: break-spaces;
  overflow-wrap: anywhere;
  word-break: break-word;
}

.message-bubble__form-response {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
}

.message-bubble__form-response-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 0;
  line-height: 1.4;
}

.message-bubble__form-response-label {
  font-size: 0.78em;
  color: color-mix(in srgb, currentColor 60%, transparent);
  font-weight: 500;
}

.message-bubble__form-response-value {
  font-size: 0.92em;
  font-weight: 600;
}

.message-bubble__content ::selection {
  background: color-mix(in srgb, var(--color-primary) 24%, transparent);
  color: inherit;
}

.message-bubble__content ::-moz-selection {
  background: color-mix(in srgb, var(--color-primary) 24%, transparent);
  color: inherit;
}

.message-bubble__attachments {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
  margin-bottom: var(--spacing-2);
}

.message-bubble__attachment-image {
  width: 72px;
  height: 72px;
  object-fit: cover;
  border-radius: var(--radius-md);
  border: 1px solid rgba(148, 163, 184, 0.35);
  background: rgba(255, 255, 255, 0.65);
  box-shadow: var(--shadow-sm);
}

/* 文件引用样式 */
/* 文件引用样式 - 更突出的视觉效果 */
.file-mention {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 10px;
  margin: 0 2px;
  background: linear-gradient(135deg, rgba(96, 165, 250, 0.12), rgba(96, 165, 250, 0.08));
  color: var(--color-primary);
  border: 1px solid rgba(96, 165, 250, 0.25);
  border-radius: var(--radius-md);
  font-size: 0.85em;
  font-family: var(--font-family-mono);
  text-decoration: none;
  transition: all var(--transition-fast) var(--easing-default);
  vertical-align: baseline;
  cursor: default;
}

.file-mention:hover {
  background: linear-gradient(135deg, rgba(96, 165, 250, 0.2), rgba(96, 165, 250, 0.15));
  border-color: rgba(96, 165, 250, 0.4);
  box-shadow: 0 2px 8px rgba(96, 165, 250, 0.15);
}

.file-mention__icon {
  flex-shrink: 0;
  font-size: 1em;
  opacity: 0.9;
}

.file-mention__path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 250px;
  font-weight: 500;
  letter-spacing: 0.02em;
}

/* 暗色模式下的文件引用 */
:global([data-theme='dark']) .file-mention,
:global(.dark) .file-mention {
  background: linear-gradient(135deg, rgba(96, 165, 250, 0.18), rgba(96, 165, 250, 0.1));
  border-color: rgba(96, 165, 250, 0.3);
}

:global([data-theme='dark']) .file-mention:hover,
:global(.dark) .file-mention:hover {
  background: linear-gradient(135deg, rgba(96, 165, 250, 0.28), rgba(96, 165, 250, 0.18));
  border-color: rgba(96, 165, 250, 0.5);
}

.message-bubble__cursor {
  display: inline-block;
  width: 2px;
  height: 1em;
  margin-left: 2px;
  background-color: var(--color-primary);
  animation: blink 1s step-end infinite;
}

/* 工具调用显示 - 底部区域，橙色渐变边框 */
.message-bubble__tool-calls-shell {
  --tool-call-shell-max-height: min(40rem, calc(3.35rem * 10 + var(--spacing-2) * 9 + 3rem));
  width: var(--message-compact-max-width);
  min-width: min(100%, var(--message-fixed-width));
  max-width: 100%;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: 0.75rem;
  border-radius: 1rem;
  border: 1px solid var(--tool-call-shell-border);
  background: var(--tool-call-shell-bg);
  box-shadow: var(--tool-call-shell-shadow);
  overflow: hidden;
}

.message-bubble__tool-calls-shell--scrollable {
  max-height: var(--tool-call-shell-max-height);
}

.message-bubble__tool-calls-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  width: 100%;
  padding: 0 0.1rem;
  background: transparent;
  color: inherit;
  cursor: pointer;
}

.message-bubble__tool-calls-title {
  font-size: 0.74rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--tool-call-shell-title);
}

.message-bubble__tool-calls-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.5rem;
  height: 1.5rem;
  padding: 0 0.4rem;
  border-radius: 999px;
  background: var(--tool-call-shell-count-bg);
  color: var(--tool-call-shell-count-text);
  font-size: 0.72rem;
  font-weight: 700;
}

.message-bubble__tool-calls-head-right {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
}

.message-bubble__tool-calls-toggle {
  font-size: 0.74rem;
  color: var(--color-text-secondary);
}

.message-bubble__tool-calls {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  min-height: 0;
  overflow: visible;
}

.message-bubble__tool-calls-footer {
  align-self: flex-end;
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.5rem;
  padding: 0;
  margin-top: 0.15rem;
  background: transparent;
  color: inherit;
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.message-bubble__tool-calls-footer:hover {
  transform: translateY(-1px);
  opacity: 0.9;
}

.message-bubble__tool-calls-shell--scrollable .message-bubble__tool-calls {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  padding-right: 0.2rem;
  scrollbar-gutter: stable;
  overscroll-behavior: contain;
  scrollbar-width: thin;
  scrollbar-color: var(--tool-call-shell-scrollbar-thumb) transparent;
}

.message-bubble__tool-calls-shell--scrollable .message-bubble__tool-calls::-webkit-scrollbar {
  width: 6px;
}

.message-bubble__tool-calls-shell--scrollable .message-bubble__tool-calls::-webkit-scrollbar-track {
  background: transparent;
}

.message-bubble__tool-calls-shell--scrollable .message-bubble__tool-calls::-webkit-scrollbar-thumb {
  background: var(--tool-call-shell-scrollbar-thumb);
  border-radius: 999px;
}

.message-bubble__runtime {
  width: var(--message-compact-max-width);
  min-width: min(100%, var(--message-fixed-width));
  max-width: 100%;
}

.message-bubble__trace-rail {
  --trace-tile-count-bg: color-mix(in srgb, var(--color-primary) 12%, transparent);
  --trace-tile-count-text: color-mix(in srgb, var(--color-primary) 72%, var(--color-text-primary) 28%);
  --trace-tile-glow: color-mix(in srgb, var(--color-primary) 16%, transparent);
  --trace-tile-surface-top: color-mix(in srgb, var(--color-bg-elevated) 82%, white 18%);
  --trace-tile-surface-bottom: color-mix(in srgb, var(--color-bg-secondary) 94%, var(--color-bg-elevated) 6%);
  --trace-tile-border: color-mix(in srgb, var(--color-border) 58%, transparent);
  --trace-tile-shadow: var(--shadow-md);
  --trace-tile-hover-border: color-mix(in srgb, var(--color-primary) 42%, var(--color-border) 58%);
  --trace-tile-hover-shadow: var(--shadow-lg);
  --trace-tile-icon-bg: color-mix(in srgb, var(--color-primary) 14%, transparent);
  --trace-tile-icon-color: color-mix(in srgb, var(--color-primary) 84%, #1d4ed8 16%);
  --trace-tile-meta-border: color-mix(in srgb, var(--color-border) 44%, transparent);
  width: fit-content;
  min-width: var(--message-min-width);
  max-width: min(100%, var(--message-trace-max-width));
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.message-bubble__trace-rail-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 0 2px;
}

.message-bubble__trace-rail-title {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.message-bubble__trace-rail-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 22px;
  height: 22px;
  padding: 0 7px;
  border-radius: 999px;
  background: var(--trace-tile-count-bg);
  color: var(--trace-tile-count-text);
  font-size: 11px;
  font-weight: 700;
}

.message-bubble__trace-strip {
  width: 100%;
  display: flex;
  gap: var(--spacing-2);
  overflow-x: auto;
  padding: 2px 2px 6px;
  scroll-snap-type: x proximity;
}

.message-bubble__trace-strip-wrap {
  position: relative;
  width: 100%;
}

.message-bubble__trace-strip::-webkit-scrollbar {
  height: 6px;
}

.message-bubble__trace-strip::-webkit-scrollbar-track {
  background: transparent;
}

.message-bubble__trace-strip::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.38);
  border-radius: 999px;
}

.message-bubble__trace-tile {
  position: relative;
  flex: 0 0 120px;
  display: flex;
  flex-direction: column;
  width: 120px;
  min-height: 120px;
  padding: 10px 10px 12px;
  gap: 8px;
  background:
    radial-gradient(circle at top right, var(--trace-tile-glow), transparent 44%),
    linear-gradient(180deg, var(--trace-tile-surface-top), var(--trace-tile-surface-bottom));
  border: 1px solid var(--trace-tile-border);
  border-radius: 18px;
  text-align: left;
  cursor: pointer;
  scroll-snap-align: start;
  box-shadow: var(--trace-tile-shadow);
  overflow: hidden;
  transition: transform var(--transition-fast) var(--easing-default), border-color var(--transition-fast) var(--easing-default), box-shadow var(--transition-fast) var(--easing-default), background var(--transition-fast) var(--easing-default);
}

.message-bubble__trace-tile::after {
  content: '';
  position: absolute;
  left: 12px;
  right: 12px;
  bottom: 0;
  height: 3px;
  border-radius: 999px 999px 0 0;
  opacity: 0.9;
}

.message-bubble__trace-tile--create::after {
  background: linear-gradient(90deg, rgba(16, 185, 129, 0.16), rgba(5, 150, 105, 0.7));
}

.message-bubble__trace-tile--modify::after {
  background: linear-gradient(90deg, rgba(96, 165, 250, 0.16), rgba(37, 99, 235, 0.72));
}

.message-bubble__trace-tile--delete::after {
  background: linear-gradient(90deg, rgba(248, 113, 113, 0.14), rgba(220, 38, 38, 0.7));
}

.message-bubble__trace-tile:hover {
  transform: translateY(-2px);
  border-color: var(--trace-tile-hover-border);
  box-shadow: var(--trace-tile-hover-shadow);
}

.message-bubble__trace-tile-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.message-bubble__trace-tile-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 10px;
  background: var(--trace-tile-icon-bg);
  color: var(--trace-tile-icon-color);
}

.message-bubble__trace-tile-name {
  display: -webkit-box;
  min-height: 32px;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  font-family: var(--font-family-sans);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: -0.01em;
  line-height: 1.35;
  color: var(--color-text-primary);
  word-break: break-word;
}

.message-bubble__trace-tile-path {
  display: -webkit-box;
  min-height: 28px;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  font-size: 10px;
  font-weight: 500;
  line-height: 1.45;
  color: var(--color-text-secondary);
  word-break: break-word;
}

.message-bubble__trace-tile-meta {
  margin-top: auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  font-family: "JetBrains Mono", "SFMono-Regular", ui-monospace, Menlo, Consolas, monospace;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.02em;
  font-variant-numeric: tabular-nums;
  color: color-mix(in srgb, var(--color-text-secondary) 88%, var(--color-text-primary) 12%);
  padding-top: 7px;
  border-top: 1px solid var(--trace-tile-meta-border);
}

.message-bubble__trace-tile-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  padding: 2px 6px;
  border-radius: var(--radius-full);
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.message-bubble__trace-tile-tag--create {
  color: color-mix(in srgb, var(--color-success) 88%, var(--color-text-primary) 12%);
  background: color-mix(in srgb, var(--color-success) 16%, transparent);
}

.message-bubble__trace-tile-tag--modify {
  color: color-mix(in srgb, var(--color-primary) 84%, var(--color-text-primary) 16%);
  background: color-mix(in srgb, var(--color-primary) 16%, transparent);
}

.message-bubble__trace-tile-tag--delete {
  color: color-mix(in srgb, var(--color-error) 82%, var(--color-text-primary) 18%);
  background: color-mix(in srgb, var(--color-error) 15%, transparent);
}

/* 元信息（时间戳和状态） */
.message-bubble__meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  width: 100%;
  padding: 0 var(--spacing-1);
  margin-top: var(--spacing-1);
  box-sizing: border-box;
}

.message-bubble--user .message-bubble__meta {
  justify-content: flex-end;
}

.message-bubble__time {
  opacity: 0.8;
}

/* 消息状态 */
.message-bubble__status {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
}

.message-bubble__elapsed {
  display: inline-flex;
  align-items: center;
  padding: 1px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 12%, transparent);
  font-family: var(--font-family-mono, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace);
  font-size: 10px;
  line-height: 1.3;
}

.status-icon {
  font-size: 10px;
}

.status-icon--loading {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.status-text {
  opacity: 0.8;
}

/* 状态颜色 */
.message-bubble__status.status--pending {
  color: var(--color-text-tertiary);
}

.message-bubble__status.status--streaming {
  color: var(--color-primary);
}

.message-bubble__status.status--completed {
  color: var(--color-success);
}

.message-bubble__status.status--error {
  color: var(--color-error);
}

.message-bubble__status.status--interrupted {
  color: var(--color-warning, #f59e0b);
}

/* 停止按钮 */
.message-bubble__stop {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: var(--radius-sm);
  background: var(--color-bg-secondary);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.message-bubble__stop:hover {
  background: var(--color-bg-tertiary);
}

.message-bubble__stop :deep(.ea-icon) {
  color: var(--color-text-secondary);
}

/* 重试按钮 */
.message-bubble__retry {
  padding: 2px 8px;
  font-size: var(--font-size-xs);
  color: var(--color-primary);
  background-color: transparent;
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.message-bubble__retry:hover {
  background-color: var(--color-primary);
  color: var(--color-text-inverse);
}

/* 错误消息提示 */
.message-bubble__error {
  margin-top: var(--spacing-1);
  padding: var(--spacing-1) var(--spacing-2);
  font-size: var(--font-size-xs);
  color: var(--color-error);
  background-color: var(--color-error-light);
  border-radius: var(--radius-sm);
  border-left: 2px solid var(--color-error);
  animation: shake 0.4s ease-out;
}

/* 动画关键帧 */
@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes fadeSlideDown {
  from {
    opacity: 0;
    transform: translateY(-8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes shake {
  0%, 100% { transform: translateX(0); }
  20%, 60% { transform: translateX(-3px); }
  40%, 80% { transform: translateX(3px); }
}

/* 思考过程动画 */
.thinking-enter-active {
  animation: fadeSlideDown 0.3s ease-out;
}

.thinking-leave-active {
  animation: fadeSlideDown 0.2s ease-in reverse;
}

/* 工具调用动画 */
.tool-call-enter-active {
  animation: toolCallEnter 0.3s ease-out;
}

.tool-call-leave-active {
  animation: toolCallEnter 0.2s ease-in reverse;
}

@keyframes toolCallEnter {
  from {
    opacity: 0;
    transform: translateY(8px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

/* 单个工具调用项动画 */
.tool-call-move {
  transition: transform 0.3s ease-out;
}

@media (max-width: 768px) {
  .message-bubble {
    --message-fixed-width: min(calc(100vw - 104px), 18.5rem);
    --message-min-width: var(--message-fixed-width);
    --message-max-width: min(calc(100vw - 104px), 24.5rem);
    --message-compact-max-width: var(--message-fixed-width);
    --message-compact-max-height: 16rem;
    --message-trace-max-width: min(calc(100vw - 104px), 24rem);
    gap: var(--spacing-2);
  }

  .message-bubble--assistant.message-bubble--form-only {
    --message-fixed-width: calc(100vw - 104px);
    --message-min-width: 0;
    --message-max-width: calc(100vw - 104px);
    --message-compact-max-width: calc(100vw - 104px);
    --thinking-display-width: calc(100vw - 104px);
  }

  .message-bubble__trace-tile {
    flex-basis: 108px;
    width: 108px;
    min-height: 108px;
  }

  .message-bubble__avatar {
    width: 28px;
    height: 28px;
  }
}
</style>
