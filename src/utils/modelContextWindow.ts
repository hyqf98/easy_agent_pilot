const KNOWN_CONTEXT_WINDOWS: Array<{ pattern: RegExp; contextWindow: number }> = [
  { pattern: /^gpt-5\.5$/i, contextWindow: 1_000_000 },
  { pattern: /^gpt-5\.4$/i, contextWindow: 1_050_000 },
  { pattern: /^gpt-5\.4-mini$/i, contextWindow: 400_000 },
  { pattern: /^claude-opus-4\.7$/i, contextWindow: 1_000_000 },
  { pattern: /^claude-sonnet-5$/i, contextWindow: 1_000_000 },
  { pattern: /^claude-opus-4\.6(\[1m\])?$/i, contextWindow: 1_000_000 },
  { pattern: /^claude-sonnet-4\.6$/i, contextWindow: 200_000 },
  { pattern: /^claude-sonnet-4\.6\[1m\]$/i, contextWindow: 1_000_000 }
]

function normalizeModelId(modelId?: string | null): string {
  return modelId?.trim().toLowerCase() || ''
}

function collectModelAliases(modelId?: string | null): string[] {
  const normalized = normalizeModelId(modelId)
  if (!normalized) {
    return []
  }

  const aliases = new Set<string>([normalized])
  const slashIndex = normalized.lastIndexOf('/')
  if (slashIndex >= 0 && slashIndex < normalized.length - 1) {
    aliases.add(normalized.slice(slashIndex + 1))
  }

  return Array.from(aliases)
}

export function resolveKnownContextWindow(
  modelId?: string | null,
  _provider?: string | null
): number | undefined {
  void _provider

  for (const alias of collectModelAliases(modelId)) {
    const matched = KNOWN_CONTEXT_WINDOWS.find(item => item.pattern.test(alias))
    if (matched) {
      return matched.contextWindow
    }
  }

  return undefined
}
