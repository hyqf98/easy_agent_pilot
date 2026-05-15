<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAgentConfigStore } from '@/stores/agentConfig'
import { EaButton, EaIcon } from '@/components/common'
import { parseContextWindowInput } from '@/utils/contextWindow'

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

const OPENCODE_DEFAULT_PROVIDER_NPM = '@ai-sdk/openai-compatible'

type ProviderMode = 'preset' | 'custom'

interface AuthProvider {
  id: string
  displayName: string
  hasKey: boolean
}

const props = defineProps<{
  agentId: string
}>()

const emit = defineEmits<{
  close: []
}>()

const agentConfigStore = useAgentConfigStore()
const isSubmitting = ref(false)

const providerMode = ref<ProviderMode>('preset')
const providerName = ref('')
const apiKey = ref('')
const showApiKey = ref(false)
const baseUrl = ref('')
const providerModelRows = ref<string[]>([''])
const providerNpm = ref('')
const mainModel = ref('')
const displayName = ref('')
const contextWindowInput = ref('128K')

const opencodeProviders = ref<AuthProvider[]>([])
const opencodeProvidersLoading = ref(false)
const opencodeProvidersError = ref('')

const opencodeModels = ref<string[]>([])
const opencodeModelsLoading = ref(false)
const opencodeModelsError = ref('')

const providerDropdownOpen = ref(false)
const providerSearch = ref('')
const providerFilter = ref('')
const providerInputRef = ref<HTMLElement | null>(null)
const providerDropdownStyle = ref<Record<string, string>>({})

const modelDropdownOpen = ref(false)
const modelSearch = ref('')
const modelInputRef = ref<HTMLElement | null>(null)
const modelDropdownStyle = ref<Record<string, string>>({})

const contextWindowFieldRef = ref<HTMLElement | null>(null)
const showContextWindowOptions = ref(false)
const contextWindowDropdownStyle = ref<Record<string, string>>({})

const isCustom = computed(() => providerMode.value === 'custom')

const selectedProviderLabel = computed(() => {
  const p = opencodeProviders.value.find(item => item.id === providerName.value.trim())
  return p?.displayName || providerName.value.trim()
})

const filteredProviders = computed(() => {
  const q = providerFilter.value.trim().toLowerCase()
  if (!q) return opencodeProviders.value
  return opencodeProviders.value.filter(p =>
    p.displayName.toLowerCase().includes(q) || p.id.toLowerCase().includes(q)
  )
})

const filteredModels = computed(() => {
  const q = modelSearch.value.toLowerCase()
  if (!q) return opencodeModels.value
  return opencodeModels.value.filter(m => m.toLowerCase().includes(q))
})

const contextWindow = computed(() => parseContextWindowInput(contextWindowInput.value))
const contextWindowError = computed(() => {
  if (!contextWindowInput.value.trim()) return '请输入上下文窗口大小'
  if (contextWindow.value === undefined) return '支持 1280000、200.4K、1.28M 这类格式'
  return ''
})

const hasValidProviderModels = computed(() => providerModelRows.value.some(item => item.trim()))

const canSave = computed(() => {
  if (!providerName.value.trim() || !mainModel.value.trim() || !displayName.value.trim()) return false
  if (contextWindow.value === undefined) return false
  if (isCustom.value) {
    return !!baseUrl.value.trim() && hasValidProviderModels.value
  }
  return true
})

async function loadProviders() {
  opencodeProvidersLoading.value = true
  opencodeProvidersError.value = ''
  try {
    const result = await invoke<AuthProvider[]>('read_opencode_auth_providers')
    opencodeProviders.value = result
  } catch (error) {
    opencodeProviders.value = []
    opencodeProvidersError.value = typeof error === 'string' ? error : '加载 Provider 列表失败'
  } finally {
    opencodeProvidersLoading.value = false
  }
}

async function loadModels(autoOpen = true) {
  const provider = providerName.value.trim()
  if (!provider) return
  opencodeModelsLoading.value = true
  opencodeModelsError.value = ''
  opencodeModels.value = []
  modelSearch.value = ''
  try {
    const result = await invoke<string[]>('list_opencode_models', { provider })
    opencodeModels.value = result
    if (autoOpen && result.length > 0) {
      await nextTick()
      updateModelDropdownPosition()
      modelDropdownOpen.value = true
    }
  } catch (error) {
    opencodeModels.value = []
    opencodeModelsError.value = typeof error === 'string' ? error : '加载模型列表失败'
  } finally {
    opencodeModelsLoading.value = false
  }
}

async function loadApiKey() {
  const provider = providerName.value.trim()
  if (!provider) return
  try {
    const key = await invoke<string | null>('read_opencode_provider_api_key', { provider })
    if (key) apiKey.value = key
  } catch { /* ignore */ }
}

function handleProviderModeChange(mode: ProviderMode) {
  providerMode.value = mode
  opencodeModelsError.value = ''
  if (mode === 'preset') {
    baseUrl.value = ''
    providerNpm.value = ''
    providerModelRows.value = ['']
  } else if (!providerNpm.value.trim()) {
    providerNpm.value = OPENCODE_DEFAULT_PROVIDER_NPM
  }
}

function updateProviderDropdownPosition() {
  if (!providerInputRef.value) return
  const rect = providerInputRef.value.getBoundingClientRect()
  providerDropdownStyle.value = {
    position: 'fixed',
    top: `${rect.bottom + 4}px`,
    left: `${rect.left}px`,
    width: `${rect.width}px`,
    zIndex: '9999',
  }
}

function updateModelDropdownPosition() {
  if (!modelInputRef.value) return
  const rect = modelInputRef.value.getBoundingClientRect()
  modelDropdownStyle.value = {
    position: 'fixed',
    top: `${rect.bottom + 4}px`,
    left: `${rect.left}px`,
    width: `${rect.width}px`,
    zIndex: '9999',
  }
}

function updateContextWindowDropdownPosition() {
  if (!contextWindowFieldRef.value) return
  const rect = contextWindowFieldRef.value.getBoundingClientRect()
  contextWindowDropdownStyle.value = {
    position: 'fixed',
    top: `${rect.bottom + 6}px`,
    left: `${rect.left}px`,
    width: `${rect.width}px`,
    zIndex: '9999',
  }
}

function onProviderFocus() {
  providerSearch.value = selectedProviderLabel.value
  providerFilter.value = ''
  updateProviderDropdownPosition()
  providerDropdownOpen.value = true
}

function onProviderInput(event: Event) {
  const val = (event.target as HTMLInputElement).value
  providerSearch.value = val
  providerFilter.value = val
  updateProviderDropdownPosition()
  providerDropdownOpen.value = true
}

function onProviderBlur() {
  providerDropdownOpen.value = false
  providerFilter.value = ''
  providerSearch.value = selectedProviderLabel.value
}

function selectProvider(provider: AuthProvider) {
  providerName.value = provider.id
  providerSearch.value = provider.displayName
  providerFilter.value = ''
  providerDropdownOpen.value = false
  mainModel.value = ''
  displayName.value = ''
  opencodeModels.value = []
  loadModels(false)
  loadApiKey()
}

function toggleProviderDropdown() {
  if (providerDropdownOpen.value) {
    providerDropdownOpen.value = false
    providerFilter.value = ''
    providerSearch.value = selectedProviderLabel.value
    return
  }
  providerSearch.value = selectedProviderLabel.value
  updateProviderDropdownPosition()
  providerDropdownOpen.value = true
}

function onModelFocus() {
  modelSearch.value = ''
  if (opencodeModels.value.length > 0) {
    updateModelDropdownPosition()
    modelDropdownOpen.value = true
  } else if (providerName.value.trim() && !opencodeModelsLoading.value) {
    loadModels()
  }
}

function onModelInput(event: Event) {
  const val = (event.target as HTMLInputElement).value
  mainModel.value = val
  modelSearch.value = val
  if (opencodeModels.value.length > 0) {
    updateModelDropdownPosition()
    modelDropdownOpen.value = true
  }
}

function selectModel(model: string) {
  mainModel.value = model
  modelDropdownOpen.value = false
  modelSearch.value = ''
  if (!displayName.value.trim()) {
    displayName.value = model
  }
}

function toggleModelDropdown() {
  if (modelDropdownOpen.value) {
    modelDropdownOpen.value = false
  } else {
    updateModelDropdownPosition()
    modelDropdownOpen.value = true
  }
}

function addModelRow() {
  providerModelRows.value.push('')
}

function removeModelRow(index: number) {
  if (providerModelRows.value.length === 1) {
    providerModelRows.value[0] = ''
  } else {
    providerModelRows.value.splice(index, 1)
  }
}

function applyContextWindowPreset(label: string) {
  contextWindowInput.value = label
  showContextWindowOptions.value = false
}

function toggleContextWindowOptions() {
  if (showContextWindowOptions.value) {
    showContextWindowOptions.value = false
  } else {
    updateContextWindowDropdownPosition()
    showContextWindowOptions.value = true
  }
}

function handleDocumentPointerDown(event: MouseEvent) {
  const target = event.target
  if (!(target instanceof Node)) return
  if (!contextWindowFieldRef.value?.contains(target)) {
    showContextWindowOptions.value = false
  }
}

async function handleSubmit() {
  if (!canSave.value || contextWindow.value === undefined) return
  isSubmitting.value = true
  try {
    await agentConfigStore.addOpencodeModelToConfig({
      agentId: props.agentId,
      provider: providerName.value.trim(),
      modelId: mainModel.value.trim(),
      displayName: displayName.value.trim(),
      contextWindow: contextWindow.value,
      baseUrl: isCustom.value ? baseUrl.value.trim() || undefined : undefined,
      apiKey: apiKey.value.trim() || undefined,
      npm: isCustom.value ? providerNpm.value.trim() || undefined : undefined,
      providerModels: isCustom.value
        ? providerModelRows.value.map(r => r.trim()).filter(Boolean)
        : undefined,
    })
    emit('close')
  } catch (error) {
    console.error('Failed to add opencode model:', error)
  } finally {
    isSubmitting.value = false
  }
}

function handleClose() {
  emit('close')
}

onMounted(async () => {
  document.addEventListener('mousedown', handleDocumentPointerDown)
  await loadProviders()
})

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', handleDocumentPointerDown)
})
</script>

<template>
  <div class="modal-overlay-root">
    <div
      class="modal-overlay"
      @click="handleClose"
    >
      <div
        class="modal-container"
        @click.stop
      >
        <div class="modal-header">
          <h3 class="modal-title">添加模型</h3>
          <button
            class="modal-close"
            @click="handleClose"
          >
            <span>&times;</span>
          </button>
        </div>

        <div class="modal-body">
          <div class="form-group">
            <label class="form-label">Provider 类型</label>
            <div class="provider-mode-switch">
              <button
                type="button"
                class="provider-mode-btn"
                :class="{ active: providerMode === 'preset' }"
                @click="handleProviderModeChange('preset')"
              >
                内置 Provider
              </button>
              <button
                type="button"
                class="provider-mode-btn"
                :class="{ active: providerMode === 'custom' }"
                @click="handleProviderModeChange('custom')"
              >
                自定义 Provider
              </button>
            </div>
          </div>

          <div class="form-group">
            <label class="form-label">Provider 名称 <span class="required">*</span></label>
            <template v-if="!isCustom">
              <div class="combobox-wrapper">
                <input
                  ref="providerInputRef"
                  :value="providerSearch"
                  type="text"
                  class="form-input combobox-input"
                  placeholder="选择或搜索 Provider"
                  :disabled="opencodeProvidersLoading"
                  @focus="onProviderFocus"
                  @input="onProviderInput"
                  @blur="onProviderBlur"
                >
                <button
                  v-if="opencodeProviders.length > 0"
                  type="button"
                  class="combobox-toggle"
                  :disabled="opencodeProvidersLoading"
                  @mousedown.prevent="toggleProviderDropdown"
                >
                  <EaIcon
                    name="chevron-down"
                    :size="14"
                  />
                </button>
              </div>
              <Teleport to="body">
                <div
                  v-if="providerDropdownOpen && filteredProviders.length > 0"
                  class="combobox-dropdown"
                  :style="providerDropdownStyle"
                  @mousedown.prevent
                >
                  <div
                    v-for="p in filteredProviders"
                    :key="p.id"
                    class="combobox-option"
                    :class="{ active: p.id === providerName }"
                    @mousedown.prevent="selectProvider(p)"
                  >
                    <span>{{ p.displayName }}</span>
                    <span
                      v-if="p.hasKey"
                      class="combobox-option-meta"
                    >Key</span>
                  </div>
                </div>
              </Teleport>
            </template>
            <template v-else>
              <input
                v-model="providerName"
                type="text"
                class="form-input"
                placeholder="自定义 Provider ID，例如 modelhub"
                @blur="loadModels(false)"
              >
            </template>
            <div
              v-if="opencodeProvidersLoading && !isCustom"
              class="form-hint"
            >加载中...</div>
            <div
              v-else-if="opencodeProvidersError && !isCustom"
              class="form-error"
            >{{ opencodeProvidersError }}</div>
          </div>

          <div class="form-group">
            <label class="form-label">API Key</label>
            <div class="api-key-input-wrapper">
              <input
                v-model="apiKey"
                :type="showApiKey ? 'text' : 'password'"
                class="form-input api-key-input"
                placeholder="输入 API Key（可选）"
              >
              <button
                type="button"
                class="api-key-toggle"
                @click="showApiKey = !showApiKey"
              >
                <EaIcon
                  :name="showApiKey ? 'eye-off' : 'eye'"
                  :size="14"
                />
              </button>
            </div>
          </div>

          <div
            v-if="isCustom"
            class="form-group"
          >
            <label class="form-label">Base URL <span class="required">*</span></label>
            <input
              v-model="baseUrl"
              type="text"
              class="form-input"
              placeholder="https://your-openai-compatible-endpoint/v1"
            >
          </div>

          <div
            v-if="isCustom"
            class="form-group"
          >
            <label class="form-label">Provider 模型列表 <span class="required">*</span></label>
            <div class="provider-model-list">
              <div
                v-for="(_, index) in providerModelRows"
                :key="`model-${index}`"
                class="provider-model-row"
              >
                <input
                  v-model="providerModelRows[index]"
                  type="text"
                  class="form-input"
                  placeholder="输入模型 ID"
                >
                <div class="provider-model-actions">
                  <button
                    v-if="providerModelRows.length > 1"
                    type="button"
                    class="provider-model-action provider-model-action--danger"
                    @click="removeModelRow(index)"
                  >
                    <EaIcon
                      name="minus"
                      :size="16"
                    />
                  </button>
                  <button
                    v-if="index === providerModelRows.length - 1"
                    type="button"
                    class="provider-model-action"
                    @click="addModelRow"
                  >
                    <EaIcon
                      name="plus"
                      :size="16"
                    />
                  </button>
                </div>
              </div>
            </div>
            <div class="form-hint">输入该 Provider 支持的模型 ID</div>
          </div>

          <div
            v-if="isCustom"
            class="form-group"
          >
            <label class="form-label">Provider 适配器包</label>
            <input
              v-model="providerNpm"
              type="text"
              class="form-input"
              :placeholder="OPENCODE_DEFAULT_PROVIDER_NPM"
            >
            <div class="form-hint">npm 适配器包名，默认 @ai-sdk/openai-compatible</div>
          </div>

          <div class="form-group">
            <label class="form-label">主模型 <span class="required">*</span></label>
            <div class="combobox-wrapper">
              <input
                ref="modelInputRef"
                :value="mainModel"
                type="text"
                class="form-input combobox-input"
                placeholder="选择或输入模型 ID"
                :disabled="!providerName.trim()"
                @focus="onModelFocus"
                @input="onModelInput"
                @blur="modelDropdownOpen = false"
              >
              <button
                v-if="opencodeModels.length > 0"
                type="button"
                class="combobox-toggle"
                @mousedown.prevent="toggleModelDropdown"
              >
                <EaIcon
                  name="chevron-down"
                  :size="14"
                />
              </button>
            </div>
            <Teleport to="body">
              <div
                v-if="modelDropdownOpen && filteredModels.length > 0"
                class="combobox-dropdown"
                :style="modelDropdownStyle"
                @mousedown.prevent
              >
                <div
                  v-for="model in filteredModels"
                  :key="model"
                  class="combobox-option"
                  :class="{ active: model === mainModel }"
                  @mousedown.prevent="selectModel(model)"
                >
                  {{ model }}
                </div>
              </div>
            </Teleport>
            <div
              v-if="opencodeModelsLoading"
              class="form-hint"
            >加载模型中...</div>
            <div
              v-else-if="opencodeModelsError"
              class="form-error"
            >{{ opencodeModelsError }}</div>
            <div
              v-else-if="providerName.trim() && opencodeModels.length === 0 && !opencodeModelsLoading"
              class="form-hint"
            >
              {{ isCustom ? '请直接输入模型 ID' : '该 Provider 尚未在 opencode.json 中配置模型，请直接输入模型 ID' }}
            </div>
          </div>

          <div class="form-group">
            <label class="form-label">显示名称</label>
            <input
              v-model="displayName"
              type="text"
              class="form-input"
              placeholder="在界面上显示的友好名称"
            >
          </div>

          <div class="form-group">
            <label class="form-label">上下文窗口</label>
            <div
              ref="contextWindowFieldRef"
              class="context-window-combobox"
            >
              <input
                v-model="contextWindowInput"
                type="text"
                class="form-input context-window-combobox__input"
                :class="{ 'form-input--error': !!contextWindowError }"
                placeholder="例如 1280000、200.4K、1.28M"
              >
              <button
                type="button"
                class="context-window-combobox__toggle"
                :aria-expanded="showContextWindowOptions"
                @click="toggleContextWindowOptions"
              >
                <EaIcon
                  name="chevron-down"
                  :size="16"
                />
              </button>
            </div>
            <Teleport to="body">
              <div
                v-if="showContextWindowOptions"
                class="context-window-combobox__menu"
                :style="contextWindowDropdownStyle"
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
            </Teleport>
            <p
              v-if="contextWindowError"
              class="form-error"
            >{{ contextWindowError }}</p>
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
            添加
          </EaButton>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-overlay-root {
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
  width: 480px;
  max-width: 90%;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
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
  flex-shrink: 0;
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
  overflow-y: auto;
  flex: 1;
  min-height: 0;
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

.required {
  color: var(--color-danger, #ef4444);
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

.form-input::placeholder {
  color: var(--color-text-tertiary);
}

.form-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.provider-mode-switch {
  display: flex;
  gap: var(--spacing-2);
}

.provider-mode-btn {
  flex: 1;
  padding: var(--spacing-2) var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-tertiary);
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.provider-mode-btn.active {
  border-color: var(--color-primary);
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

.provider-mode-btn:hover:not(.active) {
  background-color: var(--color-surface-hover);
}

.api-key-input-wrapper {
  position: relative;
}

.api-key-input {
  padding-right: 36px !important;
}

.api-key-toggle {
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
  transition: background-color var(--transition-fast), color var(--transition-fast);
}

.api-key-toggle:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.provider-model-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.provider-model-row {
  display: flex;
  gap: var(--spacing-2);
  align-items: center;
}

.provider-model-row .form-input {
  flex: 1;
}

.provider-model-actions {
  display: flex;
  gap: var(--spacing-1);
  flex-shrink: 0;
}

.provider-model-action {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg-tertiary);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.provider-model-action:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.provider-model-action--danger:hover {
  border-color: var(--color-danger);
  color: var(--color-danger);
}

.combobox-wrapper {
  position: relative;
}

.combobox-input {
  padding-right: 36px;
}

.combobox-toggle {
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
  transition: background-color var(--transition-fast), color var(--transition-fast);
}

.combobox-toggle:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.combobox-dropdown {
  max-height: 240px;
  overflow-y: auto;
  padding: 6px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background-color: var(--color-surface);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.12);
}

.combobox-option {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-primary);
  text-align: left;
  cursor: pointer;
  font-size: var(--font-size-sm);
}

.combobox-option:hover {
  background-color: var(--color-surface-hover);
}

.combobox-option.active {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

.combobox-option-meta {
  font-size: var(--font-size-xs);
  color: var(--color-success, #22c55e);
  background-color: rgba(34, 197, 94, 0.1);
  padding: 1px 6px;
  border-radius: var(--radius-sm);
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
  transition: background-color var(--transition-fast), color var(--transition-fast);
}

.context-window-combobox__toggle:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.context-window-combobox__menu {
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

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3);
  padding: var(--spacing-4) var(--spacing-5);
  border-top: 1px solid var(--color-border);
  flex-shrink: 0;
}
</style>
