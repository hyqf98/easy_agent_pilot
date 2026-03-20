import { invoke } from '@tauri-apps/api/core'
import type { AgentConfig } from '@/stores/agent'
import type { McpServerConfig } from '@/services/conversation/strategies/types'
import type {
  CliConfig,
  CliMcpServerConfig,
  RawAgentMcpConfig
} from '@/stores/skillConfigShared'

function normalizeMcpToolPrefix(name: string): string {
  return name
    .trim()
    .replace(/\s+/g, '_')
}

function stringifyOptionalRecord(value?: Record<string, string>): string | undefined {
  if (!value || Object.keys(value).length === 0) {
    return undefined
  }

  return JSON.stringify(value)
}

function stringifyOptionalArgs(args?: string[]): string | undefined {
  if (!args || args.length === 0) {
    return undefined
  }

  return JSON.stringify(args)
}

function inferTransportType(config: CliMcpServerConfig): McpServerConfig['transportType'] {
  if (config.url) {
    return config.url.includes('/sse') ? 'sse' : 'http'
  }

  if (config.command) {
    return 'stdio'
  }

  return 'builtin'
}

function mapCliMcpServer(name: string, config: CliMcpServerConfig): McpServerConfig | null {
  if (config.disabled) {
    return null
  }

  const transportType = inferTransportType(config)
  if (transportType === 'builtin') {
    return null
  }

  return {
    id: `cli-${name}`,
    name,
    transportType,
    command: config.command,
    args: stringifyOptionalArgs(config.args),
    env: stringifyOptionalRecord(config.env),
    url: config.url,
    headers: stringifyOptionalRecord(config.headers)
  }
}

function mapSdkMcpServer(config: RawAgentMcpConfig): McpServerConfig | null {
  if (!config.enabled) {
    return null
  }

  return {
    id: config.id,
    name: config.name,
    transportType: config.transport_type as McpServerConfig['transportType'],
    command: config.command,
    args: config.args,
    env: config.env,
    url: config.url,
    headers: config.headers
  }
}

export async function loadAgentMcpServers(agent: AgentConfig): Promise<McpServerConfig[]> {
  if (agent.type === 'cli') {
    if (!agent.cliPath) {
      return []
    }

    const config = await invoke<CliConfig>('read_cli_config', {
      cliPath: agent.cliPath,
      cliType: agent.provider
    })
    const mcpServers = config.mcp_servers || config.mcpServers || {}

    return Object.entries(mcpServers)
      .map(([name, server]) => mapCliMcpServer(name, server))
      .filter((server): server is McpServerConfig => Boolean(server))
  }

  const configs = await invoke<RawAgentMcpConfig[]>('list_agent_mcp_configs', {
    agentId: agent.id
  })

  return configs
    .map(mapSdkMcpServer)
    .filter((server): server is McpServerConfig => Boolean(server))
}

export function appendClaudeMcpAllowedTools(
  baseTools: string[],
  mcpServers?: McpServerConfig[]
): string[] {
  const merged = [...baseTools]

  for (const server of mcpServers ?? []) {
    const serverName = normalizeMcpToolPrefix(server.name)
    if (!serverName) continue
    const toolName = `mcp__${serverName}`
    if (!merged.includes(toolName)) {
      merged.push(toolName)
    }
  }

  return merged
}
