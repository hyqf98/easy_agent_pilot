import { invoke } from '@tauri-apps/api/core'
import { inferAgentProvider, type AgentConfig } from '@/stores/agent'
import type { RecordAgentCliUsageInput } from '@/types/agentCliUsage'
import type { PlanSplitLogRecord } from '@/types/plan'

interface UsageSnapshot {
  modelId?: string
  inputTokens?: number
  outputTokens?: number
}

function normalizeModelId(value?: string | null): string {
  return value?.trim().toLowerCase() || ''
}

function splitModelTokens(value: string): string[] {
  return value.split(/[^a-z0-9]+/).filter(Boolean)
}

function areEquivalentModelIds(left?: string | null, right?: string | null): boolean {
  const normalizedLeft = normalizeModelId(left)
  const normalizedRight = normalizeModelId(right)
  if (!normalizedLeft || !normalizedRight) {
    return false
  }

  if (normalizedLeft === normalizedRight) {
    return true
  }

  if (normalizedLeft.includes(normalizedRight) || normalizedRight.includes(normalizedLeft)) {
    return true
  }

  const leftTokens = splitModelTokens(normalizedLeft)
  const rightTokens = splitModelTokens(normalizedRight)
  if (leftTokens.length === 0 || rightTokens.length === 0) {
    return false
  }

  const rightTokenSet = new Set(rightTokens)
  const sharedCount = leftTokens.filter(token => rightTokenSet.has(token)).length
  const requiredSharedCount = Math.min(2, leftTokens.length, rightTokens.length)
  return sharedCount >= requiredSharedCount
    && sharedCount / Math.max(leftTokens.length, rightTokens.length) >= 0.5
}

/**
 * 统一 CLI 用量统计使用的模型 ID。
 *
 * 自定义 Claude 兼容源可能会上报一个通用 Anthropic 模型名，但用户真实请求的是显式配置的模型；
 * 这里优先保留显式请求模型，仅在两者明显等价时接受运行时上报值。
 */
export function resolveRecordedModelId(options: {
  reportedModelId?: string | null
  requestedModelId?: string | null
}): string | null {
  const requestedModelId = options.requestedModelId?.trim() || ''
  const reportedModelId = options.reportedModelId?.trim() || ''

  if (!requestedModelId) {
    return reportedModelId || null
  }

  if (!reportedModelId) {
    return requestedModelId
  }

  if (areEquivalentModelIds(reportedModelId, requestedModelId)) {
    return reportedModelId
  }

  return requestedModelId
}

function buildUsagePayload(
  agent: Pick<AgentConfig, 'id' | 'name' | 'provider'>,
  input: Omit<RecordAgentCliUsageInput, 'provider' | 'agentId' | 'agentNameSnapshot'>
): RecordAgentCliUsageInput | null {
  const provider = inferAgentProvider(agent)
  if (provider !== 'claude' && provider !== 'codex' && provider !== 'opencode') {
    return null
  }

  return {
    ...input,
    provider,
    agentId: agent.id,
    agentNameSnapshot: agent.name
  }
}

export function findLatestUsageSnapshot(
  logs: Pick<PlanSplitLogRecord, 'type' | 'metadata'>[]
): UsageSnapshot {
  const usageState: UsageSnapshot = {}

  for (const log of logs) {
    if (log.type !== 'usage' && log.type !== 'message_start') {
      continue
    }

    if (!log.metadata) {
      continue
    }

    try {
      const metadata = JSON.parse(log.metadata) as {
        model?: unknown
        inputTokens?: unknown
        outputTokens?: unknown
      }

      if (typeof metadata.model === 'string' && metadata.model.trim()) {
        usageState.modelId = metadata.model.trim()
      }
      if (typeof metadata.inputTokens === 'number') {
        usageState.inputTokens = metadata.inputTokens
      }
      if (typeof metadata.outputTokens === 'number') {
        usageState.outputTokens = metadata.outputTokens
      }
    } catch {
      continue
    }
  }

  return usageState
}

export async function recordAgentCliUsage(
  agent: Pick<AgentConfig, 'id' | 'name' | 'provider'>,
  input: Omit<RecordAgentCliUsageInput, 'provider' | 'agentId' | 'agentNameSnapshot'>
): Promise<void> {
  const payload = buildUsagePayload(agent, input)
  if (!payload) {
    return
  }

  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      await invoke('record_agent_cli_usage', { input: payload })
      return
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      const shouldRetry = /foreign key constraint failed/i.test(message) && attempt < 2
      if (shouldRetry) {
        await new Promise(resolve => window.setTimeout(resolve, 600 * (attempt + 1)))
        continue
      }

      console.warn('[agentCliUsageRecorder] Failed to record CLI usage:', error)
      return
    }
  }
}

export function recordAgentCliUsageInBackground(
  agent: Pick<AgentConfig, 'id' | 'name' | 'provider'>,
  input: Omit<RecordAgentCliUsageInput, 'provider' | 'agentId' | 'agentNameSnapshot'>
): void {
  void recordAgentCliUsage(agent, input)
}
