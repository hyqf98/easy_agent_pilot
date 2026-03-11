<script setup lang="ts">
import { EaButton, EaIcon, EaStateBlock } from '@/components/common'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'
import { useI18n } from 'vue-i18n'
import type { InstallSession } from '@/stores/settings'
import type { InstallSessionStatus, InstallStatusInfo } from './types'

interface Props {
  loading: boolean
  error: string | null
  sessions: InstallSession[]
  cancellingSessionId: string | null
  cleaningSessionId: string | null
}

defineProps<Props>()

const emit = defineEmits<{
  refresh: []
  cancelSession: [sessionId: string]
  cleanupSession: [sessionId: string]
}>()

const { t } = useI18n()

function formatTime(isoString: string): string {
  return new Date(isoString).toLocaleString()
}

function getStatusInfo(status: InstallSessionStatus): InstallStatusInfo {
  const statusMap: Record<string, InstallStatusInfo> = {
    active: { text: t('settings.installSessions.statusActive'), class: 'session-status--active' },
    rolling_back: { text: t('settings.installSessions.statusRollingBack'), class: 'session-status--rolling-back' },
    rolled_back: { text: t('settings.installSessions.statusRolledBack'), class: 'session-status--rolled-back' },
    rollback_failed: { text: t('settings.installSessions.statusRollbackFailed'), class: 'session-status--rollback-failed' },
    completed: { text: t('settings.installSessions.statusCompleted'), class: 'session-status--completed' },
    cancelled: { text: t('settings.installSessions.statusCancelled'), class: 'session-status--cancelled' },
    cancel_rollback_failed: { text: t('settings.installSessions.statusCancelRollbackFailed'), class: 'session-status--rollback-failed' }
  }

  return statusMap[status] || { text: status, class: '' }
}
</script>

<template>
  <SettingsSectionCard :title="t('settings.installSessions.title')">
    <template #actions>
      <EaButton
        type="ghost"
        size="small"
        :disabled="loading"
        @click="emit('refresh')"
      >
        <EaIcon
          name="refresh-cw"
          :size="14"
        />
        {{ t('common.retry') }}
      </EaButton>
    </template>

    <div
      v-if="loading"
      class="sessions-state"
    >
      <EaStateBlock
        variant="loading"
        :description="t('settings.installSessions.loading')"
      />
    </div>

    <div
      v-else-if="error"
      class="sessions-state"
    >
      <EaStateBlock
        variant="error"
        :title="t('settings.installSessions.title')"
        :description="error"
      />
    </div>

    <div
      v-else-if="sessions.length === 0"
      class="sessions-state"
    >
      <EaStateBlock
        variant="success"
        :description="t('settings.installSessions.empty')"
      />
    </div>

    <div
      v-else
      class="sessions-list"
    >
      <div
        v-for="session in sessions"
        :key="session.id"
        class="session-item"
      >
        <div class="session-item__info">
          <div class="session-item__header">
            <span class="session-item__id">{{ session.id.slice(0, 8) }}</span>
            <span
              class="session-item__status"
              :class="getStatusInfo(session.status).class"
            >
              {{ getStatusInfo(session.status).text }}
            </span>
          </div>
          <div class="session-item__meta">
            <span class="session-item__time">
              <EaIcon
                name="clock"
                :size="12"
              />
              {{ formatTime(session.created_at) }}
            </span>
            <span
              v-if="session.operations.length > 0"
              class="session-item__operations"
            >
              {{ t('settings.installSessions.operationCount', { n: session.operations.length }) }}
            </span>
          </div>
          <div
            v-if="session.error_message"
            class="session-item__error"
          >
            <EaIcon
              name="alert-triangle"
              :size="14"
            />
            {{ session.error_message }}
          </div>
        </div>
        <div class="session-item__actions">
          <EaButton
            v-if="session.status === 'active'"
            type="secondary"
            size="small"
            :disabled="cancellingSessionId === session.id"
            :loading="cancellingSessionId === session.id"
            @click="emit('cancelSession', session.id)"
          >
            {{ t('settings.installSessions.cancel') }}
          </EaButton>
          <EaButton
            v-if="['completed', 'rolled_back', 'cancelled', 'rollback_failed', 'cancel_rollback_failed'].includes(session.status)"
            type="ghost"
            size="small"
            :disabled="cleaningSessionId === session.id"
            :loading="cleaningSessionId === session.id"
            @click="emit('cleanupSession', session.id)"
          >
            {{ t('settings.installSessions.cleanup') }}
          </EaButton>
        </div>
      </div>
    </div>
  </SettingsSectionCard>
</template>

<style scoped>
.sessions-state {
  display: flex;
}

.sessions-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.session-item {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-4);
  padding: var(--spacing-3);
  border-radius: var(--radius-md);
  background-color: var(--color-bg-tertiary);
}

.session-item__info {
  min-width: 0;
  flex: 1;
}

.session-item__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  margin-bottom: var(--spacing-1);
}

.session-item__id {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  font-family: var(--font-family-mono);
}

.session-item__status {
  padding: 2px var(--spacing-2);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.session-status--active {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

.session-status--rolling-back {
  background-color: var(--color-warning-light);
  color: var(--color-warning);
}

.session-status--completed {
  background-color: var(--color-success-light);
  color: var(--color-success);
}

.session-status--rolled-back,
.session-status--cancelled {
  background-color: var(--color-bg-secondary);
  color: var(--color-text-secondary);
}

.session-status--rollback-failed {
  background-color: var(--color-error-light);
  color: var(--color-error);
}

.session-item__meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

.session-item__time,
.session-item__operations {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
}

.session-item__error {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-1);
  margin-top: var(--spacing-2);
  padding: var(--spacing-2);
  border-radius: var(--radius-sm);
  background-color: var(--color-error-light);
  color: var(--color-error);
  font-size: var(--font-size-xs);
}

.session-item__actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  flex-shrink: 0;
}
</style>
