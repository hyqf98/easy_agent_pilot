<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AITaskItem, TaskPriority } from '@/types/plan'

const props = defineProps<{
  task: AITaskItem
  tasks: AITaskItem[]
  index: number
  priorityOptions: Array<{ label: string; value: TaskPriority }>
}>()

const emit = defineEmits<{
  save: [updates: Partial<AITaskItem>]
  cancel: []
}>()

const { t } = useI18n()

const draft = ref<AITaskItem>({
  title: '',
  description: '',
  priority: 'medium',
  implementationSteps: [],
  testSteps: [],
  acceptanceCriteria: [],
  dependsOn: []
})

const isDepDropdownOpen = ref(false)
const depDropdownRef = ref<HTMLElement | null>(null)

const availableDependencyTitles = computed(() =>
  props.tasks
    .filter((_, taskIndex) => taskIndex !== props.index)
    .map(task => task.title)
)

const depDropdownDisplay = computed(() => {
  const selected = draft.value.dependsOn || []
  if (selected.length === 0) {
    return t('task.selectDependencies')
  }
  return selected.join(', ')
})

function resetDraft() {
  draft.value = {
    title: props.task.title || '',
    description: props.task.description || '',
    priority: props.task.priority || 'medium',
    implementationSteps: [...(props.task.implementationSteps || [])],
    testSteps: [...(props.task.testSteps || [])],
    acceptanceCriteria: [...(props.task.acceptanceCriteria || [])],
    dependsOn: [...(props.task.dependsOn || [])]
  }
}

function addStep(type: 'implementationSteps' | 'testSteps' | 'acceptanceCriteria') {
  draft.value[type].push('')
}

function removeStep(type: 'implementationSteps' | 'testSteps' | 'acceptanceCriteria', index: number) {
  draft.value[type].splice(index, 1)
}

function toggleDepDropdown() {
  isDepDropdownOpen.value = !isDepDropdownOpen.value
}

function handleDependencyToggle(dependencyTitle: string) {
  const dependsOn = draft.value.dependsOn || []
  draft.value.dependsOn = dependsOn.includes(dependencyTitle)
    ? dependsOn.filter(title => title !== dependencyTitle)
    : [...dependsOn, dependencyTitle]
}

function isDependencySelected(dependencyTitle: string): boolean {
  return (draft.value.dependsOn || []).includes(dependencyTitle)
}

function removeDependency(dependencyTitle: string) {
  draft.value.dependsOn = (draft.value.dependsOn || []).filter(title => title !== dependencyTitle)
}

function handleClickOutside(event: MouseEvent) {
  if (depDropdownRef.value && !depDropdownRef.value.contains(event.target as Node)) {
    isDepDropdownOpen.value = false
  }
}

function save() {
  emit('save', {
    ...draft.value,
    implementationSteps: [...draft.value.implementationSteps],
    testSteps: [...draft.value.testSteps],
    acceptanceCriteria: [...draft.value.acceptanceCriteria],
    dependsOn: [...(draft.value.dependsOn || [])]
  })
}

watch(() => [props.task, props.index], resetDraft, { immediate: true })

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<template>
  <div class="task-editor">
    <div class="form-row">
      <label>{{ t('taskSplit.form.title') }}</label>
      <input
        v-model="draft.title"
        type="text"
        :placeholder="t('taskSplit.form.titlePlaceholder')"
      >
    </div>

    <div class="form-row">
      <label>{{ t('taskSplit.form.description') }}</label>
      <textarea
        v-model="draft.description"
        :placeholder="t('taskSplit.form.descriptionPlaceholder')"
        rows="2"
      />
    </div>

    <div class="form-row">
      <label>{{ t('taskSplit.form.priority') }}</label>
      <div class="priority-select-wrap">
        <select
          v-model="draft.priority"
          class="priority-select"
        >
          <option
            v-for="option in priorityOptions"
            :key="option.value"
            :value="option.value"
          >
            {{ option.label }}
          </option>
        </select>
        <svg
          class="select-arrow"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="M6 9l6 6 6-6" />
        </svg>
      </div>
    </div>

    <div class="form-row">
      <label>
        {{ t('taskSplit.implementationSteps') }}
        <button
          type="button"
          class="btn-add-step"
          @click="addStep('implementationSteps')"
        >
          + {{ t('taskSplit.form.addItem') }}
        </button>
      </label>
      <div class="steps-list">
        <div
          v-for="(_, stepIndex) in draft.implementationSteps"
          :key="stepIndex"
          class="step-item"
        >
          <input
            v-model="draft.implementationSteps[stepIndex]"
            type="text"
          >
          <button
            type="button"
            class="btn-remove-step"
            @click="removeStep('implementationSteps', stepIndex)"
          >
            ×
          </button>
        </div>
      </div>
    </div>

    <div class="form-row">
      <label>
        {{ t('taskSplit.testSteps') }}
        <button
          type="button"
          class="btn-add-step"
          @click="addStep('testSteps')"
        >
          + {{ t('taskSplit.form.addItem') }}
        </button>
      </label>
      <div class="steps-list">
        <div
          v-for="(_, stepIndex) in draft.testSteps"
          :key="stepIndex"
          class="step-item"
        >
          <input
            v-model="draft.testSteps[stepIndex]"
            type="text"
          >
          <button
            type="button"
            class="btn-remove-step"
            @click="removeStep('testSteps', stepIndex)"
          >
            ×
          </button>
        </div>
      </div>
    </div>

    <div class="form-row">
      <label>
        {{ t('taskSplit.acceptanceCriteria') }}
        <button
          type="button"
          class="btn-add-step"
          @click="addStep('acceptanceCriteria')"
        >
          + {{ t('taskSplit.form.addItem') }}
        </button>
      </label>
      <div class="steps-list">
        <div
          v-for="(_, stepIndex) in draft.acceptanceCriteria"
          :key="stepIndex"
          class="step-item"
        >
          <input
            v-model="draft.acceptanceCriteria[stepIndex]"
            type="text"
          >
          <button
            type="button"
            class="btn-remove-step"
            @click="removeStep('acceptanceCriteria', stepIndex)"
          >
            ×
          </button>
        </div>
      </div>
    </div>

    <div
      ref="depDropdownRef"
      class="form-row dep-dropdown"
    >
      <label>{{ t('task.dependencies') }}</label>
      <div
        v-if="availableDependencyTitles.length > 0"
        class="dep-dropdown__body"
      >
        <button
          type="button"
          class="dep-trigger"
          :class="{ open: isDepDropdownOpen }"
          @click.stop="toggleDepDropdown"
        >
          <span
            class="dep-display"
            :class="{ placeholder: !(draft.dependsOn?.length || 0) }"
          >
            {{ depDropdownDisplay }}
          </span>
          <svg
            class="dep-arrow"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M6 9l6 6 6-6" />
          </svg>
        </button>

        <div
          v-if="draft.dependsOn?.length"
          class="dep-selected-tags"
        >
          <span
            v-for="title in draft.dependsOn"
            :key="title"
            class="dep-tag"
          >
            {{ title }}
            <button
              type="button"
              class="dep-tag-remove"
              @click="removeDependency(title)"
            >
              ×
            </button>
          </span>
        </div>

        <div
          v-if="isDepDropdownOpen"
          class="dep-menu"
        >
          <label
            v-for="title in availableDependencyTitles"
            :key="title"
            class="dep-option"
            :class="{ selected: isDependencySelected(title) }"
          >
            <input
              type="checkbox"
              :checked="isDependencySelected(title)"
              @change="handleDependencyToggle(title)"
            >
            <span class="dep-checkbox" />
            <span class="dep-option-label">{{ title }}</span>
          </label>
        </div>
      </div>

      <div
        v-else
        class="no-tasks-hint"
      >
        {{ t('task.noTasksAvailable') }}
      </div>
    </div>

    <div class="edit-actions">
      <button
        type="button"
        class="btn btn-secondary"
        @click="emit('cancel')"
      >
        {{ t('common.cancel') }}
      </button>
      <button
        type="button"
        class="btn btn-primary"
        @click="save"
      >
        {{ t('common.save') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.task-editor {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3, 0.75rem);
}

.form-row {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1, 0.25rem);
}

.form-row label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-text-secondary, #64748b);
}

.form-row input,
.form-row textarea,
.form-row select {
  padding: var(--spacing-2, 0.5rem);
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-md, 8px);
  font-size: var(--font-size-sm, 13px);
  background-color: var(--color-surface, #fff);
  color: var(--color-text-primary, #1e293b);
  transition: border-color var(--transition-fast, 150ms), box-shadow var(--transition-fast, 150ms);
}

.form-row input:focus,
.form-row textarea:focus,
.form-row select:focus {
  outline: none;
  border-color: var(--color-primary, #60a5fa);
}

.priority-select-wrap {
  position: relative;
}

.priority-select {
  width: 100%;
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  cursor: pointer;
  padding-right: 2rem;
  background: linear-gradient(180deg, #ffffff 0%, #f8fafc 100%);
}

.priority-select:hover {
  border-color: #cbd5e1;
  background: linear-gradient(180deg, #ffffff 0%, #f1f5f9 100%);
}

.priority-select:focus {
  box-shadow: 0 0 0 3px rgb(59 130 246 / 15%);
}

.select-arrow {
  position: absolute;
  right: 0.625rem;
  top: 50%;
  transform: translateY(-50%);
  color: #64748b;
  pointer-events: none;
  transition: transform var(--transition-fast, 150ms), color var(--transition-fast, 150ms);
}

.priority-select-wrap:focus-within .select-arrow {
  color: #3b82f6;
  transform: translateY(-50%) rotate(180deg);
}

.btn-add-step {
  padding: 0 var(--spacing-2, 0.5rem);
  border: none;
  border-radius: var(--radius-sm, 4px);
  background: transparent;
  color: var(--color-primary, #3b82f6);
  font-size: var(--font-size-xs, 12px);
  cursor: pointer;
}

.btn-add-step:hover {
  background-color: var(--color-primary-light, #dbeafe);
}

.steps-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1, 0.25rem);
}

.step-item {
  display: flex;
  gap: var(--spacing-1, 0.25rem);
}

.step-item input {
  flex: 1;
}

.btn-remove-step {
  width: 24px;
  height: 24px;
  border: none;
  border-radius: var(--radius-sm, 4px);
  background-color: var(--color-bg-secondary, #f1f5f9);
  color: var(--color-text-tertiary, #94a3b8);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-remove-step:hover {
  background-color: var(--color-error-light, #fee2e2);
  color: var(--color-error, #ef4444);
}

.dep-dropdown {
  position: relative;
}

.dep-dropdown__body {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2, 0.5rem);
}

.dep-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  min-height: 32px;
  padding: var(--spacing-2, 0.5rem);
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-md, 8px);
  background: var(--color-surface, #fff);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.dep-trigger:hover {
  border-color: var(--color-primary, #60a5fa);
}

.dep-trigger.open {
  border-color: var(--color-primary, #3b82f6);
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
}

.dep-display {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-size-sm, 13px);
  color: var(--color-text-primary, #1e293b);
}

.dep-display.placeholder {
  color: var(--color-text-tertiary, #94a3b8);
}

.dep-arrow {
  flex-shrink: 0;
  margin-left: var(--spacing-2, 0.5rem);
  color: var(--color-text-tertiary, #94a3b8);
  transition: transform var(--transition-fast, 150ms);
}

.dep-trigger.open .dep-arrow {
  transform: rotate(180deg);
}

.dep-selected-tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-1, 0.25rem);
}

.dep-tag {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-1, 0.25rem);
  padding: 2px var(--spacing-2, 0.5rem);
  background: var(--color-primary-light, #dbeafe);
  border-radius: var(--radius-full, 9999px);
  font-size: var(--font-size-xs, 12px);
  color: var(--color-primary, #3b82f6);
}

.dep-tag-remove {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: var(--color-primary, #3b82f6);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.dep-tag-remove:hover {
  background: var(--color-primary, #3b82f6);
  color: white;
}

.dep-menu {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  max-height: 200px;
  overflow-y: auto;
  background: var(--color-surface, #fff);
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-md, 8px);
  box-shadow: var(--shadow-lg, 0 10px 15px -3px rgba(0, 0, 0, 0.1));
  z-index: 100;
}

.dep-option {
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
  padding: var(--spacing-2, 0.5rem) var(--spacing-3, 0.75rem);
  cursor: pointer;
  transition: background-color var(--transition-fast, 150ms);
}

.dep-option:hover {
  background-color: var(--color-surface-hover, #f8fafc);
}

.dep-option.selected {
  background-color: var(--color-primary-light, #dbeafe);
}

.dep-option input {
  display: none;
}

.dep-checkbox {
  width: 16px;
  height: 16px;
  border: 1.5px solid var(--color-border, #e2e8f0);
  border-radius: 3px;
  background: var(--color-surface, #fff);
  transition: all var(--transition-fast, 150ms);
  flex-shrink: 0;
}

.dep-option.selected .dep-checkbox {
  border-color: var(--color-primary, #3b82f6);
  background: var(--color-primary, #3b82f6);
}

.dep-option.selected .dep-checkbox::after {
  content: '';
  display: block;
  width: 4px;
  height: 8px;
  margin: 1px 0 0 5px;
  border: solid white;
  border-width: 0 2px 2px 0;
  transform: rotate(45deg);
}

.dep-option-label {
  font-size: var(--font-size-sm, 13px);
  color: var(--color-text-primary, #1e293b);
}

.no-tasks-hint {
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-tertiary, #94a3b8);
  font-style: italic;
}

.edit-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-2, 0.5rem);
  margin-top: var(--spacing-2, 0.5rem);
}

.btn {
  padding: var(--spacing-1, 0.25rem) var(--spacing-3, 0.75rem);
  border-radius: var(--radius-sm, 4px);
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-primary {
  background-color: var(--color-primary, #3b82f6);
  color: white;
  border: none;
}

.btn-primary:hover {
  background-color: var(--color-primary-hover, #2563eb);
}

.btn-secondary {
  background-color: var(--color-surface, #fff);
  color: var(--color-text-primary, #1e293b);
  border: 1px solid var(--color-border, #e2e8f0);
}

.btn-secondary:hover {
  background-color: var(--color-surface-hover, #f8fafc);
}
</style>
