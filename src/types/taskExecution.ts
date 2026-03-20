/**
 * 任务执行相关类型定义
 */

import type { ToolCall } from '@/stores/message'

// 执行日志条目类型
export type ExecutionLogType = 'content' | 'thinking' | 'thinking_start' | 'tool_use' | 'tool_input_delta' | 'tool_result' | 'error' | 'system'

// 执行状态
export type ExecutionStatus = 'idle' | 'queued' | 'running' | 'waiting_input' | 'completed' | 'failed' | 'stopped'

// 执行日志条目
export interface ExecutionLogEntry {
  id: string
  taskId: string
  type: ExecutionLogType
  content: string
  timestamp: string
  metadata?: {
    toolName?: string
    toolCallId?: string
    toolInput?: string
    toolResult?: string
    isError?: boolean
  }
}

// 任务执行状态
export interface TaskExecutionState {
  taskId: string
  status: ExecutionStatus
  sessionId: string | null
  startedAt: string | null
  completedAt: string | null
  logs: ExecutionLogEntry[]
  accumulatedContent: string
  accumulatedThinking: string
  toolCalls: ToolCall[]
}

// 执行队列
export interface ExecutionQueue {
  planId: string
  currentTaskId: string | null
  pendingTaskIds: string[]
}

// 创建执行日志输入
export interface CreateExecutionLogInput {
  taskId: string
  type: ExecutionLogType
  content: string
  metadata?: {
    toolName?: string
    toolCallId?: string
    toolInput?: string
    toolResult?: string
    isError?: boolean
  }
}

// 后端返回的执行日志结构
export interface RustExecutionLog {
  id: string
  task_id: string
  type: string
  content: string
  metadata: string | null // JSON 字符串
  created_at: string
}

// 任务执行摘要
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
