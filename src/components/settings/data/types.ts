import type { InstallSession } from '@/stores/settings'

export interface ExportOptions {
  include_projects: boolean
  include_sessions: boolean
  include_messages: boolean
  include_agents: boolean
  include_mcp_servers: boolean
  include_cli_paths: boolean
  include_market_sources: boolean
  include_app_settings: boolean
}

export type ExportOptionKey = keyof ExportOptions

export interface ExportOptionItem {
  key: ExportOptionKey
  label: string
  checked: boolean
}

export interface ImportStats {
  projects_imported: number
  sessions_imported: number
  messages_imported: number
  agents_imported: number
  mcp_servers_imported: number
  cli_paths_imported: number
  market_sources_imported: number
  app_settings_imported: number
}

export interface ImportStatItem {
  key: keyof ImportStats
  label: string
  value: number
}

export interface InstallStatusInfo {
  text: string
  class: string
}

export type InstallSessionStatus = InstallSession['status']
