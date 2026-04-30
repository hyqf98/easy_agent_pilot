import type { AIFormRequest, Task, TaskInputRequest } from '@/types/plan'
import type { RustExecutionLog, TaskExecutionState } from '@/types/taskExecution'
import type { ToolCall } from '@/stores/message'
import { resolveTaskExecutionStatus } from '@/utils/taskExecutionStatus'
import { ACTIVE_EXECUTION_STATUSES, mapRustExecutionLog } from './taskExecutionShared'
import { mergeToolInputArguments } from '@/utils/toolInput'

export function markTaskStopRequested(taskId: string, stopRequestedTaskIds: Set<string>): void {
  stopRequestedTaskIds.add(taskId)
}

export function clearTaskStopRequested(taskId: string, stopRequestedTaskIds: Set<string>): void {
  stopRequestedTaskIds.delete(taskId)
}

export function isTaskStopRequested(taskId: string, stopRequestedTaskIds: Set<string>): boolean {
  return stopRequestedTaskIds.has(taskId)
}

export function canHydrateTaskLogs(state: TaskExecutionState | undefined): boolean {
  return !state || !ACTIVE_EXECUTION_STATUSES.has(state.status)
}

export function shouldKeepInMemoryLogs(
  state: TaskExecutionState | undefined,
  backendLogCount: number
): boolean {
  return Boolean(state && state.logs.length > 0 && state.logs.length >= backendLogCount)
}

export function hydrateTaskLogs(
  state: TaskExecutionState,
  rustLogs: RustExecutionLog[],
  task: Task | undefined
): void {
  state.logs = rustLogs.map(mapRustExecutionLog)
  state.status = resolveTaskExecutionStatus(task, state.status)

  // 从日志重建 toolCalls，以支持待办列表面板等功能
  rebuildToolCallsFromLogs(state)

  let lastModel: string | undefined
  let latestUsageMetadata: TaskExecutionState['logs'][number]['metadata']

  for (const log of state.logs) {
    if (typeof log.metadata?.model === 'string' && log.metadata.model.trim()) {
      lastModel = log.metadata.model.trim()
    }

    if (
      typeof log.metadata?.inputTokens === 'number'
      || typeof log.metadata?.outputTokens === 'number'
      || typeof log.metadata?.contextWindowOccupancy === 'number'
    ) {
      latestUsageMetadata = log.metadata
    }
  }

  state.tokenUsage = {
    inputTokens: latestUsageMetadata?.inputTokens ?? 0,
    outputTokens: latestUsageMetadata?.outputTokens ?? 0,
    model: lastModel,
    contextWindowOccupancy: latestUsageMetadata?.contextWindowOccupancy,
    resetCount: 0,
    lastUpdatedAt: state.logs[state.logs.length - 1]?.timestamp ?? null
  }
}

/**
 * 从已加载的日志重建 toolCalls 数组
 * 解析 tool_use / tool_input_delta / tool_result 类型的日志，
 * 还原完整的工具调用信息以支持待办列表面板等功能
 */
function rebuildToolCallsFromLogs(state: TaskExecutionState): void {
  const toolCalls: ToolCall[] = []
  const toolCallMap = new Map<string, ToolCall>()

  for (const log of state.logs) {
    if (log.type === 'tool_use') {
      const toolName = typeof log.metadata?.toolName === 'string' ? log.metadata.toolName : ''
      const toolCallId = typeof log.metadata?.toolCallId === 'string' ? log.metadata.toolCallId : ''
      if (!toolName || !toolCallId) continue

      let toolArguments: Record<string, unknown> = {}
      if (typeof log.metadata?.toolInput === 'string') {
        try {
          toolArguments = JSON.parse(log.metadata.toolInput)
        } catch {
          toolArguments = {}
        }
      }

      const toolCall: ToolCall = {
        id: toolCallId,
        name: toolName,
        arguments: toolArguments,
        status: 'running'
      }
      toolCalls.push(toolCall)
      toolCallMap.set(toolCallId, toolCall)
    } else if (log.type === 'tool_input_delta') {
      const toolCallId = typeof log.metadata?.toolCallId === 'string' ? log.metadata.toolCallId : ''
      if (!toolCallId) continue

      const targetToolCall = toolCallMap.get(toolCallId)
      if (!targetToolCall) continue

      let deltaInput: Record<string, unknown> = {}
      if (typeof log.metadata?.toolInput === 'string') {
        try {
          deltaInput = JSON.parse(log.metadata.toolInput)
        } catch {
          deltaInput = {}
        }
      }
      targetToolCall.arguments = mergeToolInputArguments(targetToolCall.arguments, deltaInput)
    } else if (log.type === 'tool_result') {
      const toolCallId = typeof log.metadata?.toolCallId === 'string' ? log.metadata.toolCallId : ''
      if (!toolCallId) continue

      const targetToolCall = toolCallMap.get(toolCallId)
      if (!targetToolCall) continue

      const result = typeof log.metadata?.toolResult === 'string'
        ? log.metadata.toolResult
        : log.content
      targetToolCall.result = result || ''

      const isError = Boolean(log.metadata?.isError)
      targetToolCall.status = isError ? 'error' : 'success'
      if (isError) {
        targetToolCall.errorMessage = result || ''
      }
    }
  }

  // 未收到 tool_result 的 toolCall 标记为 success
  for (const toolCall of toolCalls) {
    if (toolCall.status === 'running') {
      toolCall.status = 'success'
    }
  }

  state.toolCalls = toolCalls
}

export function buildTaskInputRequest(
  request: AIFormRequest,
  requestedAt: string = new Date().toISOString()
): TaskInputRequest {
  const formSchema = request.formSchema ?? request.forms?.[0]
  if (!formSchema) {
    throw new Error('AI 表单请求缺少表单结构')
  }

  return {
    formSchema,
    question: request.question,
    requestedAt
  }
}
