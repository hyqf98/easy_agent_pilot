import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { getErrorMessage } from '@/utils/api'
import { useLayoutStore } from './layout'

export interface Project {
  id: string
  name: string
  path: string
  description?: string
  memoryLibraryIds: string[]
  sessionCount?: number
  createdAt: string
  updatedAt: string
}

/// 文件树节点类型
export type FileNodeType = 'file' | 'directory'

/// 文件树节点
export interface FileTreeNode {
  name: string
  path: string
  nodeType: FileNodeType
  children?: FileTreeNode[]
  extension?: string
}

/// 文件操作结果
export interface FileOperationResult {
  success: boolean
  message?: string
  newPath?: string
}

export interface ProjectRuntimeCleanupResult {
  projectId: string
  clearedSessions: number
  clearedMessages: number
  clearedPlans: number
  clearedTasks: number
  clearedPlanSplitLogs: number
  clearedPlanSplitSessions: number
  clearedExecutionResults: number
  clearedExecutionLogs: number
}

/// 重命名文件输入
export interface RenameFileInput {
  oldPath: string
  newName: string
}

/// 移动文件输入
export interface MoveFileInput {
  sourcePath: string
  targetPath: string
}

/// 批量删除输入
export interface BatchDeleteInput {
  paths: string[]
}

interface ProjectFromDb {
  id: string
  name: string
  path: string
  description: string | null
  session_count: number
  memoryLibraryIds?: string[] | null
  created_at: string
  updated_at: string
}

interface CreateProjectInput {
  name: string
  path: string
  description?: string
  memoryLibraryIds: string[]
}

const LAST_PROJECT_KEY = 'ea-last-project'

function transformProject(p: ProjectFromDb): Project {
  return {
    id: p.id,
    name: p.name,
    path: p.path,
    description: p.description || undefined,
    memoryLibraryIds: p.memoryLibraryIds ?? [],
    sessionCount: p.session_count,
    createdAt: p.created_at,
    updatedAt: p.updated_at
  }
}

export const useProjectStore = defineStore('project', () => {
  // State
  const projects = ref<Project[]>([])
  const currentProjectId = ref<string | null>(null)
  const isLoading = ref(false)
  const loadError = ref<string | null>(null)

  // Getters
  const currentProject = computed(() =>
    projects.value.find(p => p.id === currentProjectId.value)
  )

  const projectCount = computed(() => projects.value.length)

  // Actions
  async function loadProjects() {
    isLoading.value = true
    loadError.value = null
    const notificationStore = useNotificationStore()
    try {
      const result = await invoke<ProjectFromDb[]>('list_projects')
      projects.value = result.map(transformProject)

      // 恢复上次选中的项目（仅在有记录时才自动选中）
      const lastProjectId = localStorage.getItem(LAST_PROJECT_KEY)
      if (lastProjectId && projects.value.some(p => p.id === lastProjectId)) {
        currentProjectId.value = lastProjectId
      }
      // 注意：不再自动选中第一个项目，让用户在欢迎页面手动选择
    } catch (error) {
      console.error('Failed to load projects:', error)
      projects.value = []
      loadError.value = getErrorMessage(error)
      notificationStore.networkError(
        '加载项目列表',
        getErrorMessage(error),
        loadProjects
      )
    } finally {
      isLoading.value = false
    }
  }

  async function createProject(project: Omit<Project, 'id' | 'createdAt' | 'updatedAt' | 'sessionCount'>) {
    const notificationStore = useNotificationStore()
    const input: CreateProjectInput = {
      name: project.name,
      path: project.path,
      description: project.description,
      memoryLibraryIds: project.memoryLibraryIds
    }

    try {
      const result = await invoke<ProjectFromDb>('create_project', { input })
      const newProject = transformProject(result)
      projects.value.push(newProject)
      return newProject
    } catch (error) {
      console.error('Failed to create project:', error)
      notificationStore.databaseError(
        '创建项目失败',
        getErrorMessage(error),
        async () => { await createProject(project) }
      )
      throw error
    }
  }

  async function updateProject(id: string, updates: Partial<Project>) {
    const notificationStore = useNotificationStore()
    const project = projects.value.find(p => p.id === id)
    if (!project) return

    const input: CreateProjectInput = {
      name: updates.name || project.name,
      path: updates.path || project.path,
      description: updates.description,
      memoryLibraryIds: updates.memoryLibraryIds ?? project.memoryLibraryIds
    }

    try {
      const result = await invoke<ProjectFromDb>('update_project', { id, input })
      const index = projects.value.findIndex(p => p.id === id)
      if (index !== -1) {
        projects.value[index] = transformProject(result)
      }
    } catch (error) {
      console.error('Failed to update project:', error)
      notificationStore.databaseError(
        '更新项目失败',
        getErrorMessage(error),
        async () => { await updateProject(id, updates) }
      )
      throw error
    }
  }

  async function deleteProject(id: string) {
    const notificationStore = useNotificationStore()

    try {
      await invoke('delete_project', { id })

      const index = projects.value.findIndex(p => p.id === id)
      if (index !== -1) {
        projects.value.splice(index, 1)
      }
      if (currentProjectId.value === id) {
        // 选择下一个项目
        if (projects.value.length > 0) {
          currentProjectId.value = projects.value[0].id
        } else {
          currentProjectId.value = null
        }
      }
    } catch (error) {
      console.error('Failed to delete project:', error)
      notificationStore.databaseError(
        '删除项目失败',
        getErrorMessage(error),
        async () => { await deleteProject(id) }
      )
      throw error
    }
  }

  async function clearProjectRuntimeData(projectId: string): Promise<ProjectRuntimeCleanupResult> {
    const notificationStore = useNotificationStore()

    try {
      const [
        { useSessionStore },
        { useMessageStore },
        { useTokenStore },
        { useAiEditTraceStore },
        { usePlanStore },
        { useTaskStore },
        { useTaskExecutionStore },
        { useTaskSplitStore }
      ] = await Promise.all([
        import('./session'),
        import('./message'),
        import('./token'),
        import('./aiEditTrace'),
        import('./plan'),
        import('./task'),
        import('./taskExecution'),
        import('./taskSplit')
      ])

      const sessionStore = useSessionStore()
      const planStore = usePlanStore()
      await sessionStore.loadSessions(projectId)
      await planStore.loadPlans(projectId)
      const sessionIds = sessionStore.sessions
        .filter(session => session.projectId === projectId)
        .map(session => session.id)
      const planIds = planStore.plans
        .filter(plan => plan.projectId === projectId)
        .map(plan => plan.id)

      const result = await invoke<ProjectRuntimeCleanupResult>('clear_project_runtime_data', { projectId })

      await sessionStore.clearProjectSessions(projectId)
      const project = projects.value.find(item => item.id === projectId)
      if (project) {
        project.sessionCount = sessionStore.sessions.filter(session => session.projectId === projectId).length
      }
      useMessageStore().clearProjectMessages(sessionIds)
      useTokenStore().clearProjectSessionTokenCaches(sessionIds)
      useAiEditTraceStore().resetSessions(sessionIds)
      useTaskSplitStore().clearProjectSplitState(planIds)
      planStore.clearProjectPlans(projectId)
      useTaskExecutionStore().clearPlansExecution(planIds)
      useTaskStore().clearPlansTasks(planIds)

      return result
    } catch (error) {
      console.error('Failed to clear project runtime data:', error)
      notificationStore.databaseError(
        '清理项目运行数据失败',
        getErrorMessage(error),
        async () => { await clearProjectRuntimeData(projectId) }
      )
      throw error
    }
  }

  function setCurrentProject(id: string | null) {
    currentProjectId.value = id

    // 记录访问时间
    if (id) {
      invoke('record_project_access', { projectId: id }).catch(console.error)

      // 自动展开左侧面板
      const layoutStore = useLayoutStore()
      if (!layoutStore.isPanelOpen) {
        layoutStore.togglePanel('unified')
      }
    }
  }

  function incrementSessionCount(projectId: string) {
    const project = projects.value.find(p => p.id === projectId)
    if (project) {
      project.sessionCount = (project.sessionCount || 0) + 1
    }
  }

  function decrementSessionCount(projectId: string) {
    const project = projects.value.find(p => p.id === projectId)
    if (project && project.sessionCount && project.sessionCount > 0) {
      project.sessionCount -= 1
    }
  }

  // 文件树相关状态
  const projectFileTrees = ref<Map<string, FileTreeNode[]>>(new Map())
  const expandedProjects = ref<Set<string>>(new Set())
  const loadingFileTrees = ref<Set<string>>(new Set())

  // 加载项目文件树（懒加载模式，只加载第一层）
  async function loadProjectFiles(projectId: string, projectPath: string) {
    if (loadingFileTrees.value.has(projectId)) {
      return
    }

    loadingFileTrees.value.add(projectId)
    const notificationStore = useNotificationStore()

    try {
      const result = await invoke<FileTreeNode[]>('list_project_files', {
        projectPath
      })
      projectFileTrees.value.set(projectId, result)
    } catch (error) {
      console.error('Failed to load project files:', error)
      notificationStore.networkError(
        '加载项目文件',
        getErrorMessage(error),
        () => loadProjectFiles(projectId, projectPath)
      )
    } finally {
      loadingFileTrees.value.delete(projectId)
    }
  }

  // 懒加载目录的子节点
  async function loadDirectoryChildren(dirPath: string): Promise<FileTreeNode[]> {
    const notificationStore = useNotificationStore()

    try {
      const result = await invoke<FileTreeNode[]>('load_directory_children', {
        dirPath
      })
      return result
    } catch (error) {
      console.error('Failed to load directory children:', error)
      notificationStore.networkError(
        '加载目录内容',
        getErrorMessage(error),
        async () => { await loadDirectoryChildren(dirPath) }
      )
      return []
    }
  }

  // 切换项目展开状态
  function toggleProjectExpand(projectId: string) {
    if (expandedProjects.value.has(projectId)) {
      expandedProjects.value.delete(projectId)
    } else {
      expandedProjects.value.add(projectId)
    }
  }

  function expandProject(projectId: string) {
    expandedProjects.value.add(projectId)
  }

  function collapseProject(projectId: string) {
    expandedProjects.value.delete(projectId)
  }

  // 检查项目是否展开
  function isProjectExpanded(projectId: string): boolean {
    return expandedProjects.value.has(projectId)
  }

  // 获取项目文件树
  function getProjectFileTree(projectId: string): FileTreeNode[] | undefined {
    return projectFileTrees.value.get(projectId)
  }

  // 检查文件树是否正在加载
  function isFileTreeLoading(projectId: string): boolean {
    return loadingFileTrees.value.has(projectId)
  }

  // 文件操作加载状态
  const fileOperationLoading = ref(false)

  /// 重命名文件/文件夹
  async function renameFileAction(oldPath: string, newName: string): Promise<FileOperationResult | null> {
    fileOperationLoading.value = true
    const notificationStore = useNotificationStore()

    try {
      const input: RenameFileInput = { oldPath, newName }
      const result = await invoke<FileOperationResult>('rename_file', { input })

      if (!result.success && result.message) {
        notificationStore.error('重命名失败', result.message)
      }

      return result
    } catch (error) {
      console.error('Failed to rename file:', error)
      notificationStore.networkError(
        '重命名文件',
        getErrorMessage(error),
        async () => { await renameFileAction(oldPath, newName) }
      )
      return null
    } finally {
      fileOperationLoading.value = false
    }
  }

  /// 删除单个文件/文件夹
  async function deleteFileAction(path: string): Promise<FileOperationResult | null> {
    fileOperationLoading.value = true
    const notificationStore = useNotificationStore()

    try {
      const result = await invoke<FileOperationResult>('delete_file', { path })

      if (!result.success && result.message) {
        notificationStore.error('删除失败', result.message)
      }

      return result
    } catch (error) {
      console.error('Failed to delete file:', error)
      notificationStore.networkError(
        '删除文件',
        getErrorMessage(error),
        async () => { await deleteFileAction(path) }
      )
      return null
    } finally {
      fileOperationLoading.value = false
    }
  }

  /// 批量删除文件/文件夹
  async function batchDeleteFilesAction(paths: string[]): Promise<FileOperationResult | null> {
    if (paths.length === 0) {
      return { success: true, message: undefined, newPath: undefined }
    }

    fileOperationLoading.value = true
    const notificationStore = useNotificationStore()

    try {
      const input: BatchDeleteInput = { paths }
      const result = await invoke<FileOperationResult>('batch_delete_files', { input })

      if (!result.success && result.message) {
        notificationStore.error('批量删除失败', result.message)
      }

      return result
    } catch (error) {
      console.error('Failed to batch delete files:', error)
      notificationStore.networkError(
        '批量删除文件',
        getErrorMessage(error),
        async () => { await batchDeleteFilesAction(paths) }
      )
      return null
    } finally {
      fileOperationLoading.value = false
    }
  }

  /// 移动文件/文件夹
  async function moveFileAction(sourcePath: string, targetPath: string): Promise<FileOperationResult | null> {
    fileOperationLoading.value = true
    const notificationStore = useNotificationStore()

    try {
      const input: MoveFileInput = { sourcePath, targetPath }
      const result = await invoke<FileOperationResult>('move_file', { input })

      if (!result.success && result.message) {
        notificationStore.error('移动失败', result.message)
      }

      return result
    } catch (error) {
      console.error('Failed to move file:', error)
      notificationStore.networkError(
        '移动文件',
        getErrorMessage(error),
        async () => { await moveFileAction(sourcePath, targetPath) }
      )
      return null
    } finally {
      fileOperationLoading.value = false
    }
  }

  /// 刷新项目文件树
  async function refreshFileTree(projectId: string, projectPath: string) {
    // 清除现有缓存
    projectFileTrees.value.delete(projectId)
    // 重新加载
    await loadProjectFiles(projectId, projectPath)
  }

  // 最近访问的项目 ID 列表
  const recentProjectIds = ref<string[]>([])

  // 获取最近访问的项目 ID 列表
  async function getRecentProjectIds(limit: number = 8): Promise<string[]> {
    try {
      const ids = await invoke<string[]>('get_recent_projects', { limit })
      recentProjectIds.value = ids
      return ids
    } catch (error) {
      console.error('Failed to get recent projects:', error)
      return []
    }
  }

  // 监听当前项目变化并保存
  watch(currentProjectId, (id) => {
    if (id) {
      localStorage.setItem(LAST_PROJECT_KEY, id)
    } else {
      localStorage.removeItem(LAST_PROJECT_KEY)
    }
  })

  return {
    // State
    projects,
    currentProjectId,
    isLoading,
    loadError,
    // Getters
    currentProject,
    projectCount,
    // Actions
    loadProjects,
    createProject,
    updateProject,
    deleteProject,
    clearProjectRuntimeData,
    setCurrentProject,
    incrementSessionCount,
    decrementSessionCount,
    // 文件树相关
    projectFileTrees,
    expandedProjects,
    loadingFileTrees,
    loadProjectFiles,
    loadDirectoryChildren,
    toggleProjectExpand,
    expandProject,
    collapseProject,
    isProjectExpanded,
    getProjectFileTree,
    isFileTreeLoading,
    // 文件操作
    fileOperationLoading,
    renameFileAction,
    deleteFileAction,
    batchDeleteFilesAction,
    moveFileAction,
    refreshFileTree,
    // 最近项目
    recentProjectIds,
    getRecentProjectIds
  }
})
