import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { getErrorMessage } from '@/utils/api'
import type { AgentConfig } from './agent'
import {
  buildMcpConfigInput,
  transformCliMcpConfig,
  transformCliPlugin,
  transformCliSkill,
  transformDbMcpConfig,
  transformDbPluginsConfig,
  transformDbSkillsConfig,
} from './skillConfigShared'
import type {
  CliCapabilities,
  CliConfig,
  CliConfigPaths,
  ClaudeConfigScanResult,
  ConfigSource,
  McpToolCallResult,
  McpToolsListResult,
  RawAgentMcpConfig,
  RawAgentPluginsConfig,
  RawAgentSkillsConfig,
  UnifiedMcpConfig,
  UnifiedPluginConfig,
  UnifiedSkillConfig,
} from './skillConfigShared'

export type {
  CliCapabilities,
  CliConfigPaths,
  ConfigSource,
  McpConfigInput,
  McpConfigScope,
  McpTool,
  McpToolCallResult,
  McpToolsListResult,
  McpTransportType,
  UnifiedConfigItem,
  UnifiedMcpConfig,
  UnifiedPluginConfig,
  UnifiedSkillConfig,
} from './skillConfigShared'

// ============================================================================
// Store 定义
// ============================================================================

export const useSkillConfigStore = defineStore('skillConfig', () => {
  // State
  const selectedAgentId = ref<string | null>(null)
  const selectedAgent = ref<AgentConfig | null>(null)
  const mcpConfigs = ref<UnifiedMcpConfig[]>([])
  const skillsConfigs = ref<UnifiedSkillConfig[]>([])
  const pluginsConfigs = ref<UnifiedPluginConfig[]>([])
  const configSource = ref<ConfigSource>('database')
  const isReadOnly = ref(false)
  const isLoading = ref(false)
  const cliConfigPaths = ref<CliConfigPaths | null>(null)
  const testingMcpConfig = ref<UnifiedMcpConfig | null>(null)
  const cliCapabilities = ref<CliCapabilities | null>(null)

  // Getters
  const isCliAgent = computed(() => selectedAgent.value?.type === 'cli')
  const isSdkAgent = computed(() => selectedAgent.value?.type === 'sdk')

  /**
   * 是否支持 Plugins（用于动态显示/隐藏 Plugins 标签页）
   * - SDK 类型：始终支持
   * - CLI 类型：根据能力信息判断
   */
  const supportsPlugins = computed(() => {
    if (!selectedAgent.value || selectedAgent.value.type !== 'cli') {
      return true // SDK类型始终支持
    }
    return cliCapabilities.value?.supportsPlugins ?? false
  })

  // ============================================================================
  // Actions - 智能体选择与配置加载
  // ============================================================================

  /**
   * 选择智能体并加载其配置
   */
  async function selectAgent(agent: AgentConfig | null) {
    if (!agent) {
      clearConfigs()
      return
    }

    selectedAgentId.value = agent.id
    selectedAgent.value = agent
    isLoading.value = true

    const notificationStore = useNotificationStore()

    try {
      if (agent.type === 'cli') {
        await loadCliConfigs(agent)
      } else {
        await loadSdkConfigs(agent.id)
      }
    } catch (error) {
      console.error('Failed to load agent configs:', error)
      notificationStore.networkError(
        '加载配置失败',
        getErrorMessage(error),
        () => selectAgent(agent)
      )
    } finally {
      isLoading.value = false
    }
  }

  /**
   * 加载 SDK 智能体配置（从数据库）
   */
  async function loadSdkConfigs(agentId: string) {
    configSource.value = 'database'
    isReadOnly.value = false
    cliConfigPaths.value = null

    const notificationStore = useNotificationStore()

    try {
      // 并行加载所有配置
      const [rawMcp, rawSkills, rawPlugins] = await Promise.all([
        invoke<RawAgentMcpConfig[]>('list_agent_mcp_configs', { agentId }),
        invoke<RawAgentSkillsConfig[]>('list_agent_skills_configs', { agentId }),
        invoke<RawAgentPluginsConfig[]>('list_agent_plugins_configs', { agentId }),
      ])

      mcpConfigs.value = rawMcp.map(transformDbMcpConfig)
      skillsConfigs.value = rawSkills.map(transformDbSkillsConfig)
      pluginsConfigs.value = rawPlugins.map(transformDbPluginsConfig)
    } catch (error) {
      console.error('Failed to load SDK configs:', error)
      notificationStore.databaseError(
        '加载 SDK 配置失败',
        getErrorMessage(error),
        async () => { await loadSdkConfigs(agentId) }
      )
      throw error
    }
  }

  /**
   * 加载 CLI 智能体配置（从文件）
   */
  async function loadCliConfigs(agent: AgentConfig) {
    configSource.value = 'file'
    isReadOnly.value = false // 允许直接编辑 CLI 配置

    const notificationStore = useNotificationStore()

    try {
      // 加载 CLI 能力信息
      if (agent.cliPath) {
        await loadCliCapabilities(agent.cliPath)
      }

      // 获取配置路径
      const paths = await invoke<CliConfigPaths>('get_cli_config_paths', {
        cliPath: agent.cliPath,
      })
      cliConfigPaths.value = paths

      // 读取配置文件
      const config = await invoke<CliConfig>('read_cli_config', {
        cliPath: agent.cliPath,
      })

      // 转换 MCP 配置
      const mcpServers = config.mcp_servers || config.mcpServers || {}
      mcpConfigs.value = Object.entries(mcpServers).map(([name, cfg]) =>
        transformCliMcpConfig(name, cfg)
      )

      // 扫描 Skills 和 Plugins
      const scanResult = await invoke<ClaudeConfigScanResult>('scan_cli_config', {
        cliPath: agent.cliPath,
      })

      // 转换 Skills
      skillsConfigs.value = scanResult.skills.map(transformCliSkill)

      // 转换 Plugins
      pluginsConfigs.value = scanResult.plugins.map(transformCliPlugin)
    } catch (error) {
      console.error('Failed to load CLI configs:', error)
      notificationStore.networkError(
        '加载 CLI 配置失败',
        getErrorMessage(error),
        () => loadCliConfigs(agent)
      )
      throw error
    }
  }

  /**
   * 刷新 CLI 配置（从文件系统重新读取）
   */
  async function refreshCliConfigs() {
    if (!selectedAgent.value || selectedAgent.value.type !== 'cli') {
      return
    }

    isLoading.value = true
    try {
      await loadCliConfigs(selectedAgent.value)
    } finally {
      isLoading.value = false
    }
  }

  /**
   * 加载 CLI 能力信息
   */
  async function loadCliCapabilities(cliPath: string) {
    try {
      cliCapabilities.value = await invoke<CliCapabilities>('get_cli_capabilities', { cliPath })
    } catch (e) {
      console.error('Failed to load CLI capabilities:', e)
      cliCapabilities.value = null
    }
  }

  /**
   * 清除配置
   */
  function clearConfigs() {
    selectedAgentId.value = null
    selectedAgent.value = null
    mcpConfigs.value = []
    skillsConfigs.value = []
    pluginsConfigs.value = []
    configSource.value = 'database'
    isReadOnly.value = false
    cliConfigPaths.value = null
    cliCapabilities.value = null
  }

  // ============================================================================
  // Actions - MCP 配置操作
  // ============================================================================

  /**
   * 创建 MCP 配置
   */
  async function createMcpConfig(config: Omit<UnifiedMcpConfig, 'id' | 'source' | 'isReadOnly'>) {
    if (!selectedAgentId.value) return

    const notificationStore = useNotificationStore()

    if (isReadOnly.value) {
      // CLI 类型：更新配置文件
      try {
        await invoke('update_cli_mcp_config', {
          cliPath: selectedAgent.value?.cliPath,
          name: config.name,
          config: {
            command: config.command,
            args: config.args,
            env: config.env,
            url: config.url,
            headers: config.headers,
            disabled: !config.enabled,
          },
        })
        await refreshCliConfigs()
      } catch (error) {
        console.error('Failed to create CLI MCP config:', error)
        notificationStore.databaseError(
          '创建 MCP 配置失败',
          getErrorMessage(error),
          async () => { await createMcpConfig(config) }
        )
        throw error
      }
    } else {
      // SDK 类型：写入数据库
      try {
        const rawConfig = await invoke<RawAgentMcpConfig>('create_agent_mcp_config', {
          input: {
            agent_id: selectedAgentId.value,
            name: config.name,
            transport_type: config.transportType,
            command: config.command,
            args: config.args?.join('\n'),
            env: config.env ? JSON.stringify(config.env) : undefined,
            url: config.url,
            headers: config.headers ? JSON.stringify(config.headers) : undefined,
            scope: config.scope,
          },
        })
        mcpConfigs.value.push(transformDbMcpConfig(rawConfig))
        return rawConfig
      } catch (error) {
        console.error('Failed to create SDK MCP config:', error)
        notificationStore.databaseError(
          '创建 MCP 配置失败',
          getErrorMessage(error),
          async () => { await createMcpConfig(config) }
        )
        throw error
      }
    }
  }

  /**
   * 更新 MCP 配置
   */
  async function updateMcpConfig(id: string, updates: Partial<UnifiedMcpConfig>) {
    if (!selectedAgentId.value) return

    const notificationStore = useNotificationStore()

    if (isReadOnly.value) {
      // CLI 类型：更新配置文件
      const config = mcpConfigs.value.find(c => c.id === id)
      if (!config) return

      try {
        await invoke('update_cli_mcp_config', {
          cliPath: selectedAgent.value?.cliPath,
          name: updates.name || config.name,
          config: {
            command: updates.command ?? config.command,
            args: updates.args ?? config.args,
            env: updates.env ?? config.env,
            url: updates.url ?? config.url,
            headers: updates.headers ?? config.headers,
            disabled: updates.enabled !== undefined ? !updates.enabled : !config.enabled,
          },
        })
        await refreshCliConfigs()
      } catch (error) {
        console.error('Failed to update CLI MCP config:', error)
        notificationStore.databaseError(
          '更新 MCP 配置失败',
          getErrorMessage(error),
          async () => { await updateMcpConfig(id, updates) }
        )
        throw error
      }
    } else {
      // SDK 类型：更新数据库
      try {
        const rawConfig = await invoke<RawAgentMcpConfig>('update_agent_mcp_config', {
          id,
          input: {
            name: updates.name,
            transport_type: updates.transportType,
            command: updates.command,
            args: updates.args?.join('\n'),
            env: updates.env ? JSON.stringify(updates.env) : undefined,
            url: updates.url,
            headers: updates.headers ? JSON.stringify(updates.headers) : undefined,
            scope: updates.scope,
            enabled: updates.enabled,
          },
        })

        const index = mcpConfigs.value.findIndex(c => c.id === id)
        if (index !== -1) {
          mcpConfigs.value[index] = transformDbMcpConfig(rawConfig)
        }
      } catch (error) {
        console.error('Failed to update SDK MCP config:', error)
        notificationStore.databaseError(
          '更新 MCP 配置失败',
          getErrorMessage(error),
          async () => { await updateMcpConfig(id, updates) }
        )
        throw error
      }
    }
  }

  /**
   * 删除 MCP 配置
   */
  async function deleteMcpConfig(id: string) {
    if (!selectedAgentId.value) return

    const notificationStore = useNotificationStore()

    if (isReadOnly.value) {
      // CLI 类型：从配置文件删除
      const config = mcpConfigs.value.find(c => c.id === id)
      if (!config) return

      try {
        await invoke('delete_cli_mcp_config', {
          cliPath: selectedAgent.value?.cliPath,
          name: config.name,
        })
        await refreshCliConfigs()
      } catch (error) {
        console.error('Failed to delete CLI MCP config:', error)
        notificationStore.databaseError(
          '删除 MCP 配置失败',
          getErrorMessage(error),
          async () => { await deleteMcpConfig(id) }
        )
        throw error
      }
    } else {
      // SDK 类型：从数据库删除
      try {
        await invoke('delete_agent_mcp_config', { id })
        const index = mcpConfigs.value.findIndex(c => c.id === id)
        if (index !== -1) {
          mcpConfigs.value.splice(index, 1)
        }
      } catch (error) {
        console.error('Failed to delete SDK MCP config:', error)
        notificationStore.databaseError(
          '删除 MCP 配置失败',
          getErrorMessage(error),
          async () => { await deleteMcpConfig(id) }
        )
        throw error
      }
    }
  }

  // ============================================================================
  // Actions - Skills 配置操作
  // ============================================================================

  async function createSkillsConfig(config: Omit<UnifiedSkillConfig, 'id' | 'source' | 'isReadOnly'>) {
    if (!selectedAgentId.value || isReadOnly.value) return

    const notificationStore = useNotificationStore()

    try {
      const rawConfig = await invoke<RawAgentSkillsConfig>('create_agent_skills_config', {
        input: {
          agent_id: selectedAgentId.value,
          name: config.name,
          description: config.description,
          skill_path: config.skillPath,
          scripts_path: config.scriptsPath,
          references_path: config.referencesPath,
          assets_path: config.assetsPath,
        },
      })
      skillsConfigs.value.push(transformDbSkillsConfig(rawConfig))
      return rawConfig
    } catch (error) {
      console.error('Failed to create Skills config:', error)
      notificationStore.databaseError(
        '创建 Skills 配置失败',
        getErrorMessage(error),
        async () => { await createSkillsConfig(config) }
      )
      throw error
    }
  }

  async function updateSkillsConfig(id: string, updates: Partial<UnifiedSkillConfig>) {
    if (!selectedAgentId.value || isReadOnly.value) return

    const notificationStore = useNotificationStore()

    try {
      const rawConfig = await invoke<RawAgentSkillsConfig>('update_agent_skills_config', {
        id,
        input: {
          name: updates.name,
          description: updates.description,
          skill_path: updates.skillPath,
          scripts_path: updates.scriptsPath,
          references_path: updates.referencesPath,
          assets_path: updates.assetsPath,
          enabled: updates.enabled,
        },
      })

      const index = skillsConfigs.value.findIndex(c => c.id === id)
      if (index !== -1) {
        skillsConfigs.value[index] = transformDbSkillsConfig(rawConfig)
      }
    } catch (error) {
      console.error('Failed to update Skills config:', error)
      notificationStore.databaseError(
        '更新 Skills 配置失败',
        getErrorMessage(error),
        async () => { await updateSkillsConfig(id, updates) }
      )
      throw error
    }
  }

  async function deleteSkillsConfig(id: string) {
    if (!selectedAgentId.value || isReadOnly.value) return

    const notificationStore = useNotificationStore()

    try {
      await invoke('delete_agent_skills_config', { id })
      const index = skillsConfigs.value.findIndex(c => c.id === id)
      if (index !== -1) {
        skillsConfigs.value.splice(index, 1)
      }
    } catch (error) {
      console.error('Failed to delete Skills config:', error)
      notificationStore.databaseError(
        '删除 Skills 配置失败',
        getErrorMessage(error),
        async () => { await deleteSkillsConfig(id) }
      )
      throw error
    }
  }

  // ============================================================================
  // Actions - Plugins 配置操作
  // ============================================================================

  async function createPluginsConfig(config: Omit<UnifiedPluginConfig, 'id' | 'source' | 'isReadOnly'>) {
    if (!selectedAgentId.value || isReadOnly.value) return

    const notificationStore = useNotificationStore()

    try {
      const rawConfig = await invoke<RawAgentPluginsConfig>('create_agent_plugins_config', {
        input: {
          agent_id: selectedAgentId.value,
          name: config.name,
          version: config.version,
          description: config.description,
          plugin_path: config.pluginPath,
        },
      })
      pluginsConfigs.value.push(transformDbPluginsConfig(rawConfig))
      return rawConfig
    } catch (error) {
      console.error('Failed to create Plugins config:', error)
      notificationStore.databaseError(
        '创建 Plugins 配置失败',
        getErrorMessage(error),
        async () => { await createPluginsConfig(config) }
      )
      throw error
    }
  }

  async function updatePluginsConfig(id: string, updates: Partial<UnifiedPluginConfig>) {
    if (!selectedAgentId.value || isReadOnly.value) return

    const notificationStore = useNotificationStore()

    try {
      const rawConfig = await invoke<RawAgentPluginsConfig>('update_agent_plugins_config', {
        id,
        input: {
          name: updates.name,
          version: updates.version,
          description: updates.description,
          plugin_path: updates.pluginPath,
          enabled: updates.enabled,
        },
      })

      const index = pluginsConfigs.value.findIndex(c => c.id === id)
      if (index !== -1) {
        pluginsConfigs.value[index] = transformDbPluginsConfig(rawConfig)
      }
    } catch (error) {
      console.error('Failed to update Plugins config:', error)
      notificationStore.databaseError(
        '更新 Plugins 配置失败',
        getErrorMessage(error),
        async () => { await updatePluginsConfig(id, updates) }
      )
      throw error
    }
  }

  async function deletePluginsConfig(id: string) {
    if (!selectedAgentId.value || isReadOnly.value) return

    const notificationStore = useNotificationStore()

    try {
      await invoke('delete_agent_plugins_config', { id })
      const index = pluginsConfigs.value.findIndex(c => c.id === id)
      if (index !== -1) {
        pluginsConfigs.value.splice(index, 1)
      }
    } catch (error) {
      console.error('Failed to delete Plugins config:', error)
      notificationStore.databaseError(
        '删除 Plugins 配置失败',
        getErrorMessage(error),
        async () => { await deletePluginsConfig(id) }
      )
      throw error
    }
  }

  // ============================================================================
  // Actions - 打开配置文件
  // ============================================================================

  /**
   * 打开配置文件编辑器
   */
  async function openConfigFile() {
    if (!cliConfigPaths.value) return

    const notificationStore = useNotificationStore()

    try {
      await invoke('open_config_file', {
        configPath: cliConfigPaths.value.configFile,
      })
    } catch (error) {
      console.error('Failed to open config file:', error)
      notificationStore.networkError(
        '打开配置文件失败',
        getErrorMessage(error),
        () => openConfigFile()
      )
      throw error
    }
  }

  // ============================================================================
  // Actions - MCP 测试相关
  // ============================================================================

  /**
   * 获取 MCP 服务器的工具列表
   */
  async function listMcpTools(config: UnifiedMcpConfig): Promise<McpToolsListResult> {
    const notificationStore = useNotificationStore()

    try {
      const result = await invoke<McpToolsListResult>('list_mcp_tools_by_config', {
        config: buildMcpConfigInput(config),
      })
      return result
    } catch (error) {
      console.error('Failed to list MCP tools:', error)
      notificationStore.networkError(
        '获取 MCP 工具列表失败',
        getErrorMessage(error),
        async () => { await listMcpTools(config) }
      )
      throw error
    }
  }

  /**
   * 调用 MCP 工具
   */
  async function callMcpTool(
    config: UnifiedMcpConfig,
    toolName: string,
    params: Record<string, unknown>
  ): Promise<McpToolCallResult> {
    const notificationStore = useNotificationStore()

    try {
      const result = await invoke<McpToolCallResult>('call_mcp_tool_by_config', {
        config: buildMcpConfigInput(config),
        toolName,
        params,
      })
      return result
    } catch (error) {
      console.error('Failed to call MCP tool:', error)
      notificationStore.networkError(
        '调用 MCP 工具失败',
        getErrorMessage(error),
        async () => { await callMcpTool(config, toolName, params) }
      )
      throw error
    }
  }

  /**
   * 设置正在测试的 MCP 配置
   */
  function setTestingMcpConfig(config: UnifiedMcpConfig) {
    testingMcpConfig.value = config
  }

  /**
   * 清除正在测试的 MCP 配置
   */
  function clearTestingMcpConfig() {
    testingMcpConfig.value = null
  }

  // ============================================================================
  // Actions - 详情视图相关
  // ============================================================================

  // 详情视图状态
  const selectedSkill = ref<UnifiedSkillConfig | null>(null)
  const selectedPlugin = ref<UnifiedPluginConfig | null>(null)

  /**
   * 查看 Skill 详情
   */
  function viewSkillDetail(skill: UnifiedSkillConfig) {
    selectedSkill.value = skill
    selectedPlugin.value = null
  }

  /**
   * 查看 Plugin 详情
   */
  function viewPluginDetail(plugin: UnifiedPluginConfig) {
    selectedPlugin.value = plugin
    selectedSkill.value = null
  }

  /**
   * 清除详情视图状态
   */
  function clearDetailState() {
    selectedSkill.value = null
    selectedPlugin.value = null
  }

  /**
   * 删除 Skill（支持 CLI 类型的文件删除）
   */
  async function deleteSkillWithFiles(skill: UnifiedSkillConfig) {
    const notificationStore = useNotificationStore()

    if (skill.source === 'file') {
      // CLI 类型：删除文件目录
      try {
        await invoke('delete_skill_directory', {
          skillPath: skill.skillPath,
        })
        // 从列表中移除
        const index = skillsConfigs.value.findIndex(c => c.id === skill.id)
        if (index !== -1) {
          skillsConfigs.value.splice(index, 1)
        }
        // 清除详情视图
        if (selectedSkill.value?.id === skill.id) {
          selectedSkill.value = null
        }
      } catch (error) {
        console.error('Failed to delete skill directory:', error)
        notificationStore.networkError(
          '删除 Skill 失败',
          getErrorMessage(error),
          async () => { await deleteSkillWithFiles(skill) }
        )
        throw error
      }
    } else {
      // SDK 类型：从数据库删除
      await deleteSkillsConfig(skill.id)
      if (selectedSkill.value?.id === skill.id) {
        selectedSkill.value = null
      }
    }
  }

  /**
   * 删除 Plugin（支持 CLI 类型的文件删除）
   */
  async function deletePluginWithFiles(plugin: UnifiedPluginConfig) {
    const notificationStore = useNotificationStore()

    if (plugin.source === 'file') {
      // CLI 类型：删除文件目录
      try {
        await invoke('delete_plugin_directory', {
          pluginPath: plugin.pluginPath,
        })
        // 从列表中移除
        const index = pluginsConfigs.value.findIndex(c => c.id === plugin.id)
        if (index !== -1) {
          pluginsConfigs.value.splice(index, 1)
        }
        // 清除详情视图
        if (selectedPlugin.value?.id === plugin.id) {
          selectedPlugin.value = null
        }
      } catch (error) {
        console.error('Failed to delete plugin directory:', error)
        notificationStore.networkError(
          '删除 Plugin 失败',
          getErrorMessage(error),
          async () => { await deletePluginWithFiles(plugin) }
        )
        throw error
      }
    } else {
      // SDK 类型：从数据库删除
      await deletePluginsConfig(plugin.id)
      if (selectedPlugin.value?.id === plugin.id) {
        selectedPlugin.value = null
      }
    }
  }

  return {
    // State
    selectedAgentId,
    selectedAgent,
    mcpConfigs,
    skillsConfigs,
    pluginsConfigs,
    configSource,
    isReadOnly,
    isLoading,
    cliConfigPaths,
    testingMcpConfig,
    cliCapabilities,

    // 详情视图状态
    selectedSkill,
    selectedPlugin,

    // Getters
    isCliAgent,
    isSdkAgent,
    supportsPlugins,

    // Actions - Agent
    selectAgent,
    clearConfigs,
    refreshCliConfigs,
    loadCliCapabilities,

    // Actions - MCP
    createMcpConfig,
    updateMcpConfig,
    deleteMcpConfig,

    // Actions - Skills
    createSkillsConfig,
    updateSkillsConfig,
    deleteSkillsConfig,

    // Actions - Plugins
    createPluginsConfig,
    updatePluginsConfig,
    deletePluginsConfig,

    // Actions - File
    openConfigFile,

    // Actions - MCP Test
    listMcpTools,
    callMcpTool,
    setTestingMcpConfig,
    clearTestingMcpConfig,

    // Actions - 详情视图
    viewSkillDetail,
    viewPluginDetail,
    clearDetailState,
    deleteSkillWithFiles,
    deletePluginWithFiles,
  }
})
