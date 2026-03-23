<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useAgentStore, useAgentConfigStore } from '@/stores'
import { inferAgentProvider } from '@/stores/agent'
import type { AgentModelConfig } from '@/stores/agentConfig'
import type { AITaskItem, TaskResplitConfig } from '@/types/plan'
import { useOverlayDismiss } from '@/composables/useOverlayDismiss'

const props = defineProps<{
  visible: boolean
  task: AITaskItem | null
  defaultGranularity: number
  defaultAgentId?: string
  defaultModelId?: string
}>()

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void
  (e: 'confirm', config: TaskResplitConfig): void
}>()

const agentStore = useAgentStore()
const agentConfigStore = useAgentConfigStore()

// 表单状态
const customPrompt = ref('')
const granularity = ref(3)
const selectedAgentId = ref<string | undefined>(undefined)
const selectedModelId = ref<string | undefined>(undefined)

const availableAgents = computed(() => {
  return [...agentStore.agents].sort((left, right) => {
    const leftOnline = left.status === 'online' ? 1 : 0
    const rightOnline = right.status === 'online' ? 1 : 0
    if (leftOnline !== rightOnline) {
      return rightOnline - leftOnline
    }
    return left.name.localeCompare(right.name)
  })
})

const availableModels = computed(() => {
  if (!selectedAgentId.value) return []
  return agentConfigStore.getModelsConfigs(selectedAgentId.value)
})

// 重置表单
function resetForm() {
  customPrompt.value = ''
  granularity.value = props.defaultGranularity || 3
  selectedAgentId.value = props.defaultAgentId
  selectedModelId.value = props.defaultModelId
}

// 关闭弹框
function close() {
  emit('update:visible', false)
}

const { handleOverlayPointerDown, handleOverlayClick } = useOverlayDismiss(close)

function handleConfirm() {
  emit('confirm', {
    taskIndex: 0, // taskIndex 由父组件设置
    customPrompt: customPrompt.value.trim() || undefined,
    granularity: granularity.value,
    agentId: selectedAgentId.value,
    modelId: selectedModelId.value
  })
  close()
}

watch(() => props.visible, (newVisible) => {
  if (newVisible) {
    resetForm()
    // 加载选中 agent 的模型列表
    if (selectedAgentId.value) {
      const provider = inferAgentProvider(agentStore.agents.find(agent => agent.id === selectedAgentId.value))
      void agentConfigStore.ensureModelsConfigs(selectedAgentId.value, provider)
    }
  }
})

watch(selectedAgentId, async (newAgentId) => {
  if (newAgentId) {
    const provider = inferAgentProvider(agentStore.agents.find(agent => agent.id === newAgentId))
    await agentConfigStore.ensureModelsConfigs(newAgentId, provider)
    const models = agentConfigStore.getModelsConfigs(newAgentId)
    const hasSelectedModel = models.some((model: AgentModelConfig) => model.modelId === selectedModelId.value)
    if (!hasSelectedModel) {
      const preferredModel = models.find((model: AgentModelConfig) => model.modelId === props.defaultModelId)
      const defaultModel = models.find((model: AgentModelConfig) => model.isDefault)
      selectedModelId.value = preferredModel?.modelId || defaultModel?.modelId || models[0]?.modelId
    }
  } else {
    selectedModelId.value = undefined
  }
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="resplit-modal-overlay"
      @pointerdown.capture="handleOverlayPointerDown"
      @click.self="handleOverlayClick"
    >
      <div class="resplit-modal">
        <div class="modal-header">
          <h4>
            <span class="modal-icon">↺</span>
            继续拆分
          </h4>
          <button
            class="btn-close"
            @click="close"
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

        <div class="modal-body">
          <div
            v-if="task"
            class="task-preview"
          >
            <div class="preview-header">
              <span class="preview-label">原任务</span>
              <span class="task-title">{{ task.title }}</span>
            </div>
            <p
              v-if="task.description"
              class="task-description"
            >
              {{ task.description }}
            </p>
            <div
              v-if="task.implementationSteps?.length"
              class="task-steps"
            >
              <span class="steps-label">实现步骤</span>
              <ul>
                <li
                  v-for="(step, i) in task.implementationSteps"
                  :key="i"
                >
                  {{ step }}
                </li>
              </ul>
            </div>
            <div
              v-if="task.testSteps?.length"
              class="task-steps"
            >
              <span class="steps-label">测试步骤</span>
              <ul>
                <li
                  v-for="(step, i) in task.testSteps"
                  :key="i"
                >
                  {{ step }}
                </li>
              </ul>
            </div>
          </div>

          <div class="config-form">
            <div class="form-row">
              <label>补充拆分要求</label>
              <textarea
                v-model="customPrompt"
                placeholder="补充新的拆分要求或限制条件..."
                rows="3"
              />
            </div>

            <div class="form-row">
              <label>拆分颗粒度</label>
              <div class="number-input-wrap">
                <input
                  v-model.number="granularity"
                  type="number"
                  min="2"
                  max="20"
                >
                <span class="input-hint">2-20</span>
              </div>
            </div>

            <div class="form-row">
              <label>拆分智能体</label>
              <div class="select-wrap">
                <select v-model="selectedAgentId">
                  <option :value="undefined">
                    跟随当前
                  </option>
                  <option
                    v-for="agent in availableAgents"
                    :key="agent.id"
                    :value="agent.id"
                  >
                    {{ agent.name }}
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

            <div
              v-if="selectedAgentId && availableModels.length > 0"
              class="form-row"
            >
              <label>拆分模型</label>
              <div class="select-wrap">
                <select v-model="selectedModelId">
                  <option :value="undefined">
                    跟随当前
                  </option>
                  <option
                    v-for="model in availableModels"
                    :key="model.id"
                    :value="model.modelId"
                  >
                    {{ model.isDefault ? `${model.displayName}（默认）` : model.displayName }}
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
          </div>
        </div>

        <div class="modal-footer">
          <button
            class="btn btn-secondary"
            @click="close"
          >
            取消
          </button>
          <button
            class="btn btn-primary"
            @click="handleConfirm"
          >
            开始拆分
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.resplit-modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--color-bg-overlay, rgba(0, 0, 0, 0.5));
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: var(--z-modal, 1050);
  backdrop-filter: blur(4px);
}

.resplit-modal {
  background-color: var(--color-surface, #fff);
  border-radius: 1rem;
  width: min(90vw, 36rem);
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-xl, 0 20px 25px -5px rgba(0, 0, 0, 0.1));
  animation: modalIn 0.2s var(--easing-out);
  border: 1px solid rgba(148, 163, 184, 0.2);
}

@keyframes modalIn {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(-10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4, 1rem) var(--spacing-5, 1.25rem);
  border-bottom: 1px solid var(--color-border, #e2e8f0);
  flex-shrink: 0;
  background: linear-gradient(90deg, rgba(239, 246, 255, 0.92), rgba(238, 242, 255, 0.9));
}

.modal-header h4 {
  margin: 0;
  font-size: var(--font-size-base, 14px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
}

.modal-icon {
  font-size: 1rem;
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

.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-4, 1rem) var(--spacing-5, 1.25rem);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4, 1rem);
}

.task-preview {
  background: linear-gradient(180deg, #f8fbff, #f1f5ff);
  border: 1px solid rgba(99, 102, 241, 0.15);
  border-radius: 0.75rem;
  padding: var(--spacing-3, 0.75rem);
}

.preview-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
  margin-bottom: var(--spacing-2, 0.5rem);
}

.preview-label {
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-text-tertiary, #94a3b8);
  padding: 0.125rem 0.5rem;
  background-color: var(--color-bg, #e2e8f0);
  border-radius: var(--radius-full, 9999px);
}

.task-title {
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
}

.task-description {
  margin: 0 0 var(--spacing-2, 0.5rem);
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-secondary, #64748b);
  line-height: 1.5;
}

.task-steps {
  margin-top: var(--spacing-2, 0.5rem);
}

.steps-label {
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-text-tertiary, #94a3b8);
}

.task-steps ul {
  margin: var(--spacing-1, 0.25rem) 0 0;
  padding-left: var(--spacing-4, 1rem);
}

.task-steps li {
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-secondary, #64748b);
  line-height: 1.6;
}

.config-form {
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
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-text-secondary, #64748b);
}

.form-row textarea,
.form-row input,
.form-row select {
  padding: var(--spacing-2, 0.5rem) var(--spacing-3, 0.75rem);
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-md, 8px);
  font-size: var(--font-size-sm, 13px);
  background-color: var(--color-surface, #fff);
  color: var(--color-text-primary, #1e293b);
  transition: border-color var(--transition-fast, 150ms), box-shadow var(--transition-fast, 150ms);
  resize: vertical;
}

.form-row textarea:focus,
.form-row input:focus,
.form-row select:focus {
  outline: none;
  border-color: var(--color-primary, #60a5fa);
  box-shadow: 0 0 0 3px rgb(59 130 246 / 15%);
}

.form-row textarea {
  min-height: 4.5rem;
}

.number-input-wrap {
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
}

.number-input-wrap input {
  width: 5rem;
  text-align: center;
}

.input-hint {
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-tertiary, #94a3b8);
}

.select-wrap {
  position: relative;
}

.select-wrap select {
  width: 100%;
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  cursor: pointer;
  padding-right: 2rem;
  background: linear-gradient(180deg, #ffffff 0%, #f8fafc 100%);
}

.select-wrap select:hover {
  border-color: #cbd5e1;
  background: linear-gradient(180deg, #ffffff 0%, #f1f5f9 100%);
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

.select-wrap:focus-within .select-arrow {
  color: #3b82f6;
  transform: translateY(-50%) rotate(180deg);
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3, 0.75rem);
  padding: var(--spacing-4, 1rem) var(--spacing-5, 1.25rem);
  border-top: 1px solid var(--color-border, #e2e8f0);
  background: linear-gradient(180deg, #f8fbff, #f1f5ff);
  flex-shrink: 0;
}

.btn {
  padding: var(--spacing-2, 0.5rem) var(--spacing-4, 1rem);
  border-radius: 0.65rem;
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-medium, 500);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-primary {
  background: linear-gradient(135deg, #0ea5e9, #6366f1);
  color: white;
  border: none;
  box-shadow: 0 9px 18px rgba(79, 70, 229, 0.24);
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 12px 24px rgba(79, 70, 229, 0.3);
}

.btn-secondary {
  background: #fff;
  color: #334155;
  border: 1px solid rgba(148, 163, 184, 0.42);
}

.btn-secondary:hover {
  background: linear-gradient(180deg, #ffffff, #f5f9ff);
  border-color: rgba(99, 102, 241, 0.35);
}
</style>
