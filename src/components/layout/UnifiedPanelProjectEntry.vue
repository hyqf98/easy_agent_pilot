<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { FileTree } from '@/components/fileTree'
import type { Project } from '@/stores/project'
import type { Session } from '@/stores/session'
import type { ProjectTabType } from '@/stores/layout'
import { EaIcon } from '@/components/common'
import UnifiedPanelSessionList from './UnifiedPanelSessionList.vue'

interface Props {
  project: Project
  isActive: boolean
  isExpanded: boolean
  currentTab: ProjectTabType
  sessionSortBy: 'updatedAt' | 'createdAt'
  sessions: Session[]
  currentSessionId: string | null
  editingSessionId: string | null
  editingSessionName: string
  importedTimeLabel: string
}

const props = defineProps<Props>()

const emit = defineEmits<{
  toggleProject: [project: Project]
  editProject: [project: Project]
  deleteProject: [project: Project]
  setTab: [projectId: string, tab: ProjectTabType]
  toggleSort: []
  addSession: [projectId: string]
  selectSession: [sessionId: string]
  togglePin: [sessionId: string]
  startEditSession: [session: Session, event: Event]
  saveEditSession: [session: Session]
  cancelEditSession: []
  deleteSession: [session: Session]
  deleteSessions: [sessions: Session[]]
  updateEditingName: [value: string]
  selectFile: [path: string, project: Project]
}>()

const { t } = useI18n()
const projectItemRef = ref<HTMLElement | null>(null)
const isCompactMenuOpen = ref(false)
const fileTreeActivated = ref(props.currentTab === 'files')
const selectedSessionIds = ref<string[]>([])
const selectedSessions = computed(() => props.sessions.filter(session => selectedSessionIds.value.includes(session.id)))
const selectedSessionCount = computed(() => selectedSessions.value.length)
const hasSelectedSessions = computed(() => selectedSessionCount.value > 0)
const allVisibleSessionsSelected = computed(() =>
  props.sessions.length > 0 && selectedSessionCount.value === props.sessions.length
)

function handleStartEditSession(session: Session, event: Event) {
  emit('startEditSession', session, event)
}

function toggleSessionSelection(sessionId: string) {
  selectedSessionIds.value = selectedSessionIds.value.includes(sessionId)
    ? selectedSessionIds.value.filter(id => id !== sessionId)
    : [...selectedSessionIds.value, sessionId]
}

function clearSelectedSessions() {
  selectedSessionIds.value = []
}

function selectAllVisibleSessions() {
  selectedSessionIds.value = props.sessions.map(session => session.id)
}

function handleBatchDeleteSessions() {
  if (!selectedSessions.value.length) {
    return
  }

  emit('deleteSessions', selectedSessions.value)
}

function closeCompactMenu(event: Event) {
  const details = (event.currentTarget as HTMLElement | null)?.closest('details')
  if (details instanceof HTMLDetailsElement) {
    details.open = false
  }
  isCompactMenuOpen.value = false
}

function closeProjectCompactMenu() {
  const root = projectItemRef.value
  if (!root) {
    isCompactMenuOpen.value = false
    return
  }

  root.querySelectorAll<HTMLDetailsElement>('details[open]').forEach((details) => {
    details.open = false
  })
  isCompactMenuOpen.value = false
}

function handleProjectMenuToggle(event: Event) {
  const details = event.currentTarget as HTMLDetailsElement | null
  if (!details) {
    return
  }

  isCompactMenuOpen.value = details.open
}

function handleDocumentMouseDown(event: MouseEvent) {
  const root = projectItemRef.value
  const target = event.target
  if (!(root && target instanceof Node)) {
    return
  }

  const clickedMenu = target instanceof Element
    ? target.closest('.project-item__menu')
    : null

  if (!clickedMenu || !root.contains(clickedMenu)) {
    closeProjectCompactMenu()
  }
}

function handleDocumentKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    closeProjectCompactMenu()
  }
}

function handleProjectCompactAction(action: 'edit' | 'delete', project: Project, event: Event) {
  event.stopPropagation()
  closeCompactMenu(event)

  if (action === 'edit') {
    emit('editProject', project)
    return
  }

  emit('deleteProject', project)
}

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', handleDocumentMouseDown)
  document.removeEventListener('keydown', handleDocumentKeydown)
})

watch(isCompactMenuOpen, (open) => {
  if (open) {
    document.addEventListener('mousedown', handleDocumentMouseDown)
    document.addEventListener('keydown', handleDocumentKeydown)
    return
  }

  document.removeEventListener('mousedown', handleDocumentMouseDown)
  document.removeEventListener('keydown', handleDocumentKeydown)
})

watch(
  () => props.currentTab,
  (tab) => {
    if (tab === 'files') {
      fileTreeActivated.value = true
    }
  }
)

watch(
  () => props.sessions,
  (sessions) => {
    const visibleIds = new Set(sessions.map(session => session.id))
    selectedSessionIds.value = selectedSessionIds.value.filter(sessionId => visibleIds.has(sessionId))
  },
  { deep: true }
)
</script>

<template>
  <div
    ref="projectItemRef"
    :class="[
      'project-item',
      {
        'project-item--active': isActive,
        'project-item--expanded': isExpanded,
        'project-item--menu-open': isCompactMenuOpen
      }
    ]"
    tabindex="0"
    role="listitem"
    :aria-selected="isActive"
    :aria-expanded="isExpanded"
    @click="emit('toggleProject', project)"
    @keydown.enter="emit('toggleProject', project)"
    @keydown.space.prevent="emit('toggleProject', project)"
  >
    <div class="project-item__arrow">
      <EaIcon
        name="chevron-right"
        :size="14"
        :class="{ 'project-item__arrow--expanded': isExpanded }"
      />
    </div>

    <div class="project-item__icon">
      <EaIcon
        name="folder"
        :size="18"
      />
    </div>

    <div class="project-item__info">
      <div class="project-item__header">
        <span class="project-item__name">{{ project.name }}</span>
      </div>
      <div class="project-item__meta">
        <span class="project-item__time">{{ importedTimeLabel }} {{ t('unified.imported') }}</span>
        <span
          class="project-item__session-count"
          :class="{ 'project-item__session-count--has': project.sessionCount && project.sessionCount > 0 }"
        >
          <EaIcon
            name="message-square"
            :size="10"
          />
          {{ t('unified.sessionCount', { count: project.sessionCount || 0 }) }}
        </span>
      </div>
    </div>

    <div class="project-item__actions">
      <button
        class="project-item__action-btn"
        :title="t('common.edit')"
        @click.stop="emit('editProject', project)"
      >
        <EaIcon
          name="edit-2"
          :size="12"
        />
      </button>
      <button
        class="project-item__action-btn project-item__action-btn--danger"
        :title="t('common.delete')"
        @click.stop="emit('deleteProject', project)"
      >
        <EaIcon
          name="x"
          :size="12"
        />
      </button>
    </div>

    <details
      class="project-item__menu"
      @click.stop
      @toggle="handleProjectMenuToggle"
    >
      <summary
        class="project-item__menu-trigger"
        @click.stop
      >
        <EaIcon
          name="ellipsis-vertical"
          :size="12"
        />
      </summary>
      <div class="project-item__menu-popover">
        <button
          class="project-item__menu-action"
          @click="handleProjectCompactAction('edit', project, $event)"
        >
          <EaIcon
            name="edit-2"
            :size="12"
          />
          <span>{{ t('common.edit') }}</span>
        </button>
        <button
          class="project-item__menu-action project-item__menu-action--danger"
          @click="handleProjectCompactAction('delete', project, $event)"
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
    v-if="isExpanded"
    class="project-content"
  >
    <div class="project-tabs">
      <button
        :class="['tab-btn', { 'tab-btn--active': currentTab === 'sessions' }]"
        @click="emit('setTab', project.id, 'sessions')"
      >
        <EaIcon
          name="message-square"
          :size="12"
        />
        {{ t('unified.sessions') }}
      </button>
      <button
        :class="['tab-btn', { 'tab-btn--active': currentTab === 'files' }]"
        @click="emit('setTab', project.id, 'files')"
      >
        <EaIcon
          name="folder-open"
          :size="12"
        />
        {{ t('unified.files') }}
      </button>
      <button
        v-if="currentTab === 'sessions'"
        class="tab-action-btn"
        :title="sessionSortBy === 'updatedAt' ? t('unified.sortByUpdated') : t('unified.sortByCreated')"
        @click="emit('toggleSort')"
      >
        <EaIcon
          :name="sessionSortBy === 'updatedAt' ? 'clock' : 'calendar'"
          :size="12"
        />
      </button>
      <button
        v-if="currentTab === 'sessions'"
        class="tab-action-btn"
        :title="t('session.createSession')"
        @click.stop="emit('addSession', project.id)"
      >
        <EaIcon
          name="plus"
          :size="12"
        />
      </button>
    </div>

    <div
      v-show="currentTab === 'sessions'"
      class="tab-content tab-content--sessions"
    >
      <div
        v-if="sessions.length > 0"
        class="session-batch-bar"
      >
        <span
          v-if="hasSelectedSessions"
          class="session-batch-bar__label"
        >
          {{ t('session.selectedCount', { count: selectedSessionCount }) }}
        </span>
        <div
          v-if="hasSelectedSessions"
          class="session-batch-bar__actions"
        >
          <button
            class="session-batch-bar__button"
            @click="allVisibleSessionsSelected ? clearSelectedSessions() : selectAllVisibleSessions()"
          >
            {{ allVisibleSessionsSelected ? t('common.clearSelection') : t('session.selectAllVisible') }}
          </button>
          <button
            class="session-batch-bar__button"
            :disabled="!hasSelectedSessions"
            @click="clearSelectedSessions"
          >
            {{ t('common.clearSelection') }}
          </button>
          <button
            class="session-batch-bar__button session-batch-bar__button--danger"
            :disabled="!hasSelectedSessions"
            @click="handleBatchDeleteSessions"
          >
            {{ t('common.batchDelete') }}
          </button>
        </div>
      </div>

      <UnifiedPanelSessionList
        :sessions="sessions"
        :current-session-id="currentSessionId"
        :editing-session-id="editingSessionId"
        :editing-session-name="editingSessionName"
        :selected-session-ids="selectedSessionIds"
        @select="emit('selectSession', $event)"
        @toggle-select="toggleSessionSelection"
        @toggle-pin="emit('togglePin', $event)"
        @start-edit="handleStartEditSession"
        @save-edit="emit('saveEditSession', $event)"
        @cancel-edit="emit('cancelEditSession')"
        @delete="emit('deleteSession', $event)"
        @update-editing-name="emit('updateEditingName', $event)"
      />
    </div>

    <div
      v-if="fileTreeActivated"
      v-show="currentTab === 'files'"
      class="tab-content tab-content--files"
    >
      <FileTree
        :project-id="project.id"
        :project-path="project.path"
        class="file-tree__content"
        @file-select="emit('selectFile', $event, project)"
      />
    </div>
  </div>
</template>

<style scoped>
.project-item {
  display: flex;
  container-type: inline-size;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-3);
  position: relative;
  border: 1px solid transparent;
  border-radius: var(--radius-lg);
  outline: none;
  cursor: pointer;
  background-color: var(--color-surface);
  transition: all var(--transition-fast) var(--easing-default);
}

.project-item:hover {
  background-color: var(--color-surface-hover);
  border-color: var(--color-border);
}

.project-item:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: -2px;
}

.project-item--active {
  background-color: var(--color-primary-light);
  border-color: var(--color-primary);
}

[data-theme='dark'] .project-item--active {
  background-color: var(--color-active-bg);
  border-color: var(--color-active-border);
}

.project-item--expanded {
  background-color: var(--color-surface-hover);
  border-color: var(--color-primary);
}

.project-item--menu-open {
  z-index: 8;
}

[data-theme='dark'] .project-item--expanded {
  background-color: var(--color-surface-hover);
  border-color: var(--color-active-border);
}

.project-item__arrow {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  color: var(--color-text-tertiary);
  transition: transform var(--transition-fast) var(--easing-default);
}

.project-item__arrow--expanded {
  transform: rotate(90deg);
}

.project-item__icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  border-radius: var(--radius-md);
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

.project-item--active .project-item__icon,
.project-item--expanded .project-item__icon {
  background-color: var(--color-primary);
  color: #fff;
}

.project-item__info {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
}

.project-item__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  min-width: 0;
}

.project-item__name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  color: var(--color-text-primary);
  font-size: var(--sidebar-font-primary);
  font-weight: var(--font-weight-medium);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-item__meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  color: var(--color-text-tertiary);
  font-size: var(--sidebar-font-meta);
  white-space: nowrap;
  min-width: 0;
  flex-wrap: nowrap;
  overflow: hidden;
}

.project-item__time {
  display: flex;
  align-items: center;
  gap: 2px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.project-item__session-count {
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 1px 6px;
  border-radius: var(--radius-full);
  background-color: var(--color-bg-tertiary);
  flex-shrink: 0;
}

.project-item__session-count--has {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

.project-item__actions {
  display: flex;
  align-items: center;
  gap: 2px;
  opacity: 0;
  transition: opacity var(--transition-fast) var(--easing-default);
}

.project-item:hover .project-item__actions {
  opacity: 1;
}

.project-item__action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
  background: transparent;
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.project-item__action-btn:hover {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

.project-item__action-btn--danger:hover {
  background-color: var(--color-error-light);
  color: var(--color-error);
}

.project-item__menu {
  position: relative;
  display: none;
  flex-shrink: 0;
}

.project-item__menu[open] {
  z-index: 9;
}

.project-item__menu-trigger {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
  list-style: none;
  cursor: pointer;
  transition: background-color var(--transition-fast) var(--easing-default), color var(--transition-fast) var(--easing-default);
}

.project-item__menu-trigger::-webkit-details-marker {
  display: none;
}

.project-item__menu-trigger:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.project-item__menu-popover {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  z-index: 10;
  display: flex;
  min-width: 104px;
  flex-direction: column;
  gap: 4px;
  padding: 6px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-surface) 96%, white);
  box-shadow: 0 12px 24px rgba(15, 23, 42, 0.12);
}

.project-item__menu-action {
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

.project-item__menu-action:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.project-item__menu-action--danger:hover {
  background-color: var(--color-error-light);
  color: var(--color-error);
}

.project-content {
  display: flex;
  flex: 1 1 auto;
  min-width: 0;
  min-height: 220px;
  flex-direction: column;
  box-sizing: border-box;
  max-width: calc(100% - var(--spacing-4));
  margin-bottom: var(--spacing-1);
  margin-left: var(--spacing-4);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background-color: var(--color-surface);
}

.project-tabs {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  width: 100%;
  min-width: 0;
  padding: var(--spacing-1);
  box-sizing: border-box;
  border-bottom: 1px solid var(--color-border);
}

.tab-btn {
  display: flex;
  flex: 1;
  min-width: 0;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-1);
  padding: var(--spacing-1) var(--spacing-2);
  border: none;
  border-radius: var(--radius-sm);
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
  background: transparent;
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.tab-btn:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.tab-btn--active {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

.tab-btn--active:hover {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

[data-theme='dark'] .tab-btn--active {
  background-color: var(--color-active-bg);
  color: var(--color-active-text);
}

[data-theme='dark'] .tab-btn--active:hover {
  background-color: var(--color-active-bg-hover);
  color: var(--color-active-text);
}

.tab-action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  margin-left: auto;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
  background: transparent;
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.tab-action-btn:hover {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

.tab-content {
  display: block;
  flex: 1 1 auto;
  width: 100%;
  height: 0;
  min-width: 0;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  box-sizing: border-box;
  scrollbar-width: thin;
  scrollbar-color: var(--scrollbar-thumb, var(--color-border)) var(--scrollbar-track, transparent);
}

.tab-content--sessions {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.tab-content::-webkit-scrollbar {
  width: var(--scrollbar-size, 6px);
}

.tab-content::-webkit-scrollbar-track {
  background: var(--scrollbar-track, transparent);
}

.tab-content::-webkit-scrollbar-thumb {
  background-color: var(--scrollbar-thumb, var(--color-border));
  border-radius: var(--radius-full, 9999px);
  border: 1px solid transparent;
  background-clip: padding-box;
}

.tab-content::-webkit-scrollbar-thumb:hover {
  background-color: var(--scrollbar-thumb-hover, var(--color-text-tertiary));
}

.tab-content--files {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  padding: var(--spacing-1);
}

.session-batch-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
  border-bottom: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-surface) 96%, white) 0%,
      var(--color-surface) 100%
    );
  flex-wrap: wrap;
  flex-shrink: 0;
}

.session-batch-bar__label {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  font-weight: var(--font-weight-medium);
}

.session-batch-bar__actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  flex-wrap: wrap;
  margin-left: auto;
}

.session-batch-bar__button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 26px;
  padding: 0 var(--spacing-2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background-color: var(--color-surface);
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.session-batch-bar__button:hover:not(:disabled) {
  border-color: var(--color-primary);
  background-color: color-mix(in srgb, var(--color-primary) 8%, var(--color-surface));
  color: var(--color-primary);
}

.session-batch-bar__button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.session-batch-bar__button--danger:hover:not(:disabled) {
  border-color: var(--color-error);
  color: var(--color-error);
  background-color: var(--color-error-light);
}

.file-tree__loading {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
}

.file-tree__content {
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.file-tree__n-tree {
  --n-font-size: var(--font-size-xs) !important;
  --n-text-color: var(--color-text-secondary) !important;
  --n-node-text-color: var(--color-text-secondary) !important;
  --n-node-text-color-hover: var(--color-text-primary) !important;
  --n-node-text-color-active: var(--color-primary) !important;
  --n-node-text-color-selected: var(--color-primary) !important;
  --n-node-color-hover: var(--color-surface-hover) !important;
  --n-node-color-active: var(--color-primary-light) !important;
  --n-node-color-selected: var(--color-primary-light) !important;
  --n-arrow-color: var(--color-text-tertiary) !important;
  --n-line-color: var(--color-border) !important;
  width: 100%;
  height: 100%;
  min-width: 0;
  overflow: auto;
  padding: var(--spacing-1) 0;
  box-sizing: border-box;
}

.file-tree__n-tree :deep(.n-tree-node) {
  padding: 2px 0;
}

.file-tree__n-tree :deep(.n-tree-node-content) {
  padding: 5px 10px !important;
  border-radius: var(--radius-sm);
}

.file-tree__n-tree :deep(.n-tree-node-wrapper) {
  padding: 0 4px;
}

.file-tree__n-tree :deep(.n-tree-switcher) {
  width: 16px !important;
  height: 16px !important;
}

.file-tree__n-tree :deep(.n-tree-node-wrapper--pending) {
  opacity: 0.6;
}

.file-tree__n-tree :deep(.file-tree-node__content) {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  width: 100%;
  min-width: 0;
}

.file-tree__n-tree :deep(.file-tree-node__icon) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.file-tree__n-tree :deep(.file-tree-node__name) {
  display: flex;
  flex: 1;
  align-items: center;
  overflow: hidden;
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 窄屏优化 - 隐藏时间标签 */
@container (max-width: 320px) {
  .project-item {
    padding: var(--spacing-2) var(--spacing-3);
  }

  .project-item__time {
    display: none;
  }

  .project-item__actions {
    display: none;
  }

  .project-item__menu {
    display: flex;
  }

  .project-item__meta {
    gap: var(--spacing-1);
  }
}

@container (max-width: 280px) {
  .project-item__actions {
    gap: 0;
  }

  .project-item__action-btn {
    width: 22px;
    height: 22px;
  }

  .project-item__session-count {
    padding: 1px 5px;
  }
}

@container (max-width: 240px) {
  .project-item__arrow {
    width: 16px;
    height: 16px;
  }

  .project-item__icon {
    width: 32px;
    height: 32px;
  }

  .project-item__name {
    font-size: 12px;
  }

  .project-item__meta {
    display: none;
  }
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.animate-spin {
  animation: spin 1s linear infinite;
}
</style>
