<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon, EaModal } from '@/components/common'
import {
  formatCliTime,
  getCliMessageColor,
  getCliMessageDisplayContent,
  getCliMessageIcon
} from '@/utils/sessionManager'
import type { CliSessionDetail, CliSessionMessage } from '@/types/cliSessionManager'

interface Props {
  visible: boolean
  loading: boolean
  error: string
  detail: CliSessionDetail | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
}>()

const { t } = useI18n()

const modalVisible = computed({
  get: () => props.visible,
  set: (value: boolean) => emit('update:visible', value)
})

const expandedMessageKeys = ref<string[]>([])
const expandedMessageKeySet = computed(() => new Set(expandedMessageKeys.value))

const formatTime = (value: string) => formatCliTime(value)

const getMessageKey = (message: CliSessionMessage) => `${message.line_no}-${message.message_type}`

const getMessageDisplayContent = (message: CliSessionMessage) =>
  getCliMessageDisplayContent(message, {
    noPreview: t('settings.sessionManager.noPreview'),
    fileSnapshot: '[File Snapshot] 历史文件快照已更新',
    progress: '[Progress] 当前记录未包含可直接展示的文本内容',
    noParsedContent: '[No parsed content] 请展开 Raw JSON 查看原始记录'
  })

const isUserMessage = (message: CliSessionMessage) =>
  message.message_type === 'user' || message.role === 'user'

const isAssistantMessage = (message: CliSessionMessage) =>
  message.message_type === 'assistant' || message.role === 'assistant'

const getMessageAlignmentClass = (message: CliSessionMessage) => {
  if (isAssistantMessage(message)) return 'message-row--assistant'
  if (isUserMessage(message)) return 'message-row--user'
  return 'message-row--event'
}

const getBubbleClass = (message: CliSessionMessage) => {
  if (isAssistantMessage(message)) return 'message-bubble--assistant'
  if (isUserMessage(message)) return 'message-bubble--user'
  return 'message-bubble--event'
}

const isExpanded = (message: CliSessionMessage) => expandedMessageKeySet.value.has(getMessageKey(message))

const toggleExpanded = (message: CliSessionMessage) => {
  const key = getMessageKey(message)
  const next = new Set(expandedMessageKeys.value)

  if (next.has(key)) {
    next.delete(key)
  } else {
    next.add(key)
  }

  expandedMessageKeys.value = Array.from(next)
}

const getCollapsedPreview = (message: CliSessionMessage) => {
  const content = getMessageDisplayContent(message)
  if (!content) {
    return t('settings.sessionManager.noPreview')
  }

  return content.replace(/\s+/g, ' ').trim()
}

watch(() => props.detail?.session_path, () => {
  expandedMessageKeys.value = []
})

watch(() => props.visible, (visible) => {
  if (!visible) {
    expandedMessageKeys.value = []
  }
})
</script>

<template>
  <EaModal
    v-model:visible="modalVisible"
    content-class="cli-session-detail-modal"
  >
    <template #header>
      <div class="modal-title-wrap">
        <h3 class="modal-title">
          {{ t('settings.sessionManager.detailTitle') }}
        </h3>
        <span
          v-if="detail"
          class="modal-subtitle"
        >
          {{ detail.message_count }} {{ t('settings.sessionManager.messages') }}
        </span>
      </div>
    </template>

    <div
      v-if="loading"
      class="loading"
    >
      <EaIcon
        name="loader"
        :size="20"
        spin
      />
      <span>{{ t('common.loading') }}</span>
    </div>

    <div
      v-else-if="error"
      class="error"
    >
      <EaIcon
        name="alert-circle"
        :size="18"
      />
      <span>{{ error }}</span>
    </div>

    <div
      v-else-if="detail"
      class="detail"
    >
      <div class="detail-summary">
        <div class="detail-summary__item">
          <EaIcon
            name="hash"
            :size="14"
          />
          <span class="detail-summary__label">ID:</span>
          <code class="detail-summary__value">{{ detail.session_id }}</code>
        </div>
        <div class="detail-summary__item">
          <EaIcon
            name="clock"
            :size="14"
          />
          <span class="detail-summary__label">{{ t('settings.sessionManager.updatedAt') }}:</span>
          <span>{{ formatTime(detail.updated_at) }}</span>
        </div>
        <div
          v-if="detail.project_path"
          class="detail-summary__item"
        >
          <EaIcon
            name="folder"
            :size="14"
          />
          <span class="detail-summary__label">{{ t('settings.sessionManager.projectPath') }}:</span>
          <span class="detail-summary__value detail-summary__value--truncate">{{ detail.project_path }}</span>
        </div>
      </div>

      <div class="message-feed">
        <div
          v-for="msg in detail.messages"
          :key="getMessageKey(msg)"
          class="message-row"
          :class="getMessageAlignmentClass(msg)"
        >
          <div
            class="message-bubble"
            :class="getBubbleClass(msg)"
          >
            <div class="message-item__header">
              <div class="message-item__type">
                <EaIcon
                  :name="getCliMessageIcon(msg.message_type)"
                  :size="14"
                  :style="{ color: getCliMessageColor(msg.message_type) }"
                />
                <span>{{ msg.message_type }}</span>
                <span
                  v-if="msg.role"
                  class="message-item__role"
                >
                  {{ msg.role }}
                </span>
              </div>
              <div class="message-item__meta">
                <span
                  v-if="msg.timestamp"
                  class="message-item__time"
                >
                  {{ formatTime(msg.timestamp) }}
                </span>
                <button
                  type="button"
                  class="message-item__toggle"
                  @click="toggleExpanded(msg)"
                >
                  <EaIcon
                    :name="isExpanded(msg) ? 'chevron-up' : 'chevron-down'"
                    :size="14"
                  />
                  <span>{{ isExpanded(msg) ? '收起' : '展开' }}</span>
                </button>
              </div>
            </div>

            <div
              v-if="!isExpanded(msg)"
              class="message-item__preview"
            >
              {{ getCollapsedPreview(msg) }}
            </div>

            <div
              v-else-if="getMessageDisplayContent(msg)"
              class="message-item__content"
            >
              {{ getMessageDisplayContent(msg) }}
            </div>

            <details
              v-if="isExpanded(msg)"
              class="message-item__raw"
            >
              <summary>{{ t('settings.sessionManager.rawJson') }}</summary>
              <pre>{{ msg.raw_json }}</pre>
            </details>
          </div>
        </div>
      </div>
    </div>
  </EaModal>
</template>

<style scoped>
.loading,
.error {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  min-height: 120px;
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.error {
  color: var(--color-error);
  white-space: pre-wrap;
}

.modal-title-wrap {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
}

.modal-title {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
}

.modal-subtitle {
  font-size: var(--font-size-sm);
  color: var(--color-text-tertiary);
}

.detail {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  min-height: 0;
}

.detail-summary {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-4);
  padding: var(--spacing-3);
  background-color: var(--color-bg-tertiary);
  border-radius: var(--radius-md);
}

.detail-summary__item {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.detail-summary__label {
  color: var(--color-text-tertiary);
}

.detail-summary__value {
  font-family: var(--font-family-mono);
  color: var(--color-text-primary);
}

.detail-summary__value--truncate {
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.message-feed {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  max-height: min(68vh, 760px);
  overflow: auto;
  padding-right: var(--spacing-2);
}

.message-row {
  display: flex;
}

.message-row--assistant {
  justify-content: flex-start;
}

.message-row--user {
  justify-content: flex-end;
}

.message-row--event {
  justify-content: flex-start;
}

.message-bubble {
  width: fit-content;
  min-width: min(18rem, 100%);
  max-width: min(100%, 980px);
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background-color: var(--color-bg-tertiary);
}

.message-bubble--assistant {
  max-width: min(86%, 980px);
  border-left: 3px solid var(--color-success);
}

.message-bubble--user {
  max-width: min(86%, 980px);
  border-left: 3px solid var(--color-primary);
  background-color: var(--color-primary-light);
}

.message-bubble--event {
  max-width: min(92%, 1040px);
  margin-left: var(--spacing-3);
}

:global(.ea-modal.cli-session-detail-modal) {
  width: min(1240px, calc(100vw - 32px));
  max-width: min(1240px, calc(100vw - 32px));
}

.message-item__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-bg-secondary);
}

.message-item__meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.message-item__type {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  text-transform: capitalize;
}

.message-item__role {
  padding: 1px 6px;
  border-radius: var(--radius-sm);
  background-color: var(--color-bg-tertiary);
  color: var(--color-text-tertiary);
  text-transform: none;
}

.message-item__time {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.message-item__toggle {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border: 0;
  background: transparent;
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
  cursor: pointer;
  padding: 0;
}

.message-item__toggle:hover {
  color: var(--color-text-primary);
}

.message-item__preview {
  padding: var(--spacing-3);
  font-size: var(--font-size-sm);
  line-height: 1.5;
  color: var(--color-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.message-item__content {
  padding: var(--spacing-3);
  font-size: var(--font-size-sm);
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--color-text-primary);
}

.message-item__raw {
  border-top: 1px solid var(--color-border);
}

.message-item__raw summary {
  padding: var(--spacing-2) var(--spacing-3);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  cursor: pointer;
}

.message-item__raw summary:hover {
  color: var(--color-text-secondary);
}

.message-item__raw pre {
  margin: 0;
  padding: var(--spacing-3);
  font-size: var(--font-size-xs);
  background-color: var(--color-bg-secondary);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 200px;
  overflow: auto;
}

@media (max-width: 900px) {
  .message-bubble--assistant,
  .message-bubble--user,
  .message-bubble--event {
    max-width: 100%;
  }
}
</style>
