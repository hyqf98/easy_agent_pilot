import type { AgentConfig, AgentProvider, AgentType } from '@/stores/agent'
import { inferAgentProvider } from '@/stores/agent'
import { appendClaudeMcpAllowedTools } from '@/utils/mcpServerConfig'
import type { ConversationContext, ExecutionRequest } from './strategies/types'

export type AgentRuntimeKey = 'claude-cli' | 'codex-cli' | 'opencode-cli' | 'claude-sdk' | 'codex-sdk'
export type AbortCommand = 'abort_cli_execution' | 'abort_sdk_execution'

type RuntimeResponseMode = ConversationContext['responseMode']
type RuntimeMcpServers = ConversationContext['mcpServers']

interface AgentRuntimeProfileDefinition {
  key: AgentRuntimeKey
  name: string
  agentType: AgentType
  provider: AgentProvider
  abortCommand: AbortCommand
  defaultCliPath?: string
  eventName: (sessionId: string) => string
  resolveAllowedTools?: (mcpServers?: RuntimeMcpServers) => string[]
  resolveCliOutputFormat?: (responseMode?: RuntimeResponseMode) => ExecutionRequest['cliOutputFormat']
  validate?: (agent: AgentConfig) => string | null
}

const runtimeProfiles: Record<AgentRuntimeKey, AgentRuntimeProfileDefinition> = {
  'claude-cli': {
    key: 'claude-cli',
    name: 'Claude CLI',
    agentType: 'cli',
    provider: 'claude',
    abortCommand: 'abort_cli_execution',
    defaultCliPath: 'claude',
    eventName: (sessionId) => `claude-stream-${sessionId}`,
    resolveAllowedTools: (mcpServers) => appendClaudeMcpAllowedTools(
      ['Read', 'Write', 'Edit', 'Glob', 'Grep', 'Bash', 'WebFetch', 'WebSearch'],
      mcpServers
    ),
    resolveCliOutputFormat: (responseMode) => responseMode === 'json_once' ? 'json' : 'stream-json'
  },
  'codex-cli': {
    key: 'codex-cli',
    name: 'Codex CLI',
    agentType: 'cli',
    provider: 'codex',
    abortCommand: 'abort_cli_execution',
    defaultCliPath: 'codex',
    eventName: (sessionId) => `codex-stream-${sessionId}`,
    resolveAllowedTools: () => ['Read', 'Write', 'Edit', 'Glob', 'Grep', 'Bash'],
    resolveCliOutputFormat: (responseMode) => responseMode === 'json_once' ? 'json' : 'stream-json'
  },
  'opencode-cli': {
    key: 'opencode-cli',
    name: 'OpenCode CLI',
    agentType: 'cli',
    provider: 'opencode',
    abortCommand: 'abort_cli_execution',
    defaultCliPath: 'opencode',
    eventName: (sessionId) => `opencode-stream-${sessionId}`,
    resolveAllowedTools: (mcpServers) => appendClaudeMcpAllowedTools(
      ['Read', 'Write', 'Edit', 'Glob', 'Grep', 'Bash', 'WebFetch', 'WebSearch'],
      mcpServers
    ),
    resolveCliOutputFormat: () => 'json'
  },
  'claude-sdk': {
    key: 'claude-sdk',
    name: 'Claude SDK',
    agentType: 'sdk',
    provider: 'claude',
    abortCommand: 'abort_sdk_execution',
    eventName: (sessionId) => `sdk-stream-${sessionId}`,
    validate: (agent) => {
      if (!agent.apiKey) {
        return 'API Key 未配置，请在智能体设置中配置 API Key'
      }
      if (!agent.modelId) {
        return '模型 ID 未配置，请在智能体设置中选择模型'
      }
      return null
    }
  },
  'codex-sdk': {
    key: 'codex-sdk',
    name: 'Codex SDK',
    agentType: 'sdk',
    provider: 'codex',
    abortCommand: 'abort_sdk_execution',
    eventName: (sessionId) => `codex-sdk-stream-${sessionId}`,
    validate: (agent) => {
      if (!agent.apiKey) {
        return 'API Key 未配置，请在智能体设置中配置 API Key'
      }
      if (!agent.modelId) {
        return '模型 ID 未配置，请在智能体设置中选择模型'
      }
      return null
    }
  }
}

function normalizeRuntimeProvider(agent: Pick<AgentConfig, 'type' | 'provider' | 'name' | 'cliPath'>): AgentProvider | undefined {
  return inferAgentProvider(agent) ?? (agent.type === 'sdk' ? 'claude' : undefined)
}

function normalizeRuntimeKey(agent: Pick<AgentConfig, 'type' | 'provider' | 'name' | 'cliPath'>): AgentRuntimeKey | null {
  const provider = normalizeRuntimeProvider(agent)
  if (!provider) {
    return null
  }

  return `${provider}-${agent.type}` as AgentRuntimeKey
}

function normalizeCliModelId(modelId?: string): string | undefined {
  const normalized = modelId?.trim()
  if (!normalized || normalized === 'default') {
    return undefined
  }
  return normalized
}

function normalizeSdkModelId(modelId?: string): string | undefined {
  const normalized = modelId?.trim()
  return normalized || undefined
}

export function getAgentRuntimeProfile(runtimeKey: AgentRuntimeKey): AgentRuntimeProfileDefinition {
  return runtimeProfiles[runtimeKey]
}

export function resolveAgentRuntimeProfile(
  agent: Pick<AgentConfig, 'type' | 'provider' | 'name' | 'cliPath'>
): AgentRuntimeProfileDefinition | null {
  const runtimeKey = normalizeRuntimeKey(agent)
  return runtimeKey ? runtimeProfiles[runtimeKey] : null
}

export function matchesAgentRuntimeProfile(
  agent: Pick<AgentConfig, 'type' | 'provider' | 'name' | 'cliPath'>,
  runtimeKey: AgentRuntimeKey
): boolean {
  return normalizeRuntimeKey(agent) === runtimeKey
}

export function validateAgentRuntime(
  agent: AgentConfig,
  runtimeKey: AgentRuntimeKey
): string | null {
  return runtimeProfiles[runtimeKey].validate?.(agent) ?? null
}

interface BuildAgentExecutionRequestOptions {
  sessionId: string
  planId?: string
  agent: AgentConfig
  messages: ExecutionRequest['messages']
  workingDirectory?: string
  mcpServers?: ExecutionRequest['mcpServers']
  tools?: ExecutionRequest['tools']
  cliOutputFormat?: ExecutionRequest['cliOutputFormat']
  jsonSchema?: ExecutionRequest['jsonSchema']
  extraCliArgs?: ExecutionRequest['extraCliArgs']
  executionMode?: ExecutionRequest['executionMode']
  responseMode?: ExecutionRequest['responseMode']
  modelId?: string
  systemPrompt?: string
  maxTokens?: number
  resumeSessionId?: string
}

export function buildAgentExecutionRequest(
  options: BuildAgentExecutionRequestOptions
): ExecutionRequest {
  const profile = resolveAgentRuntimeProfile(options.agent)
  if (!profile) {
    throw new Error(`不支持的智能体类型: ${options.agent.type} (${options.agent.provider || 'unknown'})`)
  }

  const modelId = profile.agentType === 'cli'
    ? normalizeCliModelId(options.modelId ?? options.agent.modelId)
    : normalizeSdkModelId(options.modelId ?? options.agent.modelId)

  return {
    sessionId: options.sessionId,
    planId: options.planId,
    agentType: profile.agentType,
    provider: profile.provider,
    cliPath: profile.agentType === 'cli'
      ? (options.agent.cliPath || profile.defaultCliPath)
      : undefined,
    apiKey: profile.agentType === 'sdk' ? options.agent.apiKey : undefined,
    baseUrl: profile.agentType === 'sdk' ? options.agent.baseUrl : undefined,
    modelId,
    messages: options.messages,
    workingDirectory: options.workingDirectory,
    allowedTools: profile.resolveAllowedTools?.(options.mcpServers),
    systemPrompt: options.systemPrompt,
    maxTokens: profile.agentType === 'sdk' ? (options.maxTokens ?? 4096) : undefined,
    tools: options.tools,
    cliOutputFormat: profile.agentType === 'cli'
      ? (options.cliOutputFormat ?? profile.resolveCliOutputFormat?.(options.responseMode))
      : undefined,
    jsonSchema: options.jsonSchema,
    extraCliArgs: options.extraCliArgs,
    mcpServers: options.mcpServers,
    executionMode: options.executionMode ?? 'chat',
    responseMode: options.responseMode ?? 'stream_text',
    resumeSessionId: options.resumeSessionId
  }
}
