import type { BlockReason, TaskStatus } from '@/types/plan'
import type { ExecutionStatus } from '@/types/taskExecution'

interface ExecutionStatusDescriptor {
  label: string
  color: 'primary' | 'warning' | 'success' | 'error' | 'gray'
}

const executionStatusDescriptors: Record<ExecutionStatus, ExecutionStatusDescriptor> = {
  idle: { label: '等待执行', color: 'gray' },
  queued: { label: '排队中', color: 'warning' },
  running: { label: '执行中', color: 'primary' },
  waiting_input: { label: '等待输入', color: 'warning' },
  completed: { label: '执行完成', color: 'success' },
  failed: { label: '执行失败', color: 'error' },
  stopped: { label: '已停止', color: 'gray' }
}

export function resolveTaskExecutionStatus(task?: {
  status?: TaskStatus
  blockReason?: BlockReason
} | null, fallbackStatus?: ExecutionStatus): ExecutionStatus {
  if (task?.status === 'blocked' && task.blockReason === 'waiting_input') {
    return 'waiting_input'
  }

  if (task?.status === 'in_progress') {
    if (fallbackStatus === 'queued' || fallbackStatus === 'stopped' || fallbackStatus === 'waiting_input') {
      return fallbackStatus
    }
    return 'running'
  }

  if (task?.status === 'completed') {
    return 'completed'
  }

  if (task?.status === 'failed') {
    return 'failed'
  }

  if (task?.status === 'cancelled') {
    return 'stopped'
  }

  if (task?.status === 'pending' && fallbackStatus !== 'queued') {
    return 'idle'
  }

  return fallbackStatus ?? 'idle'
}

export function getTaskExecutionStatusMeta(status: ExecutionStatus): {
  label: string
  color: 'primary' | 'warning' | 'success' | 'error' | 'gray'
} {
  return executionStatusDescriptors[status] ?? executionStatusDescriptors.idle
}
