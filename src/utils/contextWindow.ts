const CONTEXT_WINDOW_MULTIPLIERS: Record<string, number> = {
  k: 1000,
  m: 1000000
}

export function parseContextWindowInput(input: string | number | null | undefined): number | undefined {
  if (typeof input === 'number') {
    return Number.isFinite(input) && input > 0 ? Math.round(input) : undefined
  }

  const normalized = input?.trim().toLowerCase().replace(/,/g, '') || ''
  if (!normalized) {
    return undefined
  }

  const matched = normalized.match(/^(\d+(?:\.\d+)?)([km])?$/i)
  if (!matched) {
    return undefined
  }

  const baseValue = Number.parseFloat(matched[1])
  if (!Number.isFinite(baseValue) || baseValue <= 0) {
    return undefined
  }

  const unit = matched[2]?.toLowerCase()
  const multiplier = unit ? CONTEXT_WINDOW_MULTIPLIERS[unit] : 1
  const parsed = Math.round(baseValue * multiplier)

  return parsed > 0 ? parsed : undefined
}

export function formatContextWindowCount(count?: number | null): string {
  if (!count || count <= 0) {
    return '未识别'
  }

  if (count >= 1000000) {
    const value = count / 1000000
    return `${trimTrailingZeros(value)}M`
  }

  if (count >= 1000) {
    const value = count / 1000
    return `${trimTrailingZeros(value)}K`
  }

  return `${count}`
}

function trimTrailingZeros(value: number): string {
  if (Number.isInteger(value)) {
    return `${value}`
  }

  return value.toFixed(value >= 10 ? 1 : 2).replace(/\.0+$/, '').replace(/(\.\d*[1-9])0+$/, '$1')
}
