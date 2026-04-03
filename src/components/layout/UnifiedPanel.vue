<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useProjectStore, type Project } from '@/stores/project'
import { useSessionStore, type Session } from '@/stores/session'
import { useLayoutStore, type ProjectTabType } from '@/stores/layout'
import { useUIStore } from '@/stores/ui'
import { useAgentStore } from '@/stores/agent'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import { useSessionView } from '@/composables'
import { useFileEditorStore } from '@/modules/file-editor'
import { EaIcon, EaButton, EaSkeleton } from '@/components/common'
import { ProjectCreateModal } from '@/components/project'
import UnifiedPanelConfirmDialog from './UnifiedPanelConfirmDialog.vue'
import UnifiedPanelProjectEntry from './UnifiedPanelProjectEntry.vue'
import { resolveExpertRuntime } from '@/services/agentTeams/runtime'

const { t } = useI18n()

export interface UnifiedPanelProps {
  collapsed?: boolean
  showHeaderToggle?: boolean
}

defineProps<UnifiedPanelProps>()

defineEmits<{
  toggle: []
}>()

const projectStore = useProjectStore()
const sessionStore = useSessionStore()
const layoutStore = useLayoutStore()
const uiStore = useUIStore()
const agentStore = useAgentStore()
const agentTeamsStore = useAgentTeamsStore()
const fileEditorStore = useFileEditorStore()
const {
  openSessionTarget,
} = useSessionView()

// 项目相关状态
const editingProject = ref<Project | null>(null)
const showDeleteConfirm = ref(false)
const deletingProject = ref<Project | null>(null)

// 会话相关状态
const showDeleteSessionConfirm = ref(false)
const deletingSession = ref<Session | null>(null)

// 编辑会话名称状态
const editingSessionId = ref<string | null>(null)
const editingSessionName = ref('')

// 获取项目当前 Tab
const getProjectTab = (projectId: string): ProjectTabType => {
  return layoutStore.getProjectTab(projectId)
}

// 设置项目当前 Tab
const setProjectTab = async (projectId: string, tab: ProjectTabType) => {
  layoutStore.setProjectTab(projectId, tab)
  if (tab === 'sessions') {
    await sessionStore.loadSessions(projectId)
  }
}

// 按项目筛选的会话列表
const getSessionsByProject = (projectId: string) => {
  return sessionStore.sessionsByProject(projectId, layoutStore.sessionSortBy)
}

// 切换排序方式
const toggleSessionSort = () => {
  const newSortBy = layoutStore.sessionSortBy === 'updatedAt' ? 'createdAt' : 'updatedAt'
  layoutStore.setSessionSortBy(newSortBy)
}

// 项目是否有展开的会话

// 格式化导入时间
const formatImportTime = (dateStr: string): string => {
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))

  if (days === 0) return t('unified.today')
  if (days === 1) return t('unified.yesterday')
  if (days < 7) return t('unified.daysAgo', { days })
  if (days < 30) return t('unified.weeksAgo', { weeks: Math.floor(days / 7) })
  if (days < 365) return t('unified.monthsAgo', { months: Math.floor(days / 30) })
  return t('unified.yearsAgo', { years: Math.floor(days / 365) })
}

// 点击项目卡片切换展开/收起
const handleProjectCardClick = async (project: Project) => {
  projectStore.toggleProjectExpand(project.id)

  if (projectStore.isProjectExpanded(project.id) && getProjectTab(project.id) === 'sessions') {
    await sessionStore.loadSessions(project.id)
  }
}

// 生命周期
onMounted(async () => {
  await projectStore.loadProjects()
  document.addEventListener('keydown', handleModalKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleModalKeydown)
})

// ESC 键关闭模态框
const handleModalKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    if (showDeleteSessionConfirm.value) {
      closeDeleteSessionConfirm()
    } else if (showDeleteConfirm.value) {
      closeDeleteProjectConfirm()
    } else if (uiStore.projectCreateModalVisible) {
      uiStore.closeProjectCreateModal()
    } else if (uiStore.sessionCreateModalVisible) {
      uiStore.closeSessionCreateModal()
    }
  }
}

// ========== 项目操作 ==========
const handleRefresh = async () => {
  await projectStore.loadProjects()

  const expandedProjectIds = Array.from(projectStore.expandedProjects)
  const expandedProjects = projectStore.projects.filter(project => expandedProjectIds.includes(project.id))
  await Promise.all(expandedProjects
    .filter(project => getProjectTab(project.id) === 'sessions')
    .map(project => sessionStore.loadSessions(project.id, { force: true })))
}

const handleAddProject = () => {
  editingProject.value = null
  uiStore.openProjectCreateModal()
}

const handleEditProject = (project: Project) => {
  editingProject.value = project
  uiStore.openProjectCreateModal()
}

const handleCreateProject = async (data: { name: string; path: string; description?: string; memoryLibraryIds: string[] }) => {
  if (editingProject.value) {
    await projectStore.updateProject(editingProject.value.id, data)
    editingProject.value = null
  } else {
    await projectStore.createProject(data)
  }
  uiStore.closeProjectCreateModal()
}

const handleDeleteProject = (project: Project) => {
  deletingProject.value = project
  showDeleteConfirm.value = true
}

const closeDeleteProjectConfirm = () => {
  showDeleteConfirm.value = false
  deletingProject.value = null
}

const confirmDeleteProject = () => {
  if (deletingProject.value) {
    projectStore.deleteProject(deletingProject.value.id)
  }
  closeDeleteProjectConfirm()
}

// ========== 会话操作 ==========
const handleAddSession = async (projectId: string) => {
  try {
    projectStore.setCurrentProject(projectId)
    await Promise.all([
      agentStore.loadAgents(),
      agentTeamsStore.loadExperts(true)
    ])
    const expert = agentTeamsStore.builtinGeneralExpert || agentTeamsStore.enabledExperts[0] || null
    const runtime = resolveExpertRuntime(expert, agentStore.agents)
    const newSession = await sessionStore.createSession({
      projectId,
      name: t('session.unnamedSession'),
      expertId: expert?.id,
      agentId: runtime?.agent.id,
      agentType: runtime?.agent.provider || runtime?.agent.type || 'claude',
      status: 'idle'
    })
    projectStore.incrementSessionCount(projectId)
    uiStore.setAppMode('chat')
    uiStore.setMainContentMode('chat')
    await sessionStore.openSession(newSession.id)
  } catch (error) {
    console.error('[UnifiedPanel] 创建会话失败:', error)
  }
}

const handleSelectSession = async (id: string) => {
  await openSessionTarget(id, {
    onBeforeOpen: () => {
      uiStore.setMainContentMode('chat')
    }
  })
}

const handleTogglePin = (id: string) => {
  sessionStore.togglePin(id)
}

const handleDeleteSession = (session: Session) => {
  deletingSession.value = session
  showDeleteSessionConfirm.value = true
}

const closeDeleteSessionConfirm = () => {
  showDeleteSessionConfirm.value = false
  deletingSession.value = null
}

const confirmDeleteSession = () => {
  if (deletingSession.value) {
    sessionStore.deleteSession(deletingSession.value.id)
    if (projectStore.currentProjectId) {
      projectStore.decrementSessionCount(projectStore.currentProjectId)
    }
  }
  closeDeleteSessionConfirm()
}

// 编辑会话名称
const startEditSessionName = (session: Session, event: Event) => {
  event.stopPropagation()
  editingSessionId.value = session.id
  editingSessionName.value = session.name
}

const cancelEditSessionName = () => {
  editingSessionId.value = null
  editingSessionName.value = ''
}

const saveSessionName = async (session: Session) => {
  if (editingSessionName.value.trim() && editingSessionName.value !== session.name) {
    await sessionStore.updateSession(session.id, { name: editingSessionName.value.trim() })
  }
  cancelEditSessionName()
}

const handleFileSelect = async (selectedPath: string, project: Project) => {
  projectStore.setCurrentProject(project.id)

  await fileEditorStore.openFile({
    projectId: project.id,
    projectPath: project.path,
    filePath: selectedPath
  })
}
</script>

<template>
  <div :class="['unified-panel', { 'unified-panel--collapsed': collapsed }]">
    <!-- 面板头部 -->
    <div
      v-if="!collapsed"
      class="unified-panel__header"
    >
      <div class="unified-panel__header-title">
        <EaIcon
          name="layout-grid"
          :size="16"
        />
        <span>{{ t('panel.workspace') }}</span>
      </div>
      <div class="unified-panel__header-actions">
        <button
          class="header-action-btn"
          :title="t('common.refresh')"
          @click="handleRefresh"
        >
          <EaIcon
            name="refresh-cw"
            :size="14"
          />
        </button>
        <button
          class="header-action-btn"
          :title="t('project.createProject')"
          @click="handleAddProject"
        >
          <EaIcon
            name="plus"
            :size="14"
          />
        </button>
        <button
          v-if="showHeaderToggle"
          class="header-action-btn"
          :title="t('common.close')"
          @click="$emit('toggle')"
        >
          <EaIcon
            name="x"
            :size="14"
          />
        </button>
      </div>
    </div>

    <div
      v-if="!collapsed"
      class="unified-panel__content"
    >
      <!-- 加载状态 -->
      <div
        v-if="projectStore.isLoading"
        class="project-loading"
      >
        <div
          v-for="i in 3"
          :key="i"
          class="project-skeleton"
        >
          <EaSkeleton
            variant="circle"
            height="16px"
            width="16px"
            animation="wave"
          />
          <EaSkeleton
            variant="text"
            height="14px"
            :width="`${60 + Math.random() * 30}%`"
            animation="wave"
          />
        </div>
      </div>

      <!-- 错误状态 -->
      <div
        v-else-if="projectStore.loadError"
        class="project-error"
      >
        <EaIcon
          name="alert-circle"
          :size="32"
          class="project-error__icon"
        />
        <p class="project-error__text">
          {{ t('common.loadFailed') }}
        </p>
        <p class="project-error__detail">
          {{ projectStore.loadError }}
        </p>
        <EaButton
          type="primary"
          size="small"
          @click="handleRefresh"
        >
          <EaIcon
            name="refresh-cw"
            :size="14"
          />
          {{ t('common.retry') }}
        </EaButton>
      </div>

      <!-- 空状态 -->
      <div
        v-else-if="projectStore.projects.length === 0"
        class="project-empty"
      >
        <div class="project-empty__illustration">
          <EaIcon
            name="folder-plus"
            :size="48"
            class="project-empty__icon"
          />
        </div>
        <p class="project-empty__title">
          {{ t('project.noProjects') }}
        </p>
        <p class="project-empty__hint">
          {{ t('project.noProjectsHint') }}
        </p>
        <EaButton
          type="primary"
          size="medium"
          class="project-empty__button"
          @click="handleAddProject"
        >
          <EaIcon
            name="plus"
            :size="16"
          />
          {{ t('project.createFirstProject') }}
        </EaButton>
      </div>

      <!-- 项目列表 -->
      <div
        v-else
        class="project-list"
        role="list"
      >
        <UnifiedPanelProjectEntry
          v-for="project in projectStore.projects"
          :key="project.id"
          :project="project"
          :is-active="project.id === projectStore.currentProjectId"
          :is-expanded="projectStore.isProjectExpanded(project.id)"
          :current-tab="getProjectTab(project.id)"
          :session-sort-by="layoutStore.sessionSortBy"
          :sessions="getSessionsByProject(project.id)"
          :current-session-id="sessionStore.currentSessionId"
          :editing-session-id="editingSessionId"
          :editing-session-name="editingSessionName"
          :imported-time-label="formatImportTime(project.createdAt)"
          @toggle-project="handleProjectCardClick"
          @edit-project="handleEditProject"
          @delete-project="handleDeleteProject"
          @set-tab="setProjectTab"
          @toggle-sort="toggleSessionSort"
          @add-session="handleAddSession"
          @select-session="handleSelectSession"
          @toggle-pin="handleTogglePin"
          @start-edit-session="startEditSessionName"
          @save-edit-session="saveSessionName"
          @cancel-edit-session="cancelEditSessionName"
          @delete-session="handleDeleteSession"
          @update-editing-name="editingSessionName = $event"
          @select-file="handleFileSelect"
        />
      </div>
    </div>

    <!-- 创建项目弹框 -->
    <Teleport to="body">
      <Transition name="modal">
        <div
          v-if="uiStore.projectCreateModalVisible"
          class="modal-overlay"
          @click="uiStore.closeProjectCreateModal()"
        >
          <div
            class="modal-container"
            @click.stop
          >
            <ProjectCreateModal
              :project="editingProject"
              @submit="handleCreateProject"
              @cancel="uiStore.closeProjectCreateModal()"
            />
          </div>
        </div>
      </Transition>
    </Teleport>

    <UnifiedPanelConfirmDialog
      :visible="showDeleteConfirm"
      :title="t('project.confirmDeleteTitle')"
      :message="t('project.confirmDeleteMessage', { name: deletingProject?.name })"
      @cancel="closeDeleteProjectConfirm"
      @confirm="confirmDeleteProject"
    />

    <UnifiedPanelConfirmDialog
      :visible="showDeleteSessionConfirm"
      :title="t('session.confirmDeleteTitle')"
      :message="t('session.confirmDeleteMessage', { name: deletingSession?.name })"
      @cancel="closeDeleteSessionConfirm"
      @confirm="confirmDeleteSession"
    />
  </div>
</template>

<style scoped>
.unified-panel {
  display: flex;
  flex-direction: column;
  container-type: inline-size;
  height: 100%;
  min-width: 0;
  background-color: var(--color-surface);
  border-right: 1px solid var(--color-border);
  overflow: hidden;
}

.unified-panel--collapsed {
  width: 48px;
}

.unified-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-2);
  padding: var(--spacing-3);
  border-bottom: 1px solid var(--color-border);
  min-height: 44px;
  min-width: 0;
}

.unified-panel__header-title {
  display: flex;
  align-items: center;
  flex: 1;
  min-width: 0;
  gap: var(--spacing-2);
  color: var(--color-text-primary);
  font-weight: var(--font-weight-medium);
  font-size: var(--font-size-sm);
}

.unified-panel__header-title span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.unified-panel__header-actions {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  gap: var(--spacing-1);
}

.header-action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: var(--radius-sm);
  color: var(--color-text-secondary);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.header-action-btn:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.unified-panel__content {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  padding: var(--spacing-2);
}

.project-list {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  gap: var(--spacing-1);
}

/* 加载状态 */
.project-loading {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.project-skeleton {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-md);
}

/* 错误状态 */
.project-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-8) var(--spacing-4);
  text-align: center;
  min-height: 200px;
}

.project-error__icon {
  color: var(--color-error);
  margin-bottom: var(--spacing-3);
}

.project-error__text {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
  margin: 0 0 var(--spacing-1);
}

.project-error__detail {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  margin: 0 0 var(--spacing-4);
  max-width: 180px;
  line-height: 1.5;
}

/* 空状态 */
.project-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-10) var(--spacing-4);
  text-align: center;
  min-height: 200px;
}

.project-empty__illustration {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 80px;
  height: 80px;
  border-radius: var(--radius-full);
  background: linear-gradient(135deg, var(--color-primary-light) 0%, transparent 100%);
  margin-bottom: var(--spacing-4);
}

.project-empty__icon {
  color: var(--color-primary);
}

.project-empty__title {
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
  margin: 0 0 var(--spacing-2);
}

.project-empty__hint {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  margin: 0 0 var(--spacing-5);
  max-width: 180px;
  line-height: 1.5;
}

.project-empty__button {
  gap: var(--spacing-2);
}

/* 弹框样式 */
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
}

.modal-container {
  width: 420px;
  max-width: 90vw;
  background-color: var(--color-surface);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
}

/* 动画 */
.modal-enter-active,
.modal-leave-active {
  transition: opacity var(--transition-normal) var(--easing-default);
}

.modal-enter-active .modal-container,
.modal-leave-active .modal-container {
  transition: transform var(--transition-normal) var(--easing-default),
              opacity var(--transition-normal) var(--easing-default);
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal-container,
.modal-leave-to .modal-container {
  transform: scale(0.95);
  opacity: 0;
}

@container (max-width: 300px) {
  .unified-panel__header {
    padding: var(--spacing-2);
  }

  .unified-panel__content {
    padding: var(--spacing-1);
  }
}

@container (max-width: 240px) {
  .unified-panel__header-title span {
    display: none;
  }

  .unified-panel__header-actions {
    gap: 2px;
  }

  .header-action-btn {
    width: 22px;
    height: 22px;
  }
}

/* 旋转动画 */
@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

/* 小屏笔记本优化 (13-14寸) */
@media (max-width: 1280px) {
  .unified-panel {
    --sidebar-font-primary: 11px;
    --sidebar-font-meta: 10px;
  }
}

.animate-spin {
  animation: spin 1s linear infinite;
}
</style>
