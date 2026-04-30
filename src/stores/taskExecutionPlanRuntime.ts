import type { Task, TaskStatus, Plan } from '@/types/plan'
import type { ExecutionQueue, TaskExecutionState } from '@/types/taskExecution'
import { ACTIVE_EXECUTION_STATUSES, TERMINAL_TASK_STATUSES } from './taskExecutionShared'

export function getPlanTasks(tasks: Task[], planId: string): Task[] {
  return tasks.filter(task => task.planId === planId)
}

export function computePlanRuntimeUpdate(
  plan: Plan,
  tasksInPlan: Task[],
  queue: ExecutionQueue | undefined,
  executionStates?: Map<string, TaskExecutionState>
): { status: Plan['status'], executionStatus: Plan['executionStatus'], currentTaskId?: string } | null {
  const currentTaskId = queue?.currentTaskId ?? null
  const hasQueued = (queue?.pendingTaskIds.length ?? 0) > 0
  const isPaused = queue?.isPaused ?? false
  const hasBlocked = tasksInPlan.some(task => task.status === 'blocked')
  const hasPending = tasksInPlan.some(task => task.status === 'pending')
  const hasInProgress = tasksInPlan.some(task => task.status === 'in_progress')
  const hasActiveExecution = tasksInPlan.some((task) => {
    const state = executionStates?.get(task.id)
    return state?.status === 'running' || state?.status === 'queued'
  })
  const allTerminal = tasksInPlan.length > 0 && tasksInPlan.every(task =>
    TERMINAL_TASK_STATUSES.has(task.status)
  )

  if (allTerminal) {
    if (
      plan.status === 'completed'
      && plan.executionStatus === 'completed'
      && !plan.currentTaskId
    ) {
      return null
    }

    return {
      status: 'completed',
      executionStatus: 'completed',
      currentTaskId: undefined
    }
  }

  if (isPaused || (!hasActiveExecution && hasInProgress)) {
    return {
      status: 'executing',
      executionStatus: 'paused',
      currentTaskId: undefined
    }
  }

  if (currentTaskId || hasQueued || hasActiveExecution) {
    if (
      plan.status === 'executing'
      && plan.executionStatus === 'running'
      && plan.currentTaskId === currentTaskId
    ) {
      return null
    }

    return {
      status: 'executing',
      executionStatus: 'running',
      currentTaskId: currentTaskId ?? undefined
    }
  }

  if (hasBlocked || hasPending || hasInProgress || plan.executionStatus === 'running' || Boolean(plan.currentTaskId)) {
    return {
      status: 'executing',
      executionStatus: 'paused',
      currentTaskId: undefined
    }
  }

  return null
}

export function shouldQueueReadyTask(
  taskId: string,
  queue: ExecutionQueue,
  state: TaskExecutionState | undefined
): boolean {
  return !(
    queue.currentTaskId === taskId
    || queue.pendingTaskIds.includes(taskId)
    || (state?.status ? ACTIVE_EXECUTION_STATUSES.has(state.status) : false)
  )
}

export function findNextExecutableTask(
  tasks: Task[],
  candidates: string[],
  areDependenciesMet: (taskId: string) => boolean
): string | null {
  for (const taskId of candidates) {
    const task = tasks.find(item => item.id === taskId)
    if (!task) {
      continue
    }

    if (task.status === 'blocked') {
      console.log('[TaskExecution] Skipping blocked task:', task.title)
      continue
    }

    if (!areDependenciesMet(taskId)) {
      console.log('[TaskExecution] Task dependencies not met:', task.title)
      continue
    }

    return taskId
  }

  return null
}

export function resetExecutionStateRuntime(state: TaskExecutionState | undefined): void {
  if (!state) {
    return
  }

  state.logs = []
  state.accumulatedContent = ''
  state.accumulatedThinking = ''
  state.toolCalls = []
  state.tokenUsage = {
    inputTokens: 0,
    outputTokens: 0,
    contextWindowOccupancy: undefined,
    resetCount: 0,
    lastUpdatedAt: null
  }
}

export function clearPlanExecutionRuntime(
  planId: string,
  executionQueues: Map<string, ExecutionQueue>,
  executionStates: Map<string, TaskExecutionState>
): void {
  const queue = executionQueues.get(planId)
  if (!queue) {
    return
  }

  if (queue.currentTaskId) {
    executionStates.delete(queue.currentTaskId)
  }

  queue.pendingTaskIds.forEach(taskId => {
    executionStates.delete(taskId)
  })
  executionQueues.delete(planId)
}

export function isTaskReadyForRuntime(status: TaskStatus): boolean {
  return status === 'pending' || status === 'in_progress'
}
