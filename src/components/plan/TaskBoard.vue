<script setup lang="ts">
import { ref, computed, watch, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import KanbanColumn from './KanbanColumn.vue'
import TaskEditModal from './TaskEditModal.vue'
import { usePlanStore } from '@/stores/plan'
import { useTaskStore } from '@/stores/task'
import { useTaskExecutionStore } from '@/stores/taskExecution'
import { useConfirmDialog } from '@/composables'
import type { Task, TaskStatus, TaskOrderItem } from '@/types/plan'

const planStore = usePlanStore()
const taskStore = useTaskStore()
const taskExecutionStore = useTaskExecutionStore()
const confirmDialog = useConfirmDialog()
const { t } = useI18n()
const emit = defineEmits<{
  (e: 'task-click', task: Task): void
}>()

const showEditModal = ref(false)
const editingTask = ref<Task | null>(null)

const showCreateModal = ref(false)
const createMode = ref<'create' | 'edit'>('create')

const currentPlanId = computed(() => planStore.currentPlanId)

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
  if (!currentPlanId.value) return []
  return taskStore.tasks.filter(t => t.planId === currentPlanId.value)
})

const tasksByStatus = computed(() => {
  if (!currentPlanId.value) return emptyTasksByStatus

  const result: Record<TaskStatus, Task[]> = {
    pending: [],
    in_progress: [],
    completed: [],
    blocked: [],
    failed: [],
    cancelled: []
  }

  tasks.value.forEach(t => {
    if (result[t.status]) {
      result[t.status].push(t)
    }
  })

  Object.keys(result).forEach(status => {
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

async function loadTasks() {
  if (currentPlanId.value) {
    await taskStore.loadTasks(currentPlanId.value)
  }
}

watch(currentPlanId, (newPlanId) => {
  if (newPlanId) {
    loadTasks()
  }
}, { immediate: true })

async function handleTaskDrop(taskId: string, newStatus: TaskStatus) {
  const task = tasks.value.find(t => t.id === taskId)
  if (!task || task.status === newStatus) return

  if (taskExecutionStore.isTaskExecuting(taskId)) {
    return
  }

  const targetColumnTasks = tasksByStatus.value[newStatus]
  const newOrder = targetColumnTasks.length

  const oldStatus = task.status
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
    try {
      await taskStore.updateTask(taskId, {
        status: newStatus,
        order: newOrder
      })

      if (currentPlanId.value) {
        await planStore.startPlanExecution(currentPlanId.value)
        if (isCurrentPlanPaused.value) {
          await taskExecutionStore.resumeTaskExecution(taskId)
        } else {
          await taskExecutionStore.enqueueTask(currentPlanId.value, taskId)
        }
      }
    } catch (error) {
      // 回滚
      console.error('Failed to start task execution:', error)
    }
    return
  }

  try {
    // 持久化更新
    await taskStore.updateTask(taskId, {
      status: newStatus,
      order: newOrder
    })
  } catch (error) {
    // 回滚
    task.status = oldStatus
    console.error('Failed to update task:', error)
  }
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
    // 更新后端
    await taskStore.reorderTasks(orderUpdates)
  } catch (error) {
    // 失败时回退到后端最新顺序
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
    await taskExecutionStore.stopTaskExecution(task.id)
  } catch (error) {
    console.error('Failed to stop task:', error)
  }
}

async function handleTaskResume(task: Task) {
  try {
    await taskExecutionStore.resumeTaskExecution(task.id)
  } catch (error) {
    console.error('Failed to resume task:', error)
  }
}

async function handleTaskRetry(task: Task) {
  try {
    if (currentPlanId.value) {
      // 先清除持久化日志
      await taskExecutionStore.clearTaskLogs(task.id)

      await taskStore.updateTask(task.id, {
        status: 'in_progress',
        errorMessage: undefined
      })

      await taskExecutionStore.enqueueTask(currentPlanId.value, task.id)
    }
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
      await taskStore.deleteTask(task.id)
    } catch (error) {
      console.error('Failed to delete task:', error)
    }
  }
}

async function handleExecuteAll() {
  if (!currentPlanId.value) return

  const pendingTasks = tasksByStatus.value.pending
  if (pendingTasks.length === 0) return

  try {
    await taskStore.batchStartTasks(currentPlanId.value)

    await planStore.startPlanExecution(currentPlanId.value)

    for (const task of pendingTasks) {
      await taskExecutionStore.enqueueTask(currentPlanId.value, task.id)
    }
  } catch (error) {
    console.error('Failed to execute all tasks:', error)
  }
}

// 从“执行中且暂停”状态恢复整条执行流
async function handleStartExecution() {
  if (!currentPlanId.value) return

  try {
    await taskExecutionStore.resumePlanExecutionFlow(currentPlanId.value)
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
      order: tasks.value.length,
      maxRetries: taskData.maxRetries || 3,
      implementationSteps: taskData.implementationSteps,
      testSteps: taskData.testSteps,
      acceptanceCriteria: taskData.acceptanceCriteria
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
</script>

<template>
  <div class="task-board">
    <div class="board-header">
      <div class="header-left">
        <h3 class="title">
          {{ t('taskBoard.title') }}
        </h3>
      </div>
      <div class="header-right">
        <button
          v-if="isManualMode && currentPlan?.status === 'planning' && tasks.length > 0"
          class="btn btn-secondary"
          @click="markPlanAsReady"
        >
          {{ t('taskBoard.actions.markSplitReady') }}
        </button>

        <div class="task-stats">
          <span class="stat-item completed">{{ t('taskBoard.stats.completed', { count: taskStats.completed }) }}</span>
          <span class="stat-item in-progress">{{ t('taskBoard.stats.inProgress', { count: taskStats.inProgress }) }}</span>
          <span class="stat-item blocked">{{ t('taskBoard.stats.blocked', { count: taskStats.blocked }) }}</span>
          <span class="stat-item pending">{{ t('taskBoard.stats.pending', { count: taskStats.pending }) }}</span>
          <span class="stat-item failed">{{ t('taskBoard.stats.failed', { count: taskStats.failed }) }}</span>
        </div>
      </div>
    </div>

    <div
      v-if="!currentPlanId"
      class="empty-state"
    >
      <span>{{ t('taskBoard.emptyNoPlan') }}</span>
    </div>

    <div
      v-else
      class="board-columns"
    >
      <KanbanColumn
        v-for="column in columns"
        :key="column.status"
        :status="column.status"
        :title="column.label"
        :color="column.color"
        :tasks="tasksByStatus[column.status] || []"
        :global-paused="column.status === 'in_progress' ? isCurrentPlanPaused : false"
        @task-drop="handleTaskDrop"
        @task-click="selectTask"
        @task-reorder="handleTaskReorder"
        @task-edit="handleTaskEdit"
        @task-stop="handleTaskStop"
        @task-resume="handleTaskResume"
        @task-retry="handleTaskRetry"
        @task-delete="handleTaskDelete"
        @execute-all="handleExecuteAll"
        @start-execution="handleStartExecution"
        @toggle-global-execution="handleToggleGlobalExecution"
        @add-task="openCreateTaskModal"
      />
    </div>

    <TaskEditModal
      v-if="editingTask"
      v-model:visible="showEditModal"
      :task="editingTask"
      @saved="handleEditSaved"
    />

    <TaskEditModal
      v-model:visible="showCreateModal"
      :task="newTaskTemplate as Task"
      mode="create"
      @saved="handleTaskCreated"
    />
  </div>
</template>

<style scoped>
.task-board {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: var(--color-bg-secondary, #f8fafc);
}

.board-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 1rem;
  border-bottom: 1px solid var(--color-border, #e2e8f0);
  background-color: var(--color-surface, #fff);
  gap: var(--spacing-3, 0.75rem);
}

.header-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-3, 0.75rem);
}

.header-right {
  display: flex;
  align-items: center;
  gap: var(--spacing-4, 1rem);
}

.title {
  margin: 0;
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
}
.task-stats {
  display: flex;
  align-items: center;
  gap: var(--spacing-3, 0.75rem);
  font-size: var(--font-size-xs, 12px);
}
.stat-item {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}
.stat-item.completed { color: #16a34a; }
.stat-item.in-progress { color: #2563eb; }
.stat-item.blocked { color: #f59e0b; }
.stat-item.pending { color: #64748b; }
.stat-item.failed { color: #ef4444; }
.empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-tertiary, #94a3b8);
  font-size: var(--font-size-sm, 13px);
}
.board-columns {
  flex: 1;
  display: flex;
  gap: var(--spacing-3, 0.75rem);
  padding: var(--spacing-3, 0.75rem);
  overflow-x: auto;
  scrollbar-width: thin;
  scrollbar-color: var(--scrollbar-thumb, var(--color-border, #e2e8f0)) var(--scrollbar-track, transparent);
}
.board-columns::-webkit-scrollbar {
  height: var(--scrollbar-size, 6px);
}
.board-columns::-webkit-scrollbar-track {
  background: var(--scrollbar-track, transparent);
}
.board-columns::-webkit-scrollbar-thumb {
  background-color: var(--scrollbar-thumb, var(--color-border, #e2e8f0));
  border-radius: var(--radius-full, 9999px);
  border: 1px solid transparent;
  background-clip: padding-box;
}
</style>
