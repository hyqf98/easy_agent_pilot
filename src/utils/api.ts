/**
 * API utility for wrapping Tauri invoke with error handling
 */
import { invoke } from '@tauri-apps/api/core'

export interface ApiError {
  message: string
  code?: string
  details?: unknown
  type?: ErrorType
}

/// 错误类型枚举
export enum ErrorType {
  /// CLI 命令无效
  CLI_PATH_INVALID = 'CLI_PATH_INVALID',
  /// CLI 命令不存在
  CLI_PATH_NOT_FOUND = 'CLI_PATH_NOT_FOUND',
  /// CLI 执行失败
  CLI_EXECUTION_FAILED = 'CLI_EXECUTION_FAILED',
  /// API 密钥无效（认证错误）
  API_AUTH_INVALID = 'API_AUTH_INVALID',
  /// API 密钥缺失
  API_KEY_MISSING = 'API_KEY_MISSING',
  /// 网络超时
  NETWORK_TIMEOUT = 'NETWORK_TIMEOUT',
  /// 网络连接失败
  NETWORK_CONNECTION_FAILED = 'NETWORK_CONNECTION_FAILED',
  /// MCP 连接失败
  MCP_CONNECTION_FAILED = 'MCP_CONNECTION_FAILED',
  /// MCP 初始化失败
  MCP_INIT_FAILED = 'MCP_INIT_FAILED',
  /// MCP 工具列表获取失败
  MCP_TOOLS_FAILED = 'MCP_TOOLS_FAILED',
  /// 数据库错误
  DATABASE_ERROR = 'DATABASE_ERROR',
  /// 未知错误
  UNKNOWN = 'UNKNOWN'
}

export class InvokeError extends Error {
  code?: string
  details?: unknown
  type: ErrorType

  constructor(message: string, code?: string, details?: unknown, type: ErrorType = ErrorType.UNKNOWN) {
    super(message)
    this.name = 'InvokeError'
    this.code = code
    this.details = details
    this.type = type
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function formatPrimitiveError(value: unknown): string | null {
  if (typeof value === 'number') {
    return `错误代码 ${value}`
  }
  if (typeof value === 'boolean') {
    return value ? 'true' : 'false'
  }
  return null
}

function extractObjectErrorMessage(value: Record<string, unknown>): string | null {
  const candidateKeys = ['message', 'error', 'reason', 'details', 'detail', 'cause']

  for (const key of candidateKeys) {
    const candidate = value[key]
    const primitive = formatPrimitiveError(candidate)
    if (primitive) {
      return primitive
    }
    if (typeof candidate === 'string' && candidate.trim()) {
      return candidate.trim()
    }
    if (isRecord(candidate)) {
      const nested = extractObjectErrorMessage(candidate)
      if (nested) {
        return nested
      }
    }
  }

  if (typeof value.code === 'number') {
    return `错误代码 ${value.code}`
  }

  if (typeof value.code === 'string' && value.code.trim()) {
    return `错误代码 ${value.code.trim()}`
  }

  try {
    const serialized = JSON.stringify(value)
    return serialized === '{}' ? null : serialized
  } catch {
    return null
  }
}

/**
 * Parse error from Tauri invoke result
 */
function parseError(error: unknown): ApiError {
  if (error instanceof Error) {
    return {
      message: error.message,
      details: error
    }
  }

  if (typeof error === 'string') {
    return { message: error }
  }

  const primitive = formatPrimitiveError(error)
  if (primitive) {
    return { message: primitive, details: error }
  }

  if (Array.isArray(error)) {
    const messages = error
      .map(item => parseError(item).message)
      .filter(Boolean)
    if (messages.length > 0) {
      return { message: messages.join('; '), details: error }
    }
  }

  if (isRecord(error)) {
    const message = extractObjectErrorMessage(error)
    if (message) {
      return { message, details: error }
    }
  }

  return { message: 'Unknown error', details: error }
}

/**
 * Wrapper for Tauri invoke with standardized error handling
 * @param cmd - The Tauri command name
 * @param args - The arguments to pass to the command
 * @returns Promise with the result or throws InvokeError
 */
export async function invokeApi<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args)
  } catch (error) {
    const apiError = parseError(error)
    throw new InvokeError(apiError.message, apiError.code, apiError.details)
  }
}

/**
 * Create a retry-friendly wrapper for API calls
 * Use this when you want to enable retry functionality in error notifications
 */
export function createRetryableApi<T>(
  cmd: string,
  args: Record<string, unknown>,
  onSuccess?: (result: T) => void,
  onError?: (error: InvokeError) => void
): () => Promise<T> {
  return async () => {
    try {
      const result = await invokeApi<T>(cmd, args)
      onSuccess?.(result)
      return result
    } catch (error) {
      const invokeError = error instanceof InvokeError ? error : new InvokeError(String(error))
      onError?.(invokeError)
      throw invokeError
    }
  }
}

/**
 * Get user-friendly error message from an error
 */
export function getErrorMessage(error: unknown, fallback = 'Unknown error'): string {
  const parsed = parseError(error)
  return parsed.message || fallback
}

/// 从错误消息中分类错误类型
export function classifyError(error: unknown): ErrorType {
  const message = getErrorMessage(error).toLowerCase()

  // CLI 相关错误
  if (message.includes('cli') && (message.includes('not found') || message.includes('不存在'))) {
    return ErrorType.CLI_PATH_NOT_FOUND
  }
  if (message.includes('cli') && (message.includes('invalid') || message.includes('无效'))) {
    return ErrorType.CLI_PATH_INVALID
  }
  if (message.includes('cli') && (message.includes('failed') || message.includes('失败') || message.includes('unable') || message.includes('无法'))) {
    return ErrorType.CLI_EXECUTION_FAILED
  }
  if (
    message.includes('路径未配置')
    || message.includes('path not configured')
    || message.includes('命令未配置')
    || message.includes('command not configured')
  ) {
    return ErrorType.CLI_PATH_NOT_FOUND
  }
  if (message.includes('os error 206') || message.includes('文件名或扩展名太长')) {
    return ErrorType.CLI_EXECUTION_FAILED
  }

  // API 认证相关错误
  if (message.includes('401') || message.includes('unauthorized') || message.includes('认证') || message.includes('api key')) {
    return ErrorType.API_AUTH_INVALID
  }
  if (message.includes('api') && (message.includes('missing') || message.includes('缺失'))) {
    return ErrorType.API_KEY_MISSING
  }

  // 网络相关错误
  if (message.includes('timeout') || message.includes('超时')) {
    return ErrorType.NETWORK_TIMEOUT
  }
  if (message.includes('connect') || message.includes('连接') || message.includes('network') || message.includes('网络')) {
    return ErrorType.NETWORK_CONNECTION_FAILED
  }

  // MCP 相关错误
  if (message.includes('mcp') && (message.includes('failed') || message.includes('失败'))) {
    return ErrorType.MCP_CONNECTION_FAILED
  }
  if (message.includes('initialize') || message.includes('初始化')) {
    return ErrorType.MCP_INIT_FAILED
  }
  if (message.includes('tools') && (message.includes('list') || message.includes('获取'))) {
    return ErrorType.MCP_TOOLS_FAILED
  }

  // 数据库错误
  if (message.includes('database') || message.includes('数据库') || message.includes('sqlite')) {
    return ErrorType.DATABASE_ERROR
  }

  return ErrorType.UNKNOWN
}

/// 创建带类型的错误
export function createTypedError(message: string, type?: ErrorType): InvokeError {
  const errorType = type || classifyError(message)
  return new InvokeError(message, undefined, undefined, errorType)
}

/// 判断是否为超时错误
export function isTimeoutError(error: unknown): boolean {
  return classifyError(error) === ErrorType.NETWORK_TIMEOUT
}

/// 判断是否为认证错误
export function isAuthError(error: unknown): boolean {
  const type = classifyError(error)
  return type === ErrorType.API_AUTH_INVALID || type === ErrorType.API_KEY_MISSING
}

/// 判断是否为 CLI 错误
export function isCliError(error: unknown): boolean {
  const type = classifyError(error)
  return type === ErrorType.CLI_PATH_INVALID ||
         type === ErrorType.CLI_PATH_NOT_FOUND ||
         type === ErrorType.CLI_EXECUTION_FAILED
}

/// 判断是否为 MCP 错误
export function isMcpError(error: unknown): boolean {
  const type = classifyError(error)
  return type === ErrorType.MCP_CONNECTION_FAILED ||
         type === ErrorType.MCP_INIT_FAILED ||
         type === ErrorType.MCP_TOOLS_FAILED
}
