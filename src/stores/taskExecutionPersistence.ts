import { invoke } from '@tauri-apps/api/core'
import type {
  CreateExecutionLogInput,
  ExecutionLogEntry,
  PlanExecutionProgress,
  RustExecutionLog,
  SaveTaskExecutionResultInput,
  TaskExecutionResultRecord,
  TaskExecutionState
} from '@/types/taskExecution'
import {
  createExecutionLogEntry,
  createPendingLogBuffer,
  createStreamLogEntry,
  FLUSH_INTERVAL_MS,
  FLUSH_THRESHOLD_CHARS,
  type PendingLogBuffer
} from './taskExecutionShared'

export function initPendingBuffer(
  taskId: string,
  pendingLogBuffers: Map<string, PendingLogBuffer>
): PendingLogBuffer {
  let buffer = pendingLogBuffers.get(taskId)
  if (!buffer) {
    buffer = createPendingLogBuffer()
    pendingLogBuffers.set(taskId, buffer)
  }
  return buffer
}

export function clearPendingBuffer(
  taskId: string,
  pendingLogBuffers: Map<string, PendingLogBuffer>
): void {
  const buffer = pendingLogBuffers.get(taskId)
  if (!buffer) {
    return
  }

  if (buffer.flushTimer) {
    clearTimeout(buffer.flushTimer)
  }
  pendingLogBuffers.delete(taskId)
}

export async function flushPendingLogs(
  taskId: string,
  pendingLogBuffers: Map<string, PendingLogBuffer>
): Promise<void> {
  const buffer = pendingLogBuffers.get(taskId)
  if (!buffer) {
    return
  }

  if (buffer.flushTimer) {
    clearTimeout(buffer.flushTimer)
    buffer.flushTimer = null
  }

  const now = Date.now()
  const writes: Promise<void>[] = []

  if (buffer.content.trim()) {
    const content = buffer.content
    buffer.content = ''
    buffer.lastFlushTime = now
    writes.push(
      invoke('create_task_execution_log', {
        taskId,
        logType: 'content',
        content,
        metadata: null
      }).catch(error => {
        console.warn('[TaskExecution] Failed to persist content log:', error)
      }).then(() => {})
    )
  }

  if (buffer.thinking.trim()) {
    const thinking = buffer.thinking
    buffer.thinking = ''
    buffer.lastFlushTime = now
    writes.push(
      invoke('create_task_execution_log', {
        taskId,
        logType: 'thinking',
        content: thinking,
        metadata: null
      }).catch(error => {
        console.warn('[TaskExecution] Failed to persist thinking log:', error)
      }).then(() => {})
    )
  }

  await Promise.all(writes)
}

export function scheduleFlush(
  taskId: string,
  pendingLogBuffers: Map<string, PendingLogBuffer>
): void {
  const buffer = pendingLogBuffers.get(taskId)
  if (!buffer || buffer.flushTimer) {
    return
  }

  buffer.flushTimer = setTimeout(() => {
    void flushPendingLogs(taskId, pendingLogBuffers)
  }, FLUSH_INTERVAL_MS)
}

export function addStreamLogToBuffer(
  taskId: string,
  type: 'content' | 'thinking',
  content: string,
  executionStates: Map<string, TaskExecutionState>,
  pendingLogBuffers: Map<string, PendingLogBuffer>
): void {
  const buffer = initPendingBuffer(taskId, pendingLogBuffers)
  const state = executionStates.get(taskId)
  if (!state) {
    return
  }

  const entry = createStreamLogEntry(taskId, type, content)
  state.logs.push(entry)

  if (type === 'content') {
    buffer.content += content
  } else {
    buffer.thinking += content
  }

  const totalBuffered = buffer.content.length + buffer.thinking.length
  if (totalBuffered >= FLUSH_THRESHOLD_CHARS) {
    void flushPendingLogs(taskId, pendingLogBuffers)
  } else {
    scheduleFlush(taskId, pendingLogBuffers)
  }
}

export async function persistExecutionLog(input: CreateExecutionLogInput): Promise<ExecutionLogEntry> {
  const entry = createExecutionLogEntry(input)

  try {
    await invoke('create_task_execution_log', {
      taskId: input.taskId,
      logType: input.type,
      content: input.content,
      metadata: input.metadata ? JSON.stringify(input.metadata) : null
    })
  } catch (error) {
    console.warn('[TaskExecution] Failed to persist log:', error)
  }

  return entry
}

export async function saveTaskExecutionResultToBackend(input: SaveTaskExecutionResultInput): Promise<void> {
  try {
    await invoke('save_task_execution_result', { input })
  } catch (error) {
    console.warn('[TaskExecution] Failed to persist execution result:', error)
  }
}

export async function listRecentPlanResultsFromBackend(
  planId: string,
  limit: number = 5
): Promise<TaskExecutionResultRecord[]> {
  try {
    return await invoke<TaskExecutionResultRecord[]>('list_recent_plan_results', { planId, limit })
  } catch (error) {
    console.warn('[TaskExecution] Failed to load recent plan results:', error)
    return []
  }
}

export async function getPlanExecutionProgressFromBackend(planId: string): Promise<PlanExecutionProgress | null> {
  try {
    return await invoke<PlanExecutionProgress>('list_plan_execution_progress', { planId })
  } catch (error) {
    console.warn('[TaskExecution] Failed to load plan execution progress:', error)
    return null
  }
}

export async function loadTaskLogsFromBackend(taskId: string): Promise<RustExecutionLog[]> {
  return invoke<RustExecutionLog[]>('list_task_execution_logs', { taskId })
}

export async function clearTaskLogsFromBackend(taskId: string): Promise<void> {
  await invoke('clear_task_execution_logs', { taskId })
}

export async function clearPlanExecutionResultsFromBackend(planId: string): Promise<number> {
  return invoke<number>('clear_plan_execution_results', { planId })
}
