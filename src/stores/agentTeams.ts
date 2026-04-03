import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { getErrorMessage } from '@/utils/api'

export type AgentExpertCategory =
  | 'general'
  | 'planner'
  | 'architect'
  | 'developer'
  | 'tester'
  | 'writer'
  | 'designer'
  | 'reviewer'
  | 'ops'
  | 'custom'

export interface AgentExpert {
  id: string
  builtinCode?: string
  name: string
  description?: string
  prompt: string
  runtimeAgentId?: string
  defaultModelId?: string
  category: AgentExpertCategory
  tags: string[]
  recommendedScenes: string[]
  isBuiltin: boolean
  isEnabled: boolean
  sortOrder: number
  createdAt: string
  updatedAt: string
}

interface RawAgentExpert {
  id: string
  builtin_code?: string
  name: string
  description?: string
  prompt: string
  runtime_agent_id?: string
  default_model_id?: string
  category: string
  tags?: string[]
  recommended_scenes?: string[]
  is_builtin: boolean
  is_enabled: boolean
  sort_order: number
  created_at: string
  updated_at: string
}

export interface AgentExpertReferenceSummary {
  plans: number
  tasks: number
  sessions: number
}

function transformExpert(raw: RawAgentExpert): AgentExpert {
  return {
    id: raw.id,
    builtinCode: raw.builtin_code,
    name: raw.name,
    description: raw.description,
    prompt: raw.prompt,
    runtimeAgentId: raw.runtime_agent_id,
    defaultModelId: raw.default_model_id,
    category: (raw.category || 'custom') as AgentExpertCategory,
    tags: raw.tags || [],
    recommendedScenes: raw.recommended_scenes || [],
    isBuiltin: raw.is_builtin,
    isEnabled: raw.is_enabled,
    sortOrder: raw.sort_order,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

export const useAgentTeamsStore = defineStore('agentTeams', () => {
  const experts = ref<AgentExpert[]>([])
  const isLoading = ref(false)
  const selectedExpertId = ref<string | null>(null)

  const enabledExperts = computed(() => experts.value.filter(expert => expert.isEnabled))
  const selectedExpert = computed(() =>
    experts.value.find(expert => expert.id === selectedExpertId.value) || null
  )

  const builtinGeneralExpert = computed(() =>
    experts.value.find(expert => expert.builtinCode === 'builtin-general') || null
  )
  const builtinSoloCoordinatorExpert = computed(() =>
    experts.value.find(expert => expert.builtinCode === 'builtin-solo-coordinator') || null
  )
  const builtinPlannerExpert = computed(() =>
    experts.value.find(expert => expert.builtinCode === 'builtin-planner') || null
  )
  const builtinDeveloperExpert = computed(() =>
    experts.value.find(expert => expert.builtinCode === 'builtin-developer') || null
  )
  const builtinArchitectExpert = computed(() =>
    experts.value.find(expert => expert.builtinCode === 'builtin-architect') || null
  )
  const builtinTesterExpert = computed(() =>
    experts.value.find(expert => expert.builtinCode === 'builtin-tester') || null
  )
  const builtinWriterExpert = computed(() =>
    experts.value.find(expert => expert.builtinCode === 'builtin-writer') || null
  )
  const builtinDesignerExpert = computed(() =>
    experts.value.find(expert => expert.builtinCode === 'builtin-designer') || null
  )
  const builtinReviewerExpert = computed(() =>
    experts.value.find(expert => expert.builtinCode === 'builtin-reviewer') || null
  )
  const builtinOpsExpert = computed(() =>
    experts.value.find(expert => expert.builtinCode === 'builtin-ops') || null
  )

  async function loadExperts(force = false): Promise<AgentExpert[]> {
    if (isLoading.value) {
      return experts.value
    }
    if (!force && experts.value.length > 0) {
      return experts.value
    }

    const notificationStore = useNotificationStore()
    isLoading.value = true
    try {
      await invoke('seed_builtin_agent_experts')
      const rawExperts = await invoke<RawAgentExpert[]>('list_agent_experts')
      experts.value = rawExperts.map(transformExpert)

      if (selectedExpertId.value && !experts.value.some(expert => expert.id === selectedExpertId.value)) {
        selectedExpertId.value = null
      }

      return experts.value
    } catch (error) {
      notificationStore.databaseError(
        '加载 AgentTeams 失败',
        getErrorMessage(error),
        async () => { await loadExperts(true) }
      )
      experts.value = []
      throw error
    } finally {
      isLoading.value = false
    }
  }

  async function createExpert(input: Omit<AgentExpert, 'id' | 'builtinCode' | 'isBuiltin' | 'createdAt' | 'updatedAt'>) {
    const notificationStore = useNotificationStore()
    try {
      const rawExpert = await invoke<RawAgentExpert>('create_agent_expert', {
        input: {
          name: input.name,
          description: input.description,
          prompt: input.prompt,
          runtime_agent_id: input.runtimeAgentId ?? null,
          default_model_id: input.defaultModelId ?? null,
          category: input.category,
          tags: input.tags,
          recommended_scenes: input.recommendedScenes,
          is_enabled: input.isEnabled,
          sort_order: input.sortOrder
        }
      })
      const expert = transformExpert(rawExpert)
      experts.value = [expert, ...experts.value.filter(item => item.id !== expert.id)]
      return expert
    } catch (error) {
      notificationStore.databaseError('创建专家失败', getErrorMessage(error))
      throw error
    }
  }

  async function updateExpert(id: string, updates: Partial<Omit<AgentExpert, 'id' | 'builtinCode' | 'isBuiltin' | 'createdAt' | 'updatedAt'>>) {
    const notificationStore = useNotificationStore()
    try {
      const rawExpert = await invoke<RawAgentExpert>('update_agent_expert', {
        id,
        input: {
          name: updates.name,
          description: updates.description,
          prompt: updates.prompt,
          runtime_agent_id: updates.runtimeAgentId,
          default_model_id: updates.defaultModelId,
          category: updates.category,
          tags: updates.tags,
          recommended_scenes: updates.recommendedScenes,
          is_enabled: updates.isEnabled,
          sort_order: updates.sortOrder
        }
      })
      const expert = transformExpert(rawExpert)
      const index = experts.value.findIndex(item => item.id === id)
      if (index >= 0) {
        experts.value[index] = expert
      }
      return expert
    } catch (error) {
      notificationStore.databaseError('更新专家失败', getErrorMessage(error))
      throw error
    }
  }

  async function countReferences(id: string): Promise<AgentExpertReferenceSummary> {
    return invoke<AgentExpertReferenceSummary>('count_agent_expert_references', { id })
  }

  async function deleteExpert(id: string): Promise<void> {
    const notificationStore = useNotificationStore()
    try {
      await invoke('delete_agent_expert', { id })
      experts.value = experts.value.filter(expert => expert.id !== id)
      if (selectedExpertId.value === id) {
        selectedExpertId.value = null
      }
    } catch (error) {
      notificationStore.databaseError('删除专家失败', getErrorMessage(error))
      throw error
    }
  }

  function setSelectedExpert(id: string | null) {
    selectedExpertId.value = id
  }

  function getExpertById(id?: string | null): AgentExpert | null {
    if (!id) return null
    return experts.value.find(expert => expert.id === id) || null
  }

  return {
    experts,
    enabledExperts,
    isLoading,
    selectedExpertId,
    selectedExpert,
    builtinGeneralExpert,
    builtinSoloCoordinatorExpert,
    builtinPlannerExpert,
    builtinDeveloperExpert,
    builtinArchitectExpert,
    builtinTesterExpert,
    builtinWriterExpert,
    builtinDesignerExpert,
    builtinReviewerExpert,
    builtinOpsExpert,
    loadExperts,
    createExpert,
    updateExpert,
    countReferences,
    deleteExpert,
    setSelectedExpert,
    getExpertById
  }
})
