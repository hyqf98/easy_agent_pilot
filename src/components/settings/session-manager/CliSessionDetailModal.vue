<script setup lang="ts">
import { computed } from 'vue'
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

const formatTime = (value: string) => formatCliTime(value)

const getMessageDisplayContent = (message: CliSessionMessage) =>
  getCliMessageDisplayContent(message, {
    noPreview: t('settings.sessionManager.noPreview'),
    fileSnapshot: '[File Snapshot] 历史文件快照已更新',
    progress: '[Progress] 当前记录未包含可直接展示的文本内容',
    noParsedContent: '[No parsed content] 请展开 Raw JSON 查看原始记录'
  })
</script>

<template>
  <EaModal
    v-model:visible="modalVisible"
    size="large"
  >
    <template #header>
      <div class="modal-title-wrap">
        <h3 class="modal-title">{{ t('settings.sessionManager.detailTitle') }}</h3>
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

      <div class="message-list">
        <div
          v-for="msg in detail.messages"
          :key="`${msg.line_no}-${msg.message_type}`"
          class="message-item"
          :class="`message-item--${msg.message_type}`"
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
            <span
              v-if="msg.timestamp"
              class="message-item__time"
            >
              {{ formatTime(msg.timestamp) }}
            </span>
          </div>

          <div
            v-if="getMessageDisplayContent(msg)"
            class="message-item__content"
          >
            {{ getMessageDisplayContent(msg) }}
          </div>

          <details class="message-item__raw">
            <summary>{{ t('settings.sessionManager.rawJson') }}</summary>
            <pre>{{ msg.raw_json }}</pre>
          </details>
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
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.message-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  max-height: 60vh;
  overflow: auto;
  padding-right: var(--spacing-2);
}

.message-item {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background-color: var(--color-bg-tertiary);
  overflow: hidden;
}

.message-item--user {
  border-left: 3px solid var(--color-primary);
}

.message-item--assistant {
  border-left: 3px solid var(--color-success);
}

.message-item__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-bg-secondary);
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
</style>
