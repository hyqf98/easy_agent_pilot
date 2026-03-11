<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { NTree, type TreeOption } from 'naive-ui'
import type { Project } from '@/stores/project'
import type { Session } from '@/stores/session'
import type { ProjectTabType } from '@/stores/layout'
import { EaIcon, EaSkeleton } from '@/components/common'
import UnifiedPanelSessionList from './UnifiedPanelSessionList.vue'

interface TreeRenderProps {
  option: TreeOption
  checked: boolean
  selected: boolean
}

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
  isFileTreeLoading: boolean
  treeData: TreeOption[]
  expandedKeys: string[]
  importedTimeLabel: string
  renderTreeLabel: (props: TreeRenderProps) => ReturnType<typeof import('vue').h>
}

defineProps<Props>()

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
  updateEditingName: [value: string]
  expandTree: [keys: string[], projectId: string]
  selectFile: [keys: Array<string | number>, options: Array<TreeOption | null>, project: Project]
}>()

const { t } = useI18n()

function handleStartEditSession(session: Session, event: Event) {
  emit('startEditSession', session, event)
}
</script>

<template>
  <div
    :class="[
      'project-item',
      {
        'project-item--active': isActive,
        'project-item--expanded': isExpanded
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
          :name="isFileTreeLoading ? 'loader' : 'chevron-right'"
          :size="14"
          :class="{
            'project-item__arrow--expanded': isExpanded,
            'animate-spin': isFileTreeLoading
          }"
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
      v-if="currentTab === 'sessions'"
      class="tab-content"
    >
      <UnifiedPanelSessionList
        :sessions="sessions"
        :current-session-id="currentSessionId"
        :editing-session-id="editingSessionId"
        :editing-session-name="editingSessionName"
        @select="emit('selectSession', $event)"
        @toggle-pin="emit('togglePin', $event)"
        @start-edit="handleStartEditSession"
        @save-edit="emit('saveEditSession', $event)"
        @cancel-edit="emit('cancelEditSession')"
        @delete="emit('deleteSession', $event)"
        @update-editing-name="emit('updateEditingName', $event)"
      />
    </div>

    <div
      v-else-if="currentTab === 'files'"
      class="tab-content tab-content--files"
    >
      <div
        v-if="isFileTreeLoading"
        class="file-tree__loading"
      >
        <EaSkeleton
          variant="text"
          height="14px"
          width="80%"
          animation="wave"
        />
        <EaSkeleton
          variant="text"
          height="14px"
          width="60%"
          animation="wave"
        />
      </div>
      <n-tree
        v-else
        :data="treeData"
        :expanded-keys="expandedKeys"
        :render-label="renderTreeLabel"
        block-line
        expand-on-click
        selectable
        class="file-tree__n-tree"
        @update:expanded-keys="(keys: string[]) => emit('expandTree', keys, project.id)"
        @update:selected-keys="(keys: string[], options: Array<TreeOption | null>) => emit('selectFile', keys, options, project)"
      />
    </div>
  </div>
</template>

<style scoped>
.project-item {
  display: flex;
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
}

.project-item__name {
  overflow: hidden;
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-item__meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

.project-item__time {
  display: flex;
  align-items: center;
  gap: 2px;
}

.project-item__session-count {
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 1px 6px;
  border-radius: var(--radius-full);
  background-color: var(--color-bg-tertiary);
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
  scrollbar-color: var(--color-border) transparent;
}

.tab-content::-webkit-scrollbar {
  width: 6px;
}

.tab-content::-webkit-scrollbar-track {
  background: transparent;
}

.tab-content::-webkit-scrollbar-thumb {
  background-color: var(--color-border);
  border-radius: 3px;
}

.tab-content::-webkit-scrollbar-thumb:hover {
  background-color: var(--color-text-tertiary);
}

.tab-content--files {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  padding: var(--spacing-1);
}

.file-tree__loading {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
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
