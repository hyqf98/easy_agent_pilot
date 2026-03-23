<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { Message } from '@/stores/message'
import { useMessageStore } from '@/stores/message'
import { conversationService } from '@/services/conversation'
import { EaIcon } from '@/components/common'
import { FILE_MENTION_PATTERN, getMentionDisplayText } from '@/utils/fileMention'
import { parseStructuredContent } from '@/utils/structuredContent'
import StructuredContentRenderer from './StructuredContentRenderer.vue'
import ToolCallDisplay from './ToolCallDisplay.vue'
import ThinkingDisplay from './ThinkingDisplay.vue'
import CompressionMessageBubble from './CompressionMessageBubble.vue'
import RuntimeNoticeList from './RuntimeNoticeList.vue'

const { t, locale } = useI18n()
const props = defineProps<{ message: Message; sessionId?: string }>()
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
const messageAttachmentPreviews = computed(() =>
  (props.message.attachments ?? []).map(attachment => ({
    ...attachment,
    previewUrl: convertFileSrc(attachment.path)
  }))
)

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

const processedUserMessage = computed(() => {
  if (!isUser.value) return []

  const content = props.message.content
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

const assistantVisibleEditTraces = computed(() => {
  if (!isAssistant.value || !props.message.editTraces?.length) {
    return []
  }

  const latestTraceByFile = messageStore.getLatestAssistantTraceIdsByFile(props.message.sessionId)

  const latestVisibleTraces = props.message.editTraces.filter(trace => {
    const latest = latestTraceByFile.get(trace.filePath)
    return latest?.traceId === trace.id && latest.messageId === props.message.id
  })

  return latestVisibleTraces.sort((left, right) => right.timestamp.localeCompare(left.timestamp))
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
const isAssistantFormOnly = computed(() => {
  if (!isAssistant.value) {
    return false
  }

  const blocks = parseStructuredContent(props.message.content)
  return blocks.length > 0 && blocks.every(block => block.type === 'form')
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
          :interactive-forms="isAssistant"
          :animate="isAssistant && isStreaming"
          @form-submit="handleFormSubmit"
        />
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
        v-if="isAssistant && message.runtimeNotices && message.runtimeNotices.length > 0"
        class="message-bubble__runtime"
      >
        <RuntimeNoticeList :notices="message.runtimeNotices" />
      </div>

      <!-- 工具调用显示 -->
      <div
        v-if="isAssistant && message.toolCalls && message.toolCalls.length > 0"
        class="message-bubble__tool-calls-shell"
        :class="{ 'message-bubble__tool-calls-shell--scrollable': shouldClampToolCalls }"
      >
        <div class="message-bubble__tool-calls-head">
          <span class="message-bubble__tool-calls-title">工具调用</span>
          <span class="message-bubble__tool-calls-count">{{ toolCallCount }}</span>
        </div>
        <TransitionGroup
          name="tool-call"
          tag="div"
          class="message-bubble__tool-calls"
        >
          <ToolCallDisplay
            v-for="toolCall in message.toolCalls"
            :key="getToolCallRenderKey(toolCall)"
            :tool-call="toolCall"
            :live="isStreaming || toolCall.status === 'running'"
            :default-expanded="false"
            :default-result-expanded="false"
          />
        </TransitionGroup>
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
        v-if="isError && message.errorMessage"
        class="message-bubble__error"
      >
        {{ message.errorMessage }}
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
  --message-fixed-width: 28rem;
  --message-min-width: var(--message-fixed-width);
  --message-max-width: 36rem;
  --message-max-height: 30rem;
  --message-compact-max-width: var(--message-fixed-width);
  --message-compact-max-height: 20rem;
  --message-trace-max-width: 35rem;
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
  max-height: var(--message-max-height);
  box-sizing: border-box;
  animation: fadeIn 0.2s ease-out;
  overflow: auto;
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
  padding: 0 0.1rem;
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

.message-bubble__tool-calls {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  min-height: 0;
  overflow: visible;
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
  background: rgba(59, 130, 246, 0.1);
  color: #1d4ed8;
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

.message-bubble__trace-strip-wrap::before,
.message-bubble__trace-strip-wrap::after {
  content: '';
  position: absolute;
  top: 0;
  bottom: 6px;
  width: 18px;
  pointer-events: none;
  z-index: 1;
}

.message-bubble__trace-strip-wrap::before {
  left: 0;
  background: linear-gradient(90deg, rgba(255, 255, 255, 0.95), rgba(255, 255, 255, 0));
}

.message-bubble__trace-strip-wrap::after {
  right: 0;
  background: linear-gradient(270deg, rgba(255, 255, 255, 0.95), rgba(255, 255, 255, 0));
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
    radial-gradient(circle at top right, rgba(59, 130, 246, 0.12), transparent 42%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.94), rgba(248, 250, 252, 0.92)),
    var(--color-bg-secondary);
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 18px;
  text-align: left;
  cursor: pointer;
  scroll-snap-align: start;
  box-shadow: 0 10px 24px rgba(15, 23, 42, 0.06);
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
  border-color: rgba(59, 130, 246, 0.28);
  box-shadow: 0 14px 30px rgba(15, 23, 42, 0.1);
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
  background: rgba(59, 130, 246, 0.1);
  color: #2563eb;
}

.message-bubble__trace-tile-name {
  display: -webkit-box;
  min-height: 32px;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  font-family: var(--font-family-mono);
  font-size: 11px;
  font-weight: 700;
  line-height: 1.4;
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
  line-height: 1.4;
  color: var(--color-text-tertiary);
  word-break: break-word;
}

.message-bubble__trace-tile-meta {
  margin-top: auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  font-size: 10px;
  color: var(--color-text-tertiary);
  padding-top: 6px;
  border-top: 1px solid rgba(148, 163, 184, 0.12);
}

.message-bubble__trace-tile-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  padding: 2px 6px;
  border-radius: var(--radius-full);
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.message-bubble__trace-tile-tag--create {
  color: #047857;
  background: rgba(16, 185, 129, 0.14);
}

.message-bubble__trace-tile-tag--modify {
  color: #1d4ed8;
  background: rgba(59, 130, 246, 0.14);
}

.message-bubble__trace-tile-tag--delete {
  color: #b91c1c;
  background: rgba(239, 68, 68, 0.14);
}

:global([data-theme='dark']) .message-bubble__trace-rail-count,
:global(.dark) .message-bubble__trace-rail-count {
  background: rgba(59, 130, 246, 0.18);
  color: #bfdbfe;
}

:global([data-theme='dark']) .message-bubble__trace-tile,
:global(.dark) .message-bubble__trace-tile {
  background:
    radial-gradient(circle at top right, rgba(96, 165, 250, 0.12), transparent 42%),
    linear-gradient(180deg, rgba(30, 41, 59, 0.92), rgba(15, 23, 42, 0.92));
  border-color: rgba(71, 85, 105, 0.64);
  box-shadow: 0 12px 26px rgba(2, 6, 23, 0.24);
}

:global([data-theme='dark']) .message-bubble__trace-strip-wrap::before,
:global(.dark) .message-bubble__trace-strip-wrap::before {
  background: linear-gradient(90deg, rgba(15, 23, 42, 0.96), rgba(15, 23, 42, 0));
}

:global([data-theme='dark']) .message-bubble__trace-strip-wrap::after,
:global(.dark) .message-bubble__trace-strip-wrap::after {
  background: linear-gradient(270deg, rgba(15, 23, 42, 0.96), rgba(15, 23, 42, 0));
}

:global([data-theme='dark']) .message-bubble__trace-tile:hover,
:global(.dark) .message-bubble__trace-tile:hover {
  border-color: rgba(96, 165, 250, 0.4);
  box-shadow: 0 16px 32px rgba(2, 6, 23, 0.34);
}

:global([data-theme='dark']) .message-bubble__trace-tile-icon,
:global(.dark) .message-bubble__trace-tile-icon {
  background: rgba(59, 130, 246, 0.18);
  color: #bfdbfe;
}

:global([data-theme='dark']) .message-bubble__trace-tile-meta,
:global(.dark) .message-bubble__trace-tile-meta {
  border-top-color: rgba(71, 85, 105, 0.48);
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
    --message-max-height: 24rem;
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
