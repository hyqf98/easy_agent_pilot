export type ReasoningEffortLevel = 'none' | 'minimal' | 'low' | 'medium' | 'high' | 'xhigh' | 'max'

export interface ReasoningEffortOption {
  value: ReasoningEffortLevel
  label: string
}

const PROVIDER_REASONING_EFFORTS: Record<string, ReasoningEffortLevel[]> = {
  claude: ['low', 'medium', 'high', 'xhigh'],
  codex: ['minimal', 'low', 'medium', 'high', 'xhigh']
}

const EFFORT_LABEL_KEYS: Record<ReasoningEffortLevel, string> = {
  none: 'reasoning.none',
  minimal: 'reasoning.minimal',
  low: 'reasoning.low',
  medium: 'reasoning.medium',
  high: 'reasoning.high',
  xhigh: 'reasoning.xhigh',
  max: 'reasoning.max'
}

export function getProviderReasoningEfforts(provider: string): ReasoningEffortLevel[] {
  return PROVIDER_REASONING_EFFORTS[provider] ?? []
}

export function getEffortLabelKey(level: ReasoningEffortLevel): string {
  return EFFORT_LABEL_KEYS[level]
}
