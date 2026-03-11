export type PluginSection = 'skills' | 'commands' | 'agents'

export interface InternalItem {
  name: string
  path: string
  description: string | null
  item_type: string
}

export interface PluginDetails {
  name: string
  path: string
  version: string | null
  description: string | null
  author: string | null
  install_source: string | null
  internal_skills: InternalItem[]
  internal_commands: InternalItem[]
  internal_agents: InternalItem[]
}

export interface PluginFileContent {
  name: string
  path: string
  content: string
  fileType: string
}

export const PLUGIN_SECTION_KEYS: Record<PluginSection, string> = {
  skills: 'settings.plugins.internalSkills',
  commands: 'settings.plugins.internalCommands',
  agents: 'settings.plugins.internalAgents',
}

export const PLUGIN_SECTION_ICONS: Record<PluginSection, string> = {
  skills: 'lucide:book-open',
  commands: 'lucide:terminal',
  agents: 'lucide:bot',
}

export function getPluginItemIcon(type: string): string {
  switch (type) {
    case 'skill':
      return 'lucide:book-open'
    case 'command':
      return 'lucide:terminal'
    case 'agent':
      return 'lucide:bot'
    default:
      return 'lucide:file'
  }
}
