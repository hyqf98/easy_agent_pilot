<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import type { Session } from '@/stores/session'
import { useSessionView } from '@/composables'

interface SessionActionItem {
  key: string
  title: string
  icon: string
  danger?: boolean
  warning?: boolean
}

const props = defineProps<{
  session: Session
  active: boolean
  editingSessionId: string | null
  editingSessionName: string
  searchQuery?: string
  selected?: boolean
  actions: SessionActionItem[]
}>()

const emit = defineEmits<{
  select: [id: string]
  saveName: [session: Session]
  cancelEdit: []
  updateName: [value: string]
  toggleSelect: [id: string]
  action: [key: string, session: Session]
}>()

const { t } = useI18n()
const {
  getStatusText,
  formatRelativeTime,
  formatSessionCreatedAt
} = useSessionView()

const isEditing = computed(() => props.editingSessionId === props.session.id)

function getStatusBadgeClass(status: Session['status']) {
  return `session-item__status-text--${status}`
}

interface HighlightSegment {
  text: string
  matched: boolean
}

function buildHighlightSegments(source: string, query?: string): HighlightSegment[] {
  const normalizedQuery = query?.trim()
  if (!source || !normalizedQuery) {
    return [{ text: source, matched: false }]
  }

  const lowerSource = source.toLowerCase()
  const lowerQuery = normalizedQuery.toLowerCase()
  const segments: HighlightSegment[] = []
  let cursor = 0

  while (cursor < source.length) {
    const index = lowerSource.indexOf(lowerQuery, cursor)
    if (index === -1) {
      segments.push({ text: source.slice(cursor), matched: false })
      break
    }

    if (index > cursor) {
      segments.push({ text: source.slice(cursor, index), matched: false })
    }

    segments.push({
      text: source.slice(index, index + normalizedQuery.length),
      matched: true
    })
    cursor = index + normalizedQuery.length
  }

  return segments.length > 0 ? segments : [{ text: source, matched: false }]
}

const sessionNameSegments = computed(() => buildHighlightSegments(props.session.name, props.searchQuery))
const lastMessageSegments = computed(() => buildHighlightSegments(props.session.lastMessage ?? '', props.searchQuery))
</script>

<template>
  <div
    :class="['session-item', { 'session-item--active': active, 'session-item--selected': selected }]"
    tabindex="0"
    role="listitem"
    :aria-selected="active"
    @click="emit('select', session.id)"
    @keydown.enter="emit('select', session.id)"
    @keydown.space.prevent="emit('select', session.id)"
  >
    <div class="session-item__header">
      <button
        class="session-item__selector"
        :class="{ 'session-item__selector--selected': selected }"
        :title="selected ? t('session.unselectSession') : t('session.selectSession')"
        :aria-label="selected ? t('session.unselectSession') : t('session.selectSession')"
        :aria-pressed="selected"
        @click.stop="emit('toggleSelect', session.id)"
      >
        <EaIcon
          v-if="selected"
          name="check"
          :size="12"
        />
      </button>
      <input
        v-if="isEditing"
        :value="editingSessionName"
        type="text"
        class="session-item__name-input"
        :placeholder="t('session.enterSessionName')"
        @click.stop
        @input="emit('updateName', ($event.target as HTMLInputElement).value)"
        @keydown.enter="emit('saveName', session)"
        @keydown.escape="emit('cancelEdit')"
        @blur="emit('saveName', session)"
      >
      <template v-else>
        <span class="session-item__name">
          <template
            v-for="(segment, index) in sessionNameSegments"
            :key="`${session.id}-name-${index}`"
          >
            <mark
              v-if="segment.matched"
              class="session-item__highlight"
            >{{ segment.text }}</mark>
            <span v-else>{{ segment.text }}</span>
          </template>
        </span>
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
        <button
          v-if="session.pinned"
          class="session-item__pin session-item__pin--active"
          :title="t('session.unpin')"
          @click.stop="emit('action', 'pin', session)"
        >
          <EaIcon
            name="pin"
            :size="12"
          />
        </button>
      </template>
    </div>

    <div class="session-item__meta">
      <div class="session-item__meta-row">
        <span class="session-item__time">
          <EaIcon
            name="clock"
            :size="11"
          />
          {{ formatRelativeTime(session.updatedAt) }}
        </span>
        <span
          v-if="session.messageCount"
          class="session-item__count"
        >
          <EaIcon
            name="message-square"
            :size="11"
          />
          {{ session.messageCount }} 条消息
        </span>
        <span
          v-if="session.agentType"
          class="session-item__agent-type"
        >
          <EaIcon
            name="bot"
            :size="11"
          />
          {{ session.agentType }}
        </span>
      </div>
      <div class="session-item__meta-row session-item__meta-row--secondary">
        <span class="session-item__created">
          <EaIcon
            name="calendar"
            :size="11"
          />
          创建于 {{ formatSessionCreatedAt(session.createdAt) }}
        </span>
      </div>
    </div>

    <div
      v-if="session.lastMessage"
      class="session-item__preview"
    >
      <template
        v-for="(segment, index) in lastMessageSegments"
        :key="`${session.id}-preview-${index}`"
      >
        <mark
          v-if="segment.matched"
          class="session-item__highlight"
        >{{ segment.text }}</mark>
        <span v-else>{{ segment.text }}</span>
      </template>
    </div>

    <div class="session-item__actions">
      <button
        v-for="action in actions"
        :key="action.key"
        class="session-item__action"
        :class="{
          'session-item__action--danger': action.danger,
          'session-item__action--warning': action.warning
        }"
        :title="action.title"
        @click.stop="emit('action', action.key, session)"
      >
        <EaIcon
          :name="action.icon"
          :size="14"
        />
      </button>
    </div>
  </div>
</template>

<style scoped>
.session-item {
  display: flex;
  flex-direction: column;
  padding: var(--spacing-4);
  border-radius: var(--radius-lg);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
  position: relative;
  outline: none;
  background-color: var(--color-surface);
  border: 1px solid transparent;
  margin-bottom: var(--spacing-3);
}

.session-item:hover {
  background-color: var(--color-bg-tertiary);
  border-color: var(--color-border);
}

.session-item:focus-visible {
  background-color: var(--color-bg-tertiary);
  outline: 2px solid var(--color-primary);
  outline-offset: -2px;
}

.session-item--active {
  background-color: var(--color-bg-tertiary);
  border-color: var(--color-primary);
}

.session-item--active:hover {
  background-color: var(--color-bg-tertiary);
  border-color: var(--color-primary);
}

.session-item--selected {
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-primary) 28%, transparent);
}

[data-theme='dark'] .session-item--active {
  background-color: var(--color-active-bg);
  border-color: var(--color-active-border);
}

[data-theme='dark'] .session-item--active:hover {
  background-color: var(--color-active-bg-hover);
  border-color: var(--color-active-border);
}

.session-item__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
}

.session-item__selector {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 999px;
  border: 1.5px solid var(--color-border-strong, var(--color-border));
  background-color: transparent;
  color: var(--color-surface);
  flex-shrink: 0;
  transition: all var(--transition-fast) var(--easing-default);
  outline: none;
}

.session-item__selector:hover {
  border-color: var(--color-primary);
  background-color: color-mix(in srgb, var(--color-primary) 10%, transparent);
}

.session-item__selector:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;
}

.session-item__selector--selected {
  border-color: var(--color-primary);
  background-color: var(--color-primary);
  color: var(--color-surface);
}

.session-item__name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
  line-height: 1.4;
}

.session-item__name-input {
  flex: 1;
  min-width: 0;
  padding: 2px 6px;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
  background-color: var(--color-bg-tertiary);
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-sm);
  outline: none;
}

.session-item__name-input::placeholder {
  color: var(--color-text-tertiary);
}

.session-item__pin {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  color: var(--color-text-tertiary);
}

.session-item__pin--active {
  color: var(--color-warning);
}

.session-item__status-text {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  flex-shrink: 0;
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
  flex-direction: column;
  gap: var(--spacing-2);
  margin-top: var(--spacing-3);
  padding-left: calc(16px + var(--spacing-3));
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.session-item__meta-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-4);
  flex-wrap: wrap;
}

.session-item__meta-row--secondary {
  color: var(--color-text-tertiary);
  opacity: 0.8;
}

.session-item__time,
.session-item__count,
.session-item__agent-type,
.session-item__created {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
}

.session-item__agent-type {
  color: var(--color-primary);
  font-weight: var(--font-weight-medium);
}

.session-item__preview {
  margin-top: var(--spacing-3);
  padding: var(--spacing-2) var(--spacing-3);
  padding-left: calc(16px + var(--spacing-3));
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  background-color: var(--color-bg-tertiary);
  border-radius: var(--radius-sm);
  line-height: 1.5;
}

.session-item__highlight {
  padding: 0 2px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--color-warning) 24%, transparent);
  color: inherit;
}

.session-item__actions {
  position: absolute;
  right: var(--spacing-2);
  top: 50%;
  transform: translateY(-50%);
  display: none;
  align-items: center;
  gap: var(--spacing-1);
  background-color: var(--color-surface);
  padding: var(--spacing-1);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
}

.session-item:hover .session-item__actions {
  display: flex;
}

.session-item__action {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  color: var(--color-text-secondary);
  transition: all var(--transition-fast) var(--easing-default);
  outline: none;
}

.session-item__action:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.session-item__action:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: -2px;
  background-color: var(--color-surface-hover);
}

.session-item__action--danger:hover {
  background-color: var(--color-error-light);
  color: var(--color-error);
}

.session-item__action--warning:hover {
  background-color: var(--color-warning-light);
  color: var(--color-warning);
}
</style>
