<script setup lang="ts">
import { ref, h, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import type { TreeOption } from 'naive-ui'
import type { UnwatchFn } from '@tauri-apps/plugin-fs'
import { useProjectStore, type Project, type FileTreeNode } from '@/stores/project'
import { useSessionStore, type Session } from '@/stores/session'
import { useLayoutStore, type ProjectTabType } from '@/stores/layout'
import { useUIStore } from '@/stores/ui'
import { useSessionView } from '@/composables'
import { useFileEditorStore } from '@/modules/file-editor'
import { resolveFileIcon } from '@/utils/fileIcon'
import { startFsWatcher } from '@/utils/fsWatcher'
import { EaIcon, EaButton, EaSkeleton } from '@/components/common'
import { ProjectCreateModal } from '@/components/project'
import UnifiedPanelConfirmDialog from './UnifiedPanelConfirmDialog.vue'
import UnifiedPanelProjectEntry from './UnifiedPanelProjectEntry.vue'

// 定义 TreeRenderProps 类型
interface TreeRenderProps {
  option: TreeOption
  checked: boolean
  selected: boolean
}

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

// 文件树展开的节点 keys
const expandedKeysMap = ref<Map<string, Set<string>>>(new Map())

// 本地维护的 Tree 数据
const projectTreeDataMap = ref<Map<string, TreeOption[]>>(new Map())
const projectWatcherMap = ref<Map<string, UnwatchFn>>(new Map())
const projectRefreshTimerMap = ref<Map<string, ReturnType<typeof setTimeout>>>(new Map())

// 获取项目当前 Tab
const getProjectTab = (projectId: string): ProjectTabType => {
  return layoutStore.getProjectTab(projectId)
}

// 设置项目当前 Tab
const setProjectTab = async (projectId: string, tab: ProjectTabType) => {
  layoutStore.setProjectTab(projectId, tab)
  // 切换到会话 Tab 时加载会话
  if (tab === 'sessions') {
    stopProjectWatcher(projectId)
    sessionStore.loadSessions(projectId)
    return
  }

  if (tab === 'files') {
    const project = projectStore.projects.find(p => p.id === projectId)
    if (!project) return
    await refreshProjectFileTree(project)
    await startProjectWatcher(project)
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

  if (projectStore.isProjectExpanded(project.id)) {
    if (getProjectTab(project.id) === 'files') {
      await refreshProjectFileTree(project)
      await startProjectWatcher(project)
    } else {
      stopProjectWatcher(project.id)
    }
  } else {
    stopProjectWatcher(project.id)
  }

  // 展开时加载会话
  if (projectStore.isProjectExpanded(project.id)) {
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
  stopAllProjectWatchers()
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

  const activeProjectIds = new Set(projectStore.projects.map(p => p.id))
  Array.from(projectWatcherMap.value.keys()).forEach(projectId => {
    if (!activeProjectIds.has(projectId)) {
      stopProjectWatcher(projectId)
      projectTreeDataMap.value.delete(projectId)
      expandedKeysMap.value.delete(projectId)
    }
  })

  const expandedProjectIds = Array.from(projectStore.expandedProjects)
  const expandedProjects = projectStore.projects.filter(project => expandedProjectIds.includes(project.id))
  await Promise.all(expandedProjects.map(async project => {
    if (getProjectTab(project.id) === 'files') {
      await refreshProjectFileTree(project)
      await startProjectWatcher(project)
    } else {
      stopProjectWatcher(project.id)
    }
  }))
}

const handleAddProject = () => {
  editingProject.value = null
  uiStore.openProjectCreateModal()
}

const handleEditProject = (project: Project) => {
  editingProject.value = project
  uiStore.openProjectCreateModal()
}

const handleCreateProject = async (data: { name: string; path: string; description?: string }) => {
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
    const newSession = await sessionStore.createSession({
      projectId,
      name: t('session.unnamedSession'),
      agentType: 'claude',
      status: 'idle'
    })
    projectStore.incrementSessionCount(projectId)
    uiStore.setMainContentMode('chat')
    sessionStore.openSession(newSession.id)
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

// ========== 文件树操作 ==========
const convertToTreeOptions = (nodes: FileTreeNode[], projectId: string): TreeOption[] => {
  return nodes.map(node => {
    const isFile = node.nodeType === 'file'
    const option: TreeOption = {
      key: node.path,
      label: node.name,
      isLeaf: isFile,
      nodeType: node.nodeType,
      extension: node.extension,
      projectId
    }

    if (!isFile) {
      if (node.children && node.children.length > 0) {
        option.children = convertToTreeOptions(node.children, projectId)
      } else {
        // 目录节点默认给空 children，避免 Tree 组件进入无 on-load 的加载态
        option.children = []
      }
    }

    return option
  }) as TreeOption[]
}

const initProjectTreeData = (projectId: string, force: boolean = false) => {
  if (!force && projectTreeDataMap.value.has(projectId)) {
    return
  }

  const fileTree = projectStore.getProjectFileTree(projectId)
  if (!fileTree) {
    projectTreeDataMap.value.delete(projectId)
    return
  }

  const treeData = convertToTreeOptions(fileTree, projectId)
  projectTreeDataMap.value.set(projectId, treeData)
}

const getProjectTreeData = (projectId: string): TreeOption[] => {
  initProjectTreeData(projectId)
  return projectTreeDataMap.value.get(projectId) || []
}

const getProjectExpandedKeys = (projectId: string): string[] => {
  const keys = expandedKeysMap.value.get(projectId)
  return keys ? Array.from(keys) : []
}

const findTreeNodeByKey = (nodes: TreeOption[], key: string): (TreeOption & { nodeType?: string }) | null => {
  for (const node of nodes) {
    if (String(node.key) === key) {
      return node as (TreeOption & { nodeType?: string })
    }
    if (node.children?.length) {
      const matched = findTreeNodeByKey(node.children as TreeOption[], key)
      if (matched) {
        return matched
      }
    }
  }
  return null
}

const loadChildrenForNode = async (node: TreeOption): Promise<void> => {
  const projectId = (node as any).projectId as string
  const nodePath = node.key as string

  if (!projectId) {
    return
  }

  const children = await projectStore.loadDirectoryChildren(nodePath)
  node.children = children.map(child => {
    const isFile = child.nodeType === 'file'
    const option: TreeOption = {
      key: child.path,
      label: child.name,
      isLeaf: isFile,
      nodeType: child.nodeType,
      extension: child.extension,
      projectId: projectId
    }
    if (!isFile) {
      option.children = []
    }
    return option
  }) as TreeOption[]
}

const handleTreeExpand = async (expandedKeys: string[], projectId: string) => {
  const previousKeys = expandedKeysMap.value.get(projectId) || new Set<string>()
  const currentKeys = new Set(expandedKeys)
  expandedKeysMap.value.set(projectId, currentKeys)

  const justExpandedKeys = expandedKeys.filter(key => !previousKeys.has(key))
  if (justExpandedKeys.length === 0) {
    return
  }

  const treeData = getProjectTreeData(projectId)
  const targetNodes = justExpandedKeys
    .map(key => findTreeNodeByKey(treeData, key))
    .filter((node): node is TreeOption & { nodeType?: string } => !!node)
    .filter(node => node.nodeType === 'directory' || node.isLeaf === false)

  await Promise.all(targetNodes.map(async node => {
    try {
      // 每次展开目录都重新查询目录内容，避免使用历史数据缓存
      await loadChildrenForNode(node)
    } catch (error) {
      console.error('[handleTreeExpand] 加载目录失败:', error)
    }
  }))
}

const handleFileSelect = async (
  keys: Array<string | number>,
  options: Array<TreeOption | null> = [],
  project: Project
) => {
  if (!keys.length) {
    return
  }

  const selectedPath = String(keys[0])
  const selectedNodeFromEvent = (options[0] ?? null) as (TreeOption & { nodeType?: string }) | null
  const selectedNode = selectedNodeFromEvent
    ?? findTreeNodeByKey(getProjectTreeData(project.id), selectedPath)

  if (selectedNode?.nodeType === 'directory' || selectedNode?.isLeaf === false) {
    return
  }

  projectStore.setCurrentProject(project.id)

  await fileEditorStore.openFile({
    projectId: project.id,
    projectPath: project.path,
    filePath: selectedPath
  })
}

// 自定义节点渲染
const renderTreeLabel = ({ option }: TreeRenderProps) => {
  const nodeType = (option as any).nodeType as string
  const fileName = option.label as string
  const extension = (option as any).extension as string | undefined
  const iconMeta = resolveFileIcon(nodeType, fileName, extension)

  return h('div', { class: 'file-tree-node__content' }, [
    h(EaIcon, {
      name: iconMeta.icon,
      size: 14,
      class: 'file-tree-node__icon',
      style: { color: iconMeta.color }
    }),
    h('span', { class: 'file-tree-node__name' }, fileName)
  ])
}

const refreshProjectFileTree = async (project: Project) => {
  await projectStore.refreshFileTree(project.id, project.path)
  initProjectTreeData(project.id, true)
  if (!expandedKeysMap.value.has(project.id)) {
    expandedKeysMap.value.set(project.id, new Set())
  }
}

const scheduleProjectTreeRefresh = (project: Project) => {
  const oldTimer = projectRefreshTimerMap.value.get(project.id)
  if (oldTimer) {
    clearTimeout(oldTimer)
  }

  const timer = setTimeout(async () => {
    projectRefreshTimerMap.value.delete(project.id)

    if (!projectStore.isProjectExpanded(project.id)) {
      return
    }

    await refreshProjectFileTree(project)
  }, 250)

  projectRefreshTimerMap.value.set(project.id, timer)
}

const stopProjectWatcher = (projectId: string) => {
  const timer = projectRefreshTimerMap.value.get(projectId)
  if (timer) {
    clearTimeout(timer)
    projectRefreshTimerMap.value.delete(projectId)
  }

  const unwatch = projectWatcherMap.value.get(projectId)
  if (unwatch) {
    unwatch()
    projectWatcherMap.value.delete(projectId)
  }
}

const stopAllProjectWatchers = () => {
  Array.from(projectWatcherMap.value.keys()).forEach(stopProjectWatcher)
}

const startProjectWatcher = async (project: Project) => {
  stopProjectWatcher(project.id)

  try {
    const unwatch = await startFsWatcher(
      project.path,
      () => {
        scheduleProjectTreeRefresh(project)
      },
      {
        recursive: true,
        delayMs: 300
      }
    )
    if (unwatch) {
      projectWatcherMap.value.set(project.id, unwatch)
    }
  } catch (error) {
    console.error('[UnifiedPanel] 启动目录监听失败:', error)
  }
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
          :is-file-tree-loading="projectStore.isFileTreeLoading(project.id)"
          :tree-data="getProjectTreeData(project.id)"
          :expanded-keys="getProjectExpandedKeys(project.id)"
          :imported-time-label="formatImportTime(project.createdAt)"
          :render-tree-label="renderTreeLabel"
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
          @expand-tree="handleTreeExpand"
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
  height: 100%;
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
  padding: var(--spacing-3);
  border-bottom: 1px solid var(--color-border);
  min-height: 44px;
}

.unified-panel__header-title {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  color: var(--color-text-primary);
  font-weight: var(--font-weight-medium);
  font-size: var(--font-size-sm);
}

.unified-panel__header-actions {
  display: flex;
  align-items: center;
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
  min-height: 0;
  overflow: hidden;
  padding: var(--spacing-2);
}

.project-list {
  display: flex;
  flex-direction: column;
  flex: 1;
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

/* 旋转动画 */
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
