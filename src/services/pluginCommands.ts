import { invoke } from '@tauri-apps/api/core'
import type { SlashCommandDescriptor, SlashCommandPanelType } from '@/services/slashCommands'

export interface PluginSlashCommand {
  name: string
  pluginName: string
  cliType: string
  description: string | null
  argumentHint: string | null
  allowedTools: string[] | null
  disableModelInvocation: boolean
}

const CACHE_TTL_MS = 5 * 60 * 1000

interface CacheEntry {
  commands: PluginSlashCommand[]
  expiresAt: number
}

const cache = new Map<string, CacheEntry>()

function cacheKey(cliType: string, projectPath?: string): string {
  return `${cliType}:${projectPath ?? ''}`
}

export async function loadPluginSlashCommands(
  cliType: string,
  projectPath?: string
): Promise<PluginSlashCommand[]> {
  const key = cacheKey(cliType, projectPath)
  const cached = cache.get(key)
  if (cached && cached.expiresAt > Date.now()) {
    return cached.commands
  }

  try {
    const commands = await invoke<PluginSlashCommand[]>('scan_plugin_slash_commands', {
      cliType,
      projectPath: projectPath ?? null
    })
    cache.set(key, { commands, expiresAt: Date.now() + CACHE_TTL_MS })
    return commands
  } catch {
    return []
  }
}

export function clearPluginCommandsCache(): void {
  cache.clear()
}

export function toSlashCommandDescriptor(cmd: PluginSlashCommand): SlashCommandDescriptor {
  const qualifiedName = `${cmd.pluginName}:${cmd.name}`
  return {
    name: qualifiedName,
    scopes: ['main', 'mini'] as SlashCommandPanelType[],
    descriptionKey: cmd.description ?? '',
    usageKey: cmd.argumentHint ? `/${qualifiedName} ${cmd.argumentHint}` : `/${qualifiedName}`,
    insertText: `/${qualifiedName} `,
    source: 'plugin',
    pluginName: cmd.pluginName,
    cliType: cmd.cliType,
    argumentHint: cmd.argumentHint ?? undefined,
    cliCommandName: cmd.name
  }
}
