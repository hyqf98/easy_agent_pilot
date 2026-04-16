<script setup lang="ts">
import MemoryLibraryPicker from '@/components/memory/MemoryLibraryPicker.vue'
import type { Plan, PlanStatus } from '@/types/plan'
import type { AgentOption, ModelOption, PlanEditFormState } from './planListShared'
import { useOverlayDismiss } from '@/composables/useOverlayDismiss'

const props = defineProps<{
  visible: boolean
  plan: Plan | null
  form: PlanEditFormState
  agentOptions: AgentOption[]
  modelOptions: ModelOption[]
}>()

const emit = defineEmits<{
  close: []
  save: []
  'update:form': [patch: Partial<PlanEditFormState>]
}>()

function updateField<K extends keyof PlanEditFormState>(key: K, value: PlanEditFormState[K]) {
  emit('update:form', { [key]: value })
}

function canEditSchedule(status: PlanStatus | undefined): boolean {
  return status !== undefined && ['draft', 'planning', 'ready'].includes(status)
}

const minDateTime = new Date().toISOString().slice(0, 16)

function isDraftEditable(status: PlanStatus | undefined): boolean {
  return status === 'draft'
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
            <span class="dialog-icon">✏️</span>
            编辑计划
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
          <div class="form-field">
            <label>计划名称 <span class="required">*</span></label>
            <input
              :value="props.form.name"
              type="text"
              placeholder="请输入计划名称"
              autofocus
              @input="updateField('name', ($event.target as HTMLInputElement).value)"
            >
          </div>
          <div class="form-field">
            <label>计划描述</label>
            <textarea
              :value="props.form.description"
              placeholder="描述计划的目标和范围（可选）"
              rows="3"
              @input="updateField('description', ($event.target as HTMLTextAreaElement).value)"
            />
          </div>

          <template v-if="isDraftEditable(props.plan?.status)">
            <div class="form-field">
              <label>任务拆分模式</label>
              <div class="mode-options">
                <label
                  class="mode-option"
                  :class="{ active: props.form.splitMode === 'ai' }"
                >
                  <input
                    type="radio"
                    :checked="props.form.splitMode === 'ai'"
                    @change="updateField('splitMode', 'ai')"
                  >
                  <span class="mode-icon">AI</span>
                  <div class="mode-content">
                    <span class="mode-label">AI 协同</span>
                    <span class="mode-desc">AI 帮助拆分任务</span>
                  </div>
                </label>
                <label
                  class="mode-option"
                  :class="{ active: props.form.splitMode === 'manual' }"
                >
                  <input
                    type="radio"
                    :checked="props.form.splitMode === 'manual'"
                    @change="updateField('splitMode', 'manual')"
                  >
                  <span class="mode-icon">手</span>
                  <div class="mode-content">
                    <span class="mode-label">手动模式</span>
                    <span class="mode-desc">自己创建任务</span>
                  </div>
                </label>
              </div>
            </div>

            <template v-if="props.form.splitMode === 'ai'">
              <div class="form-row">
                <div class="form-field">
                  <label>拆分专家</label>
                  <select
                    :value="props.form.splitAgentId ?? ''"
                    class="project-select"
                    @change="updateField('splitAgentId', (($event.target as HTMLSelectElement).value || null))"
                  >
                    <option value="">
                      请选择专家
                    </option>
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
                  <label>拆分模型</label>
                  <select
                    :value="props.form.splitModelId"
                    class="project-select"
                    :disabled="props.modelOptions.length === 0"
                    @change="updateField('splitModelId', ($event.target as HTMLSelectElement).value)"
                  >
                    <option value="">
                      请选择模型
                    </option>
                    <option
                      v-for="option in props.modelOptions"
                      :key="option.value"
                      :value="option.value"
                    >
                      {{ option.label }}
                    </option>
                  </select>
                  <span
                    v-if="props.modelOptions.length === 0"
                    class="field-hint"
                  >当前专家绑定的运行时暂无可用模型，请先在 AgentTeams 或 Agent 设置中配置模型</span>
                </div>
              </div>

              <div class="form-row">
                <div class="form-field">
                  <label>拆分颗粒度</label>
                  <input
                    :value="props.form.granularity"
                    type="number"
                    min="5"
                    max="50"
                    placeholder="建议 5-50"
                    @input="updateField('granularity', Number(($event.target as HTMLInputElement).value))"
                  >
                  <span class="field-hint">建议拆分出的任务数量，数值越大拆分越细</span>
                </div>
                <div class="form-field">
                  <label>最大重试次数</label>
                  <input
                    :value="props.form.maxRetryCount"
                    type="number"
                    min="1"
                    max="5"
                    placeholder="建议 1-3"
                    @input="updateField('maxRetryCount', Number(($event.target as HTMLInputElement).value))"
                  >
                  <span class="field-hint">单个任务执行失败后的最大重试次数</span>
                </div>
              </div>
            </template>

            <div
              v-else
              class="hint-box hint-box-manual"
            >
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <circle
                  cx="12"
                  cy="12"
                  r="10"
                />
                <path d="M12 16v-4M12 8h.01" />
              </svg>
              <span>手动模式下，保存后可在任务看板中继续手动维护任务</span>
            </div>
          </template>

          <div
            v-if="canEditSchedule(props.plan?.status)"
            class="form-field schedule-field"
          >
            <label>执行方式</label>
            <select
              :value="props.form.executionMode"
              class="execution-mode-select"
              @change="updateField('executionMode', ($event.target as HTMLSelectElement).value as PlanEditFormState['executionMode'])"
            >
              <option value="immediate">
                立即执行
              </option>
              <option value="scheduled">
                定时执行
              </option>
            </select>
            <div
              v-if="props.form.executionMode === 'scheduled'"
              class="schedule-datetime"
            >
              <input
                :value="props.form.scheduledDateTime"
                type="datetime-local"
                :min="minDateTime"
                @input="updateField('scheduledDateTime', ($event.target as HTMLInputElement).value)"
              >
              <span
                v-if="props.form.scheduledDateTime"
                class="schedule-preview"
              >
                计划将于 {{ new Date(props.form.scheduledDateTime).toLocaleString('zh-CN') }} 执行
              </span>
            </div>
          </div>

          <div class="form-field form-field--memory">
            <MemoryLibraryPicker
              :model-value="props.form.memoryLibraryIds"
              hint="计划挂载的记忆库会作为任务默认上下文。"
              @update:model-value="updateField('memoryLibraryIds', $event)"
            />
          </div>
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
            :disabled="!props.form.name.trim()"
            @click="emit('save')"
          >
            保存
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
  padding: max(var(--spacing-4, 1rem), env(safe-area-inset-top, 0px)) var(--spacing-3, 0.75rem);
  overflow-y: auto;
  z-index: var(--z-modal-backdrop, 1040);
  backdrop-filter: blur(4px);
}

.dialog {
  background-color: var(--color-surface, #fff);
  border-radius: var(--radius-lg, 12px);
  width: min(100%, 48rem);
  max-height: min(88vh, calc(100vh - 1.5rem));
  display: flex;
  flex-direction: column;
  overflow: hidden;
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
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-5, 1.25rem);
}

.form-row {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--spacing-4, 1rem);
}

.form-field {
  margin-bottom: var(--spacing-4, 1rem);
}

.form-field--memory {
  margin-top: var(--spacing-2, 0.5rem);
  margin-bottom: 0;
  padding-top: var(--spacing-3, 0.75rem);
  border-top: 1px solid var(--color-border, #e2e8f0);
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

.form-field input,
.form-field textarea,
.form-field select {
  width: 100%;
  box-sizing: border-box;
  padding: var(--spacing-2, 0.5rem) var(--spacing-3, 0.75rem);
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-md, 8px);
  background-color: var(--color-surface, #fff);
  color: var(--color-text-primary, #1e293b);
  font-size: var(--font-size-sm, 13px);
  transition: all var(--transition-fast, 150ms);
}

.form-field input::placeholder,
.form-field textarea::placeholder {
  color: var(--color-text-tertiary, #94a3b8);
}

.form-field input:focus,
.form-field textarea:focus,
.form-field select:focus {
  outline: none;
  border-color: var(--color-primary, #60a5fa);
  box-shadow: 0 0 0 3px var(--color-primary-light, #dbeafe);
}

.field-hint,
.schedule-preview {
  display: block;
  margin-top: var(--spacing-2, 0.5rem);
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-tertiary, #94a3b8);
}

.mode-options {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--spacing-3, 0.75rem);
}

.mode-option {
  display: flex;
  align-items: center;
  gap: var(--spacing-3, 0.75rem);
  padding: var(--spacing-4, 1rem);
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-lg, 12px);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.mode-option input {
  display: none;
}

.mode-option.active {
  border-color: var(--color-primary, #60a5fa);
  background: var(--color-primary-light, #eff6ff);
}

.mode-icon {
  font-size: 1.25rem;
}

.mode-content {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.mode-label {
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
}

.mode-desc {
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-tertiary, #94a3b8);
}

.hint-box {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-2, 0.5rem);
  margin-bottom: var(--spacing-4, 1rem);
  padding: var(--spacing-3, 0.75rem);
  border-radius: var(--radius-md, 8px);
  background: var(--color-primary-light, #eff6ff);
  color: var(--color-primary, #2563eb);
  font-size: var(--font-size-xs, 12px);
}

.hint-box-manual {
  background: color-mix(in srgb, #22c55e 8%, #fff);
  color: #15803d;
}

.schedule-field {
  margin-top: var(--spacing-4, 1rem);
  padding-top: var(--spacing-3, 0.75rem);
  border-top: 1px solid var(--color-border, #e2e8f0);
}

.execution-mode-select,
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
  background-repeat: no-repeat;
  background-position: right var(--spacing-3, 0.75rem) center;
}

.execution-mode-select {
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%2364748b' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
  background-size: 16px;
}

.project-select {
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2364748b' stroke-width='2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
}

.execution-mode-select:hover,
.project-select:hover {
  border-color: var(--color-primary, #60a5fa);
}

.execution-mode-select:focus,
.project-select:focus {
  outline: none;
  border-color: var(--color-primary, #60a5fa);
  box-shadow: 0 0 0 3px var(--color-primary-light, #dbeafe);
}

.project-select:disabled,
.execution-mode-select:disabled {
  cursor: not-allowed;
  opacity: 0.6;
  background-color: var(--color-bg-secondary, #f8fafc);
}

.schedule-datetime {
  margin-top: var(--spacing-3, 0.75rem);
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3, 0.75rem);
  padding: var(--spacing-4, 1rem) var(--spacing-5, 1.25rem);
  border-top: 1px solid var(--color-border, #e2e8f0);
  background: var(--color-bg-secondary, #f8fafc);
}

.btn {
  padding: var(--spacing-2, 0.5rem) var(--spacing-4, 1rem);
  border-radius: var(--radius-md, 8px);
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-medium, 500);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-secondary {
  border: 1px solid var(--color-border, #e2e8f0);
  background: var(--color-surface, #fff);
  color: var(--color-text-secondary, #64748b);
}

.btn-primary {
  border: none;
  background: var(--color-primary, #3b82f6);
  color: #fff;
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

@media (max-width: 640px) {
  .dialog {
    width: calc(100vw - 1.5rem);
  }

  .form-row,
  .mode-options {
    grid-template-columns: 1fr;
  }
}

@media (max-height: 820px) {
  .dialog-overlay {
    align-items: flex-start;
  }

  .dialog {
    max-height: calc(100vh - 1rem);
  }
}
</style>
