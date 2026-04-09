<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import type { AITaskItem, TaskPriority } from '@/types/plan'

defineProps<{
  task: AITaskItem
  index: number
  priorityColors: Record<TaskPriority, string>
  disableActions?: boolean
}>()

const emit = defineEmits<{
  edit: []
  remove: []
}>()

const { t } = useI18n()
const agentTeamsStore = useAgentTeamsStore()

onMounted(() => {
  if (agentTeamsStore.experts.length === 0) {
    void agentTeamsStore.loadExperts()
  }
})

function getPriorityLabel(priority: TaskPriority) {
  return t(`taskSplit.priority.${priority}`)
}

function getExpertLabel(expertId?: string): string {
  if (!expertId) {
    return t('taskSplit.noExpertAssigned')
  }
  return agentTeamsStore.getExpertById(expertId)?.name || expertId
}

function onCardClick() {
  emit('edit')
}
</script>

<template>
  <div
    class="task-card"
    :class="{ 'task-card--clickable': !disableActions }"
    @click="onCardClick"
  >
    <div class="task-header">
      <div class="task-number">
        {{ index + 1 }}
      </div>
      <div class="task-title">
        {{ task.title }}
      </div>
      <span
        class="priority-badge"
        :class="priorityColors[task.priority]"
      >
        {{ getPriorityLabel(task.priority) }}
      </span>
      <div class="task-actions">
        <button
          class="btn-icon btn-danger"
          :title="t('taskSplit.delete')"
          :disabled="disableActions"
          @click.stop="emit('remove')"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
          </svg>
        </button>
      </div>
    </div>

    <p
      v-if="task.description"
      class="task-description"
    >
      {{ task.description }}
    </p>

    <div class="task-expert">
      <span class="deps-label">{{ t('taskSplit.executionExpert') }}:</span>
      <span class="deps-list">{{ getExpertLabel(task.expertId) }}</span>
    </div>

    <div
      v-if="task.implementationSteps?.length"
      class="task-steps"
    >
      <span class="steps-label">{{ t('taskSplit.implementationSteps') }}:</span>
      <ul>
        <li
          v-for="(step, stepIndex) in task.implementationSteps"
          :key="stepIndex"
        >
          {{ step }}
        </li>
      </ul>
    </div>

    <div
      v-if="task.dependsOn?.length"
      class="task-deps"
    >
      <span class="deps-label">{{ t('task.dependencies') }}:</span>
      <span class="deps-list">{{ task.dependsOn.join(', ') }}</span>
    </div>
  </div>
</template>

<style scoped>
.task-card {
  background-color: var(--color-surface, #fff);
  border: 1px solid var(--color-border-light, #f1f5f9);
  border-radius: var(--radius-md, 8px);
  padding: var(--spacing-3, 0.75rem);
  transition: all var(--transition-fast, 150ms);
}

.task-card:hover {
  border-color: var(--color-border, #e2e8f0);
  box-shadow: var(--shadow-sm, 0 1px 3px 0 rgb(0 0 0 / 0.1));
}

.task-card--clickable {
  cursor: pointer;
}

.task-card--clickable:hover {
  border-color: rgba(99, 102, 241, 0.3);
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.08);
}

.task-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
}

.task-number {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--color-bg-secondary, #f1f5f9);
  border-radius: var(--radius-sm, 4px);
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-secondary, #64748b);
}

.task-title {
  flex: 1;
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-text-primary, #1e293b);
}

.priority-badge {
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-sm, 4px);
  font-size: 0.6875rem;
  font-weight: var(--font-weight-medium, 500);
}

.priority-badge.green {
  background-color: #d1fae5;
  color: #059669;
}

.priority-badge.yellow {
  background-color: #fef3c7;
  color: #d97706;
}

.priority-badge.red {
  background-color: #fee2e2;
  color: #dc2626;
}

.task-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity var(--transition-fast, 150ms);
}

.task-card:hover .task-actions {
  opacity: 1;
}

.btn-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: var(--radius-sm, 4px);
  background: transparent;
  color: var(--color-text-tertiary, #94a3b8);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-icon:hover {
  background-color: var(--color-bg-secondary, #f1f5f9);
  color: var(--color-text-primary, #1e293b);
}

.btn-icon:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.btn-icon:disabled:hover {
  background: transparent;
  color: var(--color-text-tertiary, #94a3b8);
}

.btn-icon.btn-danger:hover {
  background-color: var(--color-error-light, #fee2e2);
  color: var(--color-error, #ef4444);
}

.task-description {
  margin: var(--spacing-2, 0.5rem) 0 0;
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-secondary, #64748b);
  line-height: 1.4;
}

.task-steps {
  margin-top: var(--spacing-2, 0.5rem);
}

.steps-label,
.deps-label {
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-text-tertiary, #94a3b8);
}

.task-steps ul {
  margin: var(--spacing-1, 0.25rem) 0 0;
  padding-left: var(--spacing-4, 1rem);
}

.task-steps li,
.deps-list {
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-secondary, #64748b);
  line-height: 1.6;
}

.task-deps {
  margin-top: var(--spacing-2, 0.5rem);
}
</style>
