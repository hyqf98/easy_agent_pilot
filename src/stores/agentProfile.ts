import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { McpServerConfig } from '@/services/conversation/strategies/types'
import { useNotificationStore } from './notification'
import { getErrorMessage } from '@/utils/api'

export interface AgentProfile {
  agentId: string
  systemPrompt?: string
  roleDefinition?: string
  workingStyle?: string
  outputStyle?: string
  toolUsagePolicy?: string
  domainTags: string[]
  capabilityTags: string[]
  readonlyResearcher: boolean
  planningCapability: boolean
  executionCapability: boolean
  defaultExecutionMode?: string
  createdAt: string
  updatedAt: string
}

export interface AgentExecutionProfile extends AgentProfile {
  mcpServers: McpServerConfig[]
  skillNames: string[]
  pluginNames: string[]
}

interface RawAgentProfile {
  agent_id: string
  system_prompt?: string
  role_definition?: string
  working_style?: string
  output_style?: string
  tool_usage_policy?: string
  domain_tags?: string[]
  capability_tags?: string[]
  readonly_researcher?: boolean
  planning_capability?: boolean
  execution_capability?: boolean
  default_execution_mode?: string
  created_at: string
  updated_at: string
}

interface RawAgentExecutionProfile extends RawAgentProfile {
  mcp_servers?: Array<{
    id: string
    name: string
    transport_type: 'stdio' | 'sse' | 'http' | 'builtin'
    command?: string
    args?: string
    env?: string
    url?: string
    headers?: string
  }>
  skill_names?: string[]
  plugin_names?: string[]
}

export interface AgentProfileDraft {
  systemPrompt?: string
  roleDefinition?: string
  workingStyle?: string
  outputStyle?: string
  toolUsagePolicy?: string
  domainTags?: string[]
  capabilityTags?: string[]
  readonlyResearcher?: boolean
  planningCapability?: boolean
  executionCapability?: boolean
  defaultExecutionMode?: string
}

function createDefaultProfile(agentId: string): AgentProfile {
  return {
    agentId,
    domainTags: [],
    capabilityTags: [],
    readonlyResearcher: false,
    planningCapability: false,
    executionCapability: true,
    createdAt: '',
    updatedAt: ''
  }
}

function transformProfile(raw: RawAgentProfile): AgentProfile {
  return {
    agentId: raw.agent_id,
    systemPrompt: raw.system_prompt,
    roleDefinition: raw.role_definition,
    workingStyle: raw.working_style,
    outputStyle: raw.output_style,
    toolUsagePolicy: raw.tool_usage_policy,
    domainTags: raw.domain_tags ?? [],
    capabilityTags: raw.capability_tags ?? [],
    readonlyResearcher: raw.readonly_researcher ?? false,
    planningCapability: raw.planning_capability ?? false,
    executionCapability: raw.execution_capability ?? true,
    defaultExecutionMode: raw.default_execution_mode,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

function transformExecutionProfile(raw: RawAgentExecutionProfile): AgentExecutionProfile {
  return {
    ...transformProfile(raw),
    mcpServers: (raw.mcp_servers ?? []).map(server => ({
      id: server.id,
      name: server.name,
      transportType: server.transport_type,
      command: server.command,
      args: server.args,
      env: server.env,
      url: server.url,
      headers: server.headers
    })),
    skillNames: raw.skill_names ?? [],
    pluginNames: raw.plugin_names ?? []
  }
}

function uniqueTrimmed(values: string[]): string[] {
  return Array.from(new Set(values.map(value => value.trim()).filter(Boolean)))
}

export function buildAgentProfileSystemPrompt(profile?: Partial<AgentProfile> | null): string | undefined {
  if (!profile) {
    return undefined
  }

  const profileWithResources = profile as Partial<AgentExecutionProfile>

  const sections = [
    profile.systemPrompt?.trim(),
    profile.roleDefinition?.trim()
      ? `角色定位：${profile.roleDefinition.trim()}`
      : undefined,
    profile.workingStyle?.trim()
      ? `工作方式：${profile.workingStyle.trim()}`
      : undefined,
    profile.outputStyle?.trim()
      ? `输出偏好：${profile.outputStyle.trim()}`
      : undefined,
    profile.toolUsagePolicy?.trim()
      ? `工具策略：${profile.toolUsagePolicy.trim()}`
      : undefined,
    profile.domainTags && profile.domainTags.length > 0
      ? `领域标签：${uniqueTrimmed(profile.domainTags).join('、')}`
      : undefined,
    profile.capabilityTags && profile.capabilityTags.length > 0
      ? `能力标签：${uniqueTrimmed(profile.capabilityTags).join('、')}`
      : undefined,
    profileWithResources.skillNames && profileWithResources.skillNames.length > 0
      ? `已挂载 Skills：${uniqueTrimmed(profileWithResources.skillNames).join('、')}`
      : undefined,
    profileWithResources.pluginNames && profileWithResources.pluginNames.length > 0
      ? `已挂载 Plugins：${uniqueTrimmed(profileWithResources.pluginNames).join('、')}`
      : undefined,
    [
      profile.readonlyResearcher ? '只读研究型' : '',
      profile.planningCapability ? '规划协调' : '',
      profile.executionCapability ? '执行交付' : ''
    ].filter(Boolean).length > 0
      ? `角色能力：${[
          profile.readonlyResearcher ? '只读研究型' : '',
          profile.planningCapability ? '规划协调' : '',
          profile.executionCapability ? '执行交付' : ''
        ].filter(Boolean).join('、')}`
      : undefined
  ].filter((value): value is string => Boolean(value && value.trim()))

  if (sections.length === 0) {
    return undefined
  }

  return sections.join('\n\n')
}

export const useAgentProfileStore = defineStore('agentProfile', () => {
  const profiles = ref<Map<string, AgentProfile>>(new Map())
  const isLoading = ref(false)

  const profilesList = computed(() => Array.from(profiles.value.values()))

  async function loadProfiles() {
    isLoading.value = true
    const notificationStore = useNotificationStore()

    try {
      const rawProfiles = await invoke<RawAgentProfile[]>('list_agent_profiles')
      profiles.value = new Map(rawProfiles.map(profile => {
        const nextProfile = transformProfile(profile)
        return [nextProfile.agentId, nextProfile]
      }))
      return profilesList.value
    } catch (error) {
      notificationStore.databaseError(
        '加载 Agent Profile 失败',
        getErrorMessage(error),
        async () => { await loadProfiles() }
      )
      throw error
    } finally {
      isLoading.value = false
    }
  }

  async function loadProfile(agentId: string) {
    const notificationStore = useNotificationStore()

    try {
      const rawProfile = await invoke<RawAgentProfile>('get_agent_profile', { agentId })
      const profile = transformProfile(rawProfile)
      profiles.value.set(agentId, profile)
      return profile
    } catch (error) {
      notificationStore.databaseError(
        '加载 Agent Profile 失败',
        getErrorMessage(error),
        async () => { await loadProfile(agentId) }
      )
      throw error
    }
  }

  async function saveProfile(agentId: string, draft: AgentProfileDraft) {
    const notificationStore = useNotificationStore()

    try {
      const rawProfile = await invoke<RawAgentProfile>('upsert_agent_profile', {
        input: {
          agent_id: agentId,
          system_prompt: draft.systemPrompt ?? null,
          role_definition: draft.roleDefinition ?? null,
          working_style: draft.workingStyle ?? null,
          output_style: draft.outputStyle ?? null,
          tool_usage_policy: draft.toolUsagePolicy ?? null,
          domain_tags: uniqueTrimmed(draft.domainTags ?? []),
          capability_tags: uniqueTrimmed(draft.capabilityTags ?? []),
          readonly_researcher: draft.readonlyResearcher ?? false,
          planning_capability: draft.planningCapability ?? false,
          execution_capability: draft.executionCapability ?? true,
          default_execution_mode: draft.defaultExecutionMode ?? null
        }
      })
      const profile = transformProfile(rawProfile)
      profiles.value.set(agentId, profile)
      return profile
    } catch (error) {
      notificationStore.databaseError('保存 Agent Profile 失败', getErrorMessage(error))
      throw error
    }
  }

  async function resolveExecutionProfile(agentId: string) {
    const notificationStore = useNotificationStore()

    try {
      const rawProfile = await invoke<RawAgentExecutionProfile>('get_agent_execution_profile', { agentId })
      const profile = transformExecutionProfile(rawProfile)
      profiles.value.set(agentId, {
        agentId: profile.agentId,
        systemPrompt: profile.systemPrompt,
        roleDefinition: profile.roleDefinition,
        workingStyle: profile.workingStyle,
        outputStyle: profile.outputStyle,
        toolUsagePolicy: profile.toolUsagePolicy,
        domainTags: profile.domainTags,
        capabilityTags: profile.capabilityTags,
        readonlyResearcher: profile.readonlyResearcher,
        planningCapability: profile.planningCapability,
        executionCapability: profile.executionCapability,
        defaultExecutionMode: profile.defaultExecutionMode,
        createdAt: profile.createdAt,
        updatedAt: profile.updatedAt
      })
      return profile
    } catch (error) {
      notificationStore.databaseError('解析 Agent 执行配置失败', getErrorMessage(error))
      throw error
    }
  }

  function getProfile(agentId?: string | null): AgentProfile | null {
    if (!agentId) {
      return null
    }

    return profiles.value.get(agentId) ?? createDefaultProfile(agentId)
  }

  return {
    profiles,
    profilesList,
    isLoading,
    loadProfiles,
    loadProfile,
    saveProfile,
    resolveExecutionProfile,
    getProfile
  }
})
