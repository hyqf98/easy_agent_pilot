import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAgentStore, type AgentConfig, type CliTool } from '@/stores/agent'

interface MigrationResult {
  success: boolean
  migrated_count: number
  skipped_count: number
  migrated_agent_ids: string[]
  error_message: string | null
}

interface TestResultState {
  visible: boolean
  success: boolean
  message: string
}

const PAGE_SIZE = 10

export function useAgentSettingsPage() {
  const agentStore = useAgentStore()

  const addingToolName = ref<string | null>(null)
  const currentPage = ref(1)
  const searchQuery = ref('')
  const activeAgentId = ref<string | null>(null)

  const showModal = ref(false)
  const editingAgent = ref<AgentConfig | null>(null)
  const showDeleteConfirm = ref(false)
  const deletingAgent = ref<AgentConfig | null>(null)

  const showModelManageModal = ref(false)
  const managingModelAgent = ref<AgentConfig | null>(null)

  const testResult = ref<TestResultState>({
    visible: false,
    success: false,
    message: ''
  })

  const showMigrationBanner = ref(false)
  const migrationPendingCount = ref(0)
  const isMigrating = ref(false)
  const migrationResult = ref<MigrationResult | null>(null)
  const showMigrationResultToast = ref(false)

  const filteredAgents = computed(() => {
    let result = [...agentStore.agents]

    if (searchQuery.value.trim()) {
      const query = searchQuery.value.toLowerCase().trim()
      result = result.filter(agent =>
        agent.name.toLowerCase().includes(query)
        || agent.provider?.toLowerCase().includes(query)
        || agent.modelId?.toLowerCase().includes(query)
      )
    }

    result.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())
    return result
  })

  const activeAgent = computed(() =>
    agentStore.agents.find(agent => agent.id === activeAgentId.value) || null
  )

  const totalPages = computed(() => Math.ceil(filteredAgents.value.length / PAGE_SIZE) || 1)

  const paginatedAgents = computed(() => {
    const start = (currentPage.value - 1) * PAGE_SIZE
    return filteredAgents.value.slice(start, start + PAGE_SIZE)
  })

  const pageNumbers = computed(() => {
    const pages: number[] = []
    const total = totalPages.value
    const current = currentPage.value

    if (total <= 7) {
      for (let i = 1; i <= total; i += 1) {
        pages.push(i)
      }
      return pages
    }

    if (current <= 4) {
      return [1, 2, 3, 4, 5, -1, total]
    }

    if (current >= total - 3) {
      pages.push(1, -1)
      for (let i = total - 4; i <= total; i += 1) {
        pages.push(i)
      }
      return pages
    }

    return [1, -1, current - 1, current, current + 1, -1, total]
  })

  function handleSearchChange() {
    currentPage.value = 1
  }

  function showTestToast(success: boolean, message: string) {
    testResult.value = { visible: true, success, message }
    setTimeout(() => {
      testResult.value.visible = false
    }, 3000)
  }

  async function checkMigrationNeeded() {
    try {
      const needed = await invoke<boolean>('check_cli_paths_migration_needed')
      if (!needed) {
        showMigrationBanner.value = false
        migrationPendingCount.value = 0
        return
      }

      migrationPendingCount.value = await invoke<number>('get_pending_migration_count')
      showMigrationBanner.value = migrationPendingCount.value > 0
    } catch (error) {
      console.error('Failed to check migration status:', error)
    }
  }

  async function handleMigration() {
    isMigrating.value = true
    try {
      const result = await invoke<MigrationResult>('migrate_cli_paths_to_agents')
      migrationResult.value = result

      if (result.success) {
        await agentStore.loadAgents()
        showMigrationBanner.value = false
        showMigrationResultToast.value = true
        setTimeout(() => {
          showMigrationResultToast.value = false
        }, 5000)
      }
    } catch (error) {
      console.error('Migration failed:', error)
      migrationResult.value = {
        success: false,
        migrated_count: 0,
        skipped_count: 0,
        migrated_agent_ids: [],
        error_message: String(error)
      }
      showMigrationResultToast.value = true
    } finally {
      isMigrating.value = false
    }
  }

  function handleMigrationLater() {
    showMigrationBanner.value = false
  }

  async function triggerMigrationCheck() {
    await checkMigrationNeeded()
  }

  async function handleQuickAdd(tool: CliTool) {
    addingToolName.value = tool.name
    try {
      const agent = await agentStore.addDetectedTool(tool)
      activeAgentId.value = agent.id
    } finally {
      addingToolName.value = null
    }
  }

  function handleFocusAgent(agent: AgentConfig) {
    activeAgentId.value = agent.id
  }

  function handleAdd() {
    editingAgent.value = null
    showModal.value = true
  }

  function handleEdit(agent: AgentConfig) {
    editingAgent.value = agent
    showModal.value = true
  }

  function handleDelete(agent: AgentConfig) {
    deletingAgent.value = agent
    showDeleteConfirm.value = true
  }

  async function confirmDelete() {
    if (deletingAgent.value) {
      const deletedId = deletingAgent.value.id
      await agentStore.deleteAgent(deletingAgent.value.id)
      if (activeAgentId.value === deletedId) {
        activeAgentId.value = agentStore.agents[0]?.id || null
      }
      if (paginatedAgents.value.length === 0 && currentPage.value > 1) {
        currentPage.value -= 1
      }
    }

    showDeleteConfirm.value = false
    deletingAgent.value = null
  }

  async function handleTest(id: string) {
    const result = await agentStore.testConnection(id)
    showTestToast(result.success, result.message)
  }

  async function handleSubmit(
    data: Omit<AgentConfig, 'id' | 'createdAt' | 'updatedAt' | 'status'>
  ) {
    if (editingAgent.value) {
      await agentStore.updateAgent(editingAgent.value.id, data)
      activeAgentId.value = editingAgent.value.id
    } else {
      const agent = await agentStore.createAgent(data)
      activeAgentId.value = agent.id
    }

    showModal.value = false
    editingAgent.value = null
  }

  function handleCancel() {
    showModal.value = false
    editingAgent.value = null
  }

  function handleOpenModelManage(agent: AgentConfig) {
    managingModelAgent.value = agent
    showModelManageModal.value = true
  }

  function handleCloseModelManage() {
    showModelManageModal.value = false
    managingModelAgent.value = null
  }

  function goToPage(page: number) {
    if (page >= 1 && page <= totalPages.value) {
      currentPage.value = page
    }
  }

  function clearSearch() {
    searchQuery.value = ''
    currentPage.value = 1
  }

  onMounted(async () => {
    await agentStore.loadAgents()
    if (!activeAgentId.value && agentStore.agents.length > 0) {
      activeAgentId.value = agentStore.agents[0].id
    }
    await agentStore.scanCliTools()
    void checkMigrationNeeded()
  })

  return {
    PAGE_SIZE,
    addingToolName,
    agentStore,
    activeAgentId,
    activeAgent,
    currentPage,
    searchQuery,
    showModal,
    editingAgent,
    showDeleteConfirm,
    deletingAgent,
    showModelManageModal,
    managingModelAgent,
    testResult,
    showMigrationBanner,
    migrationPendingCount,
    isMigrating,
    migrationResult,
    showMigrationResultToast,
    filteredAgents,
    totalPages,
    paginatedAgents,
    pageNumbers,
    handleSearchChange,
    triggerMigrationCheck,
    handleMigration,
    handleMigrationLater,
    handleQuickAdd,
    handleFocusAgent,
    handleAdd,
    handleEdit,
    handleDelete,
    confirmDelete,
    handleTest,
    handleSubmit,
    handleCancel,
    handleOpenModelManage,
    handleCloseModelManage,
    goToPage,
    clearSearch
  }
}
