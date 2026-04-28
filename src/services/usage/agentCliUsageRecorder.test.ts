import { describe, expect, it } from 'vitest'

import { findLatestUsageSnapshot } from '@/services/usage/agentCliUsageRecorder'
import type { PlanSplitLogRecord } from '@/types/plan'

function createUsageLog(metadata: Record<string, unknown>): Pick<PlanSplitLogRecord, 'type' | 'metadata'> {
  return {
    type: 'usage',
    metadata: JSON.stringify(metadata)
  }
}

describe('agentCliUsageRecorder', () => {
  it('normalizes cumulative codex usage when recovering plan split logs', () => {
    const snapshot = findLatestUsageSnapshot([
      createUsageLog({
        model: 'gpt-5-codex',
        inputTokens: 100,
        outputTokens: 1,
        rawInputTokens: 100,
        rawOutputTokens: 1,
        cacheReadInputTokens: 0,
        cacheCreationInputTokens: 0
      }),
      createUsageLog({
        model: 'gpt-5-codex',
        inputTokens: 220,
        outputTokens: 3,
        rawInputTokens: 220,
        rawOutputTokens: 3,
        cacheReadInputTokens: 0,
        cacheCreationInputTokens: 0
      })
    ], {
      provider: 'codex'
    })

    expect(snapshot.modelId).toBe('gpt-5-codex')
    expect(snapshot.inputTokens).toBe(220)
    expect(snapshot.outputTokens).toBe(3)
  })

  it('keeps non-cumulative opencode usage additive', () => {
    const snapshot = findLatestUsageSnapshot([
      createUsageLog({
        model: 'modelhub/glm-5.1',
        inputTokens: 50,
        outputTokens: 4
      }),
      createUsageLog({
        model: 'modelhub/glm-5.1',
        inputTokens: 30,
        outputTokens: 2
      })
    ], {
      provider: 'opencode'
    })

    expect(snapshot.modelId).toBe('modelhub/glm-5.1')
    expect(snapshot.inputTokens).toBe(80)
    expect(snapshot.outputTokens).toBe(6)
  })
})
