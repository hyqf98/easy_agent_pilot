<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch, type ComponentPublicInstance } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import TokenProgressBar from '@/components/common/TokenProgressBar.vue'
import CompressionConfirmDialog from '@/components/common/CompressionConfirmDialog.vue'
import { MessageList } from '@/components/message'
import type { Message } from '@/stores/message'
import { useSessionStore } from '@/stores/session'
import { useMessageStore } from '@/stores/message'
import { useAiEditTraceStore } from '@/stores/aiEditTrace'
import { useLayoutStore } from '@/stores/layout'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { useTokenStore, type CompressionStrategy } from '@/stores/token'
import { useNotificationStore } from '@/stores/notification'
import { useAgentStore } from '@/stores/agent'
import { compressionService } from '@/services/compression'
import { resolveSessionAgentId } from '@/utils/sessionAgent'
import { useOverlayDismiss } from '@/composables/useOverlayDismiss'
import AiEditTracePane from './AiEditTracePane.vue'
import PanelResizer from './PanelResizer.vue'
import ConversationComposer from './ConversationComposer.vue'

type ComposerExposed = ComponentPublicInstance & {
  focusInput: () => void
  handleMessageFormSubmit: (formId: string, values: Record<string, unknown>) => Promise<void>
  resendMessage: (content: string, attachments?: Message['attachments']) => Promise<boolean>
}

const { t } = useI18n()
const sessionStore = useSessionStore()
const messageStore = useMessageStore()
const aiEditTraceStore = useAiEditTraceStore()
const layoutStore = useLayoutStore()
const sessionExecutionStore = useSessionExecutionStore()
const tokenStore = useTokenStore()
const notificationStore = useNotificationStore()

// 压缩相关状��?
const showCompressionDialog = ref(false)
const isCompressing = ref(false)

const isMobileViewport = ref(false)
const lastObservedTraceId = ref<string | null>(null)
const workspaceRef = ref<HTMLElement | null>(null)
const composerRef = ref<ComposerExposed | null>(null)
const traceHistoryLoadToken = ref(0)

const TRACE_PANE_MIN_WIDTH = 460
const TRACE_PANE_MAX_WIDTH = 1080
const CONVERSATION_MIN_WIDTH = 360

const updateViewportMode = () => {
  isMobileViewport.value = window.innerWidth < 960

  const sessionId = sessionStore.currentSessionId
  if (!sessionId || isMobileViewport.value || !currentTraceState.value) {
    return
  }

  aiEditTraceStore.setPaneWidth(sessionId, clampTracePaneWidth(currentTraceState.value.paneWidth))
}
const handleRetry = async (message: Message) => {
  const sessionId = sessionStore.currentSessionId
  const isSending = sessionId ? sessionExecutionStore.getIsSending(sessionId) : false
  if (!sessionId || isSending) return
  const resend = async (targetMessage: Message) => {
    await composerRef.value?.resendMessage(targetMessage.content, targetMessage.attachments ?? [])
  }

  // 如果是用户消息的重试，将内容填回输入�?
  if (message.role === 'user') {
    await resend(message)
    return
  }

  if (message.role === 'assistant') {
    const messages = messageStore.messagesBySession(sessionId)
    const messageIndex = messages.findIndex(m => m.id === message.id)

    for (let i = messageIndex - 1; i >= 0; i--) {
      if (messages[i].role === 'user') {
        await resend(messages[i])
        return
      }
    }
  }
}

const currentTraceState = computed(() => {
  if (!sessionStore.currentSessionId) {
    return null
  }

  return aiEditTraceStore.getSessionState(sessionStore.currentSessionId)
})

const currentEditTraces = computed(() => {
  if (!sessionStore.currentSessionId) {
    return []
  }

  return messageStore.messagesBySession(sessionStore.currentSessionId)
    .filter(message => message.role === 'assistant' && (message.editTraces?.length ?? 0) > 0)
    .flatMap(message => (message.editTraces ?? []).map(trace => ({
      ...trace,
      messageId: message.id
    })))
})

const currentTracePagination = computed(() => {
  if (!sessionStore.currentSessionId) {
    return null
  }

  return messageStore.getPagination(sessionStore.currentSessionId)
})

const ensureTraceHistoryLoaded = async (sessionId: string) => {
  const loadToken = ++traceHistoryLoadToken.value
  let attempts = 0

  while (attempts < 8) {
    if (traceHistoryLoadToken.value !== loadToken || sessionStore.currentSessionId !== sessionId) {
      return
    }

    const traces = messageStore.messagesBySession(sessionId)
      .filter(message => message.role === 'assistant' && (message.editTraces?.length ?? 0) > 0)
      .flatMap(message => message.editTraces ?? [])

    if (traces.length > 0) {
      return
    }

    const pagination = messageStore.getPagination(sessionId)
    if (!pagination.hasMore || pagination.isLoadingMore) {
      return
    }

    attempts += 1
    await messageStore.loadMoreMessages(sessionId)
  }
}

const hasTraceContent = computed(() => currentEditTraces.value.length > 0)

// Token 使用情况
const currentTokenUsage = computed(() => {
  const sessionId = sessionStore.currentSessionId
  if (!sessionId) {
    return { used: 0, limit: 128000, percentage: 0, level: 'safe' as const }
  }
  return tokenStore.getTokenUsage(sessionId)
})

const currentMessageCount = computed(() => {
  const sessionId = sessionStore.currentSessionId
  if (!sessionId) return 0
  return messageStore.messagesBySession(sessionId).length
})

const showDesktopTraceHandle = computed(() =>
  Boolean(
    sessionStore.currentSessionId &&
    hasTraceContent.value &&
    !isMobileViewport.value &&
    !showDesktopTracePane.value
  )
)

const showDesktopTracePane = computed(() =>
  Boolean(
    sessionStore.currentSessionId &&
    hasTraceContent.value &&
    !isMobileViewport.value &&
    currentTraceState.value?.isPaneVisible
  )
)

const showMobileTraceDrawer = computed(() =>
  Boolean(
    sessionStore.currentSessionId &&
    hasTraceContent.value &&
    isMobileViewport.value &&
    currentTraceState.value?.isMobileDrawerOpen
  )
)

const showMobileTraceButton = computed(() =>
  Boolean(
    sessionStore.currentSessionId &&
    hasTraceContent.value &&
    isMobileViewport.value
  )
)

const handleHideTracePane = () => {
  if (!sessionStore.currentSessionId) {
    return
  }

  if (isMobileViewport.value) {
    aiEditTraceStore.closeMobileDrawer(sessionStore.currentSessionId)
    return
  }

  aiEditTraceStore.hidePane(sessionStore.currentSessionId)
}

const {
  handleOverlayPointerDown: handleTraceOverlayPointerDown,
  handleOverlayClick: handleTraceOverlayClick
} = useOverlayDismiss(handleHideTracePane)

const handleShowTracePane = () => {
  if (!sessionStore.currentSessionId) {
    return
  }

  aiEditTraceStore.showPane(sessionStore.currentSessionId)

  if (!isMobileViewport.value) {
    layoutStore.closePanel()
  }
}

const handleOpenMobileTrace = () => {
  if (!sessionStore.currentSessionId) {
    return
  }

  aiEditTraceStore.openMobileDrawer(sessionStore.currentSessionId)
}

const handleOpenEditTrace = (messageId: string, traceId: string) => {
  if (!sessionStore.currentSessionId) {
    return
  }

  aiEditTraceStore.selectTrace(sessionStore.currentSessionId, {
    messageId,
    traceId,
    openPane: !isMobileViewport.value,
    openMobileDrawer: isMobileViewport.value,
    userInitiated: true
  })

  if (!isMobileViewport.value) {
    layoutStore.closePanel()
  }
}

const handleComposerFocus = () => {
  if (!sessionStore.currentSessionId || !isMobileViewport.value) {
    return
  }

  aiEditTraceStore.closeMobileDrawer(sessionStore.currentSessionId)
}

const handleOpenCompress = () => {
  showCompressionDialog.value = true
}

const handleConfirmCompress = async (strategy: CompressionStrategy) => {
  const sessionId = sessionStore.currentSessionId
  if (!sessionId) return

  const session = sessionStore.currentSession
  const agentStore = useAgentStore()
  const agentId = resolveSessionAgentId(session, agentStore.agents)

  if (!agentId) {
    notificationStore.smartError('????', new Error('?????????'))
    showCompressionDialog.value = false
    return
  }

  showCompressionDialog.value = false
  isCompressing.value = true

  try {
    const result = await compressionService.compressSession(
      sessionId,
      agentId,
      { strategy }
    )

    if (result.success) {
      notificationStore.success(t('compression.success'))
    } else {
      notificationStore.error(t('compression.failed'), result.error)
    }
  } catch (error) {
    notificationStore.smartError('压缩失败', error instanceof Error ? error : new Error(String(error)))
  } finally {
    isCompressing.value = false
    showCompressionDialog.value = false
  }
}

const handleCancelCompress = () => {
  showCompressionDialog.value = false
}

const handleMessageFormSubmit = async (formId: string, values: Record<string, unknown>) => {
  await composerRef.value?.handleMessageFormSubmit(formId, values)
}

watch(() => sessionStore.currentSessionId, (sessionId) => {
  if (!sessionId) {
    traceHistoryLoadToken.value += 1
    lastObservedTraceId.value = null
    return
  }

  const traces = messageStore.messagesBySession(sessionId)
    .filter(message => message.role === 'assistant' && (message.editTraces?.length ?? 0) > 0)
    .flatMap(message => (message.editTraces ?? []).map(trace => ({
      ...trace,
      messageId: message.id
    })))

  const latestTrace = traces[traces.length - 1]
  lastObservedTraceId.value = latestTrace?.id ?? null

  if (latestTrace && !aiEditTraceStore.getSessionState(sessionId).selectedTraceId) {
    aiEditTraceStore.selectTrace(sessionId, {
      messageId: latestTrace.messageId,
      traceId: latestTrace.id
    })
  }

  if (!latestTrace) {
    void ensureTraceHistoryLoaded(sessionId)
  }
}, { immediate: true })

watch(currentEditTraces, (traces) => {
  const sessionId = sessionStore.currentSessionId
  if (!sessionId || traces.length === 0) {
    return
  }

  const latestTrace = traces[traces.length - 1]
  if (!latestTrace) {
    return
  }

  if (!lastObservedTraceId.value) {
    if (sessionExecutionStore.getIsSending(sessionId)) {
      aiEditTraceStore.handleIncomingTrace(sessionId, {
        messageId: latestTrace.messageId,
        traceId: latestTrace.id,
        shouldAutoOpen: true,
        isDesktop: !isMobileViewport.value
      })
    }
    lastObservedTraceId.value = latestTrace.id
    return
  }

  if (lastObservedTraceId.value === latestTrace.id) {
    return
  }

  aiEditTraceStore.handleIncomingTrace(sessionId, {
    messageId: latestTrace.messageId,
    traceId: latestTrace.id,
    shouldAutoOpen: sessionExecutionStore.getIsSending(sessionId),
    isDesktop: !isMobileViewport.value
  })
  lastObservedTraceId.value = latestTrace.id
}, { deep: true })

watch(
  () => [
    sessionStore.currentSessionId,
    currentEditTraces.value.length,
    currentTracePagination.value?.hasMore ?? false,
    currentTracePagination.value?.isLoadingMore ?? false,
    currentTracePagination.value?.oldestMessageCreatedAt ?? null
  ] as const,
  ([sessionId, traceCount, hasMore, isLoadingMore, oldestMessageCreatedAt]) => {
    if (!sessionId || traceCount > 0 || !hasMore || isLoadingMore || !oldestMessageCreatedAt) {
      return
    }

    void ensureTraceHistoryLoaded(sessionId)
  },
  { immediate: true }
)

watch(showDesktopTracePane, (visible) => {
  if (visible) {
    layoutStore.closePanel()
  }
})

const getTracePaneMaxWidth = () => {
  const workspaceWidth = workspaceRef.value?.clientWidth ?? window.innerWidth
  return Math.min(TRACE_PANE_MAX_WIDTH, Math.max(TRACE_PANE_MIN_WIDTH, workspaceWidth - CONVERSATION_MIN_WIDTH - 12))
}

const clampTracePaneWidth = (nextWidth: number) => {
  return Math.max(TRACE_PANE_MIN_WIDTH, Math.min(nextWidth, getTracePaneMaxWidth()))
}

const handleTracePaneResize = (delta: number) => {
  const sessionId = sessionStore.currentSessionId
  if (!sessionId || !currentTraceState.value) {
    return
  }

  aiEditTraceStore.setPaneWidth(sessionId, clampTracePaneWidth(currentTraceState.value.paneWidth + delta))
}

const handleTracePaneResizeEnd = (width: number) => {
  const sessionId = sessionStore.currentSessionId
  if (!sessionId) {
    return
  }

  aiEditTraceStore.setPaneWidth(sessionId, clampTracePaneWidth(width))
}

onMounted(() => {
  updateViewportMode()
  window.addEventListener('resize', updateViewportMode)
})

onUnmounted(() => {
  window.removeEventListener('resize', updateViewportMode)
})

</script>

<template>
  <div class="message-area">
    <template v-if="sessionStore.currentSessionId">
      <div
        ref="workspaceRef"
        class="message-area__workspace"
        :class="{ 'message-area__workspace--trace-active': showDesktopTracePane }"
      >
        <div
          v-if="showDesktopTracePane"
          class="message-area__trace-pane"
          :style="{ width: `${currentTraceState?.paneWidth ?? 640}px` }"
        >
          <AiEditTracePane
            :session-id="sessionStore.currentSessionId"
            @close="handleHideTracePane"
          />
        </div>

        <PanelResizer
          v-if="showDesktopTracePane"
          class="message-area__trace-resizer"
          :current-width="currentTraceState?.paneWidth ?? 640"
          :min-width="TRACE_PANE_MIN_WIDTH"
          :max-width="getTracePaneMaxWidth()"
          @resize="handleTracePaneResize"
          @resize-end="handleTracePaneResizeEnd"
        />

        <button
          v-if="showDesktopTraceHandle"
          class="message-area__trace-handle"
          @click="handleShowTracePane"
        >
          <EaIcon
            name="panel-left-open"
            :size="16"
          />
          <span>文件追踪</span>
          <span
            v-if="currentTraceState && currentTraceState.unseenCount > 0"
            class="message-area__trace-handle-badge"
          >
            {{ currentTraceState.unseenCount }}
          </span>
        </button>

        <div
          class="message-area__conversation"
          :class="{ 'message-area__conversation--trace-active': showDesktopTracePane }"
        >
          <!-- Token 进度�?- 固定在会话面板顶�?-->
          <div class="message-area__token-bar">
            <TokenProgressBar
              :session-id="sessionStore.currentSessionId"
              :show-compress-button="true"
              @compress="handleOpenCompress"
            />
          </div>

          <MessageList
            class="message-area__list"
            @retry="handleRetry"
            @form-submit="handleMessageFormSubmit"
            @open-edit-trace="handleOpenEditTrace"
          />

          <ConversationComposer
            ref="composerRef"
            :session-id="sessionStore.currentSessionId"
            panel-type="main"
            @focus="handleComposerFocus"
            @open-compress="handleOpenCompress"
          />
        </div>
      </div>

      <Transition name="trace-drawer">
        <div
          v-if="showMobileTraceDrawer"
          class="message-area__trace-drawer-backdrop"
          @pointerdown.capture="handleTraceOverlayPointerDown"
          @click.self="handleTraceOverlayClick"
        >
          <div class="message-area__trace-drawer">
            <AiEditTracePane
              :session-id="sessionStore.currentSessionId"
              mobile
              @close="handleHideTracePane"
            />
          </div>
        </div>
      </Transition>

      <button
        v-if="showMobileTraceButton"
        class="message-area__trace-fab"
        @click="handleOpenMobileTrace"
      >
        <EaIcon
          name="file-code"
          :size="16"
        />
        <span>文件变更</span>
        <span
          v-if="currentTraceState && currentTraceState.unseenCount > 0"
          class="message-area__trace-badge"
        >
          {{ currentTraceState.unseenCount }}
        </span>
      </button>
    </template>

    <!-- 空状�?-->
    <div
      v-else
      class="message-area__empty"
    >
      <EaIcon
        name="message-circle"
        :size="48"
        class="message-area__empty-icon"
      />
      <p class="message-area__empty-text">
        {{ t('message.noSessionSelected') }}
      </p>
      <p class="message-area__empty-hint">
        {{ t('message.startConversation') }}
      </p>
    </div>

    <!-- 压缩确认对话�?-->
    <CompressionConfirmDialog
      v-model:visible="showCompressionDialog"
      :token-usage="currentTokenUsage"
      :message-count="currentMessageCount"
      :loading="isCompressing"
      @confirm="handleConfirmCompress"
      @cancel="handleCancelCompress"
    />
  </div>
</template>

<style scoped>
.message-area {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background-color: var(--color-bg-primary);
  position: relative;
}

.message-area__workspace {
  display: flex;
  flex: 1;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  position: relative;
  background: var(--conversation-workspace-bg);
}

.message-area__workspace--trace-active {
  gap: 0;
}

.message-area__trace-pane {
  flex: 0 0 auto;
  min-width: 0;
  min-height: 0;
  background: var(--conversation-trace-pane-bg);
  box-shadow: var(--conversation-trace-pane-shadow);
}

.message-area__trace-resizer {
  background: var(--conversation-trace-resizer-bg);
}

.message-area__trace-resizer :deep(.panel-resizer) {
  width: 10px;
}

.message-area__trace-resizer :deep(.panel-resizer__line) {
  width: 2px;
  border-radius: 999px;
  background: var(--conversation-trace-resizer-line-bg);
  opacity: 0.35;
}

.message-area__conversation {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  position: relative;
}

/* Token 进度�?- 悬浮在会话面板顶�?*/
.message-area__token-bar {
  position: absolute;
  top: 8px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 10;
}

.message-area__conversation--trace-active {
  min-width: 360px;
}

.message-area__trace-handle {
  position: absolute;
  left: 14px;
  top: 18px;
  z-index: 8;
  display: inline-flex;
  align-items: center;
  gap: 10px;
  padding: 11px 15px;
  border: 1px solid var(--conversation-trace-handle-border);
  border-radius: 16px;
  background: var(--conversation-trace-handle-bg);
  color: var(--color-text-primary);
  box-shadow: var(--conversation-trace-handle-shadow);
  backdrop-filter: blur(18px);
  cursor: pointer;
  transition:
    transform var(--transition-fast) var(--easing-default),
    box-shadow var(--transition-fast) var(--easing-default),
    border-color var(--transition-fast) var(--easing-default);
}

.message-area__trace-handle:hover {
  transform: translateY(-1px);
  border-color: var(--conversation-trace-handle-hover-border);
  box-shadow: var(--conversation-trace-handle-hover-shadow);
}

.message-area__trace-handle-badge {
  min-width: 20px;
  padding: 2px 6px;
  border-radius: 999px;
  background: var(--conversation-trace-badge-bg);
  color: var(--conversation-trace-badge-text);
  font-size: 11px;
  font-weight: 600;
}

.message-area__list {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

/* 空状�?*/
.message-area__empty {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
}

/* 底部区域 */
.message-area__bottom {
  flex-shrink: 0;
  background-color: var(--color-bg-primary);
  border-top: 1px solid rgba(148, 163, 184, 0.08);
}

/* 底部状��栏：Todo + 进度�?同一�?*/
.bottom-status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-1) var(--spacing-4);
  min-height: 28px;
}

.bottom-status-bar__left {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: flex-start;
}

.bottom-status-bar__queue-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 999px;
  background: rgba(14, 165, 233, 0.1);
  color: #0369a1;
  font-size: 11px;
  font-weight: 600;
}

.bottom-status-bar__center {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.bottom-status-bar__right {
  flex: 1;
}

.message-area__empty-icon {
  color: var(--color-text-tertiary);
  margin-bottom: var(--spacing-4);
}

.message-area__empty-text {
  font-size: var(--font-size-lg);
  color: var(--color-text-secondary);
  margin-bottom: var(--spacing-2);
}

.message-area__empty-hint {
  font-size: var(--font-size-sm);
  color: var(--color-text-tertiary);
}

/* 输入区域 */
.message-input {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  padding-bottom: calc(var(--spacing-2) + env(safe-area-inset-bottom, 0px));
  margin: var(--spacing-2) var(--spacing-4);
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  transition: all var(--transition-fast) var(--easing-default);
  backdrop-filter: blur(12px);
}

.message-input:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-light);
}

.message-input__toolbar {
  display: flex;
  align-items: center;
  min-height: 24px;
}

.message-input__toolbar--top {
  justify-content: flex-start;
  gap: var(--spacing-2);
}

.message-input__toolbar--bottom {
  justify-content: space-between;
  gap: var(--spacing-2);
}

.message-input__toolbar-section {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  min-width: 0;
}

.message-input__toolbar-section--right {
  justify-content: flex-end;
  flex: 1;
}

/* 输入框编辑器容器 */
.message-input__editor {
  position: relative;
  flex: 1;
  min-height: calc(1.5em * 4); /* 4 �?*/
  max-height: calc(1.5em * 6); /* 6 �?*/
}

.message-input__file-input {
  display: none;
}

.message-input__attachments {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
  padding-bottom: var(--spacing-2);
}

.message-input__queue {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 4px 0 2px;
}

.message-input__queue-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.message-input__queue-title {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  font-weight: 600;
}

.message-input__queue-count {
  min-width: 22px;
  padding: 2px 8px;
  border-radius: 999px;
  background: rgba(59, 130, 246, 0.1);
  color: #1d4ed8;
  font-size: 11px;
  font-weight: 700;
  text-align: center;
}

.message-input__queue-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 184px;
  overflow-y: auto;
}

.message-input__queue-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 14px;
  border: 1px solid rgba(59, 130, 246, 0.12);
  background:
    linear-gradient(135deg, rgba(248, 250, 252, 0.96), rgba(239, 246, 255, 0.92));
}

.message-input__queue-item--failed {
  border-color: rgba(239, 68, 68, 0.18);
  background:
    linear-gradient(135deg, rgba(254, 242, 242, 0.98), rgba(255, 255, 255, 0.94));
}

.message-input__queue-order {
  flex-shrink: 0;
  min-width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.08);
  color: var(--color-text-secondary);
  font-size: 11px;
  font-weight: 700;
}

.message-input__queue-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.message-input__queue-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.message-input__queue-status,
.message-input__queue-images {
  font-size: 11px;
  font-weight: 600;
}

.message-input__queue-status {
  color: #0369a1;
}

.message-input__queue-item--failed .message-input__queue-status {
  color: #b91c1c;
}

.message-input__queue-images {
  color: var(--color-text-tertiary);
}

.message-input__queue-preview,
.message-input__queue-error {
  word-break: break-word;
}

.message-input__queue-preview {
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
}

.message-input__queue-error {
  font-size: 11px;
  color: #b91c1c;
}

.message-input__queue-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.message-input__queue-action {
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.06);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.message-input__queue-action:hover {
  background: rgba(15, 23, 42, 0.12);
  color: var(--color-text-primary);
}

.message-input__attachment {
  position: relative;
  width: 56px;
  height: 56px;
  border-radius: var(--radius-lg);
  overflow: hidden;
  border: 1px solid var(--color-border);
  background: var(--color-bg-secondary);
  box-shadow: var(--shadow-sm);
}

.message-input__attachment-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.message-input__attachment-remove {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--radius-full);
  background: rgba(15, 23, 42, 0.72);
  color: white;
  cursor: pointer;
}

.message-input__attachment-remove:hover {
  background: rgba(15, 23, 42, 0.88);
}

.message-input__uploading {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.message-input__render {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  padding: var(--spacing-1) 0;
  font-size: var(--font-size-sm);
  font-family: inherit;
  line-height: 1.5;
  color: var(--color-text-primary);
  white-space: pre-wrap;
  word-wrap: break-word;
  overflow-y: auto;
  overflow-x: hidden;
  pointer-events: none;
  scrollbar-width: none;
}

.message-input__render::-webkit-scrollbar {
  display: none;
}

.message-input__placeholder {
  color: var(--color-text-tertiary);
  font-style: italic;
  opacity: 0.7;
  transition: opacity var(--transition-fast) var(--easing-default);
}

.message-input__editor:focus-within .message-input__placeholder {
  opacity: 0;
}

.message-input__file-tag {
  color: var(--color-primary);
  font-weight: 500;
  background-color: color-mix(in srgb, var(--color-primary) 10%, transparent);
  border-radius: 2px;
}

.message-input__text {
  white-space: pre-wrap;
}

.message-input__textarea {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  width: 100%;
  height: 100%;
  padding: var(--spacing-1) 0;
  background: none;
  border: none;
  font-size: var(--font-size-sm);
  font-family: inherit;
  line-height: 1.5;
  color: transparent;
  -webkit-text-fill-color: transparent;
  text-shadow: none;
  caret-color: var(--color-primary);
  resize: none;
  outline: none;
  overflow-y: auto;
}

.message-input__textarea::selection {
  background: var(--color-primary-light);
}

.message-input__textarea::-moz-selection {
  background: var(--color-primary-light);
}

.message-input__textarea:focus {
  outline: none;
  border: none;
  box-shadow: none;
}

.message-input__textarea:disabled {
  cursor: not-allowed;
}

.message-input__editor:has(.message-input__textarea:disabled) .message-input__render {
  opacity: 0.6;
}

/* 小芯片��择�?*/
.input-chip {
  position: relative;
  flex-shrink: 0;
}

.input-chip__btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  background-color: var(--color-bg-tertiary);
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
  white-space: nowrap;
  max-width: 120px;
}

.input-chip__btn:hover {
  background-color: var(--color-surface-hover);
}

.input-chip__btn--compress {
  background-color: var(--color-warning-light);
}

.input-chip__btn--compress span {
  color: var(--color-warning-dark);
}

.input-chip__btn--compress svg {
  color: var(--color-warning);
}

.input-chip__btn--compress:hover:not(:disabled) {
  background-color: var(--color-warning);
}

.input-chip__btn--compress:hover:not(:disabled) span,
.input-chip__btn--compress:hover:not(:disabled) svg {
  color: white;
}

.input-chip__btn--compress:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.input-chip__icon--loading {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.input-chip--open .input-chip__btn {
  background-color: var(--color-primary-light);
}

.input-chip__btn span {
  font-size: 11px;
  font-weight: 500;
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.input-chip__btn svg {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.input-chip--open .input-chip__btn span,
.input-chip--open .input-chip__btn svg {
  color: var(--color-primary);
}

.input-chip__menu {
  position: absolute;
  bottom: calc(100% + 6px);
  left: 0;
  min-width: 180px;
  max-height: 280px;
  overflow-y: auto;
  background-color: var(--color-surface-elevated);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  z-index: 1000;
  padding: var(--spacing-1);
}

.input-chip__menu--right {
  left: auto;
  right: 0;
}

.input-chip__option {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  font-size: 13px;
  color: var(--color-text-primary);
  cursor: pointer;
  border-radius: var(--radius-md);
  transition: all var(--transition-fast) var(--easing-default);
}

.input-chip__option svg {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.input-chip__option:hover {
  background-color: var(--color-surface-hover);
}

.input-chip__option--selected {
  background-color: var(--color-primary-light);
}

.input-chip__option--selected span {
  color: var(--color-primary);
  font-weight: 500;
}

.input-chip__option--selected svg {
  color: var(--color-primary);
}

.input-chip__tag {
  margin-left: auto;
  font-size: 10px;
  font-weight: 600;
  padding: 2px 6px;
  background-color: var(--color-primary-light);
  color: var(--color-primary);
  border-radius: var(--radius-sm);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.input-chip__empty {
  padding: var(--spacing-3);
  font-size: 13px;
  color: var(--color-text-tertiary);
  text-align: center;
}

/* 下拉框动�?*/
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all var(--transition-fast) var(--easing-default);
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(4px);
}

.message-area__trace-drawer-backdrop {
  position: absolute;
  inset: 0;
  display: flex;
  justify-content: flex-end;
  background: var(--conversation-trace-drawer-backdrop);
  z-index: 30;
}

.message-area__trace-drawer {
  width: min(92vw, 720px);
  height: 100%;
  background: var(--color-surface);
  box-shadow: var(--conversation-trace-drawer-shadow);
}

.message-area__trace-fab {
  position: absolute;
  right: var(--spacing-4);
  bottom: calc(112px + env(safe-area-inset-bottom, 0px));
  z-index: 20;
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: 12px 14px;
  border: none;
  border-radius: 999px;
  background: linear-gradient(135deg, #2563eb, #0f766e);
  color: white;
  box-shadow: 0 14px 28px rgba(37, 99, 235, 0.22);
}

.message-area__trace-badge {
  min-width: 20px;
  padding: 2px 6px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.18);
  font-size: 11px;
  font-weight: 700;
}


.trace-pane-slide-enter-active,
.trace-pane-slide-leave-active,
.trace-drawer-enter-active,
.trace-drawer-leave-active {
  transition: all 0.22s ease;
}

.trace-pane-slide-enter-from,
.trace-pane-slide-leave-to {
  opacity: 0;
  transform: translateX(-12px);
}

.trace-drawer-enter-from,
.trace-drawer-leave-to {
  opacity: 0;
}

.trace-drawer-enter-from .message-area__trace-drawer,
.trace-drawer-leave-to .message-area__trace-drawer {
  transform: translateX(32px);
}

@media (max-width: 959px) {
  .message-area__workspace {
    flex-direction: column;
  }

  .message-input {
    margin: var(--spacing-2) var(--spacing-3);
  }
}
</style>
