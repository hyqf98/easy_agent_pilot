import type { AIFormRequest, Task, TaskInputRequest } from '@/types/plan'
import type { RustExecutionLog, TaskExecutionState } from '@/types/taskExecution'
import { resolveTaskExecutionStatus } from '@/utils/taskExecutionStatus'
import { ACTIVE_EXECUTION_STATUSES, mapRustExecutionLog } from './taskExecutionShared'

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
