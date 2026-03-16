/**
 * Marketplace related types.
 */

export type MarketplaceSourceId = string
export type MarketType = 'mcp' | 'skills' | 'plugins'

export interface MarketplaceSourceOption {
  id: string
  label: string
  supported_resources: Array<'mcp' | 'skills' | 'plugins'>
}

export interface MarketCategory {
  label: string
  value: string
  slug?: string | null
  count?: number | null
}

export interface McpMarketItem {
  id: string
  slug: string
  name: string
  description: string
  author: string
  category: string
  tags: string[]
  transport_type: 'stdio' | 'sse' | 'http'
  install_command?: string | null
  install_args: string[]
  env_template: Record<string, string>
  logo?: string | null
  downloads?: number | null
  rating?: number | null
  repository_url?: string | null
  source_market: string
  stars?: number | null
}

export interface McpMarketDetail extends McpMarketItem {
  full_description: string
  readme_excerpt?: string | null
}

export interface SkillMarketItem {
  id: string
  slug: string
  name: string
  description: string
  path: string
  author: string
  category: string
  category_slug?: string | null
  tags: string[]
  trigger_scenario: string
  source_market: string
  repository_url?: string | null
  downloads?: number | null
  rating?: number | null
  stars?: number | null
}

export interface SkillMarketDetail extends SkillMarketItem {
  full_description: string
  skill_content?: string | null
  download_url?: string | null
}

export interface PluginMarketItem {
  id: string
  name: string
  description: string
  version: string
  path: string
  author: string
  source_market: string
  component_types: string[]
  tags: string[]
  repository_url?: string | null
  homepage_url?: string | null
  downloads: number
  rating: number
}

export interface PluginMarketDetail extends PluginMarketItem {
  full_description: string
  components: PluginComponent[]
  version_history: PluginVersion[]
  config_options: PluginConfigOption[]
  created_at: string
  updated_at: string
}

export interface PluginComponent {
  name: string
  component_type: string
  description: string
  version: string
}

export interface PluginVersion {
  version: string
  release_notes: string
  released_at: string
}

export interface PluginConfigOption {
  name: string
  description: string
  required: boolean
  default_value: string | null
}

export interface InstallConfig {
  agentId: string
  agentType: 'cli' | 'sdk'
  outputFormat?: 'stream-json' | 'json'
  customCommand?: string
  customArgs?: string
  customEnv?: Record<string, string>
  scope?: 'global' | 'project'
  projectPath?: string | null
}

export type InstallStatus = 'idle' | 'installing' | 'success' | 'failed'

export interface InstallResult {
  success: boolean
  message: string
  session_id?: string | null
  rollback_performed?: boolean
  rollback_error?: string | null
  backup_location?: string | null
}

export interface MarketQuery {
  page?: number | null
  category?: string | null
  category_slug?: string | null
  search?: string | null
  source_market?: string | null
}

export interface MarketListResponse<T> {
  items: T[]
  total: number
  page?: number
  has_more?: boolean
  categories?: MarketCategory[]
}
