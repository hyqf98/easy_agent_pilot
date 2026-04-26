export interface UsageBaseline {
  rawInputTokens: number
  rawOutputTokens: number
}

export interface NormalizeRuntimeUsageOptions {
  provider?: string | null
  inputTokens?: number
  outputTokens?: number
  baseline?: UsageBaseline | null
}

export interface NormalizeRuntimeUsageResult {
  inputTokens?: number
  outputTokens?: number
  nextBaseline: UsageBaseline | null
  didReset: boolean
}

const CUMULATIVE_USAGE_RUNTIME_KEYWORDS = ['claude', 'codex', 'opencode'] as const

function normalizeTokenValue(value?: number): number | undefined {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return undefined
  }

  return Math.max(0, Math.floor(value))
}

export function isCumulativeUsageRuntime(provider?: string | null): boolean {
  const normalized = provider?.trim().toLowerCase() ?? ''
  return CUMULATIVE_USAGE_RUNTIME_KEYWORDS.some(keyword => normalized.includes(keyword))
}

export function normalizeRuntimeUsage(
  options: NormalizeRuntimeUsageOptions
): NormalizeRuntimeUsageResult {
  const inputTokens = normalizeTokenValue(options.inputTokens)
  const outputTokens = normalizeTokenValue(options.outputTokens)
  const baseline = options.baseline ?? null

  if (!isCumulativeUsageRuntime(options.provider)) {
    return {
      inputTokens,
      outputTokens,
      nextBaseline: null,
      didReset: false
    }
  }

  const previousInputTokens = baseline?.rawInputTokens ?? 0
  const previousOutputTokens = baseline?.rawOutputTokens ?? 0
  const didReset = (
    (typeof inputTokens === 'number' && inputTokens < previousInputTokens)
    || (typeof outputTokens === 'number' && outputTokens < previousOutputTokens)
  )

  const normalizedInputTokens = typeof inputTokens === 'number'
    ? (didReset ? inputTokens : Math.max(0, inputTokens - previousInputTokens))
    : undefined
  const normalizedOutputTokens = typeof outputTokens === 'number'
    ? (didReset ? outputTokens : Math.max(0, outputTokens - previousOutputTokens))
    : undefined
  const hasRawUsage = typeof inputTokens === 'number' || typeof outputTokens === 'number'

  return {
    inputTokens: normalizedInputTokens,
    outputTokens: normalizedOutputTokens,
    nextBaseline: hasRawUsage
      ? {
          rawInputTokens: typeof inputTokens === 'number'
            ? inputTokens
            : baseline?.rawInputTokens ?? 0,
          rawOutputTokens: typeof outputTokens === 'number'
            ? outputTokens
            : baseline?.rawOutputTokens ?? 0
        }
      : baseline,
    didReset
  }
}
