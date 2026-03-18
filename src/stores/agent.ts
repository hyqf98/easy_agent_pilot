import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { useAgentConfigStore } from './agentConfig'
import { getErrorMessage } from '@/utils/api'
// 智能体类型: cli 或 sdk
export type AgentType = 'cli' | 'sdk'
// 提供商: claude 或 codex
export type AgentProvider = 'claude' | 'codex'
// 智能体状态
export type AgentStatus = 'online' | 'offline' | 'error' | 'testing'

/**
 * 统一智能体配置接口
 * 支持 CLI 和 SDK 两种类型的智能体配置
 */
export interface AgentConfig {
  id: string
  name: string
  /** 智能体类型: cli 或 sdk */
  type: AgentType
  /** 提供商: claude 或 codex */
  provider?: AgentProvider
  /** CLI 可执行文件路径 (CLI 类型专用) */
  cliPath?: string
  /** API 密钥 (SDK 类型专用) */
  apiKey?: string
  /** API 端点 (SDK 类型专用) */
  baseUrl?: string
  /** 模型 ID */
  modelId?: string
  /** 是否启用自定义模型 */
  customModelEnabled?: boolean
  /** 兼容旧字段: 模式 */
  mode?: string
  /** 兼容旧字段: 模型 */
  model?: string
  status?: AgentStatus
  testMessage?: string
  testedAt?: string
  createdAt: string
  updatedAt: string
}

export function inferAgentProvider(
  agent?: Pick<AgentConfig, 'provider' | 'name' | 'cliPath'> | null
): AgentProvider | undefined {
  if (!agent) {
    return undefined
  }

  if (agent.provider === 'claude' || agent.provider === 'codex') {
    return agent.provider
  }

  const hint = `${agent.name || ''} ${agent.cliPath || ''}`.toLowerCase()
  if (hint.includes('claude')) {
    return 'claude'
  }
  if (hint.includes('codex')) {
    return 'codex'
  }

  return undefined
}

// 后端返回的原始数据结构（snake_case）
interface RawAgentData {
  id: string
  name: string
  type: string
  provider?: string
  cli_path?: string
  api_key?: string
  base_url?: string
  model_id?: string
  custom_model_enabled?: boolean
  mode?: string
  model?: string
  status?: string
  test_message?: string
  tested_at?: string
  created_at: string
  updated_at: string
}

// 测试连接结果
interface TestConnectionResult {
  success: boolean
  message: string
}

// 从 settings store 导入 CLI 相关类型并重新导出
import type { CliTool, DetectionResult, CliStatus } from './settings'
export type { CliTool, DetectionResult, CliStatus }

// 将后端数据转换为前端格式
function transformAgent(raw: RawAgentData): AgentConfig {
  return {
    id: raw.id,
    name: raw.name,
    type: raw.type as AgentType,
    provider: raw.provider as AgentProvider | undefined,
    cliPath: raw.cli_path,
    apiKey: raw.api_key,
    baseUrl: raw.base_url,
    modelId: raw.model_id,
    customModelEnabled: raw.custom_model_enabled,
    mode: raw.mode,
    model: raw.model,
    status: (raw.status || 'offline') as AgentStatus,
    testMessage: raw.test_message,
    testedAt: raw.tested_at,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

export const useAgentStore = defineStore('agent', () => {
  // State
  const agents = ref<AgentConfig[]>([])
  const currentAgentId = ref<string | null>(null)
  const isLoading = ref(false)
  const testingAgentId = ref<string | null>(null)
  const detectedTools = ref<CliTool[]>([])
  const isScanning = ref(false)

  // Getters
  const currentAgent = computed(() =>
    agents.value.find(a => a.id === currentAgentId.value)
  )

  const agentsByType = computed(() => {
    return (type: AgentType) =>
      agents.value.filter(a => a.type === type)
  })

  const agentsByProvider = computed(() => {
    return (provider: AgentProvider) =>
      agents.value.filter(a => a.provider === provider)
  })

  // 未添加的可用 CLI 工具
  const availableToolsToAdd = computed(() => {
    return detectedTools.value.filter(tool => {
      if (tool.status !== 'available') return false
      // 检查是否已存在相同路径的智能体
      return !agents.value.some(agent =>
        agent.type === 'cli' && agent.cliPath === tool.path
      )
    })
  })

  // Actions
  async function loadAgents() {
    isLoading.value = true
    const notificationStore = useNotificationStore()
    try {
      const rawAgents = await invoke<RawAgentData[]>('list_agents')
      agents.value = rawAgents.map(transformAgent)
    } catch (error) {
      console.error('Failed to load agents:', error)
      agents.value = []
      notificationStore.networkError(
        '加载智能体列表',
        getErrorMessage(error),
        loadAgents
      )
    } finally {
      isLoading.value = false
    }
  }

  async function createAgent(agent: Omit<AgentConfig, 'id' | 'createdAt' | 'updatedAt' | 'status'>) {
    const notificationStore = useNotificationStore()
    const agentConfigStore = useAgentConfigStore()

    try {
      const rawAgent = await invoke<RawAgentData>('create_agent', {
        input: {
          name: agent.name,
          type: agent.type,
          provider: agent.provider,
          cli_path: agent.cliPath,
          api_key: agent.apiKey,
          base_url: agent.baseUrl,
          model_id: agent.modelId,
          custom_model_enabled: agent.customModelEnabled,
          mode: agent.mode,
          model: agent.model
        }
      })
      const newAgent = transformAgent(rawAgent)
      agents.value.push(newAgent)

      // 初始化内置模型
      if (agent.provider) {
        try {
          await agentConfigStore.initBuiltinModels(newAgent.id, agent.provider)
        } catch (error) {
          console.error('Failed to init builtin models:', error)
        }
      }
      return newAgent
    } catch (error) {
      console.error('Failed to create agent:', error)
      notificationStore.databaseError(
        '创建智能体失败',
        getErrorMessage(error),
        async () => { await createAgent(agent) }
      )
      throw error
    }
  }

  async function updateAgent(id: string, updates: Partial<AgentConfig>) {
    const notificationStore = useNotificationStore()

    try {
      const rawAgent = await invoke<RawAgentData>('update_agent', {
        id,
        input: {
          name: updates.name,
          type: updates.type,
          provider: updates.provider,
          cli_path: updates.cliPath,
          api_key: updates.apiKey,
          base_url: updates.baseUrl,
          model_id: updates.modelId,
          custom_model_enabled: updates.customModelEnabled,
          mode: updates.mode,
          model: updates.model,
          status: updates.status
        }
      })
      const index = agents.value.findIndex(a => a.id === id)
      if (index !== -1) {
        agents.value[index] = transformAgent(rawAgent)
      }
    } catch (error) {
      console.error('Failed to update agent:', error)
      notificationStore.databaseError(
        '更新智能体失败',
        getErrorMessage(error),
        () => updateAgent(id, updates)
      )
      throw error
    }
  }

  async function deleteAgent(id: string) {
    const notificationStore = useNotificationStore()

    try {
      await invoke('delete_agent', { id })
      const index = agents.value.findIndex(a => a.id === id)
      if (index !== -1) {
        agents.value.splice(index, 1)
      }
      if (currentAgentId.value === id) {
        currentAgentId.value = null
      }
    } catch (error) {
      console.error('Failed to delete agent:', error)
      notificationStore.databaseError(
        '删除智能体失败',
        getErrorMessage(error),
        () => deleteAgent(id)
      )
      throw error
    }
  }

  function setCurrentAgent(id: string | null) {
    currentAgentId.value = id
  }

  async function testConnection(id: string): Promise<{ success: boolean; message: string }> {
    const agent = agents.value.find(a => a.id === id)
    if (!agent) {
      return { success: false, message: '智能体不存在' }
    }

    testingAgentId.value = id

    // 先更新前端状态为 testing
    const index = agents.value.findIndex(a => a.id === id)
    if (index !== -1) {
      agents.value[index] = { ...agents.value[index], status: 'testing' }
    }

    try {
      // 调用 Tauri 命令测试连接
      const result = await invoke<TestConnectionResult>('test_agent_connection', { id })

      // 更新前端状态
      if (index !== -1) {
        const rawAgent = await invoke<RawAgentData>('update_agent', {
          id,
          input: {
            status: result.success ? 'online' : 'error'
          }
        })
        agents.value[index] = transformAgent(rawAgent)
      }

      return result
    } catch (error) {
      // 更新状态为 error
      if (index !== -1) {
        agents.value[index] = { ...agents.value[index], status: 'error' }
      }
      return { success: false, message: String(error) }
    } finally {
      testingAgentId.value = null
    }
  }

  // 扫描系统中的 CLI 工具
  async function scanCliTools() {
    isScanning.value = true
    try {
      const result = await invoke<DetectionResult>('detect_cli_tools')
      detectedTools.value = result.tools.map(tool => ({
        name: tool.name,
        path: tool.path,
        version: tool.version,
        status: tool.status as CliStatus
      }))
    } catch (error) {
      console.error('Failed to scan CLI tools:', error)
      detectedTools.value = []
    } finally {
      isScanning.value = false
    }
  }

  // 快速添加检测到的 CLI 工具
  async function addDetectedTool(tool: CliTool) {
    const provider = tool.name.toLowerCase().includes('claude') ? 'claude' as AgentProvider
                  : tool.name.toLowerCase().includes('codex') ? 'codex' as AgentProvider
                  : undefined

    const name = tool.name === 'claude' ? 'Claude CLI'
               : tool.name === 'codex' ? 'Codex CLI'
               : tool.name

    return await createAgent({
      name,
      type: 'cli',
      provider,
      cliPath: tool.path
    })
  }

  return {
    // State
    agents,
    currentAgentId,
    isLoading,
    testingAgentId,
    detectedTools,
    isScanning,
    // Getters
    currentAgent,
    agentsByType,
    agentsByProvider,
    availableToolsToAdd,
    // Actions
    loadAgents,
    createAgent,
    updateAgent,
    deleteAgent,
    setCurrentAgent,
    testConnection,
    scanCliTools,
    addDetectedTool
  }
})
