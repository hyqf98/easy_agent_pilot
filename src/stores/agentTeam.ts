import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { getErrorMessage } from '@/utils/api'

export type AgentTeamMode = 'single' | 'coordinator_subagents'
export type AgentTeamMemberRole = 'coordinator' | 'research_subagent'
export type AgentCapabilityPreset = 'full' | 'readonly_research'

export interface AgentTeamMember {
  id: string
  teamId: string
  agentId: string
  modelId?: string
  role: AgentTeamMemberRole
  capabilityPreset: AgentCapabilityPreset
  displayName?: string
  sortOrder: number
  createdAt: string
  updatedAt: string
}

export interface AgentTeamRelation {
  id: string
  teamId: string
  parentMemberId: string
  childMemberId: string
  relationType: string
  autoDispatchAllowed: boolean
  createdAt: string
  updatedAt: string
}

export interface AgentTeam {
  id: string
  name: string
  description?: string
  mode: AgentTeamMode
  coordinatorAgentId?: string
  coordinatorModelId?: string
  subagentMaxConcurrent: number
  maxChildrenPerTask: number
  members: AgentTeamMember[]
  relations: AgentTeamRelation[]
  createdAt: string
  updatedAt: string
}

interface RawAgentTeamMember {
  id: string
  team_id: string
  agent_id: string
  model_id?: string
  role: AgentTeamMemberRole
  capability_preset: AgentCapabilityPreset
  display_name?: string
  sort_order: number
  created_at: string
  updated_at: string
}

interface RawAgentTeamRelation {
  id: string
  team_id: string
  parent_member_id: string
  child_member_id: string
  relation_type: string
  auto_dispatch_allowed: boolean
  created_at: string
  updated_at: string
}

interface RawAgentTeam {
  id: string
  name: string
  description?: string
  mode: AgentTeamMode
  coordinator_agent_id?: string
  coordinator_model_id?: string
  subagent_max_concurrent: number
  max_children_per_task: number
  members: RawAgentTeamMember[]
  relations: RawAgentTeamRelation[]
  created_at: string
  updated_at: string
}

export interface AgentTeamDraftMember {
  id?: string
  agentId: string
  modelId?: string
  role: AgentTeamMemberRole
  capabilityPreset: AgentCapabilityPreset
  displayName?: string
  sortOrder?: number
}

export interface AgentTeamDraftRelation {
  id?: string
  parentMemberId: string
  childMemberId: string
  relationType?: string
  autoDispatchAllowed?: boolean
}

export interface AgentTeamDraft {
  name: string
  description?: string
  mode: AgentTeamMode
  coordinatorAgentId?: string
  coordinatorModelId?: string
  subagentMaxConcurrent: number
  maxChildrenPerTask: number
  members: AgentTeamDraftMember[]
  relations: AgentTeamDraftRelation[]
}

function transformMember(raw: RawAgentTeamMember): AgentTeamMember {
  return {
    id: raw.id,
    teamId: raw.team_id,
    agentId: raw.agent_id,
    modelId: raw.model_id,
    role: raw.role,
    capabilityPreset: raw.capability_preset,
    displayName: raw.display_name,
    sortOrder: raw.sort_order,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

function transformRelation(raw: RawAgentTeamRelation): AgentTeamRelation {
  return {
    id: raw.id,
    teamId: raw.team_id,
    parentMemberId: raw.parent_member_id,
    childMemberId: raw.child_member_id,
    relationType: raw.relation_type,
    autoDispatchAllowed: raw.auto_dispatch_allowed,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

function transformTeam(raw: RawAgentTeam): AgentTeam {
  return {
    id: raw.id,
    name: raw.name,
    description: raw.description,
    mode: raw.mode,
    coordinatorAgentId: raw.coordinator_agent_id,
    coordinatorModelId: raw.coordinator_model_id,
    subagentMaxConcurrent: raw.subagent_max_concurrent,
    maxChildrenPerTask: raw.max_children_per_task,
    members: raw.members.map(transformMember),
    relations: raw.relations.map(transformRelation),
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

function toPayload(input: AgentTeamDraft) {
  return {
    name: input.name,
    description: input.description ?? null,
    mode: input.mode,
    coordinator_agent_id: input.coordinatorAgentId ?? null,
    coordinator_model_id: input.coordinatorModelId ?? null,
    subagent_max_concurrent: input.subagentMaxConcurrent,
    max_children_per_task: input.maxChildrenPerTask,
    members: input.members.map((member, index) => ({
      id: member.id ?? null,
      agent_id: member.agentId,
      model_id: member.modelId ?? null,
      role: member.role,
      capability_preset: member.capabilityPreset,
      display_name: member.displayName ?? null,
      sort_order: member.sortOrder ?? index
    })),
    relations: input.relations.map(relation => ({
      id: relation.id ?? null,
      parent_member_id: relation.parentMemberId,
      child_member_id: relation.childMemberId,
      relation_type: relation.relationType ?? 'delegates_to',
      auto_dispatch_allowed: relation.autoDispatchAllowed ?? true
    }))
  }
}

export const useAgentTeamStore = defineStore('agentTeam', () => {
  const teams = ref<AgentTeam[]>([])
  const isLoading = ref(false)

  const sortedTeams = computed(() =>
    [...teams.value].sort((left, right) =>
      new Date(right.updatedAt).getTime() - new Date(left.updatedAt).getTime()
    )
  )

  async function loadTeams() {
    isLoading.value = true
    const notificationStore = useNotificationStore()
    try {
      const rawTeams = await invoke<RawAgentTeam[]>('list_agent_teams')
      teams.value = rawTeams.map(transformTeam)
    } catch (error) {
      notificationStore.databaseError('加载 Agent 团队失败', getErrorMessage(error), loadTeams)
      throw error
    } finally {
      isLoading.value = false
    }
  }

  async function createTeam(input: AgentTeamDraft) {
    const notificationStore = useNotificationStore()
    try {
      const rawTeam = await invoke<RawAgentTeam>('create_agent_team', { input: toPayload(input) })
      const team = transformTeam(rawTeam)
      teams.value.unshift(team)
      return team
    } catch (error) {
      notificationStore.databaseError('创建 Agent 团队失败', getErrorMessage(error))
      throw error
    }
  }

  async function updateTeam(id: string, input: AgentTeamDraft) {
    const notificationStore = useNotificationStore()
    try {
      const rawTeam = await invoke<RawAgentTeam>('update_agent_team', { id, input: toPayload(input) })
      const team = transformTeam(rawTeam)
      const index = teams.value.findIndex(item => item.id === id)
      if (index >= 0) {
        teams.value[index] = team
      } else {
        teams.value.unshift(team)
      }
      return team
    } catch (error) {
      notificationStore.databaseError('更新 Agent 团队失败', getErrorMessage(error))
      throw error
    }
  }

  async function deleteTeam(id: string) {
    const notificationStore = useNotificationStore()
    try {
      await invoke('delete_agent_team', { id })
      teams.value = teams.value.filter(item => item.id !== id)
    } catch (error) {
      notificationStore.databaseError('删除 Agent 团队失败', getErrorMessage(error))
      throw error
    }
  }

  function getTeam(id?: string | null) {
    if (!id) return null
    return teams.value.find(team => team.id === id) ?? null
  }

  return {
    teams,
    isLoading,
    sortedTeams,
    loadTeams,
    createTeam,
    updateTeam,
    deleteTeam,
    getTeam
  }
})
