import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { getErrorMessage } from '@/utils/api'

export type StudioAgentRoleType = 'orchestrator' | 'researcher' | 'executor' | 'custom'
export type StudioAgentResourceType = 'mcp' | 'skill' | 'plugin'

export interface StudioAgentResourceBinding {
  id: string
  studioAgentId: string
  resourceType: StudioAgentResourceType
  resourceId: string
  resourceName: string
  enabled: boolean
  sortOrder: number
  createdAt: string
  updatedAt: string
}

export interface StudioAgent {
  id: string
  name: string
  description?: string
  specialization?: string
  roleType: StudioAgentRoleType
  runtimeAgentId: string
  runtimeModelId?: string
  systemPrompt?: string
  instructionNote?: string
  readonlyResearcher: boolean
  planningCapability: boolean
  executionCapability: boolean
  enabled: boolean
  createdAt: string
  updatedAt: string
  resourceBindings: StudioAgentResourceBinding[]
}

export interface StudioAgentRuntime {
  studioAgentId: string
  runtimeAgentId: string
  runtimeModelId?: string
  systemPrompt?: string
  instructionNote?: string
  readonlyResearcher: boolean
  planningCapability: boolean
  executionCapability: boolean
  enabled: boolean
  resourceBindings: StudioAgentResourceBinding[]
}

export interface AgentResourceCatalogItem {
  resourceType: StudioAgentResourceType
  resourceId: string
  name: string
  description?: string
  source?: string
  enabled: boolean
}

export interface AgentResourceCatalog {
  mcpServers: AgentResourceCatalogItem[]
  skills: AgentResourceCatalogItem[]
  plugins: AgentResourceCatalogItem[]
}

interface RawStudioAgentResourceBinding {
  id: string
  studio_agent_id: string
  resource_type: StudioAgentResourceType
  resource_id: string
  resource_name: string
  enabled: boolean
  sort_order: number
  created_at: string
  updated_at: string
}

interface RawStudioAgent {
  id: string
  name: string
  description?: string
  specialization?: string
  role_type: StudioAgentRoleType
  runtime_agent_id: string
  runtime_model_id?: string
  system_prompt?: string
  instruction_note?: string
  readonly_researcher: boolean
  planning_capability: boolean
  execution_capability: boolean
  enabled: boolean
  created_at: string
  updated_at: string
  resource_bindings: RawStudioAgentResourceBinding[]
}

interface RawStudioAgentRuntime {
  studio_agent_id: string
  runtime_agent_id: string
  runtime_model_id?: string
  system_prompt?: string
  instruction_note?: string
  readonly_researcher: boolean
  planning_capability: boolean
  execution_capability: boolean
  enabled: boolean
  resource_bindings: RawStudioAgentResourceBinding[]
}

interface RawAgentResourceCatalogItem {
  resource_type: StudioAgentResourceType
  resource_id: string
  name: string
  description?: string
  source?: string
  enabled: boolean
}

interface RawAgentResourceCatalog {
  mcp_servers: RawAgentResourceCatalogItem[]
  skills: RawAgentResourceCatalogItem[]
  plugins: RawAgentResourceCatalogItem[]
}

export interface StudioAgentBindingDraft {
  resourceType: StudioAgentResourceType
  resourceId: string
  resourceName: string
  enabled?: boolean
  sortOrder?: number
}

export interface StudioAgentDraft {
  name: string
  description?: string
  specialization?: string
  roleType?: StudioAgentRoleType
  runtimeAgentId: string
  runtimeModelId?: string
  systemPrompt?: string
  instructionNote?: string
  readonlyResearcher?: boolean
  planningCapability?: boolean
  executionCapability?: boolean
  enabled?: boolean
  resourceBindings?: StudioAgentBindingDraft[]
}

function transformBinding(raw: RawStudioAgentResourceBinding): StudioAgentResourceBinding {
  return {
    id: raw.id,
    studioAgentId: raw.studio_agent_id,
    resourceType: raw.resource_type,
    resourceId: raw.resource_id,
    resourceName: raw.resource_name,
    enabled: raw.enabled,
    sortOrder: raw.sort_order,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

function transformStudioAgent(raw: RawStudioAgent): StudioAgent {
  return {
    id: raw.id,
    name: raw.name,
    description: raw.description,
    specialization: raw.specialization,
    roleType: raw.role_type,
    runtimeAgentId: raw.runtime_agent_id,
    runtimeModelId: raw.runtime_model_id,
    systemPrompt: raw.system_prompt,
    instructionNote: raw.instruction_note,
    readonlyResearcher: raw.readonly_researcher,
    planningCapability: raw.planning_capability,
    executionCapability: raw.execution_capability,
    enabled: raw.enabled,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
    resourceBindings: (raw.resource_bindings || []).map(transformBinding)
  }
}

function transformRuntime(raw: RawStudioAgentRuntime): StudioAgentRuntime {
  return {
    studioAgentId: raw.studio_agent_id,
    runtimeAgentId: raw.runtime_agent_id,
    runtimeModelId: raw.runtime_model_id,
    systemPrompt: raw.system_prompt,
    instructionNote: raw.instruction_note,
    readonlyResearcher: raw.readonly_researcher,
    planningCapability: raw.planning_capability,
    executionCapability: raw.execution_capability,
    enabled: raw.enabled,
    resourceBindings: (raw.resource_bindings || []).map(transformBinding)
  }
}

function transformCatalogItem(raw: RawAgentResourceCatalogItem): AgentResourceCatalogItem {
  return {
    resourceType: raw.resource_type,
    resourceId: raw.resource_id,
    name: raw.name,
    description: raw.description,
    source: raw.source,
    enabled: raw.enabled
  }
}

function buildPayload(draft: StudioAgentDraft) {
  return {
    name: draft.name,
    description: draft.description ?? null,
    specialization: draft.specialization ?? null,
    role_type: draft.roleType ?? 'custom',
    runtime_agent_id: draft.runtimeAgentId,
    runtime_model_id: draft.runtimeModelId ?? null,
    system_prompt: draft.systemPrompt ?? null,
    instruction_note: draft.instructionNote ?? null,
    readonly_researcher: draft.readonlyResearcher ?? false,
    planning_capability: draft.planningCapability ?? false,
    execution_capability: draft.executionCapability ?? true,
    enabled: draft.enabled ?? true,
    resource_bindings: (draft.resourceBindings || []).map((binding, index) => ({
      resource_type: binding.resourceType,
      resource_id: binding.resourceId,
      resource_name: binding.resourceName,
      enabled: binding.enabled ?? true,
      sort_order: binding.sortOrder ?? index
    }))
  }
}

export const useAgentStudioStore = defineStore('agentStudio', () => {
  const agents = ref<StudioAgent[]>([])
  const resourceCatalog = ref<AgentResourceCatalog>({
    mcpServers: [],
    skills: [],
    plugins: []
  })
  const isLoading = ref(false)
  const isCatalogLoading = ref(false)

  const enabledAgents = computed(() => agents.value.filter(agent => agent.enabled))

  async function loadAgents() {
    isLoading.value = true
    const notificationStore = useNotificationStore()
    try {
      const rawAgents = await invoke<RawStudioAgent[]>('list_studio_agents')
      agents.value = rawAgents.map(transformStudioAgent)
      return agents.value
    } catch (error) {
      notificationStore.databaseError('加载 Agent Studio 失败', getErrorMessage(error), async () => {
        await loadAgents()
      })
      throw error
    } finally {
      isLoading.value = false
    }
  }

  async function loadResourceCatalog() {
    isCatalogLoading.value = true
    const notificationStore = useNotificationStore()
    try {
      const rawCatalog = await invoke<RawAgentResourceCatalog>('list_studio_agent_resource_catalog')
      resourceCatalog.value = {
        mcpServers: rawCatalog.mcp_servers.map(transformCatalogItem),
        skills: rawCatalog.skills.map(transformCatalogItem),
        plugins: rawCatalog.plugins.map(transformCatalogItem)
      }
      return resourceCatalog.value
    } catch (error) {
      notificationStore.networkError('加载资源目录失败', getErrorMessage(error), async () => {
        await loadResourceCatalog()
      })
      throw error
    } finally {
      isCatalogLoading.value = false
    }
  }

  async function createAgent(input: StudioAgentDraft) {
    const notificationStore = useNotificationStore()
    try {
      const rawAgent = await invoke<RawStudioAgent>('create_studio_agent', {
        input: buildPayload(input)
      })
      const agent = transformStudioAgent(rawAgent)
      agents.value.unshift(agent)
      return agent
    } catch (error) {
      notificationStore.databaseError('创建 Agent 失败', getErrorMessage(error))
      throw error
    }
  }

  async function updateAgent(id: string, input: StudioAgentDraft) {
    const notificationStore = useNotificationStore()
    try {
      const rawAgent = await invoke<RawStudioAgent>('update_studio_agent', {
        id,
        input: buildPayload(input)
      })
      const agent = transformStudioAgent(rawAgent)
      const index = agents.value.findIndex(item => item.id === id)
      if (index >= 0) {
        agents.value[index] = agent
      } else {
        agents.value.unshift(agent)
      }
      return agent
    } catch (error) {
      notificationStore.databaseError('更新 Agent 失败', getErrorMessage(error))
      throw error
    }
  }

  async function deleteAgent(id: string) {
    const notificationStore = useNotificationStore()
    try {
      await invoke('delete_studio_agent', { id })
      agents.value = agents.value.filter(item => item.id !== id)
    } catch (error) {
      notificationStore.databaseError('删除 Agent 失败', getErrorMessage(error))
      throw error
    }
  }

  async function resolveRuntime(id: string) {
    const raw = await invoke<RawStudioAgentRuntime>('resolve_studio_agent_runtime', { id })
    return transformRuntime(raw)
  }

  function getAgent(id?: string | null) {
    if (!id) return null
    return agents.value.find(agent => agent.id === id) ?? null
  }

  function getByRuntimeAgentId(runtimeAgentId?: string | null) {
    if (!runtimeAgentId) return []
    return agents.value.filter(agent => agent.runtimeAgentId === runtimeAgentId)
  }

  return {
    agents,
    enabledAgents,
    resourceCatalog,
    isLoading,
    isCatalogLoading,
    loadAgents,
    loadResourceCatalog,
    createAgent,
    updateAgent,
    deleteAgent,
    resolveRuntime,
    getAgent,
    getByRuntimeAgentId
  }
})
