import type { AgentModelConfig } from '@/stores/agentConfig'

export const DEFAULT_CONTEXT_WINDOW = 128000

function normalizeModelId(modelId?: string | null): string {
  return modelId?.trim().toLowerCase() || ''
}

interface ResolveConfiguredContextWindowOptions {
  runtimeModelId?: string | null
  selectedModelId?: string | null
  agentModelId?: string | null
  fallbackContextWindow?: number
}

export function findConfiguredModel(
  models: AgentModelConfig[],
  options: ResolveConfiguredContextWindowOptions = {}
): AgentModelConfig | undefined {
  const enabledModels = models.filter(model => model.enabled)
  if (enabledModels.length === 0) {
    return undefined
  }

  const runtimeModelId = normalizeModelId(options.runtimeModelId)
  const selectedModelId = normalizeModelId(options.selectedModelId)
  const agentModelId = normalizeModelId(options.agentModelId)

  const matchById = (modelId: string) => (
    modelId
      ? enabledModels.find(model => normalizeModelId(model.modelId) === modelId)
      : undefined
  )

  return matchById(runtimeModelId)
    ?? matchById(selectedModelId)
    ?? matchById(agentModelId)
    ?? enabledModels.find(model => model.isDefault)
    ?? enabledModels[0]
}

export function resolveConfiguredContextWindow(
  models: AgentModelConfig[],
  options: ResolveConfiguredContextWindowOptions = {}
): number {
  return findConfiguredModel(models, options)?.contextWindow
    ?? options.fallbackContextWindow
    ?? DEFAULT_CONTEXT_WINDOW
}
