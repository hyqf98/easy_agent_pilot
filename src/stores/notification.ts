import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { ErrorType, classifyError, getErrorMessage } from '@/utils/api'

export type NotificationType = 'error' | 'success' | 'warning' | 'info'

export interface Notification {
  id: string
  type: NotificationType
  title: string
  message?: string
  duration?: number
  retryAction?: () => Promise<void> | void
  retryLabel?: string
  createdAt: number
}

export const useNotificationStore = defineStore('notification', () => {
  // State
  const notifications = ref<Notification[]>([])
  const defaultDuration = 5000 // 5 seconds

  // Getters
  const visibleNotifications = computed(() => notifications.value.slice(0, 5))

  // Actions
  function generateId(): string {
    return `notification-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`
  }

  function show(notification: Omit<Notification, 'id' | 'createdAt'>): string {
    const id = generateId()
    const newNotification: Notification = {
      ...notification,
      id,
      createdAt: Date.now(),
      duration: notification.duration ?? (notification.type === 'error' ? 0 : defaultDuration)
    }

    notifications.value.push(newNotification)

    // Auto-dismiss if duration > 0
    if (newNotification.duration && newNotification.duration > 0) {
      setTimeout(() => {
        dismiss(id)
      }, newNotification.duration)
    }

    return id
  }

  function dismiss(id: string) {
    const index = notifications.value.findIndex(n => n.id === id)
    if (index !== -1) {
      notifications.value.splice(index, 1)
    }
  }

  function dismissAll() {
    notifications.value = []
  }

  // Convenience methods
  function error(title: string, message?: string, retryAction?: () => Promise<void> | void, retryLabel?: string): string {
    return show({
      type: 'error',
      title,
      message,
      retryAction,
      retryLabel,
      duration: 0 // Errors don't auto-dismiss by default
    })
  }

  function success(title: string, message?: string): string {
    return show({
      type: 'success',
      title,
      message
    })
  }

  function warning(title: string, message?: string): string {
    return show({
      type: 'warning',
      title,
      message
    })
  }

  function info(title: string, message?: string): string {
    return show({
      type: 'info',
      title,
      message
    })
  }

  // Handle network error with retry option
  function networkError(operation: string, err: unknown, retryAction?: () => Promise<void> | void): string {
    const errorMessage = err instanceof Error ? err.message : String(err)
    return error(
      operation,
      errorMessage,
      retryAction
    )
  }

  // Handle database error with retry option - similar to networkError but with specific label
  function databaseError(operation: string, err: unknown, retryAction?: () => Promise<void> | void, retryLabel?: string): string {
    const errorMessage = err instanceof Error ? err.message : String(err)
    return error(
      `数据库错误: ${operation}`,
      errorMessage,
      retryAction,
      retryLabel || '重试'
    )
  }

  // CLI 命令无效错误
  function cliPathError(path: string, err: unknown, retryAction?: () => Promise<void> | void): string {
    const errorMessage = getErrorMessage(err)
    return error(
      'CLI 命令无效',
      `命令 "${path}" 无效: ${errorMessage}`,
      retryAction,
      '重新配置'
    )
  }

  // API 认证错误
  function apiAuthError(err: unknown, retryAction?: () => Promise<void> | void): string {
    const errorMessage = getErrorMessage(err)
    return error(
      'API 认证失败',
      `API 密钥无效或已过期: ${errorMessage}`,
      retryAction,
      '检查配置'
    )
  }

  function cliExecutionError(operation: string, err: unknown, retryAction?: () => Promise<void> | void): string {
    const errorMessage = getErrorMessage(err)
    return error(
      operation,
      errorMessage,
      retryAction,
      '重试'
    )
  }

  // 网络超时错误
  function timeoutError(operation: string, retryAction?: () => Promise<void> | void): string {
    return error(
      '请求超时',
      `${operation}超时，请检查网络连接后重试`,
      retryAction,
      '重试'
    )
  }

  // MCP 连接错误
  function mcpConnectionError(mcpName: string, err: unknown, retryAction?: () => Promise<void> | void): string {
    const errorMessage = getErrorMessage(err)
    const errorType = classifyError(err)

    let detailMessage = errorMessage
    if (errorType === ErrorType.MCP_INIT_FAILED) {
      detailMessage = 'MCP 服务器初始化失败，请检查配置'
    } else if (errorType === ErrorType.MCP_TOOLS_FAILED) {
      detailMessage = '无法获取 MCP 工具列表'
    }

    return error(
      'MCP 连接失败',
      `"${mcpName}" 连接失败: ${detailMessage}`,
      retryAction,
      '重试连接'
    )
  }

  // 智能错误处理 - 根据错误类型自动选择合适的错误提示
  function smartError(operation: string, err: unknown, retryAction?: () => Promise<void> | void): string {
    const errorType = classifyError(err)
    const errorMessage = getErrorMessage(err)

    switch (errorType) {
      case ErrorType.CLI_PATH_NOT_FOUND:
      case ErrorType.CLI_PATH_INVALID:
        return cliPathError(operation, err, retryAction)
      case ErrorType.CLI_EXECUTION_FAILED:
        return cliExecutionError(operation, err, retryAction)

      case ErrorType.API_AUTH_INVALID:
      case ErrorType.API_KEY_MISSING:
        return apiAuthError(err, retryAction)

      case ErrorType.NETWORK_TIMEOUT:
        return timeoutError(operation, retryAction)

      case ErrorType.MCP_CONNECTION_FAILED:
      case ErrorType.MCP_INIT_FAILED:
      case ErrorType.MCP_TOOLS_FAILED:
        return mcpConnectionError(operation, err, retryAction)

      case ErrorType.DATABASE_ERROR:
        return databaseError(operation, err, retryAction)

      case ErrorType.NETWORK_CONNECTION_FAILED:
        return networkError(operation, errorMessage, retryAction)

      default:
        return error(operation, errorMessage, retryAction)
    }
  }

  return {
    // State
    notifications,
    visibleNotifications,
    // Actions
    show,
    dismiss,
    dismissAll,
    // Convenience methods
    error,
    success,
    warning,
    info,
    networkError,
    databaseError,
    // Specialized error methods
    cliPathError,
    cliExecutionError,
    apiAuthError,
    timeoutError,
    mcpConnectionError,
    smartError
  }
})
