<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import {
  useAgentConfigStore,
  type AgentModelConfig
} from '@/stores/agentConfig'
import { EaButton } from '@/components/common'

// 上下文窗口预设选项
const CONTEXT_WINDOW_PRESETS = [
  { label: '32K', value: 32000 },
  { label: '128K (默认)', value: 128000 },
  { label: '400K', value: 400000 },
  { label: '200K', value: 200000 },
  { label: '1M', value: 1000000 },
  { label: '自定义', value: 'custom' }
] as const

const props = defineProps<{
  agentId: string
  provider?: string
  model?: AgentModelConfig | null
}>()

// 是否是内置的默认模型（允许 modelId 为空）
const isBuiltinDefaultModel = computed(() => {
  return props.model?.isBuiltin && !props.model?.modelId
})

const emit = defineEmits<{
  close: []
}>()

const agentConfigStore = useAgentConfigStore()

const isEditMode = computed(() => !!props.model)

const providerPlaceholders = computed(() => {
  switch (props.provider) {
    case 'opencode':
      return {
        modelId: '例如: openai/gpt-4.1, modelscope/glm-5.1',
        displayName: '例如: OpenAI GPT-4.1, GLM 5.1',
        modelHint: '按 provider/模型ID 格式填写，opencode 会直接使用这个值调用 -m'
      }
    case 'codex':
      return {
        modelId: '例如: gpt-5, codex-mini, o4-mini',
        displayName: '例如: GPT-5, Codex Mini',
        modelHint: '按 Codex CLI 支持的模型 ID 填写'
      }
    default:
      return {
        modelId: '例如: opus4.6, sonnet4.5, haiku3.5',
        displayName: '例如: Claude Opus 4.6, Claude Sonnet 4.5',
        modelHint: '按 Claude CLI 支持的模型 ID 填写，例如 opus4.6'
      }
  }
})

// 保存按钮是否可用
const canSave = computed(() => {
  // 内置默认模型只需要 displayName
  if (isBuiltinDefaultModel.value) {
    return !!formData.value.displayName.trim()
  }
  // 其他模型需要 modelId 和 displayName
  return !!formData.value.modelId.trim() && !!formData.value.displayName.trim()
})

// 表单
const formData = ref({
  modelId: '',
  displayName: '',
  contextWindowPreset: '128000' as string,
  customContextWindow: 128000
})

// 是否显示自定义输入框
const showCustomInput = computed(() => formData.value.contextWindowPreset === 'custom')

// 最终的上下文窗口值
const contextWindow = computed(() => {
  if (formData.value.contextWindowPreset === 'custom') {
    return formData.value.customContextWindow
  }
  return parseInt(formData.value.contextWindowPreset, 10)
})

// 提交状态
const isSubmitting = ref(false)

// 初始化表单数据
watch(() => props.model, (model) => {
  if (model) {
    // 检查是否匹配预设值
    const presetValue = model.contextWindow?.toString()
    const matchingPreset = CONTEXT_WINDOW_PRESETS.find(p => p.value.toString() === presetValue)

    formData.value = {
      modelId: model.modelId,
      displayName: model.displayName,
      contextWindowPreset: matchingPreset ? presetValue! : 'custom',
      customContextWindow: model.contextWindow || 128000
    }
  } else {
    formData.value = {
      modelId: '',
      displayName: '',
      contextWindowPreset: '128000',
      customContextWindow: 128000
    }
  }
}, { immediate: true })

// 提交表单
const handleSubmit = async () => {
  if (!canSave.value) {
    return
  }

  isSubmitting.value = true
  try {
    if (isEditMode.value && props.model) {
      await agentConfigStore.updateModelConfig(props.model.id, props.agentId, {
        modelId: formData.value.modelId,
        displayName: formData.value.displayName,
        contextWindow: contextWindow.value
      })
    } else {
      await agentConfigStore.createModelConfig({
        agentId: props.agentId,
        modelId: formData.value.modelId,
        displayName: formData.value.displayName,
        isBuiltin: false,
        isDefault: false,
        sortOrder: 0,
        enabled: true,
        contextWindow: contextWindow.value
      })
    }
    emit('close')
  } catch (error) {
    console.error('Failed to save model:', error)
  } finally {
    isSubmitting.value = false
  }
}

// 关闭弹窗
const handleClose = () => {
  emit('close')
}
</script>

<template>
  <div class="model-edit-modal">
    <div
      class="modal-overlay"
      @click="handleClose"
    >
      <div
        class="modal-container"
        @click.stop
      >
        <div class="modal-header">
          <h3 class="modal-title">
            {{ isEditMode ? '编辑模型' : '添加模型' }}
          </h3>
          <button
            class="modal-close"
            @click="handleClose"
          >
            <span>&times;</span>
          </button>
        </div>

        <div class="modal-body">
          <div class="form-group">
            <label class="form-label">模型 ID</label>
            <input
              v-model="formData.modelId"
              type="text"
              class="form-input"
              :placeholder="isBuiltinDefaultModel ? '使用系统默认模型' : providerPlaceholders.modelId"
              :disabled="isBuiltinDefaultModel"
            >
            <p class="form-hint">
              {{ isBuiltinDefaultModel ? '此配置使用系统默认模型，无需指定模型 ID' : providerPlaceholders.modelHint }}
            </p>
          </div>

          <div class="form-group">
            <label class="form-label">显示名称</label>
            <input
              v-model="formData.displayName"
              type="text"
              class="form-input"
              :placeholder="providerPlaceholders.displayName"
            >
            <p class="form-hint">
              在界面上显示的友好名称
            </p>
          </div>

          <div class="form-group">
            <label class="form-label">上下文窗口</label>
            <select
              v-model="formData.contextWindowPreset"
              class="form-select"
            >
              <option
                v-for="preset in CONTEXT_WINDOW_PRESETS"
                :key="preset.value"
                :value="preset.value.toString()"
              >
                {{ preset.label }}
              </option>
            </select>
            <input
              v-if="showCustomInput"
              v-model.number="formData.customContextWindow"
              type="number"
              class="form-input form-input--custom"
              placeholder="输入自定义大小"
              min="1000"
              step="1000"
            >
            <p class="form-hint">
              模型的最大上下文长度（token 数）
            </p>
          </div>
        </div>

        <div class="modal-footer">
          <EaButton
            variant="secondary"
            @click="handleClose"
          >
            取消
          </EaButton>
          <EaButton
            variant="primary"
            :loading="isSubmitting"
            :disabled="!canSave"
            @click="handleSubmit"
          >
            {{ isEditMode ? '保存' : '添加' }}
          </EaButton>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.model-edit-modal {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-container {
  position: relative;
  width: 400px;
  max-width: 90%;
  background-color: var(--color-surface);
  border-radius: var(--radius-lg);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4) var(--spacing-5);
  border-bottom: 1px solid var(--color-border);
}

.modal-title {
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--color-text-primary);
}

.modal-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  background: none;
  cursor: pointer;
  color: var(--color-text-tertiary);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.modal-close:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.modal-body {
  padding: var(--spacing-5);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.form-label {
  font-size: var(--font-size-sm);
  font-weight: 500;
  color: var(--color-text-secondary);
}

.form-hint {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.form-input {
  width: 100%;
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-bg-tertiary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  transition: border-color var(--transition-fast);
}

.form-input:focus {
  outline: none;
  border-color: var(--color-primary);
}

.form-input::placeholder {
  color: var(--color-text-tertiary);
}

.form-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.form-input--custom {
  margin-top: var(--spacing-2);
}

.form-select {
  width: 100%;
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-bg-tertiary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  cursor: pointer;
  transition: border-color var(--transition-fast);
  appearance: none;
  background-image: url("data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 20 20'%3e%3cpath stroke='%236b7280' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.5' d='M6 8l4 4 4-4'/%3e%3c/svg%3e");
  background-position: right var(--spacing-2) center;
  background-repeat: no-repeat;
  background-size: 1.5em 1.5em;
  padding-right: 2.5rem;
}

.form-select:focus {
  outline: none;
  border-color: var(--color-primary);
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3);
  padding: var(--spacing-4) var(--spacing-5);
  border-top: 1px solid var(--color-border);
}
</style>
