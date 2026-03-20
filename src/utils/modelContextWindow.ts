const KNOWN_MODEL_CONTEXT_WINDOWS = new Map<string, number>([
  ['', 0],
  ['gpt-5.4', 400000],
  ['gpt-5.3-codex', 400000],
  ['gpt-5.2', 400000],
  ['gpt-5.2-codex', 400000],
  ['gpt-5.1', 400000],
  ['gpt-5.1-codex', 400000],
  ['gpt-5', 400000],
  ['claude-opus-4-6', 200000],
  ['claude-sonnet-4-6', 200000],
  ['claude-sonnet-4-5', 200000],
  ['claude-haiku-4-5', 200000]
])

function normalizeModelId(modelId?: string | null): string {
  return modelId?.trim().toLowerCase() || ''
}

export function resolveKnownContextWindow(
  modelId?: string | null,
  provider?: string | null
): number | undefined {
  const normalizedModelId = normalizeModelId(modelId)
  if (!normalizedModelId) {
    return undefined
  }

  const exactMatch = KNOWN_MODEL_CONTEXT_WINDOWS.get(normalizedModelId)
  if (typeof exactMatch === 'number' && exactMatch > 0) {
    return exactMatch
  }

  if (normalizedModelId.startsWith('claude-')) {
    if (normalizedModelId.includes('[1m]') || normalizedModelId.includes('-1m')) {
      return 1000000
    }
    return 200000
  }

  if (
    normalizedModelId.startsWith('gpt-5')
    || normalizedModelId.includes('codex')
    || provider?.trim().toLowerCase() === 'codex'
  ) {
    return 400000
  }

  return undefined
}
