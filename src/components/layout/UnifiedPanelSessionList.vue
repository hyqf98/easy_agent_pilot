<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Session } from '@/stores/session'
import { useSessionView } from '@/composables'
import { EaIcon } from '@/components/common'

interface Props {
  sessions: Session[]
  currentSessionId: string | null
  editingSessionId: string | null
  editingSessionName: string
  selectedSessionIds: string[]
}

defineProps<Props>()

const emit = defineEmits<{
  select: [id: string]
  toggleSelect: [id: string]
  togglePin: [id: string]
  startEdit: [session: Session, event: Event]
  saveEdit: [session: Session]
  cancelEdit: []
  delete: [session: Session]
  updateEditingName: [value: string]
}>()

const { t } = useI18n()
const {
  getStatusText,
  formatRelativeTime,
  formatSessionCreatedAt
} = useSessionView()
const sessionListRef = ref<HTMLElement | null>(null)
const openMenuSessionId = ref<string | null>(null)

function closeCompactMenu(event: Event) {
  const details = (event.currentTarget as HTMLElement | null)?.closest('details')
  if (details instanceof HTMLDetailsElement) {
    details.open = false
  }
  openMenuSessionId.value = null
}

function closeAllCompactMenus() {
  const root = sessionListRef.value
  if (!root) {
    openMenuSessionId.value = null
    return
  }

  root.querySelectorAll<HTMLDetailsElement>('details[open]').forEach((details) => {
    details.open = false
  })
  openMenuSessionId.value = null
}

function closeOtherMenus(currentDetails: HTMLDetailsElement) {
  const root = sessionListRef.value
  if (!root) {
    return
  }

  root.querySelectorAll<HTMLDetailsElement>('details[open]').forEach((details) => {
    if (details !== currentDetails) {
      details.open = false
    }
  })
}

function handleMenuToggle(sessionId: string, event: Event) {
  const details = event.currentTarget as HTMLDetailsElement | null
  if (!details) {
    return
  }

  if (details.open) {
    closeOtherMenus(details)
    openMenuSessionId.value = sessionId
    return
  }

  if (openMenuSessionId.value === sessionId) {
    openMenuSessionId.value = null
  }
}

function handleDocumentMouseDown(event: MouseEvent) {
  const root = sessionListRef.value
  const target = event.target
  if (!(root && target instanceof Node)) {
    return
  }

  const clickedMenu = target instanceof Element
    ? target.closest('.session-item__menu')
    : null

  if (!clickedMenu || !root.contains(clickedMenu)) {
    closeAllCompactMenus()
  }
}

function handleDocumentKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    closeAllCompactMenus()
  }
}

function handleCompactAction(
  action: 'togglePin' | 'startEdit' | 'delete',
  session: Session,
  event: Event
) {
  event.stopPropagation()
  closeCompactMenu(event)

  if (action === 'togglePin') {
    emit('togglePin', session.id)
    return
  }

  if (action === 'startEdit') {
    emit('startEdit', session, event)
    return
  }

  emit('delete', session)
}

function getStatusBadgeClass(status: Session['status']) {
  return `session-item__status-text--${status}`
}

onMounted(() => {
  document.addEventListener('mousedown', handleDocumentMouseDown)
  document.addEventListener('keydown', handleDocumentKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', handleDocumentMouseDown)
  document.removeEventListener('keydown', handleDocumentKeydown)
})
</script>

<template>
  <div
    ref="sessionListRef"
    class="session-list"
  >
    <div
      v-for="session in sessions"
      :key="session.id"
      :class="[
        'session-item',
        {
          'session-item--active': session.id === currentSessionId,
          'session-item--selected': selectedSessionIds.includes(session.id),
          'session-item--pinned': session.pinned,
          'session-item--menu-open': openMenuSessionId === session.id
        }
      ]"
      @click="emit('select', session.id)"
    >
      <div class="session-item__content">
        <div class="session-item__main">
          <button
            class="session-item__selector"
            :class="{ 'session-item__selector--selected': selectedSessionIds.includes(session.id) }"
            :title="selectedSessionIds.includes(session.id) ? t('session.unselectSession') : t('session.selectSession')"
            :aria-label="selectedSessionIds.includes(session.id) ? t('session.unselectSession') : t('session.selectSession')"
            :aria-pressed="selectedSessionIds.includes(session.id)"
            @click.stop="emit('toggleSelect', session.id)"
          >
            <span
              class="session-item__selector-indicator"
              :class="{ 'session-item__selector-indicator--selected': selectedSessionIds.includes(session.id) }"
            >
              <EaIcon
                name="check"
                :size="10"
              />
            </span>
          </button>
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
            <span
              v-if="session.status !== 'idle'"
              :class="['session-item__status-text', getStatusBadgeClass(session.status)]"
            >
              <span
                class="session-item__status-dot"
                :class="getStatusBadgeClass(session.status)"
              />
              {{ getStatusText(session.status) }}
            </span>
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

      <details
        v-if="editingSessionId !== session.id"
        class="session-item__menu"
        @click.stop
        @toggle="handleMenuToggle(session.id, $event)"
      >
        <summary
          class="session-item__menu-trigger"
          @click.stop
        >
          <EaIcon
            name="ellipsis-vertical"
            :size="12"
          />
        </summary>
        <div class="session-item__menu-popover">
          <button
            class="session-item__menu-action"
            @click="handleCompactAction('togglePin', session, $event)"
          >
            <EaIcon
              :name="session.pinned ? 'pin-off' : 'pin'"
              :size="12"
            />
            <span>{{ session.pinned ? t('session.unpin') : t('session.pin') }}</span>
          </button>
          <button
            class="session-item__menu-action"
            @click="handleCompactAction('startEdit', session, $event)"
          >
            <EaIcon
              name="edit-2"
              :size="12"
            />
            <span>{{ t('common.edit') }}</span>
          </button>
          <button
            class="session-item__menu-action session-item__menu-action--danger"
            @click="handleCompactAction('delete', session, $event)"
          >
            <EaIcon
              name="x"
              :size="12"
            />
            <span>{{ t('common.delete') }}</span>
          </button>
        </div>
      </details>
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
  flex: 1 1 auto;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
  min-height: 0;
  width: 100%;
  min-width: 0;
  overflow-y: auto;
  box-sizing: border-box;
}

.session-item {
  position: relative;
  display: flex;
  container-type: inline-size;
  align-items: flex-start;
  padding: var(--spacing-3);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
  border-radius: var(--radius-md);
  background-color: var(--color-surface);
  border: 1px solid transparent;
}

.session-item--menu-open {
  z-index: 8;
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

.session-item--selected {
  background-color: color-mix(in srgb, var(--color-primary) 10%, var(--color-surface));
  border-color: color-mix(in srgb, var(--color-primary) 32%, transparent);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-primary) 24%, transparent);
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
  flex-wrap: nowrap;
}

.session-item__selector {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  appearance: none;
  border: none;
  border: 1.5px solid var(--color-border-strong, var(--color-border));
  border-radius: 999px;
  background: transparent;
  color: var(--color-primary);
  flex-shrink: 0;
  cursor: pointer;
  box-sizing: border-box;
  transition:
    border-color var(--transition-fast) var(--easing-default),
    background-color var(--transition-fast) var(--easing-default),
    box-shadow var(--transition-fast) var(--easing-default);
}

.session-item__selector:not(.session-item__selector--selected):hover {
  border-color: var(--color-primary);
  background-color: color-mix(in srgb, var(--color-primary) 10%, transparent);
}

.session-item__selector:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--color-primary) 28%, transparent);
  outline-offset: 2px;
}

.session-item__selector--selected {
  border-color: var(--color-primary);
  background-color: var(--color-primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 16%, transparent);
}

.session-item__selector--selected:hover {
  border-color: var(--color-primary);
  background-color: var(--color-primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 16%, transparent);
}

.session-item__selector-indicator {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  color: #fff;
  opacity: 0;
  transform: scale(0.72);
}

.session-item__selector-indicator--selected {
  opacity: 1;
  transform: scale(1);
}

.session-item__selector--selected .session-item__selector-indicator {
  opacity: 1;
  transform: scale(1);
}

.session-item__name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--sidebar-font-primary);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
}

.session-item--active .session-item__name,
.session-item:hover .session-item__name {
  color: var(--color-primary);
}

.session-item__time {
  flex-shrink: 0;
  font-size: var(--sidebar-font-meta);
  color: var(--color-text-tertiary);
  white-space: nowrap;
}

.session-item__status-text {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.session-item__status-dot {
  width: 6px;
  height: 6px;
  border-radius: 999px;
  background-color: currentColor;
  flex-shrink: 0;
}

.session-item__status-text--running {
  color: var(--color-primary);
  background-color: var(--color-primary-light);
}

.session-item__status-text--completed {
  color: var(--color-success);
  background-color: var(--color-success-light);
}

.session-item__status-text--error {
  color: var(--color-error);
  background-color: var(--color-error-light);
}

.session-item__status-text--paused {
  color: var(--color-warning);
  background-color: var(--color-warning-light);
}

.session-item__meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding-left: calc(18px + var(--spacing-2));
  min-width: 0;
  flex-wrap: nowrap;
  overflow: hidden;
}

.session-item__meta-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  min-width: 0;
  white-space: nowrap;
}

.session-item__meta-item:first-child {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--color-primary);
}

.session-item__meta-item--created {
  color: var(--color-text-quaternary);
}

.session-item__preview {
  padding-left: calc(18px + var(--spacing-2));
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.4;
}

.session-item__token-progress {
  padding-left: calc(18px + var(--spacing-2));
}

.session-item__token-progress :deep(.token-progress) {
  gap: 8px;
  padding: 4px 10px;
  border-radius: 12px;
  background: color-mix(in srgb, var(--color-surface) 94%, white);
  box-shadow: none;
}

.session-item__token-progress :deep(.token-progress__bar) {
  min-width: 88px;
  height: 5px;
}

.session-item__token-progress :deep(.token-progress__text) {
  min-width: 30px;
  font-size: 11px;
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

.session-item__menu {
  position: relative;
  display: none;
  margin-left: var(--spacing-1);
  flex-shrink: 0;
}

.session-item__menu[open] {
  z-index: 9;
}

.session-item__menu-trigger {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
  list-style: none;
  cursor: pointer;
  transition: background-color var(--transition-fast) var(--easing-default), color var(--transition-fast) var(--easing-default);
}

.session-item__menu-trigger::-webkit-details-marker {
  display: none;
}

.session-item__menu-trigger:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.session-item__menu-popover {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  z-index: 10;
  display: flex;
  min-width: 112px;
  flex-direction: column;
  gap: 4px;
  padding: 6px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-surface) 96%, white);
  box-shadow: 0 12px 24px rgba(15, 23, 42, 0.12);
}

.session-item__menu-action {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 6px 8px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
  cursor: pointer;
  white-space: nowrap;
}

.session-item__menu-action:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.session-item__menu-action--danger:hover {
  background-color: var(--color-error-light);
  color: var(--color-error);
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
  font-size: var(--sidebar-font-meta);
}

@container (max-width: 320px) {
  .session-item {
    padding: var(--spacing-2) var(--spacing-3);
  }

  .session-item__meta {
    gap: var(--spacing-2);
  }

  .session-item__meta-item--created {
    display: none;
  }

  .session-item__actions {
    display: none;
  }

  .session-item__menu {
    display: flex;
  }
}

@container (max-width: 280px) {
  .session-item__time {
    display: none;
  }

  .session-item__meta {
    gap: var(--spacing-1);
  }

  .session-item__meta-item:first-child,
  .session-item__meta-item--created {
    display: none;
  }

  .session-item__meta-item:not(:first-child) {
    flex-shrink: 0;
  }

  .session-item__preview {
    padding-left: calc(14px + var(--spacing-2));
  }

  .session-item__actions {
    margin-left: var(--spacing-1);
  }
}

/* 极窄屏优化 - 隐藏更多低优先级元信息 */
@container (max-width: 240px) {
  .session-item__selector {
    display: none;
  }

  .session-item__main,
  .session-item__content {
    gap: var(--spacing-1);
  }

  .session-item__preview {
    padding-left: 0;
  }

  .session-item__meta-item:nth-child(2),
  .session-item__meta-item--created {
    display: none;
  }
}
</style>
