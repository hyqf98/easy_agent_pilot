<script setup lang="ts">
import { onMounted, onUnmounted, ref, type ComponentPublicInstance } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { EaIcon } from '@/components/common'
import TokenProgressBar from '@/components/common/TokenProgressBar.vue'
import { MessageList } from '@/components/message'
import ConversationComposer from '@/components/layout/ConversationComposer.vue'
import { useMessageStore, type Message, type MessageAttachment } from '@/stores/message'
import { useMiniPanelStore } from '@/stores/miniPanel'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { convertFileSrc } from '@tauri-apps/api/core'

const miniPanelStore = useMiniPanelStore()
const messageStore = useMessageStore()
const sessionExecutionStore = useSessionExecutionStore()

type ComposerExposed = ComponentPublicInstance & {
  focusInput: () => void
  openCompressionDialog: () => void
}

const composerRef = ref<ComposerExposed | null>(null)
let unlistenFocus: (() => void) | null = null

const toPendingImages = (attachments: MessageAttachment[]) => attachments.map(attachment => ({
  ...attachment,
  previewUrl: convertFileSrc(attachment.path)
}))

async function hideMiniPanel() {
  await invoke('hide_mini_panel')
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
    sessionExecutionStore.setPendingImages(sessionId, toPendingImages(message.attachments ?? []))
    composerRef.value?.focusInput()
    return
  }

  if (message.role === 'assistant') {
    const messages = messageStore.messagesBySession(sessionId)
    const messageIndex = messages.findIndex(item => item.id === message.id)

    for (let index = messageIndex - 1; index >= 0; index -= 1) {
      if (messages[index].role === 'user') {
        sessionExecutionStore.setInputText(sessionId, messages[index].content)
        sessionExecutionStore.setPendingImages(sessionId, toPendingImages(messages[index].attachments ?? []))
        break
      }
    }
  }

  composerRef.value?.focusInput()
}

onMounted(async () => {
  await miniPanelStore.initSessionContext()
  composerRef.value?.focusInput()

  const currentWindow = getCurrentWindow()
  unlistenFocus = await currentWindow.listen('mini-panel:focus-input', () => {
    composerRef.value?.focusInput()
  })
})

onUnmounted(() => {
  unlistenFocus?.()
})
</script>

<template>
  <div class="mini-panel">
    <header class="mini-panel__header">
      <div class="mini-panel__token-bar">
        <TokenProgressBar @compress="handleOpenCompress" />
      </div>
      <button
        class="mini-panel__close"
        @click="hideMiniPanel"
      >
        <EaIcon
          name="x"
          :size="16"
        />
      </button>
    </header>

    <div class="mini-panel__body">
      <section class="mini-panel__conversation">
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
        hide-status-bar
        compact
        class="mini-panel__composer"
      />
    </div>
  </div>
</template>

<style scoped>
.mini-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  color: var(--color-text-primary);
  background:
    radial-gradient(circle at top left, rgba(14, 165, 233, 0.12), transparent 24%),
    radial-gradient(circle at top right, rgba(59, 130, 246, 0.08), transparent 22%),
    linear-gradient(180deg, #f7fbff, #edf4fb 40%, #f8fafc);
}

.mini-panel__header {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  position: relative;
  padding: 14px 18px 10px;
  min-height: 56px;
}

.mini-panel__token-bar {
  position: absolute;
  left: 50%;
  top: 14px;
  transform: translateX(-50%);
  width: min(320px, calc(100% - 88px));
}

.mini-panel__close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  border-radius: 12px;
  border: 1px solid color-mix(in srgb, var(--color-border) 74%, transparent);
  background: rgba(255, 255, 255, 0.88);
  color: var(--color-text-secondary);
  transition: background-color var(--transition-fast), border-color var(--transition-fast), color var(--transition-fast);
  z-index: 1;
}

.mini-panel__close:hover {
  background: rgba(255, 255, 255, 0.96);
  border-color: color-mix(in srgb, var(--color-border-dark) 82%, transparent);
  color: var(--color-text-primary);
}

.mini-panel__body {
  min-height: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 0 18px 18px;
}

.mini-panel__conversation {
  min-height: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
  border-radius: 18px;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.94), rgba(248, 250, 252, 0.96));
  box-shadow: 0 18px 36px rgba(15, 23, 42, 0.06);
}

.mini-panel__messages {
  flex: 1;
  min-height: 0;
  padding-top: 6px;
}

.mini-panel__composer {
  flex-shrink: 0;
  border: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.9);
  box-shadow: 0 12px 30px rgba(15, 23, 42, 0.05);
}

:deep(.mini-panel__token-bar .token-progress) {
  width: 100%;
}

:deep(.mini-panel__composer.conversation-composer) {
  padding: 12px;
  gap: 8px;
  background: transparent;
}

:deep(.mini-panel__composer .conversation-composer__panel) {
  gap: 8px;
}

:deep(.mini-panel__composer .conversation-composer__editor-shell) {
  min-height: 108px;
  border-radius: 16px;
  background: rgba(248, 250, 252, 0.98);
}

:deep(.mini-panel__composer .conversation-composer__render),
:deep(.mini-panel__composer .conversation-composer__textarea) {
  min-height: 108px;
  padding: 14px 16px;
}

:deep(.mini-panel__messages.message-list) {
  padding: 12px 14px 22px;
}

:deep(.mini-panel__messages .message-bubble) {
  --assistant-body-width: clamp(14rem, 78%, 38rem);
  --user-body-width: clamp(11rem, 62%, 28rem);
}

:deep(.mini-panel__messages .message-bubble__body) {
  min-width: 0;
}

:deep(.mini-panel__messages .message-bubble--assistant .message-bubble__body) {
  width: min(calc(100% - 32px - var(--spacing-3)), var(--assistant-body-width));
}

:deep(.mini-panel__messages .message-bubble--user .message-bubble__body) {
  width: min(100%, var(--user-body-width));
}

:deep(.mini-panel__messages .message-bubble__content) {
  width: 100%;
}

@media (max-width: 900px) {
  .mini-panel__header {
    padding: 12px 14px 8px;
  }

  .mini-panel__body {
    padding: 0 14px 14px;
  }

  :deep(.mini-panel__messages.message-list) {
    padding: 10px 10px 18px;
  }

  :deep(.mini-panel__messages .message-bubble) {
    --assistant-body-width: min(100%, calc(100% - 28px));
    --user-body-width: min(82%, 24rem);
  }
}
</style>
