import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Ref } from 'vue'
import type { UnifiedPluginConfig } from '@/stores/skillConfig'
import type { InternalItem, PluginDetails, PluginFileContent, PluginSection } from './shared'

interface DirectoryFile {
  name: string
  path: string
}

export function usePluginDetail(plugin: Ref<UnifiedPluginConfig>) {
  const isLoading = ref(true)
  const pluginDetails = ref<PluginDetails | null>(null)
  const activeSection = ref<PluginSection>('skills')
  const isSidebarHovered = ref(false)
  const sidebarHoverTimeout = ref<ReturnType<typeof setTimeout> | null>(null)
  const selectedItem = ref<InternalItem | null>(null)
  const fileContent = ref<PluginFileContent | null>(null)
  const isLoadingFile = ref(false)
  const isEditMode = ref(false)
  const editContent = ref('')

  const currentList = computed(() => {
    if (!pluginDetails.value) {
      return []
    }

    switch (activeSection.value) {
      case 'skills':
        return pluginDetails.value.internal_skills
      case 'commands':
        return pluginDetails.value.internal_commands
      case 'agents':
        return pluginDetails.value.internal_agents
    }
  })

  const hasListItems = computed(() => currentList.value.length > 0)

  function clearHoverTimeout(): void {
    if (sidebarHoverTimeout.value) {
      clearTimeout(sidebarHoverTimeout.value)
      sidebarHoverTimeout.value = null
    }
  }

  async function loadPluginDetail(): Promise<void> {
    isLoading.value = true
    selectedItem.value = null
    fileContent.value = null
    isEditMode.value = false
    isSidebarHovered.value = false

    try {
      pluginDetails.value = await invoke<PluginDetails>('get_plugin_details', {
        pluginPath: plugin.value.pluginPath,
      })
    } finally {
      isLoading.value = false
    }
  }

  function handleSidebarMouseEnter(): void {
    clearHoverTimeout()
    isSidebarHovered.value = true
  }

  function handleSidebarMouseLeave(): void {
    clearHoverTimeout()
    sidebarHoverTimeout.value = setTimeout(() => {
      isSidebarHovered.value = false
    }, 300)
  }

  async function resolveMarkdownFile(item: InternalItem): Promise<{ path: string, content: string }> {
    if (item.path.endsWith('.md')) {
      return {
        path: item.path,
        content: await invoke<string>('read_file_content', { filePath: item.path }),
      }
    }

    const candidatePaths = [
      `${item.path}/${item.item_type}.md`,
      `${item.path}/${item.item_type.toUpperCase()}.md`,
    ]

    for (const candidatePath of candidatePaths) {
      try {
        return {
          path: candidatePath,
          content: await invoke<string>('read_file_content', { filePath: candidatePath }),
        }
      } catch {
        continue
      }
    }

    const files = await invoke<DirectoryFile[]>('list_directory_files', {
      dirPath: item.path,
      extension: '.md',
    })

    if (!files.length) {
      throw new Error('No markdown file found')
    }

    return {
      path: files[0].path,
      content: await invoke<string>('read_file_content', { filePath: files[0].path }),
    }
  }

  async function selectItem(item: InternalItem): Promise<void> {
    selectedItem.value = item
    isLoadingFile.value = true
    fileContent.value = null
    isEditMode.value = false

    try {
      const resolved = await resolveMarkdownFile(item)
      fileContent.value = {
        name: item.name,
        path: resolved.path,
        content: resolved.content,
        fileType: 'markdown',
      }
    } finally {
      isLoadingFile.value = false
    }
  }

  function switchSection(section: PluginSection): void {
    activeSection.value = section
    selectedItem.value = null
    fileContent.value = null
    isEditMode.value = false
  }

  function toggleEditMode(): void {
    if (!fileContent.value) {
      return
    }

    if (isEditMode.value) {
      isEditMode.value = false
      return
    }

    editContent.value = fileContent.value.content
    isEditMode.value = true
  }

  async function saveEdit(): Promise<void> {
    if (!fileContent.value) {
      return
    }

    await invoke('write_file_content', {
      filePath: fileContent.value.path,
      content: editContent.value,
    })
    fileContent.value.content = editContent.value
    isEditMode.value = false
  }

  function dispose(): void {
    clearHoverTimeout()
  }

  return {
    activeSection,
    currentList,
    editContent,
    fileContent,
    hasListItems,
    isEditMode,
    isLoading,
    isLoadingFile,
    isSidebarHovered,
    pluginDetails,
    selectedItem,
    dispose,
    handleSidebarMouseEnter,
    handleSidebarMouseLeave,
    loadPluginDetail,
    saveEdit,
    selectItem,
    switchSection,
    toggleEditMode,
  }
}
