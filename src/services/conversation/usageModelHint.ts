import { invoke } from '@tauri-apps/api/core'
import type { AgentConfig } from '@/stores/agent'

interface CliConfigModelProfile {
  main_model?: string | null
  codex_model?: string | null
}

/**
 * Resolve a compact model hint for runtime usage UI without altering the actual execution request.
 */
export async function resolveUsageModelHint(
  agent: Pick<AgentConfig, 'type' | 'provider' | 'modelId'>
): Promise<string | undefined> {
  const explicitModel = agent.modelId?.trim()
  if (explicitModel && explicitModel !== 'default') {
    return explicitModel
  }

  if (agent.type !== 'cli') {
    return undefined
  }

  const cliType = agent.provider === 'claude' || agent.provider === 'codex'
    ? agent.provider
    : null
  if (!cliType) {
    return undefined
  }

  try {
    const profile = await invoke<CliConfigModelProfile>('read_current_cli_config', { cliType })
    return cliType === 'codex'
      ? (profile.codex_model?.trim() || undefined)
      : (profile.main_model?.trim() || undefined)
  } catch (error) {
    console.warn('[usageModelHint] Failed to resolve usage model hint:', error)
    return undefined
  }
}
