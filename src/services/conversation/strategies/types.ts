import type { AgentConfig } from '@/stores/agent'
import type { Message, MessageAttachment } from '@/stores/message'
import type { FileEditTrace } from '@/types/fileTrace'

/**
 * MCP 服务器配置
 */
export interface McpServerConfig {
  /** MCP 配置 ID */
  id: string
  /** MCP 名称 */
  name: string
  /** 传输类型 */
  transportType: 'stdio' | 'sse' | 'http' | 'builtin'
  /** 命令 (stdio 类型) */
  command?: string
  /** 参数 (stdio 类型) */
  args?: string
  /** 环境变量 (stdio 类型) */
  env?: string
  /** URL (sse/http 类型) */
  url?: string
  /** 请求头 (sse/http 类型) */
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
 * 权限配置
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
  /** 权限配置 */
  permissions?: PermissionConfig
  /** 工作目录 */
  workingDirectory?: string
  /** MCP 服务器配置列表 */
  mcpServers?: McpServerConfig[]
  /** 执行模式（普通会话/任务拆分） */
  executionMode?: 'chat' | 'task_split'
  /** 响应模式（流式文本/单轮 JSON） */
  responseMode?: 'stream_text' | 'json_once'
  /** CLI 输出格式覆盖 */
  cliOutputFormat?: 'text' | 'json' | 'stream-json'
  /** JSON Schema（json_once 模式可选） */
  jsonSchema?: string
  /** 额外 CLI 参数 */
  extraCliArgs?: string[]
}

/**
 * 流式事件类型
 */
export type StreamEventType = 'content' | 'tool_use' | 'tool_input_delta' | 'tool_result' | 'error' | 'done' | 'thinking' | 'thinking_start' | 'file_edit' | 'usage' | 'system'

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
  /** 工具输入参数 */
  toolInput?: Record<string, unknown>
  /** 工具调用 ID */
  toolCallId?: string
  /** 工具结果 */
  toolResult?: unknown
  /** 错误信息 */
  error?: string
  /** 输入 token 数量 */
  inputTokens?: number
  /** 输出 token 数量 */
  outputTokens?: number
  /** 模型名称 */
  model?: string
  /** 文件编辑轨迹 */
  fileEdit?: FileEditTrace
}

/**
 * 智能体策略接口
 */
export interface AgentStrategy {
  /** 策略名称 */
  readonly name: string

  /**
   * 检查是否支持该智能体
   * @param agent 智能体配置
   */
  supports(agent: AgentConfig): boolean

  /**
   * 执行对话
   * @param context 对话上下文
   * @param onEvent 事件回调
   */
  execute(context: ConversationContext, onEvent: (event: StreamEvent) => void): Promise<void>

  /**
   * 中断执行
   */
  abort(): void
}

/**
 * CLI 执行请求
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
  /** MCP 服务器配置列表 */
  mcpServers?: McpServerConfig[]
}

/**
 * SDK 执行请求
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
  /** 最大令牌数 */
  maxTokens?: number
  /** 工具定义 */
  tools?: ToolDefinition[]
  /** MCP 服务器配置列表 */
  mcpServers?: McpServerConfig[]
}

/**
 * 统一执行请求
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
  executionMode?: 'chat' | 'task_split'
  responseMode?: 'stream_text' | 'json_once'
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
  /** 错误信息 */
  error?: string
  /** 输入 token 数量 */
  inputTokens?: number
  /** 输出 token 数量 */
  outputTokens?: number
  /** 模型名称 */
  model?: string
  /** 文件编辑轨迹 */
  fileEdit?: FileEditTrace
}

export type CliStreamEvent = BackendStreamEvent
export type SdkStreamEvent = BackendStreamEvent
