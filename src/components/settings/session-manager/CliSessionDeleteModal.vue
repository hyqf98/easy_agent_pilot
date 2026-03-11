<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon, EaModal } from '@/components/common'
import { displayCliSessionMessage } from '@/utils/sessionManager'
import type { ScannedCliSession } from '@/types/cliSessionManager'

interface Props {
  visible: boolean
  deleting: boolean
  sessions: ScannedCliSession[]
  error: string
}

const props = defineProps<Props>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  confirm: []
}>()

const { t } = useI18n()

const modalVisible = computed({
  get: () => props.visible,
  set: (value: boolean) => emit('update:visible', value)
})

const isBulkDelete = computed(() => props.sessions.length > 1)
const deletePreviewSessions = computed(() => props.sessions.slice(0, 5))

const displayMessage = (session: ScannedCliSession) =>
  displayCliSessionMessage(session, t('settings.sessionManager.noPreview'))
</script>

<template>
  <EaModal v-model:visible="modalVisible">
    <template #header>
      <h3 class="modal-title">
        {{ isBulkDelete ? t('settings.sessionManager.confirmBatchDeleteTitle') : t('settings.sessionManager.confirmDeleteTitle') }}
      </h3>
    </template>

    <p class="confirm-text">
      {{ isBulkDelete ? t('settings.sessionManager.confirmBatchDeleteDesc', { n: sessions.length }) : t('settings.sessionManager.confirmDeleteDesc') }}
    </p>

    <div
      v-if="sessions.length > 0"
      class="confirm-session-list"
    >
      <div
        v-for="session in deletePreviewSessions"
        :key="session.session_path"
        class="confirm-session"
      >
        <div class="confirm-session__preview">
          {{ displayMessage(session) }}
        </div>
        <code class="confirm-session__path">{{ session.session_path }}</code>
      </div>

      <div
        v-if="sessions.length > deletePreviewSessions.length"
        class="confirm-session__more"
      >
        {{ t('settings.sessionManager.moreSelected', { n: sessions.length - deletePreviewSessions.length }) }}
      </div>
    </div>

    <div
      v-if="error"
      class="error"
    >
      <EaIcon
        name="alert-circle"
        :size="16"
      />
      <span>{{ error }}</span>
    </div>

    <template #footer>
      <EaButton
        type="secondary"
        :disabled="deleting"
        @click="modalVisible = false"
      >
        {{ t('common.cancel') }}
      </EaButton>
      <EaButton
        type="danger"
        :loading="deleting"
        @click="emit('confirm')"
      >
        {{ t('settings.sessionManager.delete') }}
      </EaButton>
    </template>
  </EaModal>
</template>

<style scoped>
.modal-title {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
}

.confirm-text {
  margin: 0;
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
}

.confirm-session {
  padding: var(--spacing-3);
  background-color: var(--color-bg-tertiary);
  border-radius: var(--radius-md);
}

.confirm-session-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  margin-top: var(--spacing-3);
}

.confirm-session__preview {
  margin-bottom: var(--spacing-2);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
}

.confirm-session__path {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  word-break: break-all;
}

.confirm-session__more {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  text-align: center;
}

.error {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  margin-top: var(--spacing-3);
  color: var(--color-error);
  white-space: pre-wrap;
}
</style>
