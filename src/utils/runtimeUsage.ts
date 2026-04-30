export interface UsageBaseline {
  rawInputTokens: number
  rawOutputTokens: number
  cacheReadInputTokens: number
  cacheCreationInputTokens: number
}

export interface NormalizeRuntimeUsageOptions {
  provider?: string | null
  inputTokens?: number
  outputTokens?: number
  rawInputTokens?: number
  rawOutputTokens?: number
  cacheReadInputTokens?: number
  cacheCreationInputTokens?: number
  baseline?: UsageBaseline | null
}

export interface NormalizeRuntimeUsageResult {
  inputTokens?: number
  outputTokens?: number
  nextBaseline: UsageBaseline | null
  didReset: boolean
}

export interface RuntimeUsageCounts {
  inputTokens?: number
  outputTokens?: number
}

const CUMULATIVE_USAGE_RUNTIME_KEYWORDS = ['codex'] as const

function normalizeTokenValue(value?: number): number | undefined {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return undefined
  }

  return Math.max(0, Math.floor(value))
}

function resolveCountForDisplay(value?: number): number | undefined {
  return normalizeTokenValue(value)
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
  const rawInputTokens = normalizeTokenValue(options.rawInputTokens)
  const rawOutputTokens = normalizeTokenValue(options.rawOutputTokens)
  const cacheReadInputTokens = normalizeTokenValue(options.cacheReadInputTokens)
  const cacheCreationInputTokens = normalizeTokenValue(options.cacheCreationInputTokens)

  if (!isCumulativeUsageRuntime(options.provider)) {
    return {
      inputTokens,
      outputTokens,
      nextBaseline: null,
      didReset: false
    }
  }

  const baseline = options.baseline ?? null
  const previousInputTokens = baseline?.rawInputTokens ?? 0
  const previousOutputTokens = baseline?.rawOutputTokens ?? 0
  const previousCacheReadInputTokens = baseline?.cacheReadInputTokens ?? 0
  const previousCacheCreationInputTokens = baseline?.cacheCreationInputTokens ?? 0
  const nextRawInputTokens = rawInputTokens ?? inputTokens
  const nextRawOutputTokens = rawOutputTokens ?? outputTokens

  const didReset = (
    (typeof nextRawInputTokens === 'number' && nextRawInputTokens < previousInputTokens)
    || (typeof nextRawOutputTokens === 'number' && nextRawOutputTokens < previousOutputTokens)
    || (typeof cacheReadInputTokens === 'number' && cacheReadInputTokens < previousCacheReadInputTokens)
    || (typeof cacheCreationInputTokens === 'number' && cacheCreationInputTokens < previousCacheCreationInputTokens)
  )

  const normalizedInputTokens = typeof nextRawInputTokens === 'number'
    ? (() => {
        if (didReset) {
          return inputTokens ?? Math.max(
            0,
            nextRawInputTokens
              - (cacheReadInputTokens ?? 0)
              - (cacheCreationInputTokens ?? 0)
          )
        }

        const rawDelta = Math.max(0, nextRawInputTokens - previousInputTokens)
        const cacheReadDelta = Math.max(0, (cacheReadInputTokens ?? 0) - previousCacheReadInputTokens)
        const cacheCreationDelta = Math.max(
          0,
          (cacheCreationInputTokens ?? 0) - previousCacheCreationInputTokens
        )
        return Math.max(0, rawDelta - cacheReadDelta - cacheCreationDelta)
      })()
    : undefined
  const normalizedOutputTokens = typeof nextRawOutputTokens === 'number'
    ? (didReset
        ? (outputTokens ?? nextRawOutputTokens)
        : Math.max(0, nextRawOutputTokens - previousOutputTokens))
    : undefined
  const hasUsageBaseline = (
    typeof nextRawInputTokens === 'number'
    || typeof nextRawOutputTokens === 'number'
    || typeof cacheReadInputTokens === 'number'
    || typeof cacheCreationInputTokens === 'number'
  )

  return {
    inputTokens: normalizedInputTokens,
    outputTokens: normalizedOutputTokens,
    nextBaseline: hasUsageBaseline
      ? {
          rawInputTokens: typeof nextRawInputTokens === 'number'
            ? nextRawInputTokens
            : baseline?.rawInputTokens ?? 0,
          rawOutputTokens: typeof nextRawOutputTokens === 'number'
            ? nextRawOutputTokens
            : baseline?.rawOutputTokens ?? 0,
          cacheReadInputTokens: typeof cacheReadInputTokens === 'number'
            ? cacheReadInputTokens
            : baseline?.cacheReadInputTokens ?? 0,
          cacheCreationInputTokens: typeof cacheCreationInputTokens === 'number'
            ? cacheCreationInputTokens
            : baseline?.cacheCreationInputTokens ?? 0
        }
      : baseline,
    didReset
  }
}

export function mergeResponseUsageCounts(
  current: RuntimeUsageCounts,
  incoming: RuntimeUsageCounts,
  provider?: string | null
): RuntimeUsageCounts {
  const nextInputTokens = resolveCountForDisplay(incoming.inputTokens)
  const nextOutputTokens = resolveCountForDisplay(incoming.outputTokens)

  if (isCumulativeUsageRuntime(provider)) {
    return {
      inputTokens: nextInputTokens !== undefined
        ? (resolveCountForDisplay(current.inputTokens) ?? 0) + nextInputTokens
        : current.inputTokens,
      outputTokens: nextOutputTokens !== undefined
        ? (resolveCountForDisplay(current.outputTokens) ?? 0) + nextOutputTokens
        : current.outputTokens
    }
  }

  return {
    inputTokens: nextInputTokens ?? current.inputTokens,
    outputTokens: nextOutputTokens ?? current.outputTokens
  }
}

export function mergeFinalUsageSnapshotCounts(
  current: RuntimeUsageCounts,
  snapshot: RuntimeUsageCounts,
  provider?: string | null
): RuntimeUsageCounts {
  const snapshotInputTokens = resolveCountForDisplay(snapshot.inputTokens)
  const snapshotOutputTokens = resolveCountForDisplay(snapshot.outputTokens)

  if (!isCumulativeUsageRuntime(provider)) {
    return {
      inputTokens: snapshotInputTokens ?? current.inputTokens,
      outputTokens: snapshotOutputTokens ?? current.outputTokens
    }
  }

  const hasCurrentUsage = (resolveCountForDisplay(current.inputTokens) ?? 0) > 0
    || (resolveCountForDisplay(current.outputTokens) ?? 0) > 0

  return {
    inputTokens: hasCurrentUsage
      ? (current.inputTokens ?? snapshotInputTokens)
      : (snapshotInputTokens ?? current.inputTokens),
    outputTokens: hasCurrentUsage
      ? (current.outputTokens ?? snapshotOutputTokens)
      : (snapshotOutputTokens ?? current.outputTokens)
  }
}
