/**
 */

import type { ToolCall } from '@/stores/message'

export type ExecutionLogType = 'content' | 'thinking' | 'thinking_start' | 'tool_use' | 'tool_input_delta' | 'tool_result' | 'error' | 'system'

export type ExecutionStatus = 'idle' | 'queued' | 'running' | 'waiting_input' | 'completed' | 'failed' | 'stopped'

export interface ExecutionLogMetadata {
  toolName?: string
  toolCallId?: string
  toolInput?: string
  toolResult?: string
  isError?: boolean
  retryGroup?: string
  retryCount?: number
  maxRetries?: number
  retryDelaySeconds?: number
  errorMessage?: string
  failureKind?: string
  model?: string
  inputTokens?: number
  outputTokens?: number
  contextWindowOccupancy?: number
  strategy?: string
  runtime?: string
  expert?: string
  externalSessionId?: string
  systemMessageCount?: number
  userMessageCount?: number
  assistantMessageCount?: number
  historyMessageCount?: number
  hasPlanOverview?: boolean
  planOverview?: string
  recentResultsCount?: number
}

export interface TaskTokenUsageWindow {
  inputTokens: number
  outputTokens: number
  model?: string
  contextWindowOccupancy?: number
  resetCount: number
  lastUpdatedAt: string | null
}

export interface ExecutionLogEntry {
  id: string
  taskId: string
  type: ExecutionLogType
  content: string
  timestamp: string
  metadata?: ExecutionLogMetadata
}

export interface TaskExecutionState {
  taskId: string
  executionRunId: string | null
  status: ExecutionStatus
  sessionId: string | null
  startedAt: string | null
  completedAt: string | null
  logs: ExecutionLogEntry[]
  accumulatedContent: string
  accumulatedThinking: string
  toolCalls: ToolCall[]
  tokenUsage: TaskTokenUsageWindow
}

export interface ExecutionQueue {
  planId: string
  currentTaskId: string | null
  pendingTaskIds: string[]
  isPaused: boolean
  lastInterruptedTaskId: string | null
}

export interface CreateExecutionLogInput {
  taskId: string
  type: ExecutionLogType
  content: string
  metadata?: ExecutionLogMetadata
}

// 鍚庣杩斿洖鐨勬墽琛屾棩蹇楃粨鏋?
export interface RustExecutionLog {
  id: string
  task_id: string
  type: string
  content: string
  metadata: string | null // JSON 瀛楃涓?
  created_at: string
}

export interface TaskExecutionSummary {
  taskId: string
  status: ExecutionStatus
  logCount: number
  lastLogAt: string | null
}

export type TaskExecutionResultStatus = 'success' | 'failed'

export interface TaskExecutionResultRecord {
  id: string
  task_id: string
  plan_id: string
  task_title_snapshot: string
  task_description_snapshot: string | null
  result_status: TaskExecutionResultStatus
  result_summary: string | null
  result_files: string[]
  fail_reason: string | null
  created_at: string
}

export interface SaveTaskExecutionResultInput {
  task_id: string
  result_status: TaskExecutionResultStatus
  result_summary?: string | null
  result_files?: string[]
  fail_reason?: string | null
}

export interface PlanExecutionTaskProgress {
  task_id: string
  title: string
  status: string
  task_order: number
  expert_id?: string | null
  agent_id?: string | null
  model_id?: string | null
  last_result_status: TaskExecutionResultStatus | null
  last_result_summary: string | null
  last_result_files: string[]
  last_fail_reason: string | null
  last_result_at: string | null
  updated_at: string
}

export interface PlanExecutionProgress {
  plan_id: string
  execution_overview: string | null
  execution_overview_updated_at: string | null
  total_tasks: number
  pending_count: number
  in_progress_count: number
  completed_count: number
  blocked_count: number
  cancelled_count: number
  success_count: number
  failed_count: number
  tasks: PlanExecutionTaskProgress[]
}
