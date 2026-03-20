import type { Plan } from '@/types/plan'
import type { AgentTeamMode } from '@/stores/agentTeam'

export interface PlanTaskStats {
  total: number
  executionQueue: number
  completed: number
  failed: number
}

export interface AgentOption {
  label: string
  value: string
}

export interface ModelOption {
  label: string
  value: string
  isDefault: boolean
}

export interface ProjectOption {
  label: string
  value: string
  path: string
}

export type PlanTabKey = 'draft' | 'splitting' | 'executing' | 'completed'

export interface PlanCreateFormState {
  name: string
  description: string
  splitMode: 'ai' | 'manual'    // 拆分模式
  splitExecutionMode: AgentTeamMode
  splitTeamId: string | null
  granularity: number
  maxRetryCount: number
  splitAgentId: string | null
  splitModelId: string
  executionMode: 'immediate' | 'scheduled'
  scheduledDateTime: string
}

export interface PlanEditFormState {
  name: string
  description: string
  splitMode: 'ai' | 'manual'
  splitExecutionMode: AgentTeamMode
  splitTeamId: string | null
  granularity: number
  maxRetryCount: number
  splitAgentId: string | null
  splitModelId: string
  executionMode: 'immediate' | 'scheduled'
  scheduledDateTime: string
}

export interface PlanSplitConfigFormState {
  executionMode: AgentTeamMode
  teamId: string | null
  agentId: string | null
  modelId: string
}

export interface PlanListItemViewModel {
  plan: Plan
  isActive: boolean
  statusLabel: string
  statusColor: string
  relativeTimeLabel: string
  scheduledLabel: string
  taskStats: PlanTaskStats
  canSplit: boolean
  canResumeSplit: boolean
  canEdit: boolean
}
