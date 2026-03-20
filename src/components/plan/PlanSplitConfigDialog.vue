<script setup lang="ts">
import type { Plan } from '@/types/plan'
import type { AgentOption, ModelOption, PlanSplitConfigFormState } from './planListShared'
import { useOverlayDismiss } from '@/composables/useOverlayDismiss'

const props = defineProps<{
  visible: boolean
  plan: Plan | null
  form: PlanSplitConfigFormState
  teamOptions: AgentOption[]
  agentOptions: AgentOption[]
  modelOptions: ModelOption[]
  canStart: boolean
}>()

const emit = defineEmits<{
  close: []
  start: []
  'update:form': [patch: Partial<PlanSplitConfigFormState>]
}>()

function updateField<K extends keyof PlanSplitConfigFormState>(key: K, value: PlanSplitConfigFormState[K]) {
  emit('update:form', { [key]: value })
}

const { handleOverlayPointerDown, handleOverlayClick } = useOverlayDismiss(() => emit('close'))
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="dialog-overlay"
      @pointerdown.capture="handleOverlayPointerDown"
      @click.self="handleOverlayClick"
    >
      <div class="dialog">
        <div class="dialog-header">
          <h4>
            <span class="dialog-icon">🤖</span>
            选择拆分配置
          </h4>
          <button
            class="btn-close"
            @click="emit('close')"
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </button>
        </div>
        <div class="dialog-body">
          <p class="split-config-desc">
            计划「{{ props.plan?.name }}」尚未配置拆分智能体和模型，请先选择后继续。
          </p>
          <div class="form-field">
            <label>拆解方式 <span class="required">*</span></label>
            <select
              :value="props.form.executionMode"
              class="project-select"
              @change="updateField('executionMode', ($event.target as HTMLSelectElement).value as PlanSplitConfigFormState['executionMode'])"
            >
              <option value="single">
                单 Agent 拆解
              </option>
              <option value="coordinator_subagents">
                主控 + 研究子代理
              </option>
            </select>
          </div>

          <div
            v-if="props.form.executionMode === 'coordinator_subagents'"
            class="form-field"
          >
            <label>Agent Team <span class="required">*</span></label>
            <select
              :value="props.form.teamId ?? ''"
              class="project-select"
              @change="updateField('teamId', (($event.target as HTMLSelectElement).value || null))"
            >
              <option value="">
                请选择团队
              </option>
              <option
                v-for="option in props.teamOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
            <span
              v-if="props.teamOptions.length === 0"
              class="field-hint"
            >请先在 Agent 管理中创建团队</span>
          </div>

          <template v-else>
            <div class="form-field">
            <label>拆分智能体 <span class="required">*</span></label>
            <select
              :value="props.form.agentId ?? ''"
              class="project-select"
              @change="updateField('agentId', (($event.target as HTMLSelectElement).value || null))"
            >
              <option
                v-for="option in props.agentOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
            </div>
            <div class="form-field">
            <label>拆分模型 <span class="required">*</span></label>
            <select
              :value="props.form.modelId"
              class="project-select"
              :disabled="props.modelOptions.length === 0"
              @change="updateField('modelId', ($event.target as HTMLSelectElement).value)"
            >
              <option
                v-for="option in props.modelOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
            </div>
          </template>
        </div>
        <div class="dialog-footer">
          <button
            class="btn btn-secondary"
            @click="emit('close')"
          >
            取消
          </button>
          <button
            class="btn btn-primary"
            :disabled="!props.canStart"
            @click="emit('start')"
          >
            开始拆分
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background-color: var(--color-bg-overlay, rgba(0, 0, 0, 0.5));
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: var(--z-modal-backdrop, 1040);
  backdrop-filter: blur(4px);
}

.dialog {
  background-color: var(--color-surface, #fff);
  border-radius: var(--radius-lg, 12px);
  width: 90%;
  max-width: 32rem;
  box-shadow: var(--shadow-xl, 0 20px 25px -5px rgba(0, 0, 0, 0.1));
  animation: dialog-in 0.2s var(--easing-out);
}

@keyframes dialog-in {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(-10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4, 1rem) var(--spacing-5, 1.25rem);
  border-bottom: 1px solid var(--color-border, #e2e8f0);
}

.dialog-header h4 {
  margin: 0;
  font-size: var(--font-size-base, 14px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
}

.dialog-icon {
  font-size: 1.125rem;
}

.btn-close {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-1, 0.25rem);
  border: none;
  background: transparent;
  color: var(--color-text-tertiary, #94a3b8);
  cursor: pointer;
  border-radius: var(--radius-md, 8px);
  transition: all var(--transition-fast, 150ms);
}

.btn-close:hover {
  background-color: var(--color-surface-hover, #f8fafc);
  color: var(--color-text-primary, #1e293b);
}

.dialog-body {
  padding: var(--spacing-5, 1.25rem);
}

.split-config-desc {
  margin: 0 0 1rem;
  font-size: var(--font-size-sm, 13px);
  color: var(--color-text-secondary, #64748b);
  line-height: 1.5;
}

.form-field {
  margin-bottom: var(--spacing-4, 1rem);
}

.form-field label {
  display: block;
  margin-bottom: var(--spacing-2, 0.5rem);
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-text-secondary, #64748b);
}

.required {
  color: var(--color-error, #ef4444);
}

.project-select {
  width: 100%;
  padding: var(--spacing-2, 0.5rem) var(--spacing-8, 2rem) var(--spacing-2, 0.5rem) var(--spacing-3, 0.75rem);
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-md, 8px);
  font-size: var(--font-size-sm, 13px);
  color: var(--color-text-primary, #1e293b);
  background-color: var(--color-surface, #fff);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2364748b' stroke-width='2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right var(--spacing-3, 0.75rem) center;
}

.project-select:focus {
  outline: none;
  border-color: var(--color-primary, #60a5fa);
  box-shadow: 0 0 0 3px var(--color-primary-light, #dbeafe);
}

.project-select:disabled {
  cursor: not-allowed;
  opacity: 0.6;
  background-color: var(--color-bg-secondary, #f8fafc);
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3, 0.75rem);
  padding: var(--spacing-4, 1rem) var(--spacing-5, 1.25rem);
  border-top: 1px solid var(--color-border, #e2e8f0);
  background-color: var(--color-bg-secondary, #f8fafc);
  border-radius: 0 0 var(--radius-lg, 12px) var(--radius-lg, 12px);
}

.btn {
  padding: var(--spacing-2, 0.5rem) var(--spacing-4, 1rem);
  border-radius: var(--radius-md, 8px);
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-medium, 500);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-primary {
  background-color: var(--color-primary, #3b82f6);
  color: white;
  border: none;
}

.btn-primary:hover:not(:disabled) {
  background-color: var(--color-primary-hover, #2563eb);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-secondary {
  background-color: var(--color-surface, #fff);
  color: var(--color-text-primary, #1e293b);
  border: 1px solid var(--color-border, #e2e8f0);
}

.btn-secondary:hover {
  background-color: var(--color-surface-hover, #f8fafc);
  border-color: var(--color-border-dark, #cbd5e1);
}
</style>
