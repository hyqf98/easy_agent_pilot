import { describe, expect, it } from 'vitest'
import { resolveUpdateVersion } from './tauriUpdaterAdapter'

describe('resolveUpdateVersion', () => {
  it('prefers github tag_name over updater version', () => {
    expect(resolveUpdateVersion('1.4.2', { tag_name: 'v1.4.1' })).toBe('1.4.1')
  })

  it('strips refs tags prefix from github tag names', () => {
    expect(resolveUpdateVersion('1.4.2', { tag_name: 'refs/tags/v1.4.1' })).toBe('1.4.1')
  })

  it('falls back to updater version when tag is missing', () => {
    expect(resolveUpdateVersion('1.4.2', {})).toBe('1.4.2')
  })
})
