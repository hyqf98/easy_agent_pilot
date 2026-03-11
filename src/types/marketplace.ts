/**
 * 市场功能相关类型定义
 */

// MCP市场项（来自ModelScope）
export interface McpMarketItem {
  id: string
  name: string
  description: string
  author: string
  category: string
  tags: string[]
  transportType: 'stdio' | 'sse' | 'http'
  installCommand?: string    // 安装命令模板
  installArgs?: string       // 参数模板
  envTemplate?: string       // 环境变量模板
  logo?: string
  downloads?: number
  rating?: number
  repository_url?: string | null
  source_market: string
}

// MCP市场项详情
export interface McpMarketDetail extends McpMarketItem {
  full_description: string
  config_options: McpConfigOption[]
  version: string
  created_at: string
  updated_at: string
}

// MCP配置选项
export interface McpConfigOption {
  name: string
  description: string
  required: boolean
  default_value: string | null
}

// Skills市场项（来自Claude GitHub）
export interface SkillMarketItem {
  id: string
  name: string
  description: string
  path: string              // GitHub路径
  author: string
  category: string
  tags: string[]
  trigger_scenario: string
  source_market: string
  repository_url?: string | null
  downloads?: number
  rating?: number
}

// Skills市场项详情
export interface SkillMarketDetail extends SkillMarketItem {
  full_description: string
  skill_content: string     // skill文件内容
  created_at: string
  updated_at: string
}

// Plugin市场项（来自Claude GitHub）
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

// Plugin市场项详情
export interface PluginMarketDetail extends PluginMarketItem {
  full_description: string
  components: PluginComponent[]
  version_history: PluginVersion[]
  config_options: PluginConfigOption[]
  created_at: string
  updated_at: string
}

// Plugin组件
export interface PluginComponent {
  name: string
  component_type: string
  description: string
  version: string
}

// Plugin版本历史
export interface PluginVersion {
  version: string
  release_notes: string
  released_at: string
}

// Plugin配置选项
export interface PluginConfigOption {
  name: string
  description: string
  required: boolean
  default_value: string | null
}

// 市场类型
export type MarketType = 'mcp' | 'skills' | 'plugins'

// 安装配置
export interface InstallConfig {
  agentId: string
  agentType: 'cli' | 'sdk'
  outputFormat?: 'stream-json' | 'json'  // CLI类型需要
  customCommand?: string
  customArgs?: string
  customEnv?: Record<string, string>
  scope?: 'global' | 'project'
  projectPath?: string | null
}

// 安装状态
export type InstallStatus = 'idle' | 'installing' | 'success' | 'failed'

// 安装结果
export interface InstallResult {
  success: boolean
  message: string
  session_id?: string | null
  rollback_performed?: boolean
  rollback_error?: string | null
  backup_location?: string | null
}

// 市场查询参数
export interface MarketQuery {
  category?: string | null
  search?: string | null
  source_market?: string | null
}

// 市场列表响应
export interface MarketListResponse<T> {
  items: T[]
  total: number
}

// ============ MCP Market 网站特定类型 ============

// MCP Market 服务器卡片
export interface McpMarketServerCard {
  slug: string
  name: string
  author: string
  description: string
  category: string
  stars: number
  github_url: string
  is_official: boolean
  is_featured: boolean
  is_sponsored: boolean
}

// MCP Market 服务器详情
export interface McpMarketServerDetail extends McpMarketServerCard {
  categories: string[]
  tabs: {
    about: string
    readme: string
    faq: string
  }
  features: string[]
  use_cases: McpMarketUseCase[]
  related_skills: McpMarketSkillCard[]
  installation: {
    skillfish_command: string
  }
}

// MCP Market Skill卡片
export interface McpMarketSkillCard {
  slug: string
  name: string
  author: string
  description: string
  category: string
  stars: number
  github_url: string
  install_command: string
  download_url: string
}

// MCP Market Skill详情
export interface McpMarketSkillDetail extends McpMarketSkillCard {
  full_description: string
  skill_content: string
  use_cases: McpMarketUseCase[]
  related_servers: McpMarketServerCard[]
}

// MCP Market 分类
export interface McpMarketCategory {
  slug: string
  name: string
  icon: string
  count: number
  description: string
}

// MCP Market 使用案例
export interface McpMarketUseCase {
  title: string
  description: string
}

// MCP Market 客户端
export interface McpMarketClient {
  slug: string
  name: string
  description: string
  icon: string
  website_url: string
  features: string[]
  supported_platforms: string[]
}

// MCP Market 搜索参数
export interface McpMarketSearchParams {
  q?: string
  category?: string
  type?: 'servers' | 'skills'
  page?: number
}

// MCP Market 分页结果
export interface McpMarketPaginatedResult<T> {
  items: T[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

// MCP Market 首页数据
export interface McpMarketHomeData {
  official_servers: McpMarketServerCard[]
  featured_servers: McpMarketServerCard[]
  popular_servers: McpMarketServerCard[]
  recent_servers: McpMarketServerCard[]
  categories: McpMarketCategory[]
}
