import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type {
  MarketCategory,
  MarketListResponse,
  MarketQuery,
  MarketplaceSourceId,
  MarketplaceSourceOption,
  McpMarketDetail,
  McpMarketItem,
  PluginMarketDetail,
  PluginMarketItem,
  SkillMarketDetail,
  SkillMarketItem
} from '@/types/marketplace'

export interface McpInstallInput {
  mcp_id: string
  mcp_name: string
  cli_path: string
  command: string
  args: string[]
  env: Record<string, string>
  scope: 'global' | 'project'
  project_path: string | null
}

export interface McpInstallResult {
  success: boolean
  message: string
  config_path: string | null
  backup_path?: string | null
  rollback_performed?: boolean
  rollback_message?: string | null
}

export interface InstalledMcp {
  name: string
  config_path: string
  command: string
  args: string[]
  env: Record<string, string>
  disabled: boolean
  source_cli: string
  source_cli_path: string
  scope: string
}

export interface SkillInstallInput {
  skill_id: string
  skill_name: string
  cli_type: 'claude' | 'codex' | 'opencode'
  scope: 'global' | 'project'
  project_path: string | null
  source_market?: MarketplaceSourceId | null
}

export interface GitSkillInstallInput {
  repository_url: string
  git_ref?: string | null
  skill_name: string
  cli_type: 'claude' | 'codex' | 'opencode'
}

interface MarketPagination {
  page: number
  total: number
  hasMore: boolean
}

export interface SkillInstallResult {
  success: boolean
  message: string
  skill_path: string | null
  backup_path: string | null
}

export interface InstalledSkill {
  name: string
  file_name: string
  path: string
  description?: string | null
  disabled: boolean
  source_cli: string
  source_cli_path: string
  scope: string
  installed_at?: string | null
  triggers: string[]
}

export interface PluginInstallInput {
  plugin_id: string
  plugin_name: string
  plugin_version: string
  cli_path: string
  scope: 'global' | 'project'
  project_path: string | null
  selected_components: string[]
  config_values: Record<string, string>
}

export interface GitPluginInstallInput {
  repository_url: string
  git_ref?: string | null
  plugin_name: string
  cli_type: 'claude' | 'codex' | 'opencode'
  cli_path: string
}

export interface PluginInstallResult {
  success: boolean
  message: string
  plugin_id: string
  installed_components: Array<{
    name: string
    component_type: string
    target_path: string
  }>
  backup_path: string | null
  plugins_json_path: string | null
}

export interface InstalledPlugin {
  id: string
  name: string
  description: string
  version: string
  source_market: string
  cli_path: string
  scope: string
  components: Array<{
    name: string
    component_type: string
    target_path: string
  }>
  enabled: boolean
  installed_at: string
  config_values: Record<string, string>
}

const DEFAULT_MARKET_SOURCE = 'mcpmarket'
const DEFAULT_MARKETPLACE_SOURCES: MarketplaceSourceOption[] = [
  {
    id: 'mcpmarket',
    label: 'MCP Market',
    supported_resources: ['mcp', 'skills', 'plugins']
  },
  {
    id: 'modelscope',
    label: 'ModelScope MCP',
    supported_resources: ['mcp']
  }
]
const DEFAULT_MCP_QUERY: MarketQuery = {
  page: 1,
  search: null,
  category: null,
  category_slug: null,
  source_market: DEFAULT_MARKET_SOURCE
}
const DEFAULT_SKILLS_QUERY: MarketQuery = {
  page: 1,
  search: null,
  category: null,
  category_slug: null,
  source_market: DEFAULT_MARKET_SOURCE
}

export const useMarketplaceStore = defineStore('marketplace', () => {
  const activeMarketSource = ref<MarketplaceSourceId>(DEFAULT_MARKET_SOURCE)
  const marketplaceSources = ref<MarketplaceSourceOption[]>([])

  const mcpMarketItems = ref<McpMarketItem[]>([])
  const mcpMarketQuery = ref<MarketQuery>({ ...DEFAULT_MCP_QUERY })
  const mcpMarketCategories = ref<MarketCategory[]>([])
  const mcpMarketPagination = ref<MarketPagination>({
    page: 1,
    total: 0,
    hasMore: false
  })
  const isLoadingMcpMarket = ref(false)
  const mcpMarketError = ref<string | null>(null)
  const selectedMcpDetail = ref<McpMarketDetail | null>(null)
  const isLoadingMcpDetail = ref(false)
  const mcpDetailError = ref<string | null>(null)
  const installedMcps = ref<InstalledMcp[]>([])
  const isLoadingInstalledMcps = ref(false)
  const isInstallingMcp = ref(false)
  const mcpInstallError = ref<string | null>(null)

  const skillsMarketItems = ref<SkillMarketItem[]>([])
  const skillsMarketQuery = ref<MarketQuery>({ ...DEFAULT_SKILLS_QUERY })
  const skillsMarketCategories = ref<MarketCategory[]>([])
  const skillsMarketPagination = ref<MarketPagination>({
    page: 1,
    total: 0,
    hasMore: false
  })
  const isLoadingSkillsMarket = ref(false)
  const skillsMarketError = ref<string | null>(null)
  const selectedSkillDetail = ref<SkillMarketDetail | null>(null)
  const isLoadingSkillDetail = ref(false)
  const skillDetailError = ref<string | null>(null)
  const installedSkills = ref<InstalledSkill[]>([])
  const isLoadingInstalledSkills = ref(false)
  const isInstallingSkill = ref(false)
  const skillInstallError = ref<string | null>(null)

  const pluginsMarketItems = ref<PluginMarketItem[]>([])
  const isLoadingPluginsMarket = ref(false)
  const pluginsMarketError = ref<string | null>(null)
  const selectedPluginDetail = ref<PluginMarketDetail | null>(null)
  const isLoadingPluginDetail = ref(false)
  const pluginDetailError = ref<string | null>(null)
  const installedPlugins = ref<InstalledPlugin[]>([])
  const isLoadingInstalledPlugins = ref(false)
  const isInstallingPlugin = ref(false)
  const pluginInstallError = ref<string | null>(null)

  const activeMarketTab = ref<'mcp' | 'skills' | 'plugins'>('mcp')

  const installedMcpNames = computed(() =>
    new Set(installedMcps.value.map(item => item.name.toLowerCase()))
  )

  const installedSkillNames = computed(() =>
    new Set(
      installedSkills.value.flatMap(item => [
        item.name.toLowerCase(),
        item.file_name.toLowerCase()
      ])
    )
  )

  const installedPluginIds = computed(() =>
    new Set(installedPlugins.value.map(item => item.id))
  )

  const marketplaceSourceOptions = computed(() =>
    marketplaceSources.value.map(source => ({
      value: source.id,
      label: source.label
    }))
  )

  const activeMarketSupportedResources = computed<Array<'mcp' | 'skills' | 'plugins'>>(() => {
    const activeSource = marketplaceSources.value.find(source => source.id === activeMarketSource.value)
    return activeSource?.supported_resources ?? ['mcp', 'skills']
  })

  function withSource(query: MarketQuery = {}): MarketQuery {
    return {
      ...query,
      source_market: query.source_market ?? activeMarketSource.value
    }
  }

  function normalizeSkillsQuery(query: MarketQuery = {}): MarketQuery {
    const normalizedSearch = query.search?.trim() || null
    const normalizedCategorySlug = normalizedSearch ? null : (query.category_slug?.trim() || null)
    const normalizedCategory = normalizedSearch ? null : (query.category?.trim() || null)

    return {
      ...DEFAULT_SKILLS_QUERY,
      ...query,
      page: Math.max(1, query.page ?? skillsMarketQuery.value.page ?? 1),
      search: normalizedSearch,
      category: normalizedCategory,
      category_slug: normalizedCategorySlug,
      source_market: query.source_market ?? activeMarketSource.value
    }
  }

  function normalizeMcpQuery(query: MarketQuery = {}): MarketQuery {
    const normalizedSearch = query.search?.trim() || null
    const normalizedCategorySlug = normalizedSearch ? null : (query.category_slug?.trim() || null)
    const normalizedCategory = normalizedSearch ? null : (query.category?.trim() || null)

    return {
      ...DEFAULT_MCP_QUERY,
      ...query,
      page: Math.max(1, query.page ?? mcpMarketQuery.value.page ?? 1),
      search: normalizedSearch,
      category: normalizedCategory,
      category_slug: normalizedCategorySlug,
      source_market: query.source_market ?? activeMarketSource.value
    }
  }

  async function loadMarketplaceSources() {
    try {
      marketplaceSources.value = await invoke<MarketplaceSourceOption[]>('list_marketplace_source_options')
      if (
        marketplaceSources.value.length > 0 &&
        !marketplaceSources.value.some(source => source.id === activeMarketSource.value)
      ) {
        activeMarketSource.value = marketplaceSources.value[0].id
      }
    } catch (error) {
      console.error('Failed to load marketplace sources:', error)
      marketplaceSources.value = [...DEFAULT_MARKETPLACE_SOURCES]
    }
  }

  function sourceSupportsResource(
    source: MarketplaceSourceId,
    resource: 'mcp' | 'skills' | 'plugins'
  ): boolean {
    const sourceOption = marketplaceSources.value.find(item => item.id === source)
    return sourceOption?.supported_resources.includes(resource) ?? resource !== 'plugins'
  }

  function resolveSupportedTab(
    source: MarketplaceSourceId,
    fallback: 'mcp' | 'skills' | 'plugins' = 'mcp'
  ): 'mcp' | 'skills' | 'plugins' {
    if (sourceSupportsResource(source, fallback)) {
      return fallback
    }

    const sourceOption = marketplaceSources.value.find(item => item.id === source)
    return (sourceOption?.supported_resources[0] ?? 'mcp') as 'mcp' | 'skills' | 'plugins'
  }

  async function fetchMcpMarket(query: MarketQuery = {}, options: { append?: boolean } = {}) {
    isLoadingMcpMarket.value = true
    mcpMarketError.value = null

    try {
      const nextQuery = normalizeMcpQuery({
        ...mcpMarketQuery.value,
        ...query
      })
      const response = await invoke<MarketListResponse<McpMarketItem>>('fetch_mcp_market', {
        query: withSource(nextQuery)
      })
      const nextPage = response.page ?? nextQuery.page ?? 1
      mcpMarketItems.value = options.append
        ? [...mcpMarketItems.value, ...response.items]
        : response.items
      mcpMarketQuery.value = {
        ...nextQuery,
        page: nextPage
      }
      mcpMarketPagination.value = {
        page: nextPage,
        total: response.total,
        hasMore: response.has_more ?? false
      }
      mcpMarketCategories.value = response.categories ?? []
    } catch (error) {
      console.error('Failed to fetch MCP market:', error)
      mcpMarketError.value = error instanceof Error ? error.message : '获取 MCP 市场数据失败'
    } finally {
      isLoadingMcpMarket.value = false
    }
  }

  function clearMcpMarket() {
    mcpMarketItems.value = []
    mcpMarketQuery.value = { ...DEFAULT_MCP_QUERY, source_market: activeMarketSource.value }
    mcpMarketCategories.value = []
    mcpMarketPagination.value = {
      page: 1,
      total: 0,
      hasMore: false
    }
    mcpMarketError.value = null
  }

  async function fetchMcpDetail(mcpId: string, sourceMarket: MarketplaceSourceId = activeMarketSource.value) {
    isLoadingMcpDetail.value = true
    mcpDetailError.value = null

    try {
      const detail = await invoke<McpMarketDetail>('fetch_mcp_market_detail', {
        mcpId,
        sourceMarket
      })
      selectedMcpDetail.value = detail
    } catch (error) {
      console.error('Failed to fetch MCP detail:', error)
      mcpDetailError.value = error instanceof Error ? error.message : '获取 MCP 详情失败'
    } finally {
      isLoadingMcpDetail.value = false
    }
  }

  function clearMcpDetail() {
    selectedMcpDetail.value = null
    mcpDetailError.value = null
  }

  async function loadInstalledMcps() {
    isLoadingInstalledMcps.value = true
    try {
      installedMcps.value = await invoke<InstalledMcp[]>('list_installed_mcps')
    } catch (error) {
      console.error('Failed to load installed MCPs:', error)
    } finally {
      isLoadingInstalledMcps.value = false
    }
  }

  async function installMcp(input: McpInstallInput): Promise<McpInstallResult> {
    isInstallingMcp.value = true
    mcpInstallError.value = null

    try {
      const result = await invoke<McpInstallResult>('install_mcp_to_cli', { input })
      if (result.success) {
        await loadInstalledMcps()
      }
      return result
    } catch (error) {
      console.error('Failed to install MCP:', error)
      const message = error instanceof Error ? error.message : '安装 MCP 失败'
      mcpInstallError.value = message
      throw error
    } finally {
      isInstallingMcp.value = false
    }
  }

  async function toggleMcp(configPath: string, mcpName: string, disabled: boolean) {
    try {
      await invoke('toggle_installed_mcp', { configPath, mcpName, disabled })
      const localItem = installedMcps.value.find(
        item => item.config_path === configPath && item.name === mcpName
      )
      if (localItem) {
        localItem.disabled = disabled
      }
    } catch (error) {
      console.error('Failed to toggle MCP:', error)
      throw error
    }
  }

  async function uninstallMcp(configPath: string, mcpName: string): Promise<McpInstallResult> {
    try {
      const result = await invoke<McpInstallResult>('uninstall_mcp', { configPath, mcpName })
      if (result.success) {
        installedMcps.value = installedMcps.value.filter(
          item => !(item.config_path === configPath && item.name === mcpName)
        )
      }
      return result
    } catch (error) {
      console.error('Failed to uninstall MCP:', error)
      throw error
    }
  }

  async function fetchSkillsMarket(query: MarketQuery = {}, options: { append?: boolean } = {}) {
    isLoadingSkillsMarket.value = true
    skillsMarketError.value = null

    try {
      const nextQuery = normalizeSkillsQuery({
        ...skillsMarketQuery.value,
        ...query
      })
      const response = await invoke<MarketListResponse<SkillMarketItem>>('fetch_skills_market', {
        query: withSource(nextQuery)
      })
      const nextPage = response.page ?? nextQuery.page ?? 1
      skillsMarketItems.value = options.append
        ? [...skillsMarketItems.value, ...response.items]
        : response.items
      skillsMarketQuery.value = {
        ...nextQuery,
        page: nextPage
      }
      skillsMarketPagination.value = {
        page: nextPage,
        total: response.total,
        hasMore: response.has_more ?? false
      }
      skillsMarketCategories.value = response.categories ?? []
    } catch (error) {
      console.error('Failed to fetch Skills market:', error)
      skillsMarketError.value = error instanceof Error ? error.message : '获取 Skills 市场数据失败'
    } finally {
      isLoadingSkillsMarket.value = false
    }
  }

  function clearSkillsMarket() {
    skillsMarketItems.value = []
    skillsMarketQuery.value = { ...DEFAULT_SKILLS_QUERY, source_market: activeMarketSource.value }
    skillsMarketCategories.value = []
    skillsMarketPagination.value = {
      page: 1,
      total: 0,
      hasMore: false
    }
    skillsMarketError.value = null
  }

  async function fetchSkillDetail(skillId: string, sourceMarket: MarketplaceSourceId = activeMarketSource.value) {
    isLoadingSkillDetail.value = true
    skillDetailError.value = null

    try {
      const detail = await invoke<SkillMarketDetail>('fetch_skill_market_detail', {
        skillId,
        sourceMarket
      })
      selectedSkillDetail.value = detail
    } catch (error) {
      console.error('Failed to fetch skill detail:', error)
      skillDetailError.value = error instanceof Error ? error.message : '获取 Skill 详情失败'
    } finally {
      isLoadingSkillDetail.value = false
    }
  }

  function clearSkillDetail() {
    selectedSkillDetail.value = null
    skillDetailError.value = null
  }

  async function loadInstalledSkills() {
    isLoadingInstalledSkills.value = true
    try {
      installedSkills.value = await invoke<InstalledSkill[]>('list_installed_skills')
    } catch (error) {
      console.error('Failed to load installed skills:', error)
    } finally {
      isLoadingInstalledSkills.value = false
    }
  }

  async function installSkill(input: SkillInstallInput): Promise<SkillInstallResult> {
    isInstallingSkill.value = true
    skillInstallError.value = null

    try {
      const result = await invoke<SkillInstallResult>('install_skill_to_cli', { input })
      if (result.success) {
        await loadInstalledSkills()
      }
      return result
    } catch (error) {
      console.error('Failed to install skill:', error)
      const message = error instanceof Error ? error.message : '安装 Skill 失败'
      skillInstallError.value = message
      throw error
    } finally {
      isInstallingSkill.value = false
    }
  }

  async function installSkillFromGit(input: GitSkillInstallInput): Promise<SkillInstallResult> {
    isInstallingSkill.value = true
    skillInstallError.value = null

    try {
      const result = await invoke<SkillInstallResult>('install_skill_from_git', { input })
      if (result.success) {
        await loadInstalledSkills()
      }
      return result
    } catch (error) {
      console.error('Failed to install skill from git:', error)
      const message = error instanceof Error ? error.message : '从 Git 安装 Skill 失败'
      skillInstallError.value = message
      throw error
    } finally {
      isInstallingSkill.value = false
    }
  }

  async function toggleSkill(skillPath: string, disable: boolean): Promise<SkillInstallResult> {
    try {
      const result = await invoke<SkillInstallResult>('toggle_installed_skill', { skillPath, disable })
      if (result.success) {
        await loadInstalledSkills()
      }
      return result
    } catch (error) {
      console.error('Failed to toggle skill:', error)
      throw error
    }
  }

  async function uninstallSkill(skillPath: string): Promise<SkillInstallResult> {
    try {
      const result = await invoke<SkillInstallResult>('uninstall_skill', { skillPath })
      if (result.success) {
        installedSkills.value = installedSkills.value.filter(item => item.path !== skillPath)
      }
      return result
    } catch (error) {
      console.error('Failed to uninstall skill:', error)
      throw error
    }
  }

  async function fetchPluginsMarket(query: MarketQuery = {}) {
    isLoadingPluginsMarket.value = true
    pluginsMarketError.value = null

    try {
      const response = await invoke<MarketListResponse<PluginMarketItem>>('fetch_plugins_market', { query })
      pluginsMarketItems.value = response.items
    } catch (error) {
      console.error('Failed to fetch Plugins market:', error)
      pluginsMarketError.value = error instanceof Error ? error.message : '获取 Plugins 市场数据失败'
    } finally {
      isLoadingPluginsMarket.value = false
    }
  }

  function clearPluginsMarket() {
    pluginsMarketItems.value = []
    pluginsMarketError.value = null
  }

  async function fetchPluginDetail(pluginId: string) {
    isLoadingPluginDetail.value = true
    pluginDetailError.value = null

    try {
      const detail = await invoke<PluginMarketDetail>('fetch_plugin_detail', { pluginId })
      selectedPluginDetail.value = detail
    } catch (error) {
      console.error('Failed to fetch plugin detail:', error)
      pluginDetailError.value = error instanceof Error ? error.message : '获取 Plugin 详情失败'
    } finally {
      isLoadingPluginDetail.value = false
    }
  }

  function clearPluginDetail() {
    selectedPluginDetail.value = null
    pluginDetailError.value = null
  }

  async function loadInstalledPlugins() {
    isLoadingInstalledPlugins.value = true
    try {
      installedPlugins.value = await invoke<InstalledPlugin[]>('list_installed_plugins')
    } catch (error) {
      console.error('Failed to load installed plugins:', error)
    } finally {
      isLoadingInstalledPlugins.value = false
    }
  }

  async function installPlugin(input: PluginInstallInput): Promise<PluginInstallResult> {
    isInstallingPlugin.value = true
    pluginInstallError.value = null

    try {
      const result = await invoke<PluginInstallResult>('install_plugin', { input })
      if (result.success) {
        await loadInstalledPlugins()
      }
      return result
    } catch (error) {
      console.error('Failed to install plugin:', error)
      const message = error instanceof Error ? error.message : '安装 Plugin 失败'
      pluginInstallError.value = message
      throw error
    } finally {
      isInstallingPlugin.value = false
    }
  }

  async function installPluginFromGit(input: GitPluginInstallInput): Promise<PluginInstallResult> {
    isInstallingPlugin.value = true
    pluginInstallError.value = null

    try {
      const result = await invoke<PluginInstallResult>('install_plugin_from_git', { input })
      if (result.success) {
        await loadInstalledPlugins()
      }
      return result
    } catch (error) {
      console.error('Failed to install plugin from git:', error)
      const message = error instanceof Error ? error.message : '从 Git 安装 Plugin 失败'
      pluginInstallError.value = message
      throw error
    } finally {
      isInstallingPlugin.value = false
    }
  }

  async function togglePlugin(pluginId: string, enabled: boolean) {
    try {
      const plugin = await invoke<InstalledPlugin>('toggle_plugin', { pluginId, enabled })
      const localItem = installedPlugins.value.find(item => item.id === pluginId)
      if (localItem) {
        localItem.enabled = plugin.enabled
      }
      return plugin
    } catch (error) {
      console.error('Failed to toggle plugin:', error)
      throw error
    }
  }

  async function uninstallPlugin(pluginId: string): Promise<PluginInstallResult> {
    try {
      const result = await invoke<PluginInstallResult>('uninstall_plugin', { pluginId })
      if (result.success) {
        installedPlugins.value = installedPlugins.value.filter(item => item.id !== pluginId)
      }
      return result
    } catch (error) {
      console.error('Failed to uninstall plugin:', error)
      throw error
    }
  }

  function setActiveMarketTab(tab: 'mcp' | 'skills' | 'plugins') {
    activeMarketTab.value = resolveSupportedTab(activeMarketSource.value, tab)
  }

  function setActiveMarketSource(source: MarketplaceSourceId) {
    activeMarketSource.value = source
    activeMarketTab.value = resolveSupportedTab(source, activeMarketTab.value)
    clearMcpMarket()
    clearSkillsMarket()
    clearMcpDetail()
    clearSkillDetail()
  }

  async function loadAllInstalled() {
    await Promise.allSettled([
      loadInstalledMcps(),
      loadInstalledSkills(),
      loadInstalledPlugins()
    ])
  }

  async function refreshCurrentMarket() {
    if (activeMarketTab.value === 'mcp') {
      await fetchMcpMarket()
      return
    }

    if (activeMarketTab.value === 'skills') {
      await fetchSkillsMarket({ ...skillsMarketQuery.value, page: 1 })
      return
    }

    await fetchPluginsMarket()
  }

  function clearAllErrors() {
    mcpMarketError.value = null
    mcpDetailError.value = null
    mcpInstallError.value = null
    skillsMarketError.value = null
    skillDetailError.value = null
    skillInstallError.value = null
    pluginsMarketError.value = null
    pluginDetailError.value = null
    pluginInstallError.value = null
  }

  return {
    activeMarketSource,
    marketplaceSources,
    marketplaceSourceOptions,
    activeMarketSupportedResources,
    activeMarketTab,
    mcpMarketItems,
    mcpMarketQuery,
    mcpMarketCategories,
    mcpMarketPagination,
    isLoadingMcpMarket,
    mcpMarketError,
    selectedMcpDetail,
    isLoadingMcpDetail,
    mcpDetailError,
    installedMcps,
    isLoadingInstalledMcps,
    isInstallingMcp,
    mcpInstallError,
    skillsMarketItems,
    skillsMarketQuery,
    skillsMarketCategories,
    skillsMarketPagination,
    isLoadingSkillsMarket,
    skillsMarketError,
    selectedSkillDetail,
    isLoadingSkillDetail,
    skillDetailError,
    installedSkills,
    isLoadingInstalledSkills,
    isInstallingSkill,
    skillInstallError,
    pluginsMarketItems,
    isLoadingPluginsMarket,
    pluginsMarketError,
    selectedPluginDetail,
    isLoadingPluginDetail,
    pluginDetailError,
    installedPlugins,
    isLoadingInstalledPlugins,
    isInstallingPlugin,
    pluginInstallError,
    installedMcpNames,
    installedSkillNames,
    installedPluginIds,
    loadMarketplaceSources,
    sourceSupportsResource,
    setActiveMarketTab,
    setActiveMarketSource,
    fetchMcpMarket,
    clearMcpMarket,
    fetchMcpDetail,
    clearMcpDetail,
    loadInstalledMcps,
    installMcp,
    toggleMcp,
    uninstallMcp,
    fetchSkillsMarket,
    clearSkillsMarket,
    fetchSkillDetail,
    clearSkillDetail,
    loadInstalledSkills,
    installSkill,
    installSkillFromGit,
    toggleSkill,
    uninstallSkill,
    fetchPluginsMarket,
    clearPluginsMarket,
    fetchPluginDetail,
    clearPluginDetail,
    loadInstalledPlugins,
    installPlugin,
    installPluginFromGit,
    togglePlugin,
    uninstallPlugin,
    loadAllInstalled,
    refreshCurrentMarket,
    clearAllErrors
  }
})
