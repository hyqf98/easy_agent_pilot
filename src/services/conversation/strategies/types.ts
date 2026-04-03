import type { AgentConfig } from '@/stores/agent'
import type { Message, MessageAttachment } from '@/stores/message'
import type { FileEditTrace } from '@/types/fileTrace'

/**
 */
export interface McpServerConfig {
  id: string
  /** MCP 名称 */
  name: string
  /** 传输类型 */
  transportType: 'stdio' | 'sse' | 'http' | 'builtin'
  /** 命令 (stdio 类型) */
  command?: string
  args?: string
  env?: string
  /** URL (sse/http 类型) */
  url?: string
  headers?: string
}

/**
 * 工具定义
 */
export interface ToolDefinition {
  name: string
  description: string
  inputSchema: Record<string, unknown>
}

/**
 */
export interface PermissionConfig {
  allowFileRead: boolean
  allowFileWrite: boolean
  allowNetwork: boolean
  allowedPaths?: string[]
}

/**
 * 对话上下文
 */
export interface ConversationContext {
  /** 会话 ID */
  sessionId: string
  /** 智能体配置 */
  agent: AgentConfig
  /** 消息历史 */
  messages: Message[]
  /** 工具定义 */
  tools?: ToolDefinition[]
  permissions?: PermissionConfig
  /** 工作目录 */
  workingDirectory?: string
  mcpServers?: McpServerConfig[]
  executionMode?: 'chat' | 'task_split' | 'task_execution' | 'solo_execution'
  responseMode?: 'stream_text' | 'json_once'
  cliOutputFormat?: 'text' | 'json' | 'stream-json'
  jsonSchema?: string
  extraCliArgs?: string[]
  resumeSessionId?: string
}

/**
 * 流式事件类型
 */
export type StreamEventType =
  | 'content'
  | 'tool_use'
  | 'tool_input_delta'
  | 'tool_result'
  | 'error'
  | 'done'
  | 'thinking'
  | 'thinking_start'
  | 'file_edit'
  | 'usage'
  | 'system'

/**
 * 流式事件
 */
export interface StreamEvent {
  /** 事件类型 */
  type: StreamEventType
  /** 文本内容 */
  content?: string
  /** 工具名称 */
  toolName?: string
  toolInput?: Record<string, unknown>
  /** 工具调用 ID */
  toolCallId?: string
  /** 工具结果 */
  toolResult?: unknown
  error?: string
  /** 输入 token 数量 */
  inputTokens?: number
  /** 输出 token 数量 */
  outputTokens?: number
  /** 模型名称 */
  model?: string
  fileEdit?: FileEditTrace
  externalSessionId?: string
}

/**
 */
export interface AgentStrategy {
  /** 策略名称 */
  readonly name: string

  /**
   * @param agent 智能体配置
   */
  supports(agent: AgentConfig): boolean

  /**
   * @param context 对话上下文
   * @param onEvent 事件回调
   */
  execute(context: ConversationContext, onEvent: (event: StreamEvent) => void): Promise<void>

  /**
   */
  abort(): void
}

/**
 */
export interface CliExecutionRequest {
  /** 会话 ID */
  sessionId: string
  /** CLI 路径 */
  cliPath: string
  /** 模型 ID */
  modelId?: string
  /** 消息历史 */
  messages: Array<{
    role: 'user' | 'assistant' | 'system'
    content: string
    attachments?: MessageAttachment[]
  }>
  /** 工作目录 */
  workingDirectory?: string
  /** 允许的工具列表 */
  allowedTools?: string[]
  mcpServers?: McpServerConfig[]
}

/**
 */
export interface SdkExecutionRequest {
  /** 会话 ID */
  sessionId: string
  /** API 密钥 */
  apiKey: string
  /** API 端点 */
  baseUrl?: string
  /** 模型 ID */
  modelId: string
  /** 消息历史 */
  messages: Array<{
    role: 'user' | 'assistant' | 'system'
    content: string
    attachments?: MessageAttachment[]
  }>
  /** 系统提示 */
  systemPrompt?: string
  maxTokens?: number
  /** 工具定义 */
  tools?: ToolDefinition[]
  mcpServers?: McpServerConfig[]
}

/**
 */
export interface ExecutionRequest {
  sessionId: string
  planId?: string
  agentType: 'cli' | 'sdk'
  provider: string
  cliPath?: string
  apiKey?: string
  baseUrl?: string
  modelId?: string
  messages: Array<{
    role: 'user' | 'assistant' | 'system'
    content: string
    attachments?: MessageAttachment[]
  }>
  workingDirectory?: string
  allowedTools?: string[]
  systemPrompt?: string
  maxTokens?: number
  tools?: ToolDefinition[]
  cliOutputFormat?: 'text' | 'json' | 'stream-json'
  jsonSchema?: string
  extraCliArgs?: string[]
  mcpServers?: McpServerConfig[]
  executionMode?: 'chat' | 'task_split' | 'task_execution' | 'solo_execution'
  responseMode?: 'stream_text' | 'json_once'
  resumeSessionId?: string
}

export interface MessageInput {
  role: 'user' | 'assistant' | 'system'
  content: string
  attachments?: MessageAttachment[]
}

/**
 * 后端流式事件（CLI/SDK 共用）
 */
export interface BackendStreamEvent {
  /** 事件类型 */
  type: 'content' | 'tool_use' | 'tool_input_delta' | 'tool_result' | 'error' | 'done' | 'thinking' | 'thinking_start' | 'file_edit' | 'usage' | 'message_start' | 'system'
  /** 会话 ID */
  sessionId: string
  /** 内容 */
  content?: string
  /** 工具名称 */
  toolName?: string
  /** 工具调用 ID */
  toolCallId?: string
  /** 工具输入 */
  toolInput?: string
  /** 工具结果 */
  toolResult?: string
  error?: string
  /** 输入 token 数量 */
  inputTokens?: number
  /** 输出 token 数量 */
  outputTokens?: number
  /** 模型名称 */
  model?: string
  fileEdit?: FileEditTrace
  externalSessionId?: string
}

export type CliStreamEvent = BackendStreamEvent
export type SdkStreamEvent = BackendStreamEvent
