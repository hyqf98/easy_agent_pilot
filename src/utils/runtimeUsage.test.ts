import { describe, expect, it } from 'vitest'

import {
  mergeFinalUsageSnapshotCounts,
  mergeResponseUsageCounts,
  normalizeRuntimeUsage
} from '@/utils/runtimeUsage'

describe('runtimeUsage', () => {
  it('passes through usage directly for non-cumulative runtimes (claude)', () => {
    const result = normalizeRuntimeUsage({
      provider: 'claude',
      inputTokens: 10140,
      outputTokens: 500,
      rawInputTokens: 10140,
      rawOutputTokens: 500,
      cacheReadInputTokens: 0,
      cacheCreationInputTokens: 0,
      baseline: null
    })

    expect(result.inputTokens).toBe(10140)
    expect(result.outputTokens).toBe(500)
    expect(result.nextBaseline).toBeNull()
    expect(result.didReset).toBe(false)
  })

  it('passes through usage directly for non-cumulative runtimes (opencode)', () => {
    const result = normalizeRuntimeUsage({
      provider: 'opencode',
      inputTokens: 15198,
      outputTokens: 3,
      rawInputTokens: 50913,
      rawOutputTokens: 3,
      cacheReadInputTokens: 17856,
      cacheCreationInputTokens: 0,
      baseline: null
    })

    expect(result.inputTokens).toBe(15198)
    expect(result.outputTokens).toBe(3)
    expect(result.nextBaseline).toBeNull()
    expect(result.didReset).toBe(false)
  })

  it('computes delta for cumulative codex runtime', () => {
    const first = normalizeRuntimeUsage({
      provider: 'codex',
      inputTokens: 13429,
      outputTokens: 441,
      rawInputTokens: 13429,
      rawOutputTokens: 441,
      cacheReadInputTokens: 3456,
      cacheCreationInputTokens: 0,
      baseline: null
    })

    expect(first.inputTokens).toBe(9973)
    expect(first.outputTokens).toBe(441)
    expect(first.nextBaseline).toEqual({
      rawInputTokens: 13429,
      rawOutputTokens: 441,
      cacheReadInputTokens: 3456,
      cacheCreationInputTokens: 0
    })

    const second = normalizeRuntimeUsage({
      provider: 'codex',
      inputTokens: 46088,
      outputTokens: 696,
      rawInputTokens: 46088,
      rawOutputTokens: 696,
      cacheReadInputTokens: 16640,
      cacheCreationInputTokens: 0,
      baseline: first.nextBaseline
    })

    expect(second.inputTokens).toBe(19475)
    expect(second.outputTokens).toBe(255)
  })

  it('detects reset when raw tokens decrease', () => {
    const first = normalizeRuntimeUsage({
      provider: 'codex',
      inputTokens: 50000,
      outputTokens: 1000,
      rawInputTokens: 50000,
      rawOutputTokens: 1000,
      cacheReadInputTokens: 10000,
      cacheCreationInputTokens: 0,
      baseline: null
    })

    const afterReset = normalizeRuntimeUsage({
      provider: 'codex',
      inputTokens: 5000,
      outputTokens: 200,
      rawInputTokens: 5000,
      rawOutputTokens: 200,
      cacheReadInputTokens: 0,
      cacheCreationInputTokens: 0,
      baseline: first.nextBaseline
    })

    expect(afterReset.didReset).toBe(true)
    expect(afterReset.inputTokens).toBe(5000)
    expect(afterReset.outputTokens).toBe(200)
  })

  it('replaces non-cumulative response usage with the latest usage event', () => {
    const merged = mergeResponseUsageCounts({
      inputTokens: 1200,
      outputTokens: 80
    }, {
      inputTokens: 900,
      outputTokens: 12
    }, 'opencode')

    expect(merged.inputTokens).toBe(900)
    expect(merged.outputTokens).toBe(12)
  })

  it('accumulates cumulative response usage deltas for codex', () => {
    const merged = mergeResponseUsageCounts({
      inputTokens: 1200,
      outputTokens: 80
    }, {
      inputTokens: 300,
      outputTokens: 15
    }, 'codex')

    expect(merged.inputTokens).toBe(1500)
    expect(merged.outputTokens).toBe(95)
  })

  it('prefers live codex message usage over final session snapshot totals', () => {
    const merged = mergeFinalUsageSnapshotCounts({
      inputTokens: 1900,
      outputTokens: 140
    }, {
      inputTokens: 5100,
      outputTokens: 420
    }, 'codex')

    expect(merged.inputTokens).toBe(1900)
    expect(merged.outputTokens).toBe(140)
  })

  it('uses final snapshot totals for non-cumulative runtimes', () => {
    const merged = mergeFinalUsageSnapshotCounts({
      inputTokens: undefined,
      outputTokens: undefined
    }, {
      inputTokens: 15098,
      outputTokens: 33
    }, 'claude')

    expect(merged.inputTokens).toBe(15098)
    expect(merged.outputTokens).toBe(33)
  })
})
