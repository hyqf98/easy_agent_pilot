import { ref, computed, watch, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { usePlanStore } from '@/stores/plan'
import { useProjectStore } from '@/stores/project'
import { useTaskStore } from '@/stores/task'
import { useTaskExecutionStore } from '@/stores/taskExecution'
import { useNotificationStore } from '@/stores/notification'
import { useConfirmDialog } from '@/composables'
import type { Task, TaskStatus, TaskOrderItem } from '@/types/plan'

interface UseTaskBoardOptions {
  emit: (event: 'task-click', task: Task) => void
}

export function useTaskBoard(options: UseTaskBoardOptions) {
  const planStore = usePlanStore()
  const projectStore = useProjectStore()
  const taskStore = useTaskStore()
  const taskExecutionStore = useTaskExecutionStore()
  const notificationStore = useNotificationStore()
  const confirmDialog = useConfirmDialog()
  const { t } = useI18n()
  const emit = options.emit

  const showEditModal = ref(false)
  const editingTask = ref<Task | null>(null)

  const showCreateModal = ref(false)
  const createMode = ref<'create' | 'edit'>('create')

  const currentPlanId = computed(() => planStore.currentPlanId)
  const currentProjectId = computed(() => projectStore.currentProjectId)

  const currentPlan = computed(() => planStore.currentPlan)

  const currentExecutionQueue = computed(() => {
    if (!currentPlanId.value) return undefined
    return taskExecutionStore.getExecutionQueue(currentPlanId.value)
  })

  const hasInterruptedTasksAwaitingResume = computed(() => {
    if (!currentPlan.value || tasksByStatus.value.in_progress.length === 0) {
      return false
    }

    const queue = currentExecutionQueue.value
    const hasActiveQueueWork = Boolean(queue?.currentTaskId) || (queue?.pendingTaskIds.length ?? 0) > 0
    return !hasActiveQueueWork
      && (
        currentPlan.value.executionStatus === 'paused'
        || currentPlan.value.executionStatus === 'running'
      )
  })

  const isCurrentPlanPaused = computed(() =>
    currentExecutionQueue.value?.isPaused
    ?? hasInterruptedTasksAwaitingResume.value
  )

  // 是否为手动模式
  const isManualMode = computed(() => currentPlan.value?.splitMode === 'manual')

  // 新建任务模板
  const newTaskTemplate = reactive<Partial<Task>>({
    planId: '',
    title: '',
    description: '',
    status: 'pending',
    priority: 'medium',
    order: 0,
    retryCount: 0,
    maxRetries: 3
  })

  const emptyTasksByStatus: Record<TaskStatus, Task[]> = {
    pending: [],
    in_progress: [],
    completed: [],
    blocked: [],
    failed: [],
    cancelled: []
  }

  const tasks = computed(() => {
    if (currentPlanId.value) {
      return taskStore.tasks.filter(t => t.planId === currentPlanId.value)
    }

    if (!currentProjectId.value) {
      return []
    }

    return taskStore.getProjectLooseTasks(currentProjectId.value)
  })

  const tasksByStatus = computed(() => {
    if (tasks.value.length === 0) {
      return emptyTasksByStatus
    }

    const result: Record<TaskStatus, Task[]> = {
      pending: [],
      in_progress: [],
      completed: [],
      blocked: [],
      failed: [],
      cancelled: []
    }

    tasks.value.forEach(task => {
      if (result[task.status]) {
        result[task.status].push(task)
      }
    })

    Object.keys(result).forEach(status => {
      if (status === 'in_progress') {
        result[status as TaskStatus].sort((a, b) => {
          const aRunning = currentPlanId.value && taskExecutionStore.isTaskRunning(a.id) ? 0 : 1
          const bRunning = currentPlanId.value && taskExecutionStore.isTaskRunning(b.id) ? 0 : 1
          if (aRunning !== bRunning) return aRunning - bRunning
          return a.order - b.order
        })
        return
      }

      if (status === 'completed') {
        result[status as TaskStatus].sort((a, b) =>
          new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
        )
        return
      }

      result[status as TaskStatus].sort((a, b) => a.order - b.order)
    })

    return result
  })

  // 统计信息
  const taskStats = computed(() => ({
    total: tasks.value.length,
    pending: tasks.value.filter(t => t.status === 'pending').length,
    inProgress: tasks.value.filter(t => t.status === 'in_progress').length,
    completed: tasks.value.filter(t => t.status === 'completed').length,
    blocked: tasks.value.filter(t => t.status === 'blocked').length,
    failed: tasks.value.filter(t => t.status === 'failed').length
  }))

  const columns = computed<Array<{ status: TaskStatus; label: string; color: string }>>(() => [
    { status: 'pending', label: t('taskBoard.columns.pending'), color: 'gray' },
    { status: 'in_progress', label: t('taskBoard.columns.in_progress'), color: 'blue' },
    { status: 'completed', label: t('taskBoard.columns.completed'), color: 'green' },
    { status: 'blocked', label: t('taskBoard.columns.blocked'), color: 'yellow' },
    { status: 'failed', label: t('taskBoard.columns.failed'), color: 'red' }
  ])

  function warnDetachedTaskExecutionUnsupported(): void {
    notificationStore.warning('无法直接执行', '当前任务未绑定到有效计划，请先挂载到计划后再启动执行。')
  }

  function isTaskTrackedByExecution(taskId: string): boolean {
    const state = taskExecutionStore.getExecutionState(taskId)
    if (state && state.status !== 'idle' && state.status !== 'completed' && state.status !== 'failed') {
      return true
    }

    const queue = currentExecutionQueue.value
    return Boolean(
      queue
      && (
        queue.currentTaskId === taskId
        || queue.lastInterruptedTaskId === taskId
        || queue.pendingTaskIds.includes(taskId)
      )
    )
  }

  async function showUnmetDependencyDialog(task: Task) {
    const dependencyNames = taskStore.getUnmetDependencyTitles(task.id)
    const firstDependency = dependencyNames[0] ?? ''
    const dependencyList = dependencyNames.join('、')

    await confirmDialog.show({
      type: 'info',
      title: t('taskBoard.dependencyBlockedTitle'),
      message: dependencyNames.length > 0
        ? t('taskBoard.dependencyBlockedMessage', {
          task: task.title,
          dependencies: dependencyList,
          nextTask: firstDependency
        })
        : t('taskBoard.dependencyBlockedFallback', { task: task.title }),
      confirmLabel: t('common.gotIt'),
      cancelLabel: t('common.close'),
      confirmButtonType: 'primary'
    })
  }

  async function loadTasks() {
    if (currentPlanId.value) {
      await taskStore.loadTasks(currentPlanId.value)
      return
    }

    if (currentProjectId.value) {
      await taskStore.loadProjectLooseTasks(currentProjectId.value)
    }
  }

  watch([currentPlanId, currentProjectId], () => {
    void loadTasks()
  }, { immediate: true })

  async function handleTaskDrop(taskId: string, newStatus: TaskStatus) {
    const task = tasks.value.find(t => t.id === taskId)
    if (!task || task.status === newStatus) return

    const oldStatus = task.status
    const oldOrder = task.order

    if (taskExecutionStore.isTaskRunning(taskId)) {
      return
    }

    if (newStatus === 'in_progress' && oldStatus === 'pending' && !taskStore.areDependenciesMet(taskId)) {
      await showUnmetDependencyDialog(task)
      return
    }

    const targetColumnTasks = tasksByStatus.value[newStatus]
    // 使用最大 order + 1 确保新任务始终排在目标列底部
    const maxOrder = targetColumnTasks.reduce((max, t) => Math.max(max, t.order), -1)
    const newOrder = maxOrder + 1

    task.status = newStatus
    task.order = newOrder

    if (newStatus === 'pending' && oldStatus !== 'pending') {
      try {
        await taskExecutionStore.clearTaskLogs(taskId)
      } catch (error) {
        console.warn('Failed to clear task logs:', error)
      }
    }

    if (newStatus === 'in_progress' && oldStatus === 'pending') {
      if (!currentPlanId.value) {
        task.status = oldStatus
        task.order = oldOrder
        warnDetachedTaskExecutionUnsupported()
        return
      }

      try {
        await taskStore.updateTask(taskId, {
          status: newStatus,
          order: newOrder,
          errorMessage: undefined,
          blockReason: undefined
        })

        if (currentPlanId.value) {
          await taskExecutionStore.startTaskExecution(taskId)
        }
      } catch (error) {
        task.status = oldStatus
        task.order = oldOrder
        await loadTasks()
        console.error('Failed to start task execution:', error)
      }
      return
    }

    try {
      if (isTaskTrackedByExecution(taskId)) {
        await taskExecutionStore.detachTaskFromExecution(taskId)
      }

      await taskStore.updateTask(taskId, {
        status: newStatus,
        order: newOrder
      })

      if (currentPlanId.value) {
        await taskExecutionStore.synchronizePlanExecutionQueue(currentPlanId.value)
      }
    } catch (error) {
      task.status = oldStatus
      task.order = oldOrder
      await loadTasks()
      console.error('Failed to update task:', error)
    }
  }

  function collectTaskAndDescendantIds(taskId: string): string[] {
    const ids = new Set<string>([taskId])
    let changed = true

    while (changed) {
      changed = false
      tasks.value.forEach((task) => {
        if (task.parentId && ids.has(task.parentId) && !ids.has(task.id)) {
          ids.add(task.id)
          changed = true
        }
      })
    }

    return Array.from(ids)
  }

  async function handleTaskReorder(taskId: string, targetIndex: number) {
    const movedTask = tasks.value.find(t => t.id === taskId)
    if (!movedTask) return

    const sameStatusTasks = tasksByStatus.value[movedTask.status] as Task[]
    if (sameStatusTasks.length <= 1) return

    const currentIndex = sameStatusTasks.findIndex(t => t.id === taskId)
    if (currentIndex === -1 || currentIndex === targetIndex) return

    const newTaskList = sameStatusTasks.filter(t => t.id !== taskId)
    const insertIndex = Math.max(0, Math.min(targetIndex, newTaskList.length))
    newTaskList.splice(insertIndex, 0, movedTask)

    // 重新计算排序
    const orderUpdates: TaskOrderItem[] = newTaskList.map((task, index) => ({
      id: task.id,
      order: index
    }))

    newTaskList.forEach((task, index) => {
      task.order = index
    })

    try {
      await taskStore.reorderTasks(orderUpdates)
      if (currentPlanId.value && movedTask.status === 'in_progress') {
        await taskExecutionStore.synchronizePlanExecutionQueue(currentPlanId.value)
      }
    } catch (error) {
      loadTasks()
      console.error('Failed to reorder tasks:', error)
    }
  }

  function selectTask(task: Task) {
    taskStore.setCurrentTask(task.id)
    taskExecutionStore.setCurrentViewingTask(task.id)
    void taskExecutionStore.loadTaskLogs(task.id)
    emit('task-click', task)
  }

  function handleTaskEdit(task: Task) {
    editingTask.value = task
    showEditModal.value = true
  }

  async function handleTaskStop(task: Task) {
    try {
      // 单个任务停止：不暂停队列，允许自动推进到下一个可执行任务
      await taskExecutionStore.stopTaskExecution(task.id, {
        pauseQueue: false,
        autoAdvance: true
      })

      if (currentPlanId.value) {
        await taskExecutionStore.synchronizePlanExecutionQueue(currentPlanId.value)
      }
    } catch (error) {
      console.error('Failed to stop task:', error)
    }
  }

  async function handleTaskStart(task: Task) {
    if (!currentPlanId.value) {
      warnDetachedTaskExecutionUnsupported()
      return
    }

    if (!taskStore.areDependenciesMet(task.id)) {
      await showUnmetDependencyDialog(task)
      return
    }

    try {
      await taskExecutionStore.startTaskExecution(task.id)
    } catch (error) {
      notificationStore.warning('无法开始任务', error instanceof Error ? error.message : String(error))
    }
  }

  async function handleTaskResume(task: Task) {
    if (!currentPlanId.value) {
      warnDetachedTaskExecutionUnsupported()
      return
    }

    try {
      await taskExecutionStore.resumeTaskExecution(task.id)
      if (currentPlanId.value) {
        await taskExecutionStore.synchronizePlanExecutionQueue(currentPlanId.value)
      }
    } catch (error) {
      console.error('Failed to resume task:', error)
    }
  }

  async function handleTaskRetry(task: Task) {
    if (!currentPlanId.value) {
      warnDetachedTaskExecutionUnsupported()
      return
    }

    try {
      // 先清除持久化日志
      await taskExecutionStore.clearTaskLogs(task.id)

      await taskStore.updateTask(task.id, {
        status: 'in_progress',
        errorMessage: undefined
      })

      await taskExecutionStore.startTaskExecution(task.id)
    } catch (error) {
      console.error('Failed to retry task:', error)
    }
  }

  async function handleTaskDelete(task: Task) {
    const confirmed = await confirmDialog.danger(
      t('taskBoard.deleteTaskMessage', { name: task.title }),
      t('taskBoard.deleteTaskTitle')
    )

    if (confirmed) {
      try {
        const deletedTaskIds = collectTaskAndDescendantIds(task.id)
        const trackedTaskIds = deletedTaskIds.filter(isTaskTrackedByExecution)

        for (const trackedTaskId of trackedTaskIds) {
          await taskExecutionStore.detachTaskFromExecution(trackedTaskId)
        }

        await taskStore.deleteTask(task.id)

        if (currentPlanId.value) {
          await taskExecutionStore.synchronizePlanExecutionQueue(currentPlanId.value)
        }
      } catch (error) {
        console.error('Failed to delete task:', error)
      }
    }
  }

  async function handleExecuteAll() {
    if (!currentPlanId.value) return

    const pendingTasks = [...tasksByStatus.value.pending]
    if (pendingTasks.length === 0) return

    try {
      await planStore.startPlanExecution(currentPlanId.value)

      for (const task of pendingTasks) {
        await taskExecutionStore.enqueueTask(currentPlanId.value, task.id)
      }
      await taskExecutionStore.synchronizePlanExecutionQueue(currentPlanId.value)
    } catch (error) {
      console.error('Failed to execute all tasks:', error)
    }
  }

  // 仅恢复进行中列里的任务，不将待办任务加入执行队列
  async function handleStartExecution() {
    if (!currentPlanId.value) return

    try {
      await taskExecutionStore.resumeInProgressExecutionFlow(currentPlanId.value)
    } catch (error) {
      console.error('Failed to start execution:', error)
    }
  }

  async function handleToggleGlobalExecution() {
    if (!currentPlanId.value) return

    try {
      if (isCurrentPlanPaused.value) {
        await taskExecutionStore.resumePlanExecutionFlow(currentPlanId.value)
      } else {
        await taskExecutionStore.pausePlanExecutionFlow(currentPlanId.value)
      }
    } catch (error) {
      console.error('Failed to toggle global execution:', error)
    }
  }

  function handleEditSaved() {
    showEditModal.value = false
    editingTask.value = null
  }

  function openCreateTaskModal() {
    if (!currentPlanId.value) return

    Object.assign(newTaskTemplate, {
      planId: currentPlanId.value,
      title: '',
      description: '',
      status: 'pending',
      priority: 'medium',
      order: tasks.value.length,
      retryCount: 0,
      maxRetries: 3,
      implementationSteps: [],
      testSteps: [],
      acceptanceCriteria: []
    })

    createMode.value = 'create'
    showCreateModal.value = true
  }

  async function handleTaskCreated(taskData: Partial<Task>) {
    if (!currentPlanId.value) return

    try {
      await taskStore.createTask({
        planId: currentPlanId.value,
        title: taskData.title || '',
        description: taskData.description,
        priority: taskData.priority,
        expertId: taskData.expertId,
        agentId: taskData.agentId,
        modelId: taskData.modelId,
        order: tasks.value.length,
        maxRetries: taskData.maxRetries || 3,
        implementationSteps: taskData.implementationSteps,
        testSteps: taskData.testSteps,
        acceptanceCriteria: taskData.acceptanceCriteria,
        dependencies: taskData.dependencies,
        memoryLibraryIds: taskData.memoryLibraryIds
      })
      showCreateModal.value = false
    } catch (error) {
      console.error('Failed to create task:', error)
    }
  }

  async function markPlanAsReady() {
    if (!currentPlanId.value) return

    try {
      await planStore.markPlanAsReady(currentPlanId.value)
    } catch (error) {
      console.error('Failed to mark plan as ready:', error)
    }
  }

  return {
    planStore,
    taskStore,
    taskExecutionStore,
    notificationStore,
    confirmDialog,
    showEditModal,
    editingTask,
    showCreateModal,
    createMode,
    currentPlanId,
    currentProjectId,
    currentPlan,
    currentExecutionQueue,
    hasInterruptedTasksAwaitingResume,
    isCurrentPlanPaused,
    isManualMode,
    newTaskTemplate,
    tasks,
    tasksByStatus,
    taskStats,
    columns,
    isTaskTrackedByExecution,
    showUnmetDependencyDialog,
    loadTasks,
    handleTaskDrop,
    collectTaskAndDescendantIds,
    handleTaskReorder,
    selectTask,
    handleTaskEdit,
    handleTaskStop,
    handleTaskStart,
    handleTaskResume,
    handleTaskRetry,
    handleTaskDelete,
    handleExecuteAll,
    handleStartExecution,
    handleToggleGlobalExecution,
    handleEditSaved,
    openCreateTaskModal,
    handleTaskCreated,
    markPlanAsReady,
  }
}
