<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Session } from '@/stores/session'
import { useSessionView } from '@/composables'
import { EaIcon } from '@/components/common'

interface Props {
  sessions: Session[]
  currentSessionId: string | null
  editingSessionId: string | null
  editingSessionName: string
}

defineProps<Props>()

const emit = defineEmits<{
  select: [id: string]
  togglePin: [id: string]
  startEdit: [session: Session, event: Event]
  saveEdit: [session: Session]
  cancelEdit: []
  delete: [session: Session]
  updateEditingName: [value: string]
}>()

const { t } = useI18n()
const {
  getStatusIcon,
  getStatusClass,
  isRunningStatus,
  formatRelativeTime,
  formatSessionCreatedAt
} = useSessionView()
</script>

<template>
  <div class="session-list">
    <div
      v-for="session in sessions"
      :key="session.id"
      :class="[
        'session-item',
        {
          'session-item--active': session.id === currentSessionId,
          'session-item--pinned': session.pinned
        }
      ]"
      @click="emit('select', session.id)"
    >
      <div class="session-item__content">
        <div class="session-item__main">
          <EaIcon
            :name="getStatusIcon(session.status)"
            :size="14"
            :class="['session-item__status', getStatusClass(session.status), { 'animate-spin': isRunningStatus(session.status) }]"
          />
          <div
            v-if="editingSessionId === session.id"
            class="session-item__name-edit"
          >
            <input
              :value="editingSessionName"
              type="text"
              class="session-name-input"
              @click.stop
              @input="emit('updateEditingName', ($event.target as HTMLInputElement).value)"
              @keydown.enter="emit('saveEdit', session)"
              @keydown.escape="emit('cancelEdit')"
            >
            <button
              class="edit-action-btn"
              @click.stop="emit('saveEdit', session)"
            >
              <EaIcon
                name="check"
                :size="12"
              />
            </button>
            <button
              class="edit-action-btn"
              @click.stop="emit('cancelEdit')"
            >
              <EaIcon
                name="x"
                :size="12"
              />
            </button>
          </div>
          <template v-else>
            <span
              class="session-item__name"
              :title="session.name"
            >
              {{ session.name }}
            </span>
            <span class="session-item__time">{{ formatRelativeTime(session.updatedAt) }}</span>
          </template>
        </div>

        <div
          v-if="editingSessionId !== session.id"
          class="session-item__meta"
        >
          <span
            v-if="session.agentType"
            class="session-item__meta-item"
          >
            <EaIcon
              name="bot"
              :size="10"
            />
            {{ session.agentType }}
          </span>
          <span
            v-if="session.messageCount"
            class="session-item__meta-item"
          >
            <EaIcon
              name="message-square"
              :size="10"
            />
            {{ t('unified.messages', { count: session.messageCount }) }}
          </span>
          <span class="session-item__meta-item session-item__meta-item--created">
            <EaIcon
              name="calendar"
              :size="10"
            />
            {{ formatSessionCreatedAt(session.createdAt) }}
          </span>
        </div>

        <div
          v-if="session.lastMessage && editingSessionId !== session.id"
          class="session-item__preview"
          :title="session.lastMessage"
        >
          {{ session.lastMessage }}
        </div>
      </div>

      <div class="session-item__actions">
        <button
          v-if="editingSessionId !== session.id"
          class="session-action-btn"
          :title="session.pinned ? t('session.unpin') : t('session.pin')"
          @click.stop="emit('togglePin', session.id)"
        >
          <EaIcon
            :name="session.pinned ? 'pin-off' : 'pin'"
            :size="12"
          />
        </button>
        <button
          v-if="editingSessionId !== session.id"
          class="session-action-btn"
          :title="t('common.edit')"
          @click.stop="emit('startEdit', session, $event)"
        >
          <EaIcon
            name="edit-2"
            :size="12"
          />
        </button>
        <button
          v-if="editingSessionId !== session.id"
          class="session-action-btn session-action-btn--danger"
          :title="t('common.delete')"
          @click.stop="emit('delete', session)"
        >
          <EaIcon
            name="x"
            :size="12"
          />
        </button>
      </div>
    </div>

    <div
      v-if="sessions.length === 0"
      class="session-empty"
    >
      <p>{{ t('session.noSessions') }}</p>
    </div>
  </div>
</template>

<style scoped>
.session-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
  height: 100%;
  width: 100%;
  min-width: 0;
  overflow-y: auto;
  box-sizing: border-box;
}

.session-item {
  display: flex;
  align-items: flex-start;
  padding: var(--spacing-3);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
  border-radius: var(--radius-md);
  background-color: var(--color-surface);
  border: 1px solid transparent;
}

.session-item:hover {
  background-color: var(--color-primary-light);
  border-color: var(--color-primary-light);
}

.session-item--active {
  background-color: var(--color-primary-light);
  border-color: var(--color-primary);
}

.session-item--active:hover {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 1px var(--color-primary);
}

[data-theme='dark'] .session-item--active {
  background-color: var(--color-active-bg);
  border-color: var(--color-active-border);
}

[data-theme='dark'] .session-item--active:hover {
  background-color: var(--color-active-bg-hover);
  border-color: var(--color-active-border);
  box-shadow: 0 0 0 1px var(--color-active-border);
}

.session-item__content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.session-item__main {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  flex: 1;
  min-width: 0;
}

.session-item__status {
  flex-shrink: 0;
}

.session-item__status--running {
  color: var(--color-primary);
}

.session-item__status--completed {
  color: var(--color-success);
}

.session-item__status--error {
  color: var(--color-error);
}

.session-item__status--paused {
  color: var(--color-warning);
}

.session-item__name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
}

.session-item--active .session-item__name,
.session-item:hover .session-item__name {
  color: var(--color-primary);
}

.session-item__time {
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  margin-left: var(--spacing-2);
}

.session-item__meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding-left: calc(14px + var(--spacing-2));
  flex-wrap: wrap;
}

.session-item__meta-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.session-item__meta-item:first-child {
  color: var(--color-primary);
}

.session-item__meta-item--created {
  color: var(--color-text-quaternary);
}

.session-item__preview {
  padding-left: calc(14px + var(--spacing-2));
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.4;
}

.session-item__name-edit {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  flex: 1;
}

.session-name-input {
  flex: 1;
  padding: 2px var(--spacing-1);
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  background: var(--color-surface);
  color: var(--color-text-primary);
  outline: none;
}

.edit-action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: var(--radius-sm);
  color: var(--color-text-secondary);
  background: transparent;
  border: none;
  cursor: pointer;
}

.edit-action-btn:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-primary);
}

.session-item__actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  margin-left: var(--spacing-2);
  flex-shrink: 0;
  visibility: hidden;
  opacity: 0;
  transition: opacity var(--transition-fast) var(--easing-default);
}

.session-item:hover .session-item__actions {
  visibility: visible;
  opacity: 1;
}

.session-action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
  background: transparent;
  border: none;
  cursor: pointer;
}

.session-action-btn:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-secondary);
}

.session-action-btn--danger:hover {
  background-color: var(--color-error-light);
  color: var(--color-error);
}

.session-empty {
  padding: var(--spacing-4);
  text-align: center;
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}
</style>
