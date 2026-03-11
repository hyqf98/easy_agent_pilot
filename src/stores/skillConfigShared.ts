export type ConfigSource = 'database' | 'file'

export interface UnifiedConfigItem {
  id: string
  name: string
  enabled: boolean
  source: ConfigSource
  isReadOnly: boolean
}

export type McpTransportType = 'stdio' | 'sse' | 'http' | 'builtin'

export type McpConfigScope = 'user' | 'local' | 'project'

export interface UnifiedMcpConfig extends UnifiedConfigItem {
  transportType: McpTransportType
  scope: McpConfigScope
  command?: string
  args?: string[]
  env?: Record<string, string>
  url?: string
  headers?: Record<string, string>
}

export interface UnifiedSkillConfig extends UnifiedConfigItem {
  description?: string
  skillPath: string
  scriptsPath?: string
  referencesPath?: string
  assetsPath?: string
}

export interface UnifiedPluginConfig extends UnifiedConfigItem {
  version?: string
  description?: string
  pluginPath: string
}

export interface CliConfigPaths {
  configDir: string
  configFile: string
  cliType: string
}

export interface McpTool {
  name: string
  description: string
  inputSchema: Record<string, unknown>
}

export interface McpToolsListResult {
  success: boolean
  message: string
  tools: McpTool[]
}

export interface McpToolCallResult {
  success: boolean
  message: string
  result: Record<string, unknown>
  error?: string
}

export interface McpConfigInput {
  name: string
  transport_type: string
  command?: string
  args?: string[]
  env?: Record<string, string>
  url?: string
  headers?: Record<string, string>
}

export interface CliCapabilities {
  supportsMcp: boolean
  supportsSkills: boolean
  supportsPlugins: boolean
  mcpAddCommand: string | null
}

export interface ScannedMcpServer {
  name: string
  transport: 'stdio' | 'sse' | 'http'
  scope: 'user' | 'local' | 'project'
  command?: string
  args?: string[]
  env?: Record<string, string>
  url?: string
  headers?: Record<string, string>
}

export interface ScannedSkill {
  name: string
  path: string
  description: string | null
  frontmatter_name: string | null
  subdirectories: {
    has_scripts: boolean
    has_references: boolean
    has_assets: boolean
  }
}

export interface ScannedPlugin {
  name: string
  path: string
  enabled: boolean
  version: string | null
  description: string | null
  author: string | null
  subdirectories: {
    has_agents: boolean
    has_commands: boolean
    has_skills: boolean
    has_hooks: boolean
    has_scripts: boolean
  }
}

export interface ClaudeConfigScanResult {
  claude_dir: string
  mcp_servers: ScannedMcpServer[]
  skills: ScannedSkill[]
  plugins: ScannedPlugin[]
  scan_success: boolean
  error_message: string | null
}

export interface CliConfig {
  mcp_servers?: Record<string, CliMcpServerConfig>
  mcpServers?: Record<string, CliMcpServerConfig>
}

export interface CliMcpServerConfig {
  command?: string
  args?: string[]
  env?: Record<string, string>
  url?: string
  headers?: Record<string, string>
  disabled?: boolean
}

export interface RawAgentMcpConfig {
  id: string
  agent_id: string
  name: string
  transport_type: string
  command?: string
  args?: string
  env?: string
  url?: string
  headers?: string
  scope: string
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface RawAgentSkillsConfig {
  id: string
  agent_id: string
  name: string
  description?: string
  skill_path: string
  scripts_path?: string
  references_path?: string
  assets_path?: string
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface RawAgentPluginsConfig {
  id: string
  agent_id: string
  name: string
  version?: string
  description?: string
  plugin_path: string
  enabled: boolean
  created_at: string
  updated_at: string
}

export function transformDbMcpConfig(raw: RawAgentMcpConfig): UnifiedMcpConfig {
  return {
    id: raw.id,
    name: raw.name,
    enabled: raw.enabled,
    source: 'database',
    isReadOnly: false,
    transportType: raw.transport_type as McpTransportType,
    scope: raw.scope as McpConfigScope,
    command: raw.command,
    args: raw.args ? raw.args.split('\n').filter(Boolean) : undefined,
    env: raw.env ? JSON.parse(raw.env) : undefined,
    url: raw.url,
    headers: raw.headers ? JSON.parse(raw.headers) : undefined,
  }
}

export function transformDbSkillsConfig(raw: RawAgentSkillsConfig): UnifiedSkillConfig {
  return {
    id: raw.id,
    name: raw.name,
    enabled: raw.enabled,
    source: 'database',
    isReadOnly: false,
    description: raw.description,
    skillPath: raw.skill_path,
    scriptsPath: raw.scripts_path,
    referencesPath: raw.references_path,
    assetsPath: raw.assets_path,
  }
}

export function transformDbPluginsConfig(raw: RawAgentPluginsConfig): UnifiedPluginConfig {
  return {
    id: raw.id,
    name: raw.name,
    enabled: raw.enabled,
    source: 'database',
    isReadOnly: false,
    version: raw.version,
    description: raw.description,
    pluginPath: raw.plugin_path,
  }
}

export function transformCliMcpConfig(name: string, config: CliMcpServerConfig): UnifiedMcpConfig {
  return {
    id: `cli-${name}`,
    name,
    enabled: !config.disabled,
    source: 'file',
    isReadOnly: true,
    transportType: config.url ? (config.url.includes('/sse') ? 'sse' : 'http') : 'stdio',
    scope: 'user',
    command: config.command,
    args: config.args,
    env: config.env,
    url: config.url,
    headers: config.headers,
  }
}

export function transformCliSkill(skill: ScannedSkill): UnifiedSkillConfig {
  return {
    id: `cli-skill-${skill.name}`,
    name: skill.name,
    enabled: true,
    source: 'file',
    isReadOnly: true,
    description: skill.description || undefined,
    skillPath: skill.path,
    scriptsPath: skill.subdirectories.has_scripts ? `${skill.path}/scripts` : undefined,
    referencesPath: skill.subdirectories.has_references ? `${skill.path}/references` : undefined,
    assetsPath: skill.subdirectories.has_assets ? `${skill.path}/assets` : undefined,
  }
}

export function transformCliPlugin(plugin: ScannedPlugin): UnifiedPluginConfig {
  return {
    id: `cli-plugin-${plugin.name}`,
    name: plugin.name,
    enabled: plugin.enabled,
    source: 'file',
    isReadOnly: true,
    version: plugin.version || undefined,
    description: plugin.description || undefined,
    pluginPath: plugin.path,
  }
}

export function buildMcpConfigInput(config: UnifiedMcpConfig): McpConfigInput {
  return {
    name: config.name,
    transport_type: config.transportType,
    command: config.command,
    args: config.args,
    env: config.env,
    url: config.url,
    headers: config.headers,
  }
}
