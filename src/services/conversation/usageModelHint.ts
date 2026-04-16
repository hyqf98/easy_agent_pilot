import { invoke } from '@tauri-apps/api/core'
import type { AgentConfig } from '@/stores/agent'

interface CliConfigModelProfile {
  main_model?: string | null
  codex_model?: string | null
  provider_name?: string | null
}

function resolveOfficialCliDefaultModel(cliType: 'claude' | 'codex' | 'opencode'): string | undefined {
  if (cliType === 'codex') {
    return 'gpt-5-codex'
  }

  // Claude Code default is account-dependent, so do not hardcode a single full model id.
  return undefined
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

  const cliType = agent.provider === 'claude' || agent.provider === 'codex' || agent.provider === 'opencode'
    ? agent.provider
    : null
  if (!cliType) {
    return undefined
  }

  try {
    const profile = await invoke<CliConfigModelProfile>('read_current_cli_config', { cliType })
    const configuredModel = (() => {
      if (cliType === 'codex') {
        return profile.codex_model?.trim() || undefined
      }

      if (cliType === 'opencode') {
        const provider = profile.provider_name?.trim()
        const model = profile.main_model?.trim()
        if (!model) {
          return undefined
        }

        if (!provider || model.startsWith(`${provider}/`)) {
          return model
        }

        return `${provider}/${model}`
      }

      return profile.main_model?.trim() || undefined
    })()
    return configuredModel || resolveOfficialCliDefaultModel(cliType)
  } catch (error) {
    console.warn('[usageModelHint] Failed to resolve usage model hint:', error)
    return resolveOfficialCliDefaultModel(cliType)
  }
}
