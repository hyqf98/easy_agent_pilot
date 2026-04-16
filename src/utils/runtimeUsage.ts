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
}

function isCodexRuntime(provider?: string | null): boolean {
  const normalized = provider?.trim().toLowerCase() ?? ''
  return normalized.includes('codex')
}

function normalizeCodexCounter(nextValue: number | undefined, previousValue: number): number | undefined {
  if (typeof nextValue !== 'number') {
    return undefined
  }

  if (previousValue > 0 && nextValue >= previousValue) {
    const delta = nextValue - previousValue
    return delta > 0 ? delta : undefined
  }

  return nextValue
}

/**
 * Codex CLI 在 resume 多轮时返回的是线程累计 usage。
 * 这里统一转换成当前轮次的增量，避免 UI 将累计值误认为上下文窗口占用。
 */
export function normalizeRuntimeUsage(
  options: NormalizeRuntimeUsageOptions
): NormalizeRuntimeUsageResult {
  const { provider, inputTokens, outputTokens, baseline } = options
  if (!isCodexRuntime(provider)) {
    return {
      inputTokens,
      outputTokens,
      nextBaseline: baseline ?? null
    }
  }

  const previousInputTokens = baseline?.rawInputTokens ?? 0
  const previousOutputTokens = baseline?.rawOutputTokens ?? 0

  return {
    inputTokens: normalizeCodexCounter(inputTokens, previousInputTokens),
    outputTokens: normalizeCodexCounter(outputTokens, previousOutputTokens),
    nextBaseline: {
      rawInputTokens: typeof inputTokens === 'number' ? inputTokens : previousInputTokens,
      rawOutputTokens: typeof outputTokens === 'number' ? outputTokens : previousOutputTokens
    }
  }
}
