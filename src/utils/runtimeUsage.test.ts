import { describe, expect, it } from 'vitest'

import { normalizeRuntimeUsage } from '@/utils/runtimeUsage'

describe('runtimeUsage', () => {
  it('normalizes raw claude usage even when provider is plain claude', () => {
    const first = normalizeRuntimeUsage({
      provider: 'claude',
      rawInputTokens: 10140,
      rawOutputTokens: 0,
      cacheReadInputTokens: 0,
      cacheCreationInputTokens: 0,
      baseline: null
    })

    expect(first.inputTokens).toBe(10140)
    expect(first.outputTokens).toBe(0)
    expect(first.nextBaseline).toEqual({
      rawInputTokens: 10140,
      rawOutputTokens: 0,
      cacheReadInputTokens: 0,
      cacheCreationInputTokens: 0
    })
  })

  it('deduplicates repeated cumulative raw usage across claude events', () => {
    const first = normalizeRuntimeUsage({
      provider: 'claude',
      rawInputTokens: 10140,
      rawOutputTokens: 0,
      cacheReadInputTokens: 0,
      cacheCreationInputTokens: 0,
      baseline: null
    })

    const second = normalizeRuntimeUsage({
      provider: 'claude',
      rawInputTokens: 65630,
      rawOutputTokens: 3,
      cacheReadInputTokens: 320,
      cacheCreationInputTokens: 0,
      baseline: first.nextBaseline
    })

    expect(second.inputTokens).toBe(55170)
    expect(second.outputTokens).toBe(3)
  })
})
