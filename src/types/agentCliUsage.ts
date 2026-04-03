export type AgentCliUsageGranularity = 'year' | 'month' | 'day'
export type AgentCliUsageDimension = 'agent' | 'model'
export type AgentCliUsageProviderFilter = 'all' | 'claude' | 'codex'

export interface RecordAgentCliUsageInput {
  executionId: string
  executionMode: 'chat' | 'task_split' | 'task_execution' | 'solo_execution'
  provider: 'claude' | 'codex'
  agentId?: string | null
  agentNameSnapshot?: string | null
  modelId?: string | null
  projectId?: string | null
  sessionId?: string | null
  taskId?: string | null
  messageId?: string | null
  inputTokens?: number | null
  outputTokens?: number | null
  occurredAt?: string | null
}

export interface QueryAgentCliUsageStatsInput {
  startAt?: string
  endAt?: string
  granularity: AgentCliUsageGranularity
  dimension: AgentCliUsageDimension
  providerFilter?: AgentCliUsageProviderFilter
  modelKeyword?: string
}

export interface AgentCliUsageSummary {
  totalCalls: number
  inputTokens: number
  outputTokens: number
  totalTokens: number
  estimatedTotalCostUsd: number
  unpricedCalls: number
}

export interface AgentCliUsageTimelinePoint {
  bucket: string
  label: string
  callCount: number
  inputTokens: number
  outputTokens: number
  totalTokens: number
  estimatedTotalCostUsd: number
}

export interface AgentCliUsageBreakdownRow {
  dimensionId: string
  label: string
  provider: string
  callCount: number
  inputTokens: number
  outputTokens: number
  totalTokens: number
  estimatedTotalCostUsd: number
  unpricedCalls: number
}

export interface AgentCliUsageStackedPoint {
  bucket: string
  label: string
  dimensionId: string
  dimensionLabel: string
  provider: string
  callCount: number
  inputTokens: number
  outputTokens: number
  totalTokens: number
  estimatedTotalCostUsd: number
}

export interface AgentCliUsageMeta {
  startAt?: string
  endAt?: string
  granularity: AgentCliUsageGranularity
  dimension: AgentCliUsageDimension
  providerFilter: AgentCliUsageProviderFilter
  modelKeyword?: string
  pricingVersion: string
  costPartial: boolean
}

export interface AgentCliUsageStatsResponse {
  summary: AgentCliUsageSummary
  timeline: AgentCliUsageTimelinePoint[]
  breakdown: AgentCliUsageBreakdownRow[]
  stackedTimeline: AgentCliUsageStackedPoint[]
  meta: AgentCliUsageMeta
}

export interface RepairAgentCliUsageHistoryResult {
  provider: string
  targetModelId?: string | null
  updatedCount: number
  skippedReason?: string | null
}
