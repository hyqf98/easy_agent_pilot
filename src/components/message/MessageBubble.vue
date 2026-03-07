<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Message } from '@/stores/message'
import { conversationService } from '@/services/conversation'
import { EaIcon } from '@/components/common'
import MarkdownRenderer from './MarkdownRenderer.vue'
import ToolCallDisplay from './ToolCallDisplay.vue'
import ThinkingDisplay from './ThinkingDisplay.vue'
import CompressionMessageBubble from './CompressionMessageBubble.vue'

const { t } = useI18n()
const props = defineProps<{ message: Message; sessionId?: string }>()
const emit = defineEmits<{ retry: [message: Message] }>()

const isUser = computed(() => props.message.role === 'user')
const isAssistant = computed(() => props.message.role === 'assistant')
const isCompression = computed(() => props.message.role === 'compression')
const isStreaming = computed(() => props.message.status === 'streaming')
const isInterrupted = computed(() => props.message.status === 'interrupted')
const isError = computed(() => props.message.status === 'error')

// 停止流式输出
const handleStop = () => {
  if (props.message.status === 'streaming' && props.sessionId) {
    conversationService.abort(props.sessionId, props.message.id)
  }
}

// 格式化时间戳为 HH:MM 格式
const formattedTime = computed(() => {
  const date = new Date(props.message.createdAt)
  return date.toLocaleTimeString('zh-CN', {
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

  // 匹配 @文件路径 的正则表达式
  // @后面跟非空格字符，直到遇到空格或行尾
  const regex = /@([^\s]+)/g
  let lastIndex = 0
  let match

  while ((match = regex.exec(content)) !== null) {
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
      content: match[1] // 不包含 @ 符号
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

// 用户消息状态文本和图标
const statusInfo = computed(() => {
  if (!isUser.value) return null

  switch (props.message.status) {
    case 'pending':
      return { text: '发送中', icon: 'loading', class: 'status--pending' }
    case 'error':
      return { text: '发送失败', icon: 'error', class: 'status--error' }
    case 'completed':
      return { text: '已发送', icon: 'check', class: 'status--completed' }
    default:
      return null
  }
})

// AI 消息状态
const assistantStatusInfo = computed(() => {
  if (!isAssistant.value) return null

  switch (props.message.status) {
    case 'streaming':
      return { text: '生成中', icon: 'loading', class: 'status--streaming' }
    case 'interrupted':
      return { text: t('message.status.interrupted'), icon: 'square', class: 'status--interrupted' }
    case 'error':
      return { text: '生成失败', icon: 'error', class: 'status--error' }
    case 'completed':
      return { text: '已完成', icon: 'check', class: 'status--completed' }
    default:
      return null
  }
})

// 失败原因
const errorMessage = computed(() => props.message.errorMessage || t('message.failed'))

// 处理重试
const handleRetry = () => {
  emit('retry', props.message)
}
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
    :class="['message-bubble', { 'message-bubble--user': isUser, 'message-bubble--assistant': isAssistant }]"
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
          v-if="isAssistant && message.thinking"
          class="message-bubble__thinking"
        >
          <ThinkingDisplay :thinking="message.thinking" />
        </div>
      </Transition>

      <div class="message-bubble__content">
        <MarkdownRenderer
          v-if="!isUser"
          :content="message.content"
        />
        <div
          v-else
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
        <span
          v-if="isStreaming"
          class="message-bubble__cursor"
        />
      </div>

      <!-- 工具调用显示 -->
      <TransitionGroup
        v-if="isAssistant && message.toolCalls && message.toolCalls.length > 0"
        name="tool-call"
        tag="div"
        class="message-bubble__tool-calls"
      >
        <ToolCallDisplay
          v-for="toolCall in message.toolCalls"
          :key="toolCall.id"
          :tool-call="toolCall"
        />
      </TransitionGroup>

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
  </div>
</template>

<style scoped>
.message-bubble {
  display: flex;
  flex-direction: row;
  width: fit-content;
  max-width: 80%;
  gap: var(--spacing-3);
}

.message-bubble--user {
  margin-left: auto;
  flex-direction: row-reverse;
}

.message-bubble--assistant {
  align-items: flex-start;
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

.avatar-icon {
  font-size: 16px;
}

/* 消息主体 */
.message-bubble__body {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  min-width: 0;
  width: 100%;
  max-width: 100%;
}

/* 思考过程显示 - 顶部区域，淡紫色渐变背景 */
.message-bubble__thinking {
  width: 100%;
  animation: fadeSlideDown 0.3s ease-out;
}

/* 消息内容 - 中间区域 */
.message-bubble__content {
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-lg);
  font-size: var(--font-size-sm);
  line-height: 1.6;
  width: 100%;
  animation: fadeIn 0.2s ease-out;
}

/* AI 消息样式 */
.message-bubble--assistant .message-bubble__content {
  background-color: var(--color-bg-tertiary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg) var(--radius-lg) var(--radius-lg) var(--radius-sm);
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

.message-bubble__text {
  white-space: pre-wrap;
  word-break: break-word;
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
.message-bubble__tool-calls {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

/* 元信息（时间戳和状态） */
.message-bubble__meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  padding: 0 var(--spacing-1);
  margin-top: var(--spacing-1);
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
</style>
