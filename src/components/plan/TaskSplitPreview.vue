<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AITaskItem, TaskPriority } from '@/types/plan'
import { useConfirmDialog } from '@/composables'
import EaModal from '@/components/common/EaModal.vue'
import TaskSplitPreviewCard from './TaskSplitPreviewCard.vue'
import TaskSplitPreviewEditor from './TaskSplitPreviewEditor.vue'

const props = defineProps<{
  tasks: AITaskItem[]
  disableActions?: boolean
}>()

const emit = defineEmits<{
  (e: 'update', index: number, updates: Partial<AITaskItem>): void
  (e: 'remove', index: number): void
  (e: 'add', task: AITaskItem): void
}>()

const editingIndex = ref<number | null>(null)
const editorRef = ref<InstanceType<typeof TaskSplitPreviewEditor> | null>(null)
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

const editingTask = computed(() => {
  if (editingIndex.value === null) {
    return null
  }
  return props.tasks[editingIndex.value] ?? null
})

function startEdit(index: number) {
  if (props.disableActions) {
    return
  }
  editingIndex.value = index
}

function cancelEdit() {
  editingIndex.value = null
}

function saveEdit(index: number, updates: Partial<AITaskItem>) {
  emit('update', index, updates)
  editingIndex.value = null
}

function saveEditFromModal() {
  editorRef.value?.triggerSave()
}

async function removeTask(index: number) {
  if (props.disableActions) {
    return
  }
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
  if (props.disableActions) {
    return
  }

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
      <div class="preview-actions">
        <button
          class="btn-add"
          :disabled="disableActions"
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
    </div>

    <div class="task-list">
      <div
        v-for="(task, index) in tasks"
        :key="index"
        class="task-item"
      >
        <TaskSplitPreviewCard
          :task="task"
          :index="index"
          :priority-colors="priorityColors"
          :disable-actions="disableActions"
          @edit="startEdit(index)"
          @remove="removeTask(index)"
        />
      </div>
    </div>

    <EaModal
      :visible="editingIndex !== null && !!editingTask"
      content-class="task-split-preview-modal"
      overlay-class="task-split-preview-modal-overlay"
      @update:visible="value => !value && cancelEdit()"
    >
      <template #header>
        <div class="editor-modal-header">
          <div class="editor-modal-title">
            {{ editingTask?.title?.trim() || t('taskSplit.newTask') }}
          </div>
          <button
            type="button"
            class="editor-modal-close"
            @click="cancelEdit"
          >
            ×
          </button>
        </div>
      </template>

      <TaskSplitPreviewEditor
        v-if="editingTask !== null && editingIndex !== null"
        ref="editorRef"
        :task="editingTask"
        :tasks="tasks"
        :index="editingIndex"
        :priority-options="priorityOptions"
        @save="saveEdit(editingIndex, $event)"
        @cancel="cancelEdit"
      />

      <template #footer>
        <div class="editor-modal-footer">
          <button
            type="button"
            class="btn btn-secondary"
            @click="cancelEdit"
          >
            {{ t('common.cancel') }}
          </button>
          <button
            type="button"
            class="btn btn-primary"
            @click="saveEditFromModal"
          >
            {{ t('common.save') }}
          </button>
        </div>
      </template>
    </EaModal>
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

.preview-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
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

.btn-add:hover:not(:disabled) {
  background-color: var(--color-primary-light, #dbeafe);
  border-color: var(--color-primary, #60a5fa);
  color: var(--color-primary, #3b82f6);
}

.btn-add:disabled {
  cursor: not-allowed;
  opacity: 0.55;
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

.editor-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3, 0.75rem);
}

.editor-modal-title {
  font-size: var(--font-size-base, 14px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
}

.editor-modal-close {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: var(--radius-md, 8px);
  background: transparent;
  color: var(--color-text-secondary, #64748b);
  font-size: 1.25rem;
  line-height: 1;
  cursor: pointer;
}

.editor-modal-close:hover {
  background: var(--color-bg-secondary, #f1f5f9);
  color: var(--color-text-primary, #1e293b);
}

:global(.task-split-preview-modal-overlay) {
  z-index: calc(var(--z-modal, 1050) + 10);
}

:global(.ea-modal.task-split-preview-modal) {
  width: min(720px, calc(100vw - 2rem));
  max-width: 720px;
}

.editor-modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-2, 0.5rem);
}

.editor-modal-footer .btn {
  padding: var(--spacing-2, 0.5rem) var(--spacing-4, 1rem);
  border-radius: var(--radius-md, 8px);
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-medium, 500);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.editor-modal-footer .btn-primary {
  background-color: var(--color-primary, #3b82f6);
  color: white;
  border: none;
}

.editor-modal-footer .btn-primary:hover {
  background-color: var(--color-primary-hover, #2563eb);
}

.editor-modal-footer .btn-secondary {
  background-color: var(--color-surface, #fff);
  color: var(--color-text-primary, #1e293b);
  border: 1px solid var(--color-border, #e2e8f0);
}

.editor-modal-footer .btn-secondary:hover {
  background-color: var(--color-surface-hover, #f8fafc);
}
</style>
