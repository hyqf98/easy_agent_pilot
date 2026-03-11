<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AITaskItem, TaskPriority } from '@/types/plan'
import { useConfirmDialog } from '@/composables'
import TaskSplitPreviewCard from './TaskSplitPreviewCard.vue'
import TaskSplitPreviewEditor from './TaskSplitPreviewEditor.vue'

const props = defineProps<{
  tasks: AITaskItem[]
}>()

const emit = defineEmits<{
  (e: 'update', index: number, updates: Partial<AITaskItem>): void
  (e: 'remove', index: number): void
  (e: 'add', task: AITaskItem): void
  (e: 'resplit', index: number): void
}>()

const editingIndex = ref<number | null>(null)
const confirmDialog = useConfirmDialog()
const { t } = useI18n()

const priorityOptions = computed(() => [
  { label: t('taskSplit.priority.low'), value: 'low' as const },
  { label: t('taskSplit.priority.medium'), value: 'medium' as const },
  { label: t('taskSplit.priority.high'), value: 'high' as const }
])

const priorityColors: Record<TaskPriority, string> = {
  low: 'green',
  medium: 'yellow',
  high: 'red'
}

function startEdit(index: number) {
  editingIndex.value = index
}

function cancelEdit() {
  editingIndex.value = null
}

function saveEdit(index: number, updates: Partial<AITaskItem>) {
  emit('update', index, updates)
  editingIndex.value = null
}

async function removeTask(index: number) {
  const task = props.tasks[index]
  const taskName = task?.title?.trim() || `${t('taskSplit.newTask')} ${index + 1}`
  const confirmed = await confirmDialog.danger(
    t('taskSplit.removeTaskConfirmMessage', { name: taskName }),
    t('taskSplit.removeTaskConfirmTitle')
  )

  if (confirmed) {
    emit('remove', index)
    if (editingIndex.value === index) {
      editingIndex.value = null
    }
  }
}

function addTask() {
  const newTask: AITaskItem = {
    title: t('taskSplit.newTask'),
    description: '',
    priority: 'medium',
    implementationSteps: [],
    testSteps: [],
    acceptanceCriteria: [],
    dependsOn: []
  }

  const nextIndex = props.tasks.length
  emit('add', newTask)
  editingIndex.value = nextIndex
}
</script>

<template>
  <div class="task-preview">
    <div class="preview-header">
      <h4>
        <span class="header-icon">📋</span>
        {{ t('taskSplit.taskList') }}
        <span class="task-count">{{ t('taskSplit.taskCount', { count: tasks.length }) }}</span>
      </h4>
      <button
        class="btn-add"
        @click="addTask"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="M12 5v14M5 12h14" />
        </svg>
        {{ t('taskSplit.addTask') }}
      </button>
    </div>

    <div class="task-list">
      <div
        v-for="(task, index) in tasks"
        :key="index"
        class="task-item"
        :class="{ editing: editingIndex === index }"
      >
        <TaskSplitPreviewCard
          v-if="editingIndex !== index"
          :task="task"
          :index="index"
          :priority-colors="priorityColors"
          @edit="startEdit(index)"
          @remove="removeTask(index)"
          @resplit="emit('resplit', index)"
        />

        <TaskSplitPreviewEditor
          v-else
          :task="task"
          :tasks="tasks"
          :index="index"
          :priority-options="priorityOptions"
          @save="saveEdit(index, $event)"
          @cancel="cancelEdit"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.task-preview {
  height: 100%;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-top: none;
  overflow: hidden;
}

.preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3, 0.75rem) var(--spacing-4, 1rem);
  background-color: var(--color-bg-secondary, #f8fafc);
  border-bottom: 1px solid var(--color-border, #e2e8f0);
}

.preview-header h4 {
  margin: 0;
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
}

.header-icon {
  font-size: 1rem;
}

.task-count {
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-normal, 400);
  color: var(--color-text-secondary, #64748b);
  padding: 0.125rem 0.5rem;
  background-color: var(--color-bg, #e2e8f0);
  border-radius: var(--radius-full, 9999px);
}

.btn-add {
  display: flex;
  align-items: center;
  gap: var(--spacing-1, 0.25rem);
  padding: var(--spacing-1, 0.25rem) var(--spacing-3, 0.75rem);
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-md, 8px);
  background-color: var(--color-surface, #fff);
  color: var(--color-text-secondary, #64748b);
  font-size: var(--font-size-xs, 12px);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-add:hover {
  background-color: var(--color-primary-light, #dbeafe);
  border-color: var(--color-primary, #60a5fa);
  color: var(--color-primary, #3b82f6);
}

.task-list {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-3, 0.75rem);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2, 0.5rem);
}

.task-item {
  transition: all var(--transition-fast, 150ms);
}

.task-item.editing {
  border-radius: var(--radius-md, 8px);
  box-shadow: 0 0 0 3px var(--color-primary-light, #dbeafe);
}
</style>
