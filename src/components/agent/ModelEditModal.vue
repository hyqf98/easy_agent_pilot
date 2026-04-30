<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  useAgentConfigStore,
  type AgentModelConfig
} from '@/stores/agentConfig'
import { EaButton, EaIcon } from '@/components/common'
import {
  formatContextWindowCount,
  parseContextWindowInput
} from '@/utils/contextWindow'

const CONTEXT_WINDOW_PRESETS = [
  { label: '32K', value: 32000 },
  { label: '64K', value: 64000 },
  { label: '128K (默认)', value: 128000 },
  { label: '200K', value: 200000 },
  { label: '256K', value: 256000 },
  { label: '400K', value: 400000 },
  { label: '1M', value: 1000000 },
  { label: '1.05M', value: 1050000 },
  { label: '1.28M', value: 1280000 }
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
const contextWindowFieldRef = ref<HTMLElement | null>(null)
const showContextWindowOptions = ref(false)

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
  const hasValidContextWindow = contextWindow.value !== undefined
  // 内置默认模型只需要 displayName
  if (isBuiltinDefaultModel.value) {
    return !!formData.value.displayName.trim() && hasValidContextWindow
  }
  // 其他模型需要 modelId 和 displayName
  return !!formData.value.modelId.trim() && !!formData.value.displayName.trim() && hasValidContextWindow
})

// 表单
const formData = ref({
  modelId: '',
  displayName: '',
  contextWindowInput: '128K'
})

const contextWindow = computed(() => {
  return parseContextWindowInput(formData.value.contextWindowInput)
})

const contextWindowError = computed(() => {
  if (!formData.value.contextWindowInput.trim()) {
    return '请输入上下文窗口大小'
  }

  if (contextWindow.value === undefined) {
    return '支持 1280000、200.4K、1.28M 这类格式'
  }

  return ''
})

const contextWindowPreview = computed(() => {
  if (contextWindow.value === undefined) {
    return ''
  }

  return formatContextWindowCount(contextWindow.value)
})

function applyContextWindowPreset(label: string) {
  formData.value.contextWindowInput = label
  showContextWindowOptions.value = false
}

function toggleContextWindowOptions() {
  showContextWindowOptions.value = !showContextWindowOptions.value
}

function handleDocumentPointerDown(event: MouseEvent) {
  const target = event.target
  if (!(target instanceof Node)) {
    return
  }

  if (!contextWindowFieldRef.value?.contains(target)) {
    showContextWindowOptions.value = false
  }
}

onMounted(() => {
  document.addEventListener('mousedown', handleDocumentPointerDown)
})

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', handleDocumentPointerDown)
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
      contextWindowInput: matchingPreset?.label || formatContextWindowCount(model.contextWindow || 128000)
    }
  } else {
    formData.value = {
      modelId: '',
      displayName: '',
      contextWindowInput: '128K'
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
    if (contextWindow.value === undefined) {
      return
    }

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
            <div
              ref="contextWindowFieldRef"
              class="context-window-combobox"
            >
              <input
                v-model="formData.contextWindowInput"
                type="text"
                class="form-input context-window-combobox__input"
                :class="{ 'form-input--error': !!contextWindowError }"
                placeholder="例如 1280000、200.4K、1.28M"
              >
              <button
                type="button"
                class="context-window-combobox__toggle"
                :aria-expanded="showContextWindowOptions"
                aria-label="展开上下文窗口常用选项"
                @click="toggleContextWindowOptions"
              >
                <EaIcon
                  name="chevron-down"
                  :size="16"
                />
              </button>
              <div
                v-if="showContextWindowOptions"
                class="context-window-combobox__menu"
              >
                <button
                  v-for="preset in CONTEXT_WINDOW_PRESETS"
                  :key="preset.value"
                  type="button"
                  class="context-window-combobox__option"
                  @click="applyContextWindowPreset(preset.label)"
                >
                  <span>{{ preset.label }}</span>
                  <span class="context-window-combobox__option-value">{{ preset.value.toLocaleString() }}</span>
                </button>
              </div>
            </div>
            <p class="form-hint">
              模型的最大上下文长度（token 数）
              <template v-if="contextWindowPreview">
                ，当前识别为 {{ contextWindowPreview }}
              </template>
            </p>
            <p
              v-if="contextWindowError"
              class="form-error"
            >
              {{ contextWindowError }}
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

.form-error {
  font-size: var(--font-size-xs);
  color: var(--color-danger, #ef4444);
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

.form-input--error {
  border-color: var(--color-danger, #ef4444);
}

.context-window-combobox {
  position: relative;
}

.context-window-combobox__input {
  padding-right: 44px;
}

.context-window-combobox__toggle {
  position: absolute;
  top: 50%;
  right: 8px;
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-tertiary);
  cursor: pointer;
  transform: translateY(-50%);
  transition:
    background-color var(--transition-fast),
    color var(--transition-fast);
}

.context-window-combobox__toggle:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.context-window-combobox__menu {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  right: 0;
  z-index: 10;
  max-height: 220px;
  overflow-y: auto;
  padding: 6px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background-color: var(--color-surface);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.12);
}

.context-window-combobox__option {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  padding: 10px 12px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-primary);
  text-align: left;
  cursor: pointer;
}

.context-window-combobox__option:hover {
  background-color: var(--color-surface-hover);
}

.context-window-combobox__option-value {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  font-variant-numeric: tabular-nums;
}

.form-input::placeholder {
  color: var(--color-text-tertiary);
}

.form-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3);
  padding: var(--spacing-4) var(--spacing-5);
  border-top: 1px solid var(--color-border);
}
</style>
