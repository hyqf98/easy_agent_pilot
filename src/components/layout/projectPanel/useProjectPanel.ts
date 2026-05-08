import { onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useProjectStore, type Project } from '@/stores/project'
import { useUIStore } from '@/stores/ui'
import { openProjectFileInWorkspace } from '@/modules/fileEditor'
import { refreshProjectFileTreeView } from '@/components/fileTree'

export interface ProjectPanelProps {
  collapsed?: boolean
  showHeaderToggle?: boolean
}

export type ProjectPanelEmits = {
  (e: 'toggle'): void
}

export interface ProjectCreatePayload {
  name: string
  path: string
  description?: string
  memoryLibraryIds: string[]
}

/**
 * 管理项目面板的项目装载、模态框状态和文件树展开刷新逻辑。
 */
export function useProjectPanel() {
  const { t } = useI18n()
  const projectStore = useProjectStore()
  const uiStore = useUIStore()
  const editingProject = ref<Project | null>(null)
  const showDeleteConfirm = ref(false)
  const deletingProject = ref<Project | null>(null)

  function handleModalKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') {
      return
    }

    if (showDeleteConfirm.value) {
      showDeleteConfirm.value = false
      return
    }

    if (uiStore.projectCreateModalVisible) {
      uiStore.closeProjectCreateModal()
    }
  }

  onMounted(() => {
    void projectStore.loadProjects()
    document.addEventListener('keydown', handleModalKeydown)
  })

  onUnmounted(() => {
    document.removeEventListener('keydown', handleModalKeydown)
  })

  const handleRefresh = async () => {
    await projectStore.loadProjects()

    const expandedProjectIds = Array.from(projectStore.expandedProjects)
    const expandedProjects = projectStore.projects.filter((project) =>
      expandedProjectIds.includes(project.id)
    )

    await Promise.all(
      expandedProjects.map(async (project) => {
        await projectStore.refreshFileTree(project.id, project.path)
        await refreshProjectFileTreeView(project.id, project.path)
      })
    )
  }

  const handleAdd = () => {
    editingProject.value = null
    uiStore.openProjectCreateModal()
  }

  const handleEditProject = (project: Project) => {
    editingProject.value = project
    uiStore.openProjectCreateModal()
  }

  const handleSelectProject = (id: string) => {
    projectStore.setCurrentProject(id)
  }

  const handleCreateProject = async (data: ProjectCreatePayload) => {
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

  const confirmDelete = () => {
    if (deletingProject.value) {
      projectStore.deleteProject(deletingProject.value.id)
    }

    showDeleteConfirm.value = false
    deletingProject.value = null
  }

  const handleToggleExpand = async (project: Project, event: Event) => {
    event.stopPropagation()
    projectStore.toggleProjectExpand(project.id)

    if (projectStore.isProjectExpanded(project.id)) {
      await projectStore.refreshFileTree(project.id, project.path)
      await refreshProjectFileTreeView(project.id, project.path)
    }
  }

  const handleFileSelect = async (path: string, project: Project) => {
    await openProjectFileInWorkspace({
      projectId: project.id,
      projectPath: project.path,
      filePath: path,
    })
  }

  return {
    confirmDelete,
    deletingProject,
    editingProject,
    handleAdd,
    handleCreateProject,
    handleDeleteProject,
    handleEditProject,
    handleFileSelect,
    handleRefresh,
    handleSelectProject,
    handleToggleExpand,
    projectStore,
    showDeleteConfirm,
    t,
    uiStore,
  }
}
