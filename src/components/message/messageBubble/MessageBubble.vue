<script setup lang="ts">
import AttachmentThumbnail from '@/components/common/AttachmentThumbnail.vue'
import StructuredContentRenderer from '../StructuredContentRenderer.vue'
import ToolCallDisplay from '../ToolCallDisplay.vue'
import ThinkingDisplay from '../ThinkingDisplay.vue'
import CompressionMessageBubble from '../CompressionMessageBubble.vue'
import RuntimeNoticeList from '../RuntimeNoticeList.vue'
import {
  useMessageBubble,
  type MessageBubbleEmits,
  type MessageBubbleProps
} from './useMessageBubble'

const props = withDefaults(defineProps<MessageBubbleProps>(), {
  sessionId: undefined,
  hideContextStrategyNotice: false
})
const emit = defineEmits<MessageBubbleEmits>()
const messageAttachmentWrapperStyle = {
  width: '84px',
  height: '84px',
  overflow: 'hidden',
  borderRadius: '16px',
  border: '1px solid color-mix(in srgb, var(--color-border) 76%, transparent)',
  background: 'linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(241, 245, 249, 0.96))',
  boxShadow: '0 10px 24px rgba(15, 23, 42, 0.08)'
} satisfies Record<string, string>

const messageAttachmentImageStyle = {
  width: '100%',
  height: '100%',
  objectFit: 'cover'
} as const

const {
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
  resolvedFormResponse,
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
} = useMessageBubble(props, emit)
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
          v-if="isUser && (message.attachments?.length ?? 0) > 0"
          class="message-bubble__attachments"
        >
          <AttachmentThumbnail
            v-for="attachment in message.attachments"
            :key="attachment.id"
            :attachment="attachment"
            wrapper-class="message-bubble__attachment"
            media-class="message-bubble__attachment-image"
            :wrapper-style="messageAttachmentWrapperStyle"
            :media-style="messageAttachmentImageStyle"
            :preview-max-width="460"
            :preview-max-height="520"
          />
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
          v-if="isAssistant && isStreaming && isCurrentStreamingMessage"
          class="message-bubble__stop"
          :title="t('common.stop')"
          @click="handleStop"
        >
          ⏹
        </button>
        <!-- 重试按钮 - 用户消息失败/中断 -->
        <button
          v-if="isUser && canRetry"
          class="message-bubble__retry"
          :title="isInterrupted ? t('message.status.interrupted') : errorMessage"
          @click="handleRetry"
        >
          {{ t('common.retry') }}
        </button>
        <!-- 重试按钮 - AI 消息 -->
        <button
          v-if="isAssistant && !isStreaming && (canRetry || message.content)"
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

<style scoped src="./styles.css"></style>
