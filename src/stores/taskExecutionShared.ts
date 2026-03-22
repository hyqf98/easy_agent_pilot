import type { TaskStatus } from '@/types/plan'
import type {
  CreateExecutionLogInput,
  ExecutionLogEntry,
  ExecutionLogType,
  ExecutionQueue,
  RustExecutionLog,
  TaskExecutionState
} from '@/types/taskExecution'

export interface PendingLogBuffer {
  content: string
  thinking: string
  lastFlushTime: number
  flushTimer: ReturnType<typeof setTimeout> | null
}

export const FLUSH_INTERVAL_MS = 2000
export const FLUSH_THRESHOLD_CHARS = 500
export const ACTIVE_EXECUTION_STATUSES = new Set<TaskExecutionState['status']>(['running', 'queued'])
export const TERMINAL_TASK_STATUSES = new Set<TaskStatus>(['completed', 'failed', 'cancelled'])

export function createExecutionState(taskId: string): TaskExecutionState {
  return {
    taskId,
    status: 'idle',
    sessionId: null,
    startedAt: null,
    completedAt: null,
    logs: [],
    accumulatedContent: '',
    accumulatedThinking: '',
    toolCalls: [],
    tokenUsage: {
      inputTokens: 0,
      outputTokens: 0,
      resetCount: 0,
      lastUpdatedAt: null
    }
  }
}

export function createExecutionQueue(planId: string): ExecutionQueue {
  return {
    planId,
    currentTaskId: null,
    pendingTaskIds: [],
    isPaused: false,
    lastInterruptedTaskId: null
  }
}

export function createPendingLogBuffer(): PendingLogBuffer {
  return {
    content: '',
    thinking: '',
    lastFlushTime: Date.now(),
    flushTimer: null
  }
}

export function createExecutionLogEntry(input: CreateExecutionLogInput): ExecutionLogEntry {
  return {
    id: `log-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`,
    taskId: input.taskId,
    type: input.type,
    content: input.content,
    timestamp: new Date().toISOString(),
    metadata: input.metadata
  }
}

export function createStreamLogEntry(
  taskId: string,
  type: Extract<ExecutionLogType, 'content' | 'thinking'>,
  content: string
): ExecutionLogEntry {
  return createExecutionLogEntry({
    taskId,
    type,
    content
  })
}

export function parseExecutionLogMetadata(metadata: string | null): ExecutionLogEntry['metadata'] {
  if (!metadata) return undefined

  try {
    return JSON.parse(metadata) as ExecutionLogEntry['metadata']
  } catch {
    return undefined
  }
}

export function mapRustExecutionLog(log: RustExecutionLog): ExecutionLogEntry {
  return {
    id: log.id,
    taskId: log.task_id,
    type: log.type as ExecutionLogType,
    content: log.content,
    timestamp: log.created_at,
    metadata: parseExecutionLogMetadata(log.metadata)
  }
}
