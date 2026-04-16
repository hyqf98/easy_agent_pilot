<script setup lang="ts">
import { onMounted, onUnmounted, ref, type ComponentPublicInstance } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import TokenProgressBar from '@/components/common/TokenProgressBar.vue'
import { MessageList } from '@/components/message'
import ConversationComposer from '@/components/layout/conversationComposer/ConversationComposer.vue'
import { useMessageStore, type Message, type MessageAttachment } from '@/stores/message'
import { useMiniPanelStore } from '@/stores/miniPanel'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { resolveAttachmentPreviewUrl } from '@/utils/attachmentPreview'

const miniPanelStore = useMiniPanelStore()
const messageStore = useMessageStore()
const sessionExecutionStore = useSessionExecutionStore()

type ComposerExposed = ComponentPublicInstance & {
  focusInput: () => void
  openCompressionDialog: () => void
}

const composerRef = ref<ComposerExposed | null>(null)
let unlistenFocus: (() => void) | null = null

async function toPendingImages(attachments: MessageAttachment[]) {
  return Promise.all(attachments.map(async attachment => ({
    ...attachment,
    previewUrl: await resolveAttachmentPreviewUrl(attachment)
  })))
}

async function hideMiniPanel() {
  await invoke('hide_mini_panel')
}

function handleEscapeKey(event: KeyboardEvent) {
  if (event.key !== 'Escape' || event.defaultPrevented) {
    return
  }

  event.preventDefault()
  void hideMiniPanel()
}

function handleOpenCompress() {
  composerRef.value?.openCompressionDialog()
}

async function handleRetry(message: Message) {
  const sessionId = miniPanelStore.sessionId
  const isSending = sessionId ? sessionExecutionStore.getIsSending(sessionId) : false
  if (!sessionId || isSending) {
    return
  }

  if (message.role === 'user') {
    sessionExecutionStore.setInputText(sessionId, message.content)
    sessionExecutionStore.setPendingImages(sessionId, await toPendingImages(message.attachments ?? []))
    composerRef.value?.focusInput()
    return
  }

  if (message.role === 'assistant') {
    const messages = messageStore.messagesBySession(sessionId)
    const messageIndex = messages.findIndex(item => item.id === message.id)

    for (let index = messageIndex - 1; index >= 0; index -= 1) {
      if (messages[index].role === 'user') {
        sessionExecutionStore.setInputText(sessionId, messages[index].content)
        sessionExecutionStore.setPendingImages(sessionId, await toPendingImages(messages[index].attachments ?? []))
        break
      }
    }
  }

  composerRef.value?.focusInput()
}

onMounted(async () => {
  await miniPanelStore.initSessionContext()
  composerRef.value?.focusInput()
  document.addEventListener('keydown', handleEscapeKey, true)

  const currentWindow = getCurrentWindow()
  unlistenFocus = await currentWindow.listen('mini-panel:focus-input', () => {
    composerRef.value?.focusInput()
  })
})

onUnmounted(() => {
  unlistenFocus?.()
  document.removeEventListener('keydown', handleEscapeKey, true)
})
</script>

<template>
  <div class="mini-panel">
    <div class="mini-panel__body">
      <section class="mini-panel__conversation">
        <div class="mini-panel__overlay">
          <div class="mini-panel__token-bar-shell">
            <div class="mini-panel__token-bar">
              <TokenProgressBar
                :session-id="miniPanelStore.sessionId"
                @compress="handleOpenCompress"
              />
            </div>
          </div>
        </div>

        <MessageList
          class="mini-panel__messages"
          :session-id="miniPanelStore.sessionId || undefined"
          @retry="handleRetry"
        />
      </section>

      <ConversationComposer
        ref="composerRef"
        panel-type="mini"
        :session-id="miniPanelStore.sessionId"
        :working-directory="miniPanelStore.workingDirectory"
        :set-working-directory="miniPanelStore.setWorkingDirectory"
        show-working-directory
        compact
        class="mini-panel__composer"
      />
    </div>
  </div>
</template>

<style scoped>
.mini-panel {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;
  color: var(--color-text-primary);
  background:
    radial-gradient(circle at top left, rgba(14, 165, 233, 0.12), transparent 24%),
    radial-gradient(circle at top right, rgba(59, 130, 246, 0.08), transparent 22%),
    linear-gradient(180deg, #f7fbff, #edf4fb 40%, #f8fafc);
}

.mini-panel__body {
  min-height: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 12px 12px;
}

.mini-panel__conversation {
  position: relative;
  isolation: isolate;
  min-height: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
  border-radius: 14px;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.94), rgba(248, 250, 252, 0.96));
  box-shadow: 0 10px 24px rgba(15, 23, 42, 0.05);
}

.mini-panel__overlay {
  position: absolute;
  top: 10px;
  left: 0;
  right: 0;
  z-index: 4;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  pointer-events: none;
}

.mini-panel__token-bar-shell {
  width: min(196px, calc(100% - 28px));
  pointer-events: auto;
  min-width: 0;
}

.mini-panel__token-bar {
  width: 100%;
}

.mini-panel__messages {
  flex: 1;
  min-height: 0;
  padding-top: 2px;
}

.mini-panel__composer {
  flex-shrink: 0;
  border: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.9);
  box-shadow: 0 8px 18px rgba(15, 23, 42, 0.04);
}

:deep(.mini-panel__token-bar .token-progress) {
  width: 100%;
  gap: 6px;
  padding: 4px 8px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.58);
  border-color: color-mix(in srgb, var(--color-border) 42%, transparent);
  box-shadow: 0 8px 18px rgba(15, 23, 42, 0.08);
  backdrop-filter: blur(10px);
}

:deep(.mini-panel__token-bar .token-progress:hover) {
  background: rgba(255, 255, 255, 0.74);
  box-shadow: 0 10px 20px rgba(15, 23, 42, 0.1);
}

:deep(.mini-panel__token-bar .token-progress__bar) {
  min-width: 88px;
  height: 4px;
}

:deep(.mini-panel__token-bar .token-progress__text) {
  min-width: 28px;
  font-size: 10px;
  line-height: 1;
}

:deep(.mini-panel__composer.conversation-composer) {
  padding: 8px;
  gap: 6px;
  background: transparent;
}

:deep(.mini-panel__composer .conversation-composer__path-row) {
  margin-bottom: -2px;
}

:deep(.mini-panel__composer .conversation-composer__status) {
  gap: 6px;
  align-items: center;
}

:deep(.mini-panel__composer .conversation-composer__status-left),
:deep(.mini-panel__composer .conversation-composer__status-right) {
  gap: 6px;
  flex-wrap: wrap;
}

:deep(.mini-panel__composer .conversation-composer__path) {
  gap: 5px;
  min-height: 24px;
  max-width: min(100%, 220px);
  padding: 0 8px;
  border-radius: 999px;
  font-size: 10px;
}

:deep(.mini-panel__composer .conversation-composer__panel) {
  gap: 6px;
}

:deep(.mini-panel__composer .conversation-todo-panel) {
  margin: 0;
  padding: 8px 10px;
  border-radius: 12px;
}

:deep(.mini-panel__composer .conversation-todo-panel__head) {
  margin-bottom: 6px;
}

:deep(.mini-panel__composer .conversation-todo-panel__title) {
  font-size: 12px;
}

:deep(.mini-panel__composer .conversation-todo-panel__summary),
:deep(.mini-panel__composer .conversation-todo-panel__status),
:deep(.mini-panel__composer .conversation-todo-panel__hint) {
  font-size: 10px;
}

:deep(.mini-panel__composer .conversation-todo-panel__items-inner) {
  gap: 6px;
}

:deep(.mini-panel__composer .conversation-todo-panel__item) {
  gap: 8px;
  padding: 8px 9px;
  border-radius: 10px;
}

:deep(.mini-panel__composer .conversation-todo-panel__dot) {
  width: 7px;
  height: 7px;
  margin-top: 5px;
}

:deep(.mini-panel__composer .conversation-todo-panel__text) {
  font-size: 12px;
  line-height: 1.4;
}

:deep(.mini-panel__composer .conversation-composer__queue) {
  gap: 5px;
}

:deep(.mini-panel__composer .conversation-composer__queue-item) {
  grid-template-columns: 18px minmax(0, 1fr) auto;
  gap: 6px;
  padding: 6px 7px;
  border-radius: 10px;
  background: rgba(248, 250, 252, 0.94);
}

:deep(.mini-panel__composer .conversation-composer__queue-index) {
  width: 18px;
  height: 18px;
  min-width: 18px;
  font-size: 10px;
}

:deep(.mini-panel__composer .conversation-composer__queue-top) {
  gap: 6px;
  font-size: 10px;
}

:deep(.mini-panel__composer .conversation-composer__queue-preview) {
  margin-top: 1px;
  font-size: 11px;
  line-height: 1.35;
  -webkit-line-clamp: 2;
}

:deep(.mini-panel__composer .conversation-composer__queue-actions) {
  gap: 4px;
}

:deep(.mini-panel__composer .conversation-composer__queue-action) {
  width: 20px;
  height: 20px;
  border-radius: 6px;
}

:deep(.mini-panel__composer .conversation-composer__queue-error) {
  margin-top: 3px;
  font-size: 10px;
}

:deep(.mini-panel__composer .conversation-composer__editor-shell) {
  min-height: 92px;
  border-radius: 12px;
  background: rgba(248, 250, 252, 0.98);
}

:deep(.mini-panel__composer .conversation-composer__render),
:deep(.mini-panel__composer .conversation-composer__textarea) {
  min-height: 92px;
  padding: 10px 12px;
}

:deep(.mini-panel__composer .composer-chip),
:deep(.mini-panel__composer .conversation-composer__send) {
  min-height: 24px;
  padding: 0 8px;
  gap: 5px;
  border-radius: 999px;
}

:deep(.mini-panel__composer .composer-chip__button) {
  min-height: 24px;
  padding: 0 8px;
  gap: 4px;
  font-size: 11px;
}

:deep(.mini-panel__composer .composer-chip--dropdown) {
  max-width: 134px;
}

:deep(.mini-panel__composer .composer-chip--dropdown .composer-chip__button) {
  width: 100%;
  max-width: 134px;
}

:deep(.mini-panel__composer .composer-chip__button > span) {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

:deep(.mini-panel__composer .composer-chip__menu) {
  min-width: 168px;
  padding: 6px;
  border-radius: 12px;
}

:deep(.mini-panel__composer .composer-chip__option) {
  gap: 6px;
  padding: 7px 8px;
  border-radius: 9px;
  font-size: 11px;
}

:deep(.mini-panel__composer .composer-chip__option > span:first-of-type) {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

:deep(.mini-panel__composer .composer-chip__tag) {
  font-size: 9px;
}

:deep(.mini-panel__composer .composer-chip--image) {
  min-width: 52px;
  padding: 0 8px;
  font-size: 11px;
}

:deep(.mini-panel__messages.message-list) {
  padding: 40px 10px 12px;
}

:deep(.mini-panel__messages .message-bubble) {
  --assistant-body-width: clamp(11.75rem, 72%, 18rem);
  --user-body-width: clamp(8.75rem, 56%, 14rem);
  --message-fixed-width: 14.75rem;
  --message-min-width: var(--message-fixed-width);
  --message-max-width: 18rem;
  --message-max-height: 22rem;
  --message-compact-max-width: var(--message-fixed-width);
  --message-compact-max-height: 13.5rem;
  --message-trace-max-width: 18rem;
  --thinking-display-width: var(--message-fixed-width);
  gap: 8px;
}

:deep(.mini-panel__messages .message-bubble__avatar) {
  width: 24px;
  height: 24px;
}

:deep(.mini-panel__messages .avatar-icon) {
  font-size: 12px;
}

:deep(.mini-panel__messages .message-bubble__body) {
  min-width: 0;
  gap: 6px;
}

:deep(.mini-panel__messages .message-bubble--assistant .message-bubble__body) {
  width: min(calc(100% - 24px - 8px), var(--assistant-body-width));
}

:deep(.mini-panel__messages .message-bubble--user .message-bubble__body) {
  width: fit-content;
  max-width: min(100%, var(--user-body-width));
}

:deep(.mini-panel__messages .message-bubble__content) {
  width: 100%;
  padding: 8px 10px;
  border-radius: 12px;
  font-size: 12px;
  line-height: 1.45;
}

:deep(.mini-panel__messages .message-bubble--user .message-bubble__content) {
  width: fit-content;
  min-width: 0;
  max-width: min(100%, var(--user-body-width));
}

:deep(.mini-panel__messages .message-bubble__thinking),
:deep(.mini-panel__messages .message-bubble__tool-calls),
:deep(.mini-panel__messages .message-bubble__runtime) {
  width: min(100%, var(--message-compact-max-width));
  min-width: 0;
}

:deep(.mini-panel__messages .message-bubble__tool-calls) {
  gap: 6px;
}

:deep(.mini-panel__messages .message-bubble__tool-calls-shell) {
  padding: 0.55rem;
  border-radius: 0.85rem;
}

:deep(.mini-panel__messages .message-bubble__tool-calls-head) {
  gap: 0.5rem;
}

:deep(.mini-panel__messages .message-bubble__tool-calls-title),
:deep(.mini-panel__messages .message-bubble__tool-calls-toggle) {
  font-size: 0.64rem;
}

:deep(.mini-panel__messages .message-bubble__tool-calls-count) {
  min-width: 1.2rem;
  height: 1.2rem;
  font-size: 0.64rem;
}

:deep(.mini-panel__messages .message-bubble__runtime) {
  width: min(100%, 15rem);
}

:deep(.mini-panel__messages .runtime-notice-list) {
  gap: 4px;
}

:deep(.mini-panel__messages .runtime-notice) {
  border-radius: 10px;
  box-shadow: 0 4px 10px rgba(15, 23, 42, 0.04);
}

:deep(.mini-panel__messages .runtime-notice__summary) {
  grid-template-columns: minmax(0, 1fr);
  gap: 5px;
  padding: 6px 8px;
}

:deep(.mini-panel__messages .runtime-notice__summary-divider) {
  display: none;
}

:deep(.mini-panel__messages .runtime-notice__summary-runtime) {
  align-items: flex-start;
  gap: 5px;
}

:deep(.mini-panel__messages .runtime-notice__header) {
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
}

:deep(.mini-panel__messages .runtime-notice__header-main) {
  min-width: 0;
  flex: 0 1 auto;
  flex-direction: row;
  align-items: center;
  gap: 5px;
}

:deep(.mini-panel__messages .runtime-notice__eyebrow) {
  font-size: 0.52rem;
  line-height: 1;
}

:deep(.mini-panel__messages .runtime-notice__title) {
  font-size: 0.7rem;
  font-weight: 700;
  white-space: nowrap;
}

:deep(.mini-panel__messages .runtime-notice__header-side) {
  gap: 4px;
  min-width: 0;
  flex: 1 1 auto;
}

:deep(.mini-panel__messages .runtime-notice__summary-usage),
:deep(.mini-panel__messages .runtime-notice__usage) {
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: flex-start;
  gap: 5px;
  padding: 0;
}

:deep(.mini-panel__messages .runtime-notice__usage) {
  padding: 6px 8px;
}

:deep(.mini-panel__messages .runtime-notice__usage-main) {
  width: 100%;
  min-width: 0;
  gap: 4px;
}

:deep(.mini-panel__messages .runtime-notice__usage-label) {
  font-size: 0.5rem;
}

:deep(.mini-panel__messages .runtime-notice__usage-model) {
  min-width: 0;
  font-size: 0.68rem;
  line-height: 1.2;
}

:deep(.mini-panel__messages .runtime-notice__usage-stats) {
  width: 100%;
  flex-wrap: wrap;
  justify-content: flex-start;
  gap: 4px;
}

:deep(.mini-panel__messages .runtime-notice__usage-chip) {
  max-width: 100%;
  padding: 0.14rem 0.34rem;
  font-size: 0.56rem;
  line-height: 1.15;
  white-space: normal;
}

:deep(.mini-panel__messages .runtime-notice__chips) {
  flex-wrap: nowrap;
  gap: 3px;
  overflow: hidden;
  white-space: nowrap;
}

:deep(.mini-panel__messages .runtime-notice__chip) {
  flex: 0 0 auto;
  padding: 0.14rem 0.38rem;
  font-size: 0.58rem;
  line-height: 1.1;
}

:deep(.mini-panel__messages .runtime-notice__chevron) {
  font-size: 0.58rem;
}

:deep(.mini-panel__messages .runtime-notice__content) {
  padding: 0 8px 8px;
}

:deep(.mini-panel__messages .message-bubble__meta) {
  gap: 6px;
  padding: 0;
  margin-top: 0;
  font-size: 10px;
}

:deep(.mini-panel__messages .message-bubble__attachments) {
  gap: 6px;
  margin-bottom: 6px;
}

:deep(.mini-panel__messages .message-bubble__attachment-image) {
  width: 56px;
  height: 56px;
  border-radius: 10px;
}

:deep(.mini-panel__messages .file-mention) {
  gap: 4px;
  padding: 2px 6px;
  margin: 0 1px;
  border-radius: 8px;
  font-size: 0.76em;
}

:deep(.mini-panel__messages .file-mention__path) {
  max-width: 132px;
}

:deep(.mini-panel__messages .message-bubble__trace-rail) {
  gap: 6px;
}

:deep(.mini-panel__messages .message-bubble__trace-rail-head) {
  gap: 8px;
}

:deep(.mini-panel__messages .message-bubble__trace-strip) {
  gap: 6px;
  padding-bottom: 4px;
}

:deep(.mini-panel__messages .message-bubble__trace-tile) {
  flex-basis: 100px;
  width: 100px;
  min-height: 102px;
  padding: 8px 8px 10px;
  border-radius: 14px;
}

@media (max-width: 900px) {
  .mini-panel__body {
    padding: 8px 10px 10px;
  }

  .mini-panel__overlay {
    top: 8px;
  }

  :deep(.mini-panel__messages.message-list) {
    padding: 38px 8px 10px;
  }

  :deep(.mini-panel__messages .message-bubble) {
    --assistant-body-width: min(100%, calc(100% - 22px));
    --user-body-width: min(78%, 13rem);
  }
}
</style>
