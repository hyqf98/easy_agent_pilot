import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { getErrorMessage } from '@/utils/api'

// ============================================================================
// 类型定义
// ============================================================================

/** MCP 传输类型 */
export type McpTransportType = 'stdio' | 'sse' | 'http'

/** MCP 配置范围 */
export type McpConfigScope = 'user' | 'local' | 'project'

/** SDK 智能体 MCP 配置 */
export interface AgentMcpConfig {
  id: string
  agentId: string
  name: string
  transportType: McpTransportType
  command?: string
  args?: string
  env?: string
  url?: string
  headers?: string
  scope: McpConfigScope
  enabled: boolean
  createdAt: string
  updatedAt: string
}

/** SDK 智能体 Skills 配置 */
export interface AgentSkillsConfig {
  id: string
  agentId: string
  name: string
  description?: string
  skillPath: string
  scriptsPath?: string
  referencesPath?: string
  assetsPath?: string
  enabled: boolean
  createdAt: string
  updatedAt: string
}

/** SDK 智能体 Plugins 配置 */
export interface AgentPluginsConfig {
  id: string
  agentId: string
  name: string
  version?: string
  description?: string
  pluginPath: string
  enabled: boolean
  createdAt: string
  updatedAt: string
}

/** SDK 智能体模型配置 */
export interface AgentModelConfig {
  id: string
  agentId: string
  modelId: string
  displayName: string
  isBuiltin: boolean
  isDefault: boolean
  sortOrder: number
  enabled: boolean
  contextWindow?: number
  createdAt: string
  updatedAt: string
}

// 后端返回的原始数据结构（snake_case）
interface RawAgentMcpConfig {
  id: string
  agent_id: string
  name: string
  transport_type: string
  command?: string
  args?: string
  env?: string
  url?: string
  headers?: string
  scope: string
  enabled: boolean
  created_at: string
  updated_at: string
}

interface RawAgentSkillsConfig {
  id: string
  agent_id: string
  name: string
  description?: string
  skill_path: string
  scripts_path?: string
  references_path?: string
  assets_path?: string
  enabled: boolean
  created_at: string
  updated_at: string
}

interface RawAgentPluginsConfig {
  id: string
  agent_id: string
  name: string
  version?: string
  description?: string
  plugin_path: string
  enabled: boolean
  created_at: string
  updated_at: string
}

interface RawAgentModelConfig {
  id: string
  agent_id: string
  model_id: string
  display_name: string
  is_builtin: boolean
  is_default: boolean
  sort_order: number
  enabled: boolean
  context_window?: number
  created_at: string
  updated_at: string
}

// ============================================================================
// 转换函数
// ============================================================================

function transformMcpConfig(raw: RawAgentMcpConfig): AgentMcpConfig {
  return {
    id: raw.id,
    agentId: raw.agent_id,
    name: raw.name,
    transportType: raw.transport_type as McpTransportType,
    command: raw.command,
    args: raw.args,
    env: raw.env,
    url: raw.url,
    headers: raw.headers,
    scope: raw.scope as McpConfigScope,
    enabled: raw.enabled,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

function transformSkillsConfig(raw: RawAgentSkillsConfig): AgentSkillsConfig {
  return {
    id: raw.id,
    agentId: raw.agent_id,
    name: raw.name,
    description: raw.description,
    skillPath: raw.skill_path,
    scriptsPath: raw.scripts_path,
    referencesPath: raw.references_path,
    assetsPath: raw.assets_path,
    enabled: raw.enabled,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

function transformPluginsConfig(raw: RawAgentPluginsConfig): AgentPluginsConfig {
  return {
    id: raw.id,
    agentId: raw.agent_id,
    name: raw.name,
    version: raw.version,
    description: raw.description,
    pluginPath: raw.plugin_path,
    enabled: raw.enabled,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

function transformModelConfig(raw: RawAgentModelConfig): AgentModelConfig {
  return {
    id: raw.id,
    agentId: raw.agent_id,
    modelId: raw.model_id,
    displayName: raw.display_name,
    isBuiltin: raw.is_builtin,
    isDefault: raw.is_default,
    sortOrder: raw.sort_order,
    enabled: raw.enabled,
    contextWindow: raw.context_window,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

// ============================================================================
// Store 定义
// ============================================================================

export const useAgentConfigStore = defineStore('agentConfig', () => {
  // State
  const mcpConfigs = ref<Map<string, AgentMcpConfig[]>>(new Map())
  const skillsConfigs = ref<Map<string, AgentSkillsConfig[]>>(new Map())
  const pluginsConfigs = ref<Map<string, AgentPluginsConfig[]>>(new Map())
  const modelConfigs = ref<Map<string, AgentModelConfig[]>>(new Map())
  const isLoading = ref(false)

  // Actions - MCP Configs

  async function loadMcpConfigs(agentId: string) {
    isLoading.value = true
    const notificationStore = useNotificationStore()
    try {
      const rawConfigs = await invoke<RawAgentMcpConfig[]>('list_agent_mcp_configs', { agentId })
      const configs = rawConfigs.map(transformMcpConfig)
      mcpConfigs.value.set(agentId, configs)
      return configs
    } catch (error) {
      console.error('Failed to load MCP configs:', error)
      notificationStore.databaseError(
        '加载 MCP 配置失败',
        getErrorMessage(error),
        async () => { void await loadMcpConfigs(agentId) }
      )
      return []
    } finally {
      isLoading.value = false
    }
  }

  async function createMcpConfig(config: Omit<AgentMcpConfig, 'id' | 'createdAt' | 'updatedAt'>) {
    const notificationStore = useNotificationStore()
    try {
      const rawConfig = await invoke<RawAgentMcpConfig>('create_agent_mcp_config', {
        input: {
          agent_id: config.agentId,
          name: config.name,
          transport_type: config.transportType,
          command: config.command,
          args: config.args,
          env: config.env,
          url: config.url,
          headers: config.headers,
          scope: config.scope
        }
      })
      const newConfig = transformMcpConfig(rawConfig)
      const configs = mcpConfigs.value.get(config.agentId) || []
      configs.push(newConfig)
      mcpConfigs.value.set(config.agentId, configs)
      return newConfig
    } catch (error) {
      console.error('Failed to create MCP config:', error)
      notificationStore.databaseError(
        '创建 MCP 配置失败',
        getErrorMessage(error),
        async () => { await createMcpConfig(config) }
      )
      throw error
    }
  }

  async function updateMcpConfig(id: string, agentId: string, updates: Partial<AgentMcpConfig>) {
    const notificationStore = useNotificationStore()
    try {
      const rawConfig = await invoke<RawAgentMcpConfig>('update_agent_mcp_config', {
        id,
        input: {
          name: updates.name,
          transport_type: updates.transportType,
          command: updates.command,
          args: updates.args,
          env: updates.env,
          url: updates.url,
          headers: updates.headers,
          scope: updates.scope,
          enabled: updates.enabled
        }
      })
      const updatedConfig = transformMcpConfig(rawConfig)
      const configs = mcpConfigs.value.get(agentId) || []
      const index = configs.findIndex(c => c.id === id)
      if (index !== -1) {
        configs[index] = updatedConfig
        mcpConfigs.value.set(agentId, [...configs])
      }
      return updatedConfig
    } catch (error) {
      console.error('Failed to update MCP config:', error)
      notificationStore.databaseError(
        '更新 MCP 配置失败',
        getErrorMessage(error),
        async () => { void await updateMcpConfig(id, agentId, updates) }
      )
      throw error
    }
  }

  async function deleteMcpConfig(id: string, agentId: string) {
    const notificationStore = useNotificationStore()
    try {
      await invoke('delete_agent_mcp_config', { id })
      const configs = mcpConfigs.value.get(agentId) || []
      const index = configs.findIndex(c => c.id === id)
      if (index !== -1) {
        configs.splice(index, 1)
        mcpConfigs.value.set(agentId, [...configs])
      }
    } catch (error) {
      console.error('Failed to delete MCP config:', error)
      notificationStore.databaseError(
        '删除 MCP 配置失败',
        getErrorMessage(error),
        async () => { void await deleteMcpConfig(id, agentId) }
      )
      throw error
    }
  }

  function getMcpConfigs(agentId: string): AgentMcpConfig[] {
    return mcpConfigs.value.get(agentId) || []
  }

  // Actions - Skills Configs

  async function loadSkillsConfigs(agentId: string) {
    isLoading.value = true
    const notificationStore = useNotificationStore()
    try {
      const rawConfigs = await invoke<RawAgentSkillsConfig[]>('list_agent_skills_configs', { agentId })
      const configs = rawConfigs.map(transformSkillsConfig)
      skillsConfigs.value.set(agentId, configs)
      return configs
    } catch (error) {
      console.error('Failed to load Skills configs:', error)
      notificationStore.databaseError(
        '加载 Skills 配置失败',
        getErrorMessage(error),
        async () => { void await loadSkillsConfigs(agentId) }
      )
      return []
    } finally {
      isLoading.value = false
    }
  }

  async function createSkillsConfig(config: Omit<AgentSkillsConfig, 'id' | 'createdAt' | 'updatedAt'>) {
    const notificationStore = useNotificationStore()
    try {
      const rawConfig = await invoke<RawAgentSkillsConfig>('create_agent_skills_config', {
        input: {
          agent_id: config.agentId,
          name: config.name,
          description: config.description,
          skill_path: config.skillPath,
          scripts_path: config.scriptsPath,
          references_path: config.referencesPath,
          assets_path: config.assetsPath
        }
      })
      const newConfig = transformSkillsConfig(rawConfig)
      const configs = skillsConfigs.value.get(config.agentId) || []
      configs.push(newConfig)
      skillsConfigs.value.set(config.agentId, configs)
      return newConfig
    } catch (error) {
      console.error('Failed to create Skills config:', error)
      notificationStore.databaseError(
        '创建 Skills 配置失败',
        getErrorMessage(error),
        async () => { void await createSkillsConfig(config) }
      )
      throw error
    }
  }

  async function updateSkillsConfig(id: string, agentId: string, updates: Partial<AgentSkillsConfig>) {
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
          enabled: updates.enabled
        }
      })
      const updatedConfig = transformSkillsConfig(rawConfig)
      const configs = skillsConfigs.value.get(agentId) || []
      const index = configs.findIndex(c => c.id === id)
      if (index !== -1) {
        configs[index] = updatedConfig
        skillsConfigs.value.set(agentId, [...configs])
      }
      return updatedConfig
    } catch (error) {
      console.error('Failed to update Skills config:', error)
      notificationStore.databaseError(
        '更新 Skills 配置失败',
        getErrorMessage(error),
        async () => { void await updateSkillsConfig(id, agentId, updates) }
      )
      throw error
    }
  }

  async function deleteSkillsConfig(id: string, agentId: string) {
    const notificationStore = useNotificationStore()
    try {
      await invoke('delete_agent_skills_config', { id })
      const configs = skillsConfigs.value.get(agentId) || []
      const index = configs.findIndex(c => c.id === id)
      if (index !== -1) {
        configs.splice(index, 1)
        skillsConfigs.value.set(agentId, [...configs])
      }
    } catch (error) {
      console.error('Failed to delete Skills config:', error)
      notificationStore.databaseError(
        '删除 Skills 配置失败',
        getErrorMessage(error),
        async () => { void await deleteSkillsConfig(id, agentId) }
      )
      throw error
    }
  }

  function getSkillsConfigs(agentId: string): AgentSkillsConfig[] {
    return skillsConfigs.value.get(agentId) || []
  }

  // Actions - Plugins Configs

  async function loadPluginsConfigs(agentId: string) {
    isLoading.value = true
    const notificationStore = useNotificationStore()
    try {
      const rawConfigs = await invoke<RawAgentPluginsConfig[]>('list_agent_plugins_configs', { agentId })
      const configs = rawConfigs.map(transformPluginsConfig)
      pluginsConfigs.value.set(agentId, configs)
      return configs
    } catch (error) {
      console.error('Failed to load Plugins configs:', error)
      notificationStore.databaseError(
        '加载 Plugins 配置失败',
        getErrorMessage(error),
        async () => { void await loadPluginsConfigs(agentId) }
      )
      return []
    } finally {
      isLoading.value = false
    }
  }

  async function createPluginsConfig(config: Omit<AgentPluginsConfig, 'id' | 'createdAt' | 'updatedAt'>) {
    const notificationStore = useNotificationStore()
    try {
      const rawConfig = await invoke<RawAgentPluginsConfig>('create_agent_plugins_config', {
        input: {
          agent_id: config.agentId,
          name: config.name,
          version: config.version,
          description: config.description,
          plugin_path: config.pluginPath
        }
      })
      const newConfig = transformPluginsConfig(rawConfig)
      const configs = pluginsConfigs.value.get(config.agentId) || []
      configs.push(newConfig)
      pluginsConfigs.value.set(config.agentId, configs)
      return newConfig
    } catch (error) {
      console.error('Failed to create Plugins config:', error)
      notificationStore.databaseError(
        '创建 Plugins 配置失败',
        getErrorMessage(error),
        async () => { void await createPluginsConfig(config) }
      )
      throw error
    }
  }

  async function updatePluginsConfig(id: string, agentId: string, updates: Partial<AgentPluginsConfig>) {
    const notificationStore = useNotificationStore()
    try {
      const rawConfig = await invoke<RawAgentPluginsConfig>('update_agent_plugins_config', {
        id,
        input: {
          name: updates.name,
          version: updates.version,
          description: updates.description,
          plugin_path: updates.pluginPath,
          enabled: updates.enabled
        }
      })
      const updatedConfig = transformPluginsConfig(rawConfig)
      const configs = pluginsConfigs.value.get(agentId) || []
      const index = configs.findIndex(c => c.id === id)
      if (index !== -1) {
        configs[index] = updatedConfig
        pluginsConfigs.value.set(agentId, [...configs])
      }
      return updatedConfig
    } catch (error) {
      console.error('Failed to update Plugins config:', error)
      notificationStore.databaseError(
        '更新 Plugins 配置失败',
        getErrorMessage(error),
        async () => { void await updatePluginsConfig(id, agentId, updates) }
      )
      throw error
    }
  }

  async function deletePluginsConfig(id: string, agentId: string) {
    const notificationStore = useNotificationStore()
    try {
      await invoke('delete_agent_plugins_config', { id })
      const configs = pluginsConfigs.value.get(agentId) || []
      const index = configs.findIndex(c => c.id === id)
      if (index !== -1) {
        configs.splice(index, 1)
        pluginsConfigs.value.set(agentId, [...configs])
      }
    } catch (error) {
      console.error('Failed to delete Plugins config:', error)
      notificationStore.databaseError(
        '删除 Plugins 配置失败',
        getErrorMessage(error),
        async () => { void await deletePluginsConfig(id, agentId) }
      )
      throw error
    }
  }

  function getPluginsConfigs(agentId: string): AgentPluginsConfig[] {
    return pluginsConfigs.value.get(agentId) || []
  }

  // ============================================================================
  // 模型配置 Actions
  // ============================================================================

  async function loadModelsConfigs(agentId: string) {
    isLoading.value = true
    const notificationStore = useNotificationStore()
    try {
      const rawConfigs = await invoke<RawAgentModelConfig[]>('list_agent_models', { agentId })
      const configs = rawConfigs.map(transformModelConfig)
      modelConfigs.value.set(agentId, configs)
      return configs
    } catch (error) {
      console.error('Failed to load model configs:', error)
      notificationStore.databaseError(
        '加载模型配置失败',
        getErrorMessage(error),
        async () => { void await loadModelsConfigs(agentId) }
      )
      return []
    } finally {
      isLoading.value = false
    }
  }

  async function ensureModelsConfigs(agentId: string, provider?: string) {
    const configs = await loadModelsConfigs(agentId)
    if (!provider) {
      return configs
    }

    try {
      return await initBuiltinModels(agentId, provider)
    } catch (error) {
      console.error('Failed to ensure model configs:', error)
      return getModelsConfigs(agentId)
    }
  }

  async function createModelConfig(config: Omit<AgentModelConfig, 'id' | 'createdAt' | 'updatedAt'>) {
    const notificationStore = useNotificationStore()
    try {
      const rawConfig = await invoke<RawAgentModelConfig>('create_agent_model', {
        input: {
          agent_id: config.agentId,
          model_id: config.modelId,
          display_name: config.displayName,
          is_builtin: config.isBuiltin,
          is_default: config.isDefault,
          sort_order: config.sortOrder,
          context_window: config.contextWindow
        }
      })
      const newConfig = transformModelConfig(rawConfig)
      const configs = modelConfigs.value.get(config.agentId) || []

      // 本地缓存保持与后端一致：仅允许一个默认模型
      const nextConfigs = config.isDefault
        ? configs.map(c => ({ ...c, isDefault: false }))
        : [...configs]

      nextConfigs.push(newConfig)
      modelConfigs.value.set(config.agentId, nextConfigs)
      return newConfig
    } catch (error) {
      console.error('Failed to create model config:', error)
      notificationStore.databaseError(
        '创建模型配置失败',
        getErrorMessage(error),
        async () => { await createModelConfig(config) }
      )
      throw error
    }
  }

  async function initBuiltinModels(agentId: string, provider: string) {
    const notificationStore = useNotificationStore()
    try {
      const rawConfigs = await invoke<RawAgentModelConfig[]>('create_builtin_models', {
        input: {
          agent_id: agentId,
          provider
        }
      })
      const configs = rawConfigs.map(transformModelConfig)
      modelConfigs.value.set(agentId, configs)
      return configs
    } catch (error) {
      console.error('Failed to init builtin models:', error)
      notificationStore.databaseError(
        '初始化内置模型失败',
        getErrorMessage(error),
        async () => { await initBuiltinModels(agentId, provider) }
      )
      throw error
    }
  }

  async function updateModelConfig(id: string, agentId: string, updates: Partial<AgentModelConfig>) {
    const notificationStore = useNotificationStore()
    try {
      const rawConfig = await invoke<RawAgentModelConfig>('update_agent_model', {
        id,
        input: {
          model_id: updates.modelId,
          display_name: updates.displayName,
          is_default: updates.isDefault,
          sort_order: updates.sortOrder,
          enabled: updates.enabled,
          context_window: updates.contextWindow
        }
      })
      const updatedConfig = transformModelConfig(rawConfig)
      const configs = modelConfigs.value.get(agentId) || []
      const index = configs.findIndex(c => c.id === id)
      if (index !== -1) {
        const nextConfigs = [...configs]
        nextConfigs[index] = updatedConfig

        // 本地缓存保持与后端一致：设置默认时重置其它默认标记
        if (updates.isDefault === true) {
          modelConfigs.value.set(
            agentId,
            nextConfigs.map(c => (c.id === id ? c : { ...c, isDefault: false }))
          )
        } else {
          modelConfigs.value.set(agentId, nextConfigs)
        }
      }
      return updatedConfig
    } catch (error) {
      console.error('Failed to update model config:', error)
      notificationStore.databaseError(
        '更新模型配置失败',
        getErrorMessage(error),
        async () => { void await updateModelConfig(id, agentId, updates) }
      )
      throw error
    }
  }

  async function deleteModelConfig(id: string, agentId: string) {
    const notificationStore = useNotificationStore()
    try {
      await invoke('delete_agent_model', { id })
      const configs = modelConfigs.value.get(agentId) || []
      const index = configs.findIndex(c => c.id === id)
      if (index !== -1) {
        configs.splice(index, 1)
        modelConfigs.value.set(agentId, [...configs])
      }
    } catch (error) {
      console.error('Failed to delete model config:', error)
      notificationStore.databaseError(
        '删除模型配置失败',
        getErrorMessage(error),
        async () => { void await deleteModelConfig(id, agentId) }
      )
      throw error
    }
  }

  // 获取模型配置
  function getModelsConfigs(agentId: string) {
    return modelConfigs.value.get(agentId) || []
  }

  // 加载所有配置
  async function loadAllConfigs(agentId: string) {
    await Promise.all([
      loadMcpConfigs(agentId),
      loadSkillsConfigs(agentId),
      loadPluginsConfigs(agentId),
      loadModelsConfigs(agentId)
    ])
  }

  // 清除指定智能体的配置缓存
  function clearConfigs(agentId: string) {
    mcpConfigs.value.delete(agentId)
    skillsConfigs.value.delete(agentId)
    pluginsConfigs.value.delete(agentId)
    modelConfigs.value.delete(agentId)
  }

  return {
    // State
    mcpConfigs,
    skillsConfigs,
    pluginsConfigs,
    modelConfigs,
    isLoading,
    // MCP Actions
    loadMcpConfigs,
    createMcpConfig,
    updateMcpConfig,
    deleteMcpConfig,
    getMcpConfigs,
    // Skills Actions
    loadSkillsConfigs,
    createSkillsConfig,
    updateSkillsConfig,
    deleteSkillsConfig,
    getSkillsConfigs,
    // Plugins Actions
    loadPluginsConfigs,
    createPluginsConfig,
    updatePluginsConfig,
    deletePluginsConfig,
    getPluginsConfigs,
    // Model Actions
    loadModelsConfigs,
    ensureModelsConfigs,
    createModelConfig,
    initBuiltinModels,
    updateModelConfig,
    deleteModelConfig,
    getModelsConfigs,
    // General
    loadAllConfigs,
    clearConfigs
  }
})
