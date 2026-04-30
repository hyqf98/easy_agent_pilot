import { describe, expect, it } from 'vitest'
import { shouldAutoCompressByThreshold } from './autoCompression'

describe('shouldAutoCompressByThreshold', () => {
  it('triggers once usage reaches the configured threshold', () => {
    expect(shouldAutoCompressByThreshold({
      autoCompressionEnabled: true,
      meaningfulMessageCount: 2,
      usagePercentage: 81,
      threshold: 80
    })).toBe(true)
  })

  it('does not trigger when auto compression is disabled', () => {
    expect(shouldAutoCompressByThreshold({
      autoCompressionEnabled: false,
      meaningfulMessageCount: 12,
      usagePercentage: 95,
      threshold: 80
    })).toBe(false)
  })

  it('does not trigger for empty sessions', () => {
    expect(shouldAutoCompressByThreshold({
      autoCompressionEnabled: true,
      meaningfulMessageCount: 0,
      usagePercentage: 95,
      threshold: 80
    })).toBe(false)
  })
})
