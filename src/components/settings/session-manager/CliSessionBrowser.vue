<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon } from '@/components/common'
import {
  displayCliSessionMessage,
  formatCliMessageCount,
  formatCliRelativeTime,
  getCliProjectName,
  shortenCliSessionId
} from '@/utils/sessionManager'
import type { ScannedCliSession } from '@/types/cliSessionManager'

interface Props {
  cliName: string
  sessionRoot: string
  sessions: ScannedCliSession[]
  groupedSessions: Record<string, ScannedCliSession[]>
  isLoadingSessions: boolean
  sessionsError: string
  selectedSessionPaths: string[]
  selectedCount: number
  allVisibleSelected: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  refresh: []
  toggleSelectAll: []
  requestDeleteSelected: []
  selectionChange: [sessionPath: string, event: Event]
  openDetail: [session: ScannedCliSession]
  requestDelete: [session: ScannedCliSession]
}>()

const { t } = useI18n()

const selectedSessionPathSet = computed(() => new Set(props.selectedSessionPaths))

const formatRelativeTime = (value: string) => formatCliRelativeTime(value, {
  justNow: t('settings.sessionManager.justNow'),
  minutesAgo: n => t('settings.sessionManager.minutesAgo', { n }),
  hoursAgo: n => t('settings.sessionManager.hoursAgo', { n }),
  daysAgo: n => t('settings.sessionManager.daysAgo', { n })
})

const displayMessage = (session: ScannedCliSession) =>
  displayCliSessionMessage(session, t('settings.sessionManager.noPreview'))

const formatMessageCount = (value: number) => formatCliMessageCount(value)

const shortSessionId = (sessionId: string) => shortenCliSessionId(sessionId)

const getProjectName = (path: string) =>
  getCliProjectName(path, t('settings.sessionManager.noProject'))
</script>

<template>
  <div class="settings-card">
    <div class="settings-card__header">
      <h4 class="settings-card__title settings-card__title--no-border">
        {{ t('settings.sessionManager.sessionList') }}
      </h4>
      <div class="header-meta">
        <span class="cli-badge">{{ cliName || '-' }}</span>
        <span class="session-count">{{ sessions.length }} {{ t('settings.sessionManager.sessionCount') }}</span>
        <span
          v-if="selectedCount > 0"
          class="selected-count"
        >
          {{ t('settings.sessionManager.selectedCount', { n: selectedCount }) }}
        </span>
        <EaButton
          v-if="sessions.length > 0"
          type="ghost"
          size="small"
          @click="emit('toggleSelectAll')"
        >
          {{ allVisibleSelected ? t('settings.sessionManager.clearSelection') : t('settings.sessionManager.selectAll') }}
        </EaButton>
        <EaButton
          v-if="sessions.length > 0"
          type="danger"
          size="small"
          :disabled="selectedCount === 0"
          @click="emit('requestDeleteSelected')"
        >
          {{ t('settings.sessionManager.batchDelete') }}
        </EaButton>
      </div>
    </div>

    <div
      v-if="sessionRoot"
      class="root-path"
    >
      <EaIcon
        name="folder"
        :size="12"
      />
      <code class="root-path__value">{{ sessionRoot }}</code>
    </div>

    <div
      v-if="isLoadingSessions"
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
      v-else-if="sessionsError"
      class="error"
    >
      <EaIcon
        name="alert-circle"
        :size="18"
      />
      <span>{{ sessionsError }}</span>
    </div>

    <div
      v-else-if="sessions.length === 0"
      class="empty-state"
    >
      <EaIcon
        name="inbox"
        :size="24"
      />
      <span>{{ t('settings.sessionManager.noSessions') }}</span>
    </div>

    <div
      v-else
      class="session-groups"
    >
      <div
        v-for="(groupSessions, projectPath) in groupedSessions"
        :key="projectPath"
        class="session-group"
      >
        <div class="session-group__header">
          <EaIcon
            name="folder"
            :size="14"
          />
          <span class="session-group__name">{{ getProjectName(projectPath) }}</span>
          <span class="session-group__count">{{ groupSessions.length }}</span>
        </div>

        <div class="session-group__list">
          <div
            v-for="session in groupSessions"
            :key="session.session_path"
            :class="['session-card', { 'session-card--selected': selectedSessionPathSet.has(session.session_path) }]"
          >
            <div class="session-card__select">
              <input
                :checked="selectedSessionPathSet.has(session.session_path)"
                class="session-card__checkbox"
                type="checkbox"
                @change="emit('selectionChange', session.session_path, $event)"
              >
            </div>
            <div class="session-card__main">
              <div class="session-card__header">
                <span class="session-card__id">{{ shortSessionId(session.session_id) }}</span>
                <span class="session-card__time">{{ formatRelativeTime(session.updated_at) }}</span>
              </div>
              <p class="session-card__preview">
                {{ displayMessage(session) }}
              </p>
              <div class="session-card__footer">
                <span class="session-card__messages">
                  <EaIcon
                    name="message-square"
                    :size="12"
                  />
                  {{ formatMessageCount(session.message_count) }}
                </span>
              </div>
            </div>

            <div class="session-card__actions">
              <button
                class="action-btn action-btn--view"
                :title="t('settings.sessionManager.view')"
                @click="emit('openDetail', session)"
              >
                <EaIcon
                  name="eye"
                  :size="14"
                />
              </button>
              <button
                class="action-btn action-btn--delete"
                :title="t('settings.sessionManager.delete')"
                @click="emit('requestDelete', session)"
              >
                <EaIcon
                  name="trash-2"
                  :size="14"
                />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  padding: var(--spacing-5);
  background-color: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
}

.settings-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
}

.settings-card__title {
  margin: 0;
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
}

.settings-card__title--no-border {
  padding-bottom: 0;
  border-bottom: none;
}

.header-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--spacing-3);
  justify-content: flex-end;
}

.cli-badge {
  padding: 2px 8px;
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  color: var(--color-primary);
  background-color: var(--color-primary-bg);
  border-radius: var(--radius-sm);
}

.session-count {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.selected-count {
  font-size: var(--font-size-xs);
  color: var(--color-primary);
}

.root-path {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-bg-tertiary);
  border-radius: var(--radius-sm);
}

.root-path__value {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  word-break: break-all;
}

.loading,
.error,
.empty-state {
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

.session-groups {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.session-group {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.session-group__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) 0;
  color: var(--color-text-secondary);
  border-bottom: 1px solid var(--color-border);
}

.session-group__name {
  flex: 1;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-group__count {
  padding: 1px 6px;
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  background-color: var(--color-bg-tertiary);
  border-radius: var(--radius-sm);
}

.session-group__list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: var(--spacing-3);
}

.session-card {
  display: flex;
  gap: var(--spacing-2);
  padding: var(--spacing-3);
  background-color: var(--color-bg-tertiary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  transition: border-color 0.2s, box-shadow 0.2s;
}

.session-card--selected {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-primary) 20%, transparent);
}

.session-card:hover {
  border-color: var(--color-primary);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.session-card__select {
  display: flex;
  align-items: flex-start;
  padding-top: 2px;
}

.session-card__checkbox {
  width: 16px;
  height: 16px;
  margin: 0;
  accent-color: var(--color-primary);
  cursor: pointer;
}

.session-card__main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.session-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-2);
}

.session-card__id {
  font-size: var(--font-size-xs);
  font-family: var(--font-family-mono);
  color: var(--color-text-tertiary);
}

.session-card__time {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.session-card__preview {
  flex: 1;
  margin: 0;
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.session-card__footer {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
}

.session-card__messages {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.session-card__actions {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: none;
  border-radius: var(--radius-sm);
  background-color: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: background-color 0.2s, color 0.2s;
}

.action-btn:hover {
  background-color: var(--color-bg-secondary);
}

.action-btn--view:hover {
  color: var(--color-primary);
}

.action-btn--delete:hover {
  color: var(--color-error);
}

@media (max-width: 860px) {
  .session-group__list {
    grid-template-columns: 1fr;
  }
}
</style>
