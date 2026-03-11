<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import draggable from 'vuedraggable'
import KanbanCard from './KanbanCard.vue'
import { useTaskExecutionStore } from '@/stores/taskExecution'
import type { Task, TaskStatus } from '@/types/plan'

const props = withDefaults(defineProps<{
  status: TaskStatus
  title: string
  color: string
  tasks: Task[]
}>(), {
  tasks: () => []
})

const emit = defineEmits<{
  (e: 'taskDrop', taskId: string, status: TaskStatus): void
  (e: 'taskClick', task: Task): void
  (e: 'taskReorder', taskId: string, targetIndex: number): void
  (e: 'taskEdit', task: Task): void
  (e: 'taskStop', task: Task): void
  (e: 'taskRetry', task: Task): void
  (e: 'taskDelete', task: Task): void
  (e: 'executeAll'): void
  (e: 'startExecution'): void
}>()

const taskExecutionStore = useTaskExecutionStore()
const { t } = useI18n()

// 本地任务列表（用于 vuedraggable）
const localTasks = ref<Task[]>([...(props.tasks || [])])

// 记录上一次的任务数据快照，用于检测变化
let lastTasksSnapshot: string = ''

// 监听外部 tasks 变化，同步到本地
// 需要检测任务属性变化（如 dependencies），而不仅仅是 ID 列表变化
watch(() => props.tasks, (newTasks) => {
  // 序列化任务数据以检测任何属性变化
  const snapshot = JSON.stringify((newTasks || []).map(t => ({
    id: t.id,
    status: t.status,
    dependencies: t.dependencies,
    order: t.order
  })))
  if (snapshot !== lastTasksSnapshot) {
    localTasks.value = [...(newTasks || [])]
    lastTasksSnapshot = snapshot
  }
}, { immediate: true })

// 拖拽组配置 - 只禁止正在运行的任务拖出（排队中的任务可以拖动）
const dragGroup = computed(() => ({
  name: 'tasks',
  pull: (value: any) => {
    const taskId = value?.element?.id
    // 只禁止正在运行的任务拖出（不包括排队中）
    if (taskId && taskExecutionStore.isTaskRunning(taskId)) {
      return false
    }
    return true
  },
  put: true
}))

// 检查是否允许移动 - 只禁止正在运行的任务拖拽（排队中的任务可以拖动）
function checkMove(evt: any): boolean {
  const task = evt.draggedContext?.element
  if (!task) return true

  // 只禁止正在运行的任务拖拽（不包括排队中）
  if (taskExecutionStore.isTaskRunning(task.id)) {
    return false
  }
  return true
}

// 拖拽变化处理
function onDragChange(evt: any) {
  if (evt.added) {
    // 从其他列拖入
    const { element } = evt.added
    emit('taskDrop', element.id, props.status)
  } else if (evt.moved) {
    // 同列内移动
    const { element, newIndex } = evt.moved
    emit('taskReorder', element.id, newIndex)
  }
}

// 处理任务点击
function handleTaskClick(task: Task) {
  emit('taskClick', task)
}

// 处理任务编辑
function handleTaskEdit(task: Task) {
  emit('taskEdit', task)
}

// 处理任务停止
function handleTaskStop(task: Task) {
  emit('taskStop', task)
}

// 处理任务重试
function handleTaskRetry(task: Task) {
  emit('taskRetry', task)
}

// 处理任务删除
function handleTaskDelete(task: Task) {
  emit('taskDelete', task)
}

// 处理一键执行
function handleExecuteAll() {
  emit('executeAll')
}

// 处理开始执行
function handleStartExecution() {
  emit('startExecution')
}
</script>

<template>
  <div class="kanban-column">
    <div class="column-header">
      <div class="header-left">
        <span
          class="column-dot"
          :class="color"
        />
        <span class="column-label">{{ title }}</span>
        <span class="column-count">{{ tasks.length }}</span>
      </div>
      <div class="header-right">
        <!-- 待办列：一键执行按钮 -->
        <button
          v-if="status === 'pending' && tasks.length > 0"
          class="btn-header btn-execute-all"
          :title="t('taskBoard.tooltips.executeAll')"
          @click="handleExecuteAll"
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polygon points="5 3 19 12 5 21 5 3" />
          </svg>
          <span>{{ t('taskBoard.actions.executeAll') }}</span>
        </button>
        <!-- 进行中列：开始执行按钮 -->
        <button
          v-if="status === 'in_progress' && tasks.length > 0"
          class="btn-header btn-start"
          :title="t('taskBoard.tooltips.startExecution')"
          @click="handleStartExecution"
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polygon points="5 3 19 12 5 21 5 3" />
          </svg>
          <span>{{ t('taskBoard.actions.startExecution') }}</span>
        </button>
      </div>
    </div>

    <draggable
      v-model="localTasks"
      :group="dragGroup"
      :move="checkMove"
      :animation="150"
      ghost-class="ghost-card"
      chosen-class="chosen-card"
      drag-class="dragging-card"
      class="column-body"
      item-key="id"
      :delay="100"
      :force-fallback="true"
      :fallback-tolerance="5"
      @change="onDragChange"
    >
      <template #item="{ element: task }">
        <div
          class="drag-item"
          :class="{ 'is-running': taskExecutionStore.isTaskRunning(task.id) }"
          :data-task-id="task.id"
        >
          <KanbanCard
            :task="task"
            @click="handleTaskClick"
            @edit="handleTaskEdit"
            @stop="handleTaskStop"
            @retry="handleTaskRetry"
            @delete="handleTaskDelete"
          />
        </div>
      </template>

      <template #footer>
        <div
          v-if="tasks.length === 0"
          class="empty-column"
        >
          <span>{{ t('taskBoard.emptyColumn') }}</span>
        </div>
      </template>
    </draggable>
  </div>
</template>

<style scoped>
.kanban-column {
  flex: 1;
  min-width: 280px;
  max-width: 400px;
  display: flex;
  flex-direction: column;
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-surface) 96%, #ffffff) 0%,
      color-mix(in srgb, var(--color-bg-secondary) 92%, #f8fbff) 100%
    );
  border-radius: var(--radius-lg, 12px);
  transition: background-color var(--transition-fast, 150ms), border-color var(--transition-fast, 150ms);
  border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
  box-shadow: 0 10px 30px rgba(15, 23, 42, 0.06);
}

.column-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3, 0.75rem);
  font-weight: var(--font-weight-semibold, 600);
}

.header-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
}

.column-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.column-dot.gray { background-color: #94a3b8; }
.column-dot.blue { background-color: #3b82f6; }
.column-dot.green { background-color: #10b981; }
.column-dot.yellow { background-color: #f59e0b; }
.column-dot.red { background-color: #ef4444; }

.column-label {
  font-size: var(--font-size-sm, 13px);
  color: var(--color-text-primary, #1e293b);
}

.header-right {
  display: flex;
  align-items: center;
  gap: var(--spacing-1, 0.25rem);
}

.btn-header {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.5rem;
  border: none;
  border-radius: var(--radius-sm, 4px);
  background-color: var(--color-primary, #3b82f6);
  color: #fff;
  font-size: 0.6875rem;
  font-weight: var(--font-weight-medium, 500);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-header:hover {
  background-color: var(--color-primary-dark, #2563eb);
  transform: translateY(-1px);
}

.btn-header:active {
  transform: translateY(0);
}

.btn-header span {
  line-height: 1;
}

.column-count {
  padding: 0.125rem 0.5rem;
  background-color: color-mix(in srgb, var(--color-surface) 88%, var(--color-bg-secondary));
  border: 1px solid color-mix(in srgb, var(--color-border) 56%, transparent);
  border-radius: var(--radius-full, 9999px);
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-text-secondary, #64748b);
  box-shadow: var(--shadow-xs, 0 1px 2px 0 rgb(0 0 0 / 0.05));
}

.column-body {
  flex: 1;
  padding: var(--spacing-2, 0.5rem);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2, 0.5rem);
  scrollbar-width: thin;
  scrollbar-color: var(--color-border, #e2e8f0) transparent;
  min-height: 100px;
}

.column-body::-webkit-scrollbar {
  width: 6px;
}

.column-body::-webkit-scrollbar-track {
  background: transparent;
}

.column-body::-webkit-scrollbar-thumb {
  background-color: var(--color-border, #e2e8f0);
  border-radius: var(--radius-full, 9999px);
}

.drag-item {
  padding: 0.375rem;
  border: 1px dashed color-mix(in srgb, var(--color-border-dark) 72%, transparent);
  border-radius: calc(var(--radius-md, 8px) + 2px);
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-surface) 78%, transparent),
      color-mix(in srgb, var(--color-bg-secondary) 62%, transparent)
    );
  cursor: grab;
  touch-action: none;
}

.drag-item:active {
  cursor: grabbing;
}

.drag-item.is-running {
  cursor: not-allowed;
}

.drag-item.is-running:active {
  cursor: not-allowed;
}

.empty-column {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-4, 1rem);
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-tertiary, #94a3b8);
  min-height: 80px;
  border: 2px dashed color-mix(in srgb, var(--color-border-dark) 72%, transparent);
  border-radius: var(--radius-md, 8px);
  margin: var(--spacing-2, 0.5rem);
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-surface) 82%, transparent),
      color-mix(in srgb, var(--color-bg-secondary) 76%, transparent)
    );
}

/* vuedraggable 拖拽样式 */
.ghost-card {
  opacity: 0.5;
  background: #c8ebfb !important;
  border: 2px dashed var(--color-primary, #3b82f6) !important;
  border-radius: var(--radius-md, 8px);
}

.chosen-card {
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  transform: scale(1.02);
}

.dragging-card {
  opacity: 0.8;
  background: #e0f2fe !important;
  cursor: grabbing !important;
}

[data-theme='dark'] .kanban-column {
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-surface) 94%, #0f172a) 0%,
      color-mix(in srgb, var(--color-bg-secondary) 90%, #020617) 100%
    );
  border-color: rgba(96, 165, 250, 0.12);
  box-shadow: 0 16px 34px rgba(2, 6, 23, 0.34);
}

[data-theme='dark'] .column-label {
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .column-count {
  background-color: rgba(15, 23, 42, 0.56);
  border-color: rgba(148, 163, 184, 0.18);
}

[data-theme='dark'] .empty-column {
  color: rgba(203, 213, 225, 0.72);
  border-color: rgba(148, 163, 184, 0.28);
  background:
    linear-gradient(
      180deg,
      rgba(15, 23, 42, 0.28),
      rgba(30, 41, 59, 0.4)
    );
}

[data-theme='dark'] .drag-item {
  border-color: rgba(148, 163, 184, 0.26);
  background:
    linear-gradient(
      180deg,
      rgba(15, 23, 42, 0.18),
      rgba(30, 41, 59, 0.34)
    );
}
</style>
