<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import TokenProgressBar from '@/components/common/TokenProgressBar.vue'
import CompressionConfirmDialog from '@/components/common/CompressionConfirmDialog.vue'
import { MessageList } from '@/components/message'
import ConversationComposer from '../conversationComposer/ConversationComposer.vue'
import { useSessionStore } from '@/stores/session'
import { useMessageStore } from '@/stores/message'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { useTokenStore, type CompressionStrategy } from '@/stores/token'
import { useSplitPaneStore } from '@/stores/splitPane'
import { compressionService } from '@/services/compression'
import { conversationService } from '@/services/conversation'
import { useNotificationStore } from '@/stores/notification'
import { useAgentStore } from '@/stores/agent'
import { resolveSessionAgentId } from '@/utils/sessionAgent'
import type { Message } from '@/stores/message'
import type { ComponentPublicInstance } from 'vue'

const props = defineProps<{
  paneId: string
  sessionId: string
}>()

const emit = defineEmits<{
  close: [paneId: string]
  dragstart: [paneId: string]
}>()

const { t } = useI18n()
const splitPaneStore = useSplitPaneStore()
const sessionStore = useSessionStore()
const messageStore = useMessageStore()
const sessionExecutionStore = useSessionExecutionStore()
const tokenStore = useTokenStore()
const notificationStore = useNotificationStore()

type ComposerExposed = ComponentPublicInstance & {
  focusInput: () => void
  handleMessageFormSubmit: (
    formId: string,
    values: Record<string, unknown>,
    assistantMessageId?: string
  ) => Promise<void>
  retryMessage: (
    messageId: string,
    content: string,
    attachments?: Message['attachments'],
    replaceMessageId?: string
  ) => Promise<boolean>
}

const composerRef = ref<ComposerExposed | null>(null)
const wrapperRef = ref<HTMLElement | null>(null)
const showCompressionDialog = ref(false)
const isCompressing = ref(false)
const paneWidth = ref(800)
const paneHeight = ref(800)

let resizeObserver: ResizeObserver | null = null

onMounted(() => {
  if (wrapperRef.value) {
    paneWidth.value = wrapperRef.value.clientWidth
    paneHeight.value = wrapperRef.value.clientHeight
    resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        paneWidth.value = entry.contentRect.width
        paneHeight.value = entry.contentRect.height
      }
    })
    resizeObserver.observe(wrapperRef.value)
  }
})

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
})

const isFocused = computed(() => splitPaneStore.focusedPaneId === props.paneId)
const canClose = computed(() => splitPaneStore.paneCount > 1)

const sessionName = computed(() => {
  const session = sessionStore.sessions.find(s => s.id === props.sessionId)
  return session?.name ?? ''
})

const isCompactMode = computed(() => paneWidth.value < 500)
const isMiniMode = computed(() => paneWidth.value < 360)
const isHeightCompact = computed(() => paneHeight.value < 650)
const isHeightMini = computed(() => paneHeight.value < 450)

const currentTokenUsage = computed(() =>
  tokenStore.getTokenUsage(props.sessionId)
)

const currentMessageCount = computed(() =>
  messageStore.messagesBySession(props.sessionId).length
)

const isSending = computed(() =>
  sessionExecutionStore.getIsSending(props.sessionId)
)

function handleFocus() {
  splitPaneStore.focusPane(props.paneId)
}

function handleClose() {
  emit('close', props.paneId)
}

function handleDragHandleMouseDown(e: MouseEvent) {
  e.preventDefault()
  emit('dragstart', props.paneId)
}

function handleComposerFocus() {
  splitPaneStore.focusPane(props.paneId)
}

async function handleRetry(message: Message) {
  if (isSending.value) return

  const retry = async (targetMessage: Message, replaceMessageId?: string) => {
    await composerRef.value?.retryMessage(
      targetMessage.id,
      targetMessage.content,
      targetMessage.attachments ?? [],
      replaceMessageId
    )
  }

  if (message.role === 'user') {
    await retry(message)
    return
  }

  if (message.role === 'assistant') {
    const messages = messageStore.messagesBySession(props.sessionId)
    const messageIndex = messages.findIndex(m => m.id === message.id)
    for (let i = messageIndex - 1; i >= 0; i--) {
      if (messages[i].role === 'user') {
        await retry(messages[i], message.id)
        return
      }
    }
  }
}

async function handleMessageFormSubmit(
  formId: string,
  values: Record<string, unknown>,
  assistantMessageId?: string
) {
  await composerRef.value?.handleMessageFormSubmit(formId, values, assistantMessageId)
}

function handleOpenCompress() {
  showCompressionDialog.value = true
}

async function handleConfirmCompress(strategy: CompressionStrategy) {
  const sessionId = props.sessionId
  const session = sessionStore.sessions.find(s => s.id === sessionId)
  const agentStore = useAgentStore()
  const agentId = resolveSessionAgentId(session, agentStore.agents)

  if (!agentId) {
    notificationStore.smartError('压缩失败', new Error('未找到可用智能体'))
    showCompressionDialog.value = false
    return
  }

  showCompressionDialog.value = false
  isCompressing.value = true

  try {
    const result = await compressionService.compressSession(
      sessionId,
      agentId,
      { strategy, triggerSource: 'manual' }
    )
    if (result.success) {
      notificationStore.success(t('compression.success'))
      await conversationService.drainQueue(sessionId)
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

function handleCancelCompress() {
  showCompressionDialog.value = false
}
</script>

<template>
  <div
    ref="wrapperRef"
    :class="[
      'pane-wrapper',
      {
        'pane-wrapper--focused': isFocused,
        'pane-wrapper--compact': isCompactMode,
        'pane-wrapper--mini': isMiniMode,
        'pane-wrapper--h-compact': isHeightCompact,
        'pane-wrapper--h-mini': isHeightMini
      }
    ]"
  >
    <div
      class="pane-header"
      @click="handleFocus"
    >
      <div
        class="pane-header__left"
        @mousedown="handleDragHandleMouseDown"
      >
        <span class="pane-header__drag-handle">
          <EaIcon
            name="grip-vertical"
            :size="12"
          />
        </span>
        <span class="pane-header__title">{{ sessionName || t('splitPane.newPane') }}</span>
      </div>
      <div
        class="pane-header__right"
        @mousedown.stop
      >
        <button
          v-if="canClose"
          class="pane-header__close"
          :title="t('splitPane.closePane')"
          @click.stop="handleClose"
        >
          <EaIcon
            name="x"
            :size="12"
          />
        </button>
      </div>
    </div>

    <div class="pane-wrapper__content">
      <div
        v-if="!isMiniMode && !isHeightMini"
        class="pane-wrapper__token-bar"
      >
        <TokenProgressBar
          :session-id="sessionId"
          :show-compress-button="true"
          @compress="handleOpenCompress"
        />
      </div>

      <MessageList
        class="pane-wrapper__list"
        :session-id="sessionId"
        :top-safe-inset="30"
        @retry="handleRetry"
        @form-submit="handleMessageFormSubmit"
      />

      <ConversationComposer
        ref="composerRef"
        :session-id="sessionId"
        :panel-type="isMiniMode ? 'mini' : 'main'"
        :compact="isCompactMode"
        @focus="handleComposerFocus"
        @open-compress="handleOpenCompress"
      />
    </div>

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
.pane-wrapper {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  background: var(--color-bg-primary);
  position: relative;
  min-width: 0;
}

.pane-wrapper--focused {
  background: var(--color-bg-primary);
}

.pane-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px;
  height: 30px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface);
  user-select: none;
  cursor: pointer;
}

.pane-wrapper--focused .pane-header {
  border-bottom-color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary-light, var(--color-surface)) 40%, var(--color-surface));
}

.pane-header__left {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  flex: 1;
  cursor: grab;
}

.pane-header__left:active {
  cursor: grabbing;
}

.pane-header__drag-handle {
  display: flex;
  align-items: center;
  color: var(--color-text-quaternary);
  flex-shrink: 0;
  opacity: 0.4;
  transition: opacity var(--transition-fast) var(--easing-default);
}

.pane-header__drag-handle:hover {
  opacity: 1;
}

.pane-header__title {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  pointer-events: none;
}

.pane-wrapper--focused .pane-header__title {
  color: var(--color-primary);
}

.pane-wrapper--mini .pane-header__title {
  max-width: 80px;
}

.pane-header__right {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

.pane-header__close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-tertiary);
  cursor: pointer;
  opacity: 0;
  transition: all var(--transition-fast) var(--easing-default);
}

.pane-header:hover .pane-header__close {
  opacity: 1;
}

.pane-header__close:hover {
  background: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.pane-wrapper__content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.pane-wrapper__token-bar {
  flex-shrink: 0;
}

.pane-wrapper__list {
  flex: 1;
  min-height: 0;
}

.pane-wrapper--compact .pane-wrapper__content {
  gap: 0;
}

.pane-wrapper--compact :deep(.conversation-composer__main-header) {
  gap: 4px;
  padding-bottom: 4px;
}

.pane-wrapper--compact :deep(.conversation-composer__main-header-left) {
  gap: 3px;
}

.pane-wrapper--compact :deep(.composer-chip--main) {
  min-height: 24px;
}

.pane-wrapper--compact :deep(.composer-chip--main .composer-chip__button) {
  min-height: 24px;
  padding: 0 6px;
  gap: 3px;
  font-size: 11px;
}

.pane-wrapper--compact :deep(.composer-chip--main .composer-chip__button > span:last-child:not(svg span)) {
  display: none;
}

.pane-wrapper--compact :deep(.composer-chip--image span) {
  display: none;
}

.pane-wrapper--compact :deep(.conversation-composer--main .conversation-composer__editor-shell) {
  --conversation-composer-main-max-lines: 4;
  border-radius: 8px;
}

.pane-wrapper--mini .pane-wrapper__content {
  gap: 0;
}

.pane-wrapper--mini :deep(.conversation-composer) {
  padding: 6px 8px;
  gap: 0;
}

.pane-wrapper--mini :deep(.conversation-composer__main-header) {
  gap: 2px;
  padding-bottom: 2px;
}

.pane-wrapper--mini :deep(.conversation-composer__main-header-left) {
  gap: 2px;
}

.pane-wrapper--mini :deep(.composer-chip--main) {
  min-height: 22px;
}

.pane-wrapper--mini :deep(.composer-chip--main .composer-chip__button) {
  min-height: 22px;
  padding: 0 5px;
  gap: 2px;
  font-size: 10px;
}

.pane-wrapper--mini :deep(.composer-chip--main .composer-chip__button > span:last-child:not(svg span)) {
  display: none;
}

.pane-wrapper--mini :deep(.composer-chip--image span) {
  display: none;
}

.pane-wrapper--mini :deep(.conversation-composer--main .conversation-composer__editor-shell) {
  --conversation-composer-main-max-lines: 3;
  --conversation-composer-main-padding-y: 8px;
  --conversation-composer-main-padding-x: 10px;
  border-radius: 6px;
}

.pane-wrapper--mini :deep(.conversation-composer--main .conversation-composer__textarea) {
  font-size: 12px;
}

.pane-wrapper--mini :deep(.conversation-composer__ghost-hints) {
  display: none;
}

/* Height-compact: panes shorter than 550px (typical 2-row layout) */
.pane-wrapper--h-compact .pane-header {
  height: 26px;
  padding: 0 6px;
}

.pane-wrapper--h-compact .pane-header__title {
  font-size: 11px;
}

.pane-wrapper--h-compact :deep(.conversation-composer) {
  padding: 4px 8px;
  gap: 2px;
}

.pane-wrapper--h-compact :deep(.conversation-composer__main-header) {
  gap: 3px;
  padding-bottom: 2px;
}

.pane-wrapper--h-compact :deep(.composer-chip--main) {
  min-height: 22px;
}

.pane-wrapper--h-compact :deep(.composer-chip--main .composer-chip__button) {
  min-height: 22px;
  padding: 0 5px;
  gap: 2px;
  font-size: 11px;
}

.pane-wrapper--h-compact :deep(.composer-chip--main .composer-chip__button > span:last-child:not(svg span)) {
  display: none;
}

.pane-wrapper--h-compact :deep(.conversation-composer--main .conversation-composer__editor-shell) {
  --conversation-composer-main-max-lines: 3;
  --conversation-composer-main-padding-y: 6px;
  border-radius: 6px;
}

.pane-wrapper--h-compact :deep(.message-bubble) {
  margin-bottom: 6px;
}

.pane-wrapper--h-compact :deep(.message-bubble__avatar) {
  width: 24px;
  height: 24px;
}

.pane-wrapper--h-compact :deep(.message-bubble__avatar img) {
  width: 24px;
  height: 24px;
}

/* Height-mini: panes shorter than 400px (3-row or very cramped) */
.pane-wrapper--h-mini .pane-header {
  height: 22px;
  padding: 0 4px;
}

.pane-wrapper--h-mini .pane-header__drag-handle {
  display: none;
}

.pane-wrapper--h-mini :deep(.conversation-composer) {
  padding: 2px 6px;
  gap: 0;
}

.pane-wrapper--h-mini :deep(.conversation-composer__main-header) {
  gap: 2px;
  padding-bottom: 1px;
}

.pane-wrapper--h-mini :deep(.composer-chip--main) {
  min-height: 20px;
}

.pane-wrapper--h-mini :deep(.composer-chip--main .composer-chip__button) {
  min-height: 20px;
  padding: 0 4px;
  font-size: 10px;
}

.pane-wrapper--h-mini :deep(.conversation-composer--main .conversation-composer__editor-shell) {
  --conversation-composer-main-max-lines: 2;
  --conversation-composer-main-padding-y: 4px;
  --conversation-composer-main-padding-x: 8px;
  border-radius: 4px;
}

.pane-wrapper--h-mini :deep(.conversation-composer--main .conversation-composer__textarea) {
  font-size: 12px;
  line-height: 1.4;
}

.pane-wrapper--h-mini :deep(.conversation-composer__ghost-hints) {
  display: none;
}

.pane-wrapper--h-mini :deep(.message-bubble) {
  margin-bottom: 4px;
}

.pane-wrapper--h-mini :deep(.message-bubble__avatar) {
  width: 20px;
  height: 20px;
}

.pane-wrapper--h-mini :deep(.message-bubble__avatar img) {
  width: 20px;
  height: 20px;
}

.pane-wrapper--h-mini :deep(.message-bubble__meta) {
  margin-bottom: 0;
}
</style>
