import type { BlockReason, TaskStatus } from '@/types/plan'
import type { ExecutionStatus } from '@/types/taskExecution'

export function resolveTaskExecutionStatus(task?: {
  status?: TaskStatus
  blockReason?: BlockReason
} | null, fallbackStatus?: ExecutionStatus): ExecutionStatus {
  if (task?.status === 'blocked' && task.blockReason === 'waiting_input') {
    return 'waiting_input'
  }

  if (task?.status === 'in_progress') {
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
  switch (status) {
    case 'queued':
      return { label: '排队中', color: 'warning' }
    case 'running':
      return { label: '执行中', color: 'primary' }
    case 'waiting_input':
      return { label: '等待输入', color: 'warning' }
    case 'completed':
      return { label: '执行完成', color: 'success' }
    case 'failed':
      return { label: '执行失败', color: 'error' }
    case 'stopped':
      return { label: '已停止', color: 'gray' }
    default:
      return { label: '等待执行', color: 'gray' }
  }
}
