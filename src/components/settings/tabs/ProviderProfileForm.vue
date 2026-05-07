<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ProviderProfile, CliType, CreateProviderProfileInput, UpdateProviderProfileInput } from '@/stores/providerProfile'
import { EaButton, EaIcon } from '@/components/common'
import { useOverlayDismiss } from '@/composables/useOverlayDismiss'
import { invoke } from '@tauri-apps/api/core'

const OPENCODE_DEFAULT_PROVIDER_NPM = '@ai-sdk/openai-compatible'
type OpenCodeProviderMode = 'preset' | 'custom'

const props = defineProps<{
  visible: boolean
  profile: ProviderProfile | null
  cliType: CliType
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  save: [input: CreateProviderProfileInput | UpdateProviderProfileInput]
}>()

const { t } = useI18n()

// 表单数据
const form = ref({
  name: '',
  apiKey: '',
  baseUrl: '',
  providerName: '',
  mainModel: '',
  opencodeProviderModels: '',
  opencodeProviderNpm: '',
  reasoningModel: '',
  haikuModel: '',
  sonnetDefault: '',
  opusDefault: '',
  codexModel: ''
})

// 是否是编辑模式
const isEditMode = computed(() => !!props.profile)
const isCurrentConfig = computed(() => isEditMode.value && props.profile?.id === '')

// 弹窗标题
const modalTitle = computed(() =>
  isCurrentConfig.value
    ? t('settings.providerSwitch.form.editCurrentTitle')
    : isEditMode.value
    ? t('settings.providerSwitch.form.editTitle')
    : t('settings.providerSwitch.form.addTitle')
)

// 保存中状态
const saving = ref(false)

// 重置表单
function resetForm() {
  form.value = {
    name: '',
    apiKey: '',
    baseUrl: '',
    providerName: '',
    mainModel: '',
    opencodeProviderModels: '',
    opencodeProviderNpm: '',
    reasoningModel: '',
    haikuModel: '',
    sonnetDefault: '',
    opusDefault: '',
    codexModel: ''
  }
  opencodeProviderModelRows.value = ['']
}

// 填充表单（编辑模式）
function populateForm(profile: ProviderProfile) {
  form.value = {
    name: profile.name || '',
    apiKey: profile.apiKey || '',
    baseUrl: profile.baseUrl || '',
    providerName: profile.providerName || '',
    mainModel: profile.mainModel || '',
    opencodeProviderModels: profile.opencodeProviderModels || '',
    opencodeProviderNpm: profile.opencodeProviderNpm || '',
    reasoningModel: profile.reasoningModel || '',
    haikuModel: profile.haikuModel || '',
    sonnetDefault: profile.sonnetDefault || '',
    opusDefault: profile.opusDefault || '',
    codexModel: profile.codexModel || ''
  }
  syncOpenCodeProviderModelRows(profile.opencodeProviderModels || '')
}

// 关闭弹窗
function handleClose() {
  emit('update:visible', false)
  resetForm()
}

const { handleOverlayPointerDown, handleOverlayClick } = useOverlayDismiss(handleClose)

interface AuthProvider {
  id: string
  displayName: string
  hasKey: boolean
}

function formatInvokeError(error: unknown): string {
  if (typeof error === 'string') return error
  if (error && typeof error === 'object' && 'message' in error) {
    const message = (error as { message?: unknown }).message
    return typeof message === 'string' ? message : t('common.unknownError')
  }
  return t('common.unknownError')
}

const opencodeProviders = ref<AuthProvider[]>([])
const opencodeProvidersLoaded = ref(false)
const opencodeProvidersLoading = ref(false)
const opencodeProvidersError = ref('')
const opencodeModels = ref<string[]>([])
const opencodeModelsLoading = ref(false)
const opencodeModelsError = ref('')
const opencodeModelDropdownOpen = ref(false)
const opencodeModelSearch = ref('')
const opencodeProviderDropdownOpen = ref(false)
const opencodeProviderSearch = ref('')
const opencodeProviderFilter = ref('')
const comboboxInputRef = ref<HTMLElement | null>(null)
const comboboxDropdownStyle = ref<Record<string, string>>({})
const providerComboboxInputRef = ref<HTMLElement | null>(null)
const providerDropdownStyle = ref<Record<string, string>>({})
const opencodeProviderMode = ref<OpenCodeProviderMode>('preset')
const opencodeProviderModelRows = ref<string[]>([''])
const showApiKeyValue = ref(false)

const hasOpenCodeProviderOptions = computed(() => opencodeProviders.value.length > 0)
const isOpenCodeCustomProvider = computed(() => opencodeProviderMode.value === 'custom')
const hasValidOpenCodeProviderModels = computed(() =>
  opencodeProviderModelRows.value.some(item => item.trim())
)
const isSubmitDisabled = computed(() => {
  if (!isCurrentConfig.value && !form.value.name.trim()) {
    return true
  }

  if (props.cliType !== 'opencode') {
    return false
  }

  if (!form.value.providerName.trim() || !form.value.mainModel.trim()) {
    return true
  }

  if (!isOpenCodeCustomProvider.value) {
    return false
  }

  return !form.value.baseUrl.trim() || !hasValidOpenCodeProviderModels.value
})

function syncOpenCodeProviderModelRows(raw: string) {
  const items = raw
    .split(/\r?\n/)
    .map(item => item.trim())
    .filter(Boolean)
  opencodeProviderModelRows.value = items.length > 0 ? items : ['']
}

function syncOpenCodeProviderModelsField() {
  form.value.opencodeProviderModels = opencodeProviderModelRows.value
    .map(item => item.trim())
    .filter(Boolean)
    .join('\n')
}

function addOpenCodeProviderModelRow() {
  opencodeProviderModelRows.value.push('')
}

function removeOpenCodeProviderModelRow(index: number) {
  if (opencodeProviderModelRows.value.length === 1) {
    opencodeProviderModelRows.value[0] = ''
  } else {
    opencodeProviderModelRows.value.splice(index, 1)
  }
  syncOpenCodeProviderModelsField()
}

watch(
  () => props.profile,
  (profile) => {
    if (profile) {
      populateForm(profile)
    } else {
      resetForm()
    }
  },
  { immediate: true }
)

function syncOpenCodeProviderMode() {
  const provider = form.value.providerName.trim()
  const hasCustomFields = Boolean(
    form.value.baseUrl.trim()
    || opencodeProviderModelRows.value.some(item => item.trim())
    || form.value.opencodeProviderNpm.trim()
  )
  const matchesKnownProvider = opencodeProviders.value.some(item => item.id === provider)

  if (hasCustomFields || (provider && !matchesKnownProvider)) {
    opencodeProviderMode.value = 'custom'
    if (!form.value.opencodeProviderNpm.trim()) {
      form.value.opencodeProviderNpm = OPENCODE_DEFAULT_PROVIDER_NPM
    }
    return
  }

  opencodeProviderMode.value = 'preset'
  syncOpenCodeProviderSearch()
}

function handleOpenCodeProviderModeChange(mode: OpenCodeProviderMode) {
  opencodeProviderMode.value = mode
  opencodeModelsError.value = ''
  if (mode === 'preset') {
    form.value.baseUrl = ''
    form.value.opencodeProviderNpm = ''
    opencodeProviderModelRows.value = ['']
    syncOpenCodeProviderModelsField()
  } else if (!form.value.opencodeProviderNpm.trim()) {
    form.value.opencodeProviderNpm = OPENCODE_DEFAULT_PROVIDER_NPM
  }
}

async function loadOpenCodeProviders() {
  if (opencodeProvidersLoaded.value) return
  opencodeProvidersLoading.value = true
  opencodeProvidersError.value = ''
  try {
    const result = await invoke<AuthProvider[]>('read_opencode_auth_providers')
    opencodeProviders.value = result
    syncOpenCodeProviderMode()
    syncOpenCodeProviderSearch()
  } catch (error) {
    opencodeProviders.value = []
    opencodeProvidersError.value = formatInvokeError(error)
  } finally {
    opencodeProvidersLoaded.value = true
    opencodeProvidersLoading.value = false
  }
}

async function loadOpenCodeModels(autoOpen = true) {
  const provider = form.value.providerName.trim()
  if (!provider) return
  opencodeModelsLoading.value = true
  opencodeModelsError.value = ''
  opencodeModels.value = []
  opencodeModelSearch.value = ''
  try {
    const result = await invoke<string[]>('list_opencode_models', { provider })
    opencodeModels.value = result
    if (autoOpen && result.length > 0) {
      updateDropdownPosition()
      opencodeModelDropdownOpen.value = true
    }
  } catch (error) {
    opencodeModels.value = []
    opencodeModelsError.value = formatInvokeError(error)
  } finally {
    opencodeModelsLoading.value = false
  }
}

async function loadOpenCodeProviderApiKey() {
  const provider = form.value.providerName.trim()
  if (!provider) return
  try {
    const apiKey = await invoke<string | null>('read_opencode_provider_api_key', { provider })
    if (apiKey) {
      form.value.apiKey = apiKey
    }
  } catch {
    // silently ignore
  }
}

function handleOpenCodeProviderChange() {
  loadOpenCodeModels(false)
  loadOpenCodeProviderApiKey()
}

const selectedOpenCodeProviderLabel = computed(() => {
  const provider = opencodeProviders.value.find(item => item.id === form.value.providerName.trim())
  return provider?.displayName || form.value.providerName.trim()
})

const filteredProviders = computed(() => {
  const query = opencodeProviderFilter.value.trim().toLowerCase()
  if (!query) {
    return opencodeProviders.value
  }

  return opencodeProviders.value.filter(provider =>
    provider.displayName.toLowerCase().includes(query)
    || provider.id.toLowerCase().includes(query)
  )
})

function syncOpenCodeProviderSearch() {
  opencodeProviderSearch.value = selectedOpenCodeProviderLabel.value
}

function updateProviderDropdownPosition() {
  if (!providerComboboxInputRef.value) return
  const rect = providerComboboxInputRef.value.getBoundingClientRect()
  providerDropdownStyle.value = {
    position: 'fixed',
    top: `${rect.bottom + 4}px`,
    left: `${rect.left}px`,
    width: `${rect.width}px`,
    zIndex: '9999',
  }
}

function openProviderDropdown() {
  opencodeProviderFilter.value = ''
  updateProviderDropdownPosition()
  opencodeProviderDropdownOpen.value = true
}

function onProviderFocus() {
  syncOpenCodeProviderSearch()
  openProviderDropdown()
}

function onProviderInput(event: Event) {
  const value = (event.target as HTMLInputElement).value
  opencodeProviderSearch.value = value
  opencodeProviderFilter.value = value
  openProviderDropdown()
}

function onProviderBlur() {
  opencodeProviderDropdownOpen.value = false
  opencodeProviderFilter.value = ''
  syncOpenCodeProviderSearch()
}

function toggleProviderDropdown() {
  if (opencodeProviderDropdownOpen.value) {
    opencodeProviderDropdownOpen.value = false
    opencodeProviderFilter.value = ''
    syncOpenCodeProviderSearch()
    return
  }

  syncOpenCodeProviderSearch()
  openProviderDropdown()
}

function selectOpenCodeProvider(provider: AuthProvider) {
  form.value.providerName = provider.id
  opencodeProviderSearch.value = provider.displayName
  opencodeProviderFilter.value = ''
  opencodeProviderDropdownOpen.value = false
  handleOpenCodeProviderChange()
}

function selectOpenCodeModel(model: string) {
  form.value.mainModel = model
  opencodeModelDropdownOpen.value = false
  opencodeModelSearch.value = ''
}

function onModelInput(e: Event) {
  const val = (e.target as HTMLInputElement).value
  form.value.mainModel = val
  opencodeModelSearch.value = val
  if (opencodeModels.value.length > 0) {
    updateDropdownPosition()
    opencodeModelDropdownOpen.value = true
  }
}

function onModelFocus() {
  opencodeModelSearch.value = ''
  if (opencodeModels.value.length > 0) {
    updateDropdownPosition()
    opencodeModelDropdownOpen.value = true
  }
}

function toggleModelDropdown() {
  if (opencodeModelDropdownOpen.value) {
    opencodeModelDropdownOpen.value = false
  } else {
    updateDropdownPosition()
    opencodeModelDropdownOpen.value = true
  }
}

function updateDropdownPosition() {
  if (!comboboxInputRef.value) return
  const rect = comboboxInputRef.value.getBoundingClientRect()
  comboboxDropdownStyle.value = {
    position: 'fixed',
    top: `${rect.bottom + 4}px`,
    left: `${rect.left}px`,
    width: `${rect.width}px`,
    zIndex: '9999',
  }
}

const filteredModels = computed(() => {
  const q = opencodeModelSearch.value.toLowerCase()
  if (!q) return opencodeModels.value
  return opencodeModels.value.filter(m => m.toLowerCase().includes(q))
})

watch(
  () => props.visible,
  (v) => {
    if (v && props.cliType === 'opencode') {
      opencodeProvidersLoaded.value = false
      opencodeProvidersError.value = ''
      opencodeModelsError.value = ''
      opencodeProviderDropdownOpen.value = false
      syncOpenCodeProviderMode()
      loadOpenCodeProviders()
      if (form.value.providerName) {
        nextTick(() => loadOpenCodeModels(false))
      }
    } else {
      opencodeProviderDropdownOpen.value = false
      opencodeModelDropdownOpen.value = false
    }
  },
  { immediate: true }
)

watch(
  () => props.profile,
  () => {
    if (props.cliType === 'opencode') {
      syncOpenCodeProviderModelRows(props.profile?.opencodeProviderModels || '')
      syncOpenCodeProviderMode()
      syncOpenCodeProviderSearch()
    }
  }
)

// 提交表单
async function handleSubmit() {
  syncOpenCodeProviderModelsField()

  if (isSubmitDisabled.value) {
    return
  }

  saving.value = true

  const opencodeProviderNpm = props.cliType === 'opencode' && isOpenCodeCustomProvider.value
    ? (form.value.opencodeProviderNpm.trim() || OPENCODE_DEFAULT_PROVIDER_NPM)
    : undefined
  const opencodeProviderModels = props.cliType === 'opencode' && isOpenCodeCustomProvider.value
    ? (form.value.opencodeProviderModels.trim() || undefined)
    : undefined

  try {
    if (isCurrentConfig.value) {
      const input: UpdateProviderProfileInput = {
        apiKey: form.value.apiKey || undefined,
        baseUrl: form.value.baseUrl || undefined,
        providerName: form.value.providerName || undefined,
        mainModel: form.value.mainModel || undefined,
        opencodeProviderModels,
        opencodeProviderNpm,
        reasoningModel: form.value.reasoningModel || undefined,
        haikuModel: form.value.haikuModel || undefined,
        sonnetDefault: form.value.sonnetDefault || undefined,
        opusDefault: form.value.opusDefault || undefined,
        codexModel: form.value.codexModel || undefined
      }
      emit('save', input)
    } else if (isEditMode.value) {
      // 编辑模式
      const input: UpdateProviderProfileInput = {
        name: form.value.name,
        apiKey: form.value.apiKey || undefined,
        baseUrl: form.value.baseUrl || undefined,
        providerName: form.value.providerName || undefined,
        mainModel: form.value.mainModel || undefined,
        opencodeProviderModels,
        opencodeProviderNpm,
        reasoningModel: form.value.reasoningModel || undefined,
        haikuModel: form.value.haikuModel || undefined,
        sonnetDefault: form.value.sonnetDefault || undefined,
        opusDefault: form.value.opusDefault || undefined,
        codexModel: form.value.codexModel || undefined
      }
      emit('save', input)
    } else {
      // 创建模式
      const input: CreateProviderProfileInput = {
        name: form.value.name,
        cliType: props.cliType,
        apiKey: form.value.apiKey || undefined,
        baseUrl: form.value.baseUrl || undefined,
        providerName: form.value.providerName || undefined,
        mainModel: form.value.mainModel || undefined,
        opencodeProviderModels,
        opencodeProviderNpm,
        reasoningModel: form.value.reasoningModel || undefined,
        haikuModel: form.value.haikuModel || undefined,
        sonnetDefault: form.value.sonnetDefault || undefined,
        opusDefault: form.value.opusDefault || undefined,
        codexModel: form.value.codexModel || undefined
      }
      emit('save', input)
    }
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="modal-overlay"
      @pointerdown.capture="handleOverlayPointerDown"
      @click.self="handleOverlayClick"
    >
      <div class="modal-content">
        <div class="modal-header">
          <h3>{{ modalTitle }}</h3>
          <button
            class="close-btn"
            @click="handleClose"
          >
            <EaIcon
              name="x"
              :size="20"
            />
          </button>
        </div>

        <form
          class="modal-body"
          @submit.prevent="handleSubmit"
        >
          <!-- 基本信息 -->
          <div
            v-if="!isCurrentConfig"
            class="form-section"
          >
            <div class="form-group">
              <label class="form-label">
                {{ t('settings.providerSwitch.form.name') }} <span class="required">*</span>
              </label>
              <input
                v-model="form.name"
                type="text"
                class="form-input"
                :placeholder="t('settings.providerSwitch.form.namePlaceholder')"
                required
              >
            </div>
          </div>

          <!-- Claude CLI 配置 -->
          <template v-if="cliType === 'claude'">
            <div class="form-section">
              <h4 class="section-title">
                {{ t('settings.providerSwitch.form.claudeConfig') }}
              </h4>

              <div class="form-group">
                <label class="form-label">{{ t('settings.providerSwitch.form.apiKey') }}</label>
                <div class="api-key-input-wrapper">
                  <input
                    v-model="form.apiKey"
                    :type="showApiKeyValue ? 'text' : 'password'"
                    class="form-input api-key-input"
                    :placeholder="t('settings.providerSwitch.form.apiKeyPlaceholder')"
                  >
                  <button
                    type="button"
                    class="api-key-toggle"
                    @click="showApiKeyValue = !showApiKeyValue"
                  >
                    <EaIcon
                      :name="showApiKeyValue ? 'eye-off' : 'eye'"
                      :size="14"
                    />
                  </button>
                </div>
              </div>

              <div class="form-group">
                <label class="form-label">{{ t('settings.providerSwitch.form.baseUrl') }}</label>
                <input
                  v-model="form.baseUrl"
                  type="text"
                  class="form-input"
                  :placeholder="t('settings.providerSwitch.form.baseUrlPlaceholder')"
                >
              </div>

              <div class="form-group">
                <label class="form-label">{{ t('settings.providerSwitch.form.mainModel') }}</label>
                <input
                  v-model="form.mainModel"
                  type="text"
                  class="form-input"
                  :placeholder="t('settings.providerSwitch.form.mainModelPlaceholder')"
                >
              </div>

              <div class="form-group">
                <label class="form-label">{{ t('settings.providerSwitch.form.reasoningModel') }}</label>
                <input
                  v-model="form.reasoningModel"
                  type="text"
                  class="form-input"
                  :placeholder="t('settings.providerSwitch.form.reasoningModelPlaceholder')"
                >
              </div>

              <div class="form-group">
                <label class="form-label">{{ t('settings.providerSwitch.form.haikuModel') }}</label>
                <input
                  v-model="form.haikuModel"
                  type="text"
                  class="form-input"
                  :placeholder="t('settings.providerSwitch.form.haikuModelPlaceholder')"
                >
              </div>

              <div class="form-row">
                <div class="form-group">
                  <label class="form-label">{{ t('settings.providerSwitch.form.sonnetDefault') }}</label>
                  <input
                    v-model="form.sonnetDefault"
                    type="text"
                    class="form-input"
                  >
                </div>

                <div class="form-group">
                  <label class="form-label">{{ t('settings.providerSwitch.form.opusDefault') }}</label>
                  <input
                    v-model="form.opusDefault"
                    type="text"
                    class="form-input"
                  >
                </div>
              </div>
            </div>
          </template>

          <!-- Codex CLI 配置 -->
          <template v-if="cliType === 'codex'">
            <div class="form-section">
              <h4 class="section-title">
                {{ t('settings.providerSwitch.form.codexConfig') }}
              </h4>

              <div class="form-group">
                <label class="form-label">{{ t('settings.providerSwitch.form.apiKey') }}</label>
                <div class="api-key-input-wrapper">
                  <input
                    v-model="form.apiKey"
                    :type="showApiKeyValue ? 'text' : 'password'"
                    class="form-input api-key-input"
                    :placeholder="t('settings.providerSwitch.form.apiKeyPlaceholder')"
                  >
                  <button
                    type="button"
                    class="api-key-toggle"
                    @click="showApiKeyValue = !showApiKeyValue"
                  >
                    <EaIcon
                      :name="showApiKeyValue ? 'eye-off' : 'eye'"
                      :size="14"
                    />
                  </button>
                </div>
              </div>

              <div class="form-group">
                <label class="form-label">{{ t('settings.providerSwitch.form.baseUrl') }}</label>
                <input
                  v-model="form.baseUrl"
                  type="text"
                  class="form-input"
                  :placeholder="t('settings.providerSwitch.form.baseUrlPlaceholder')"
                >
              </div>

              <div class="form-group">
                <label class="form-label">{{ t('settings.providerSwitch.form.codexModel') }}</label>
                <input
                  v-model="form.codexModel"
                  type="text"
                  class="form-input"
                  :placeholder="t('settings.providerSwitch.form.codexModelPlaceholder')"
                >
              </div>
            </div>
          </template>

          <!-- OpenCode CLI 配置 -->
          <template v-if="cliType === 'opencode'">
            <div class="form-section">
              <h4 class="section-title">
                {{ t('settings.providerSwitch.form.opencodeConfig') }}
              </h4>

              <div class="form-group">
                <label class="form-label">{{ t('settings.providerSwitch.form.opencodeProviderMode') }}</label>
                <div class="provider-mode-switch">
                  <button
                    type="button"
                    class="provider-mode-btn"
                    :class="{ active: opencodeProviderMode === 'preset' }"
                    @click="handleOpenCodeProviderModeChange('preset')"
                  >
                    {{ t('settings.providerSwitch.form.opencodeProviderModePreset') }}
                  </button>
                  <button
                    type="button"
                    class="provider-mode-btn"
                    :class="{ active: opencodeProviderMode === 'custom' }"
                    @click="handleOpenCodeProviderModeChange('custom')"
                  >
                    {{ t('settings.providerSwitch.form.opencodeProviderModeCustom') }}
                  </button>
                </div>
              </div>

              <div class="form-group">
                <label class="form-label">
                  {{ t('settings.providerSwitch.form.providerName') }} <span class="required">*</span>
                </label>
                <template v-if="!isOpenCodeCustomProvider">
                  <div class="combobox-wrapper">
                    <input
                      ref="providerComboboxInputRef"
                      :value="opencodeProviderSearch"
                      type="text"
                      class="form-input combobox-input"
                      :placeholder="t('settings.providerSwitch.form.opencodeProviderPlaceholder')"
                      :disabled="opencodeProvidersLoading"
                      required
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
                      v-if="opencodeProviderDropdownOpen && filteredProviders.length > 0"
                      class="combobox-dropdown"
                      :style="providerDropdownStyle"
                      @mousedown.prevent
                    >
                      <div
                        v-for="provider in filteredProviders"
                        :key="provider.id"
                        class="combobox-option"
                        :class="{ active: provider.id === form.providerName }"
                        @mousedown.prevent="selectOpenCodeProvider(provider)"
                      >
                        <span>{{ provider.displayName }}</span>
                        <span
                          v-if="provider.hasKey"
                          class="combobox-option-meta"
                        >
                          Key
                        </span>
                      </div>
                    </div>
                  </Teleport>
                </template>
                <template v-else>
                  <input
                    v-model="form.providerName"
                    type="text"
                    class="form-input"
                    :placeholder="t('settings.providerSwitch.form.opencodeCustomProviderPlaceholder')"
                    required
                    @blur="loadOpenCodeModels(false)"
                  >
                </template>
                <div
                  v-if="opencodeProvidersLoading && !isOpenCodeCustomProvider"
                  class="form-hint"
                >
                  {{ t('common.loading') }}
                </div>
                <div
                  v-else-if="opencodeProvidersError && !isOpenCodeCustomProvider"
                  class="form-error"
                >
                  {{ t('settings.providerSwitch.form.opencodeProvidersLoadFailed', { error: opencodeProvidersError }) }}
                </div>
                <div
                  v-else-if="!hasOpenCodeProviderOptions && !isOpenCodeCustomProvider"
                  class="form-hint"
                >
                  {{ t('settings.providerSwitch.form.opencodeProvidersEmpty') }}
                </div>
              </div>

              <div class="form-group">
                <label class="form-label">{{ t('settings.providerSwitch.form.apiKey') }}</label>
                <div class="api-key-input-wrapper">
                  <input
                    v-model="form.apiKey"
                    :type="showApiKeyValue ? 'text' : 'password'"
                    class="form-input api-key-input"
                    :placeholder="t('settings.providerSwitch.form.apiKeyPlaceholder')"
                  >
                  <button
                    type="button"
                    class="api-key-toggle"
                    @click="showApiKeyValue = !showApiKeyValue"
                  >
                    <EaIcon
                      :name="showApiKeyValue ? 'eye-off' : 'eye'"
                      :size="14"
                    />
                  </button>
                </div>
              </div>

              <div
                v-if="isOpenCodeCustomProvider"
                class="form-group"
              >
                <label class="form-label">
                  {{ t('settings.providerSwitch.form.baseUrl') }} <span class="required">*</span>
                </label>
                <input
                  v-model="form.baseUrl"
                  type="text"
                  class="form-input"
                  :placeholder="t('settings.providerSwitch.form.opencodeBaseUrlPlaceholder')"
                  required
                >
              </div>

              <div
                v-if="isOpenCodeCustomProvider"
                class="form-group"
              >
                <label class="form-label">
                  {{ t('settings.providerSwitch.form.opencodeProviderModels') }} <span class="required">*</span>
                </label>
                <div class="provider-model-list">
                  <div
                    v-for="(_, index) in opencodeProviderModelRows"
                    :key="`model-${index}`"
                    class="provider-model-row"
                  >
                    <input
                      v-model="opencodeProviderModelRows[index]"
                      type="text"
                      class="form-input"
                      :placeholder="t('settings.providerSwitch.form.opencodeProviderModelItemPlaceholder')"
                      @input="syncOpenCodeProviderModelsField"
                    >
                    <div class="provider-model-actions">
                      <button
                        v-if="opencodeProviderModelRows.length > 1"
                        type="button"
                        class="provider-model-action provider-model-action--danger"
                        @click="removeOpenCodeProviderModelRow(index)"
                      >
                        <EaIcon
                          name="minus"
                          :size="16"
                        />
                      </button>
                      <button
                        v-if="index === opencodeProviderModelRows.length - 1"
                        type="button"
                        class="provider-model-action"
                        @click="addOpenCodeProviderModelRow"
                      >
                        <EaIcon
                          name="plus"
                          :size="16"
                        />
                      </button>
                    </div>
                  </div>
                </div>
                <div class="form-hint">
                  {{ t('settings.providerSwitch.form.opencodeProviderModelsHint') }}
                </div>
              </div>

              <div
                v-if="isOpenCodeCustomProvider"
                class="form-group"
              >
                <label class="form-label">{{ t('settings.providerSwitch.form.opencodeProviderNpm') }}</label>
                <input
                  v-model="form.opencodeProviderNpm"
                  type="text"
                  class="form-input"
                  :placeholder="OPENCODE_DEFAULT_PROVIDER_NPM"
                >
                <div class="form-hint">
                  {{ t('settings.providerSwitch.form.opencodeProviderNpmHint') }}
                </div>
              </div>

              <div class="form-group model-combobox">
                <label class="form-label">
                  {{ t('settings.providerSwitch.form.mainModel') }} <span class="required">*</span>
                </label>
                <div class="combobox-wrapper">
                  <input
                    ref="comboboxInputRef"
                    :value="form.mainModel"
                    type="text"
                    class="form-input combobox-input"
                    :placeholder="t('settings.providerSwitch.form.opencodeModelPlaceholder')"
                    required
                    @focus="onModelFocus"
                    @input="onModelInput"
                    @blur="opencodeModelDropdownOpen = false"
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
                    v-if="opencodeModelDropdownOpen && filteredModels.length > 0"
                    class="combobox-dropdown"
                    :style="comboboxDropdownStyle"
                    @mousedown.prevent
                  >
                    <div
                      v-for="model in filteredModels"
                      :key="model"
                      class="combobox-option"
                      :class="{ active: model === form.mainModel }"
                      @mousedown.prevent="selectOpenCodeModel(model)"
                    >
                      {{ model }}
                    </div>
                  </div>
                </Teleport>
                <div
                  v-if="opencodeModelsLoading"
                  class="model-loading"
                >
                  {{ t('settings.providerSwitch.form.loadingModels') }}
                </div>
                <div
                  v-else-if="opencodeModelsError"
                  class="form-error"
                >
                  {{ t('settings.providerSwitch.form.opencodeModelsLoadFailed', { error: opencodeModelsError }) }}
                </div>
                <div
                  v-else-if="form.providerName && opencodeModels.length === 0"
                  class="model-hint"
                >
                  {{ isOpenCodeCustomProvider
                    ? t('settings.providerSwitch.form.opencodeCustomModelHint')
                    : t('settings.providerSwitch.form.modelHint') }}
                </div>
              </div>
            </div>
          </template>
        </form>

        <div class="modal-footer">
          <EaButton
            type="secondary"
            @click="handleClose"
          >
            {{ t('common.cancel') }}
          </EaButton>
          <EaButton
            type="primary"
            :loading="saving"
            :disabled="isSubmitDisabled"
            @click="handleSubmit"
          >
            {{ t('common.save') }}
          </EaButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-4);
  background: color-mix(in srgb, var(--color-bg-overlay, rgba(0, 0, 0, 0.5)) 36%, transparent);
  backdrop-filter: blur(10px) saturate(115%);
  z-index: calc(var(--z-modal, 1050) + 10);
}

.modal-content {
  width: min(560px, calc(100vw - 32px));
  max-height: min(84vh, 900px);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  border-radius: var(--radius-xl);
  border: 1px solid color-mix(in srgb, var(--color-border, #e0e0e0) 88%, white 12%);
  background: var(--color-surface, #ffffff);
  box-shadow: var(--shadow-2xl);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--color-border, #e0e0e0);
}

.modal-header h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text-primary, #1a1a1a);
}

.close-btn {
  background: none;
  border: none;
  cursor: pointer;
  padding: 4px;
  color: var(--color-text-secondary, #666);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: all 0.2s;
}

.close-btn:hover {
  color: var(--color-text-primary, #1a1a1a);
  background: var(--color-surface-hover, rgba(0, 0, 0, 0.05));
}

.modal-body {
  padding: 20px;
  overflow-y: auto;
  flex: 1;
}

.form-section {
  margin-bottom: 24px;
}

.form-section:last-child {
  margin-bottom: 0;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-secondary, #666);
  margin: 0 0 16px 0;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--color-border, #e0e0e0);
}

.form-group {
  margin-bottom: 16px;
}

.form-group:last-child {
  margin-bottom: 0;
}

.form-label {
  display: block;
  font-size: 13px;
  font-weight: 500;
  margin-bottom: 6px;
  color: var(--color-text-primary, #1a1a1a);
}

.required {
  color: var(--color-danger, #ef4444);
}

.form-input {
  width: 100%;
  padding: 10px 14px;
  font-size: 14px;
  border: 1px solid var(--color-border, #e0e0e0);
  border-radius: 8px;
  background: var(--color-bg-primary, #ffffff);
  color: var(--color-text-primary, #1a1a1a);
  transition: all 0.2s;
  box-sizing: border-box;
}

.form-input:focus {
  outline: none;
  border-color: var(--color-primary, #7c3aed);
  box-shadow: 0 0 0 3px rgba(124, 58, 237, 0.15);
}

.form-input::placeholder {
  color: var(--color-text-tertiary, #999);
}

.form-textarea {
  min-height: 96px;
  resize: vertical;
}

.form-select {
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='%236b7280' d='M6 8L1 3h10z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
  padding-right: 32px;
  cursor: pointer;
}

.model-combobox {
  position: relative;
}

.combobox-wrapper {
  position: relative;
}

.combobox-input {
  padding-right: 32px !important;
}

.combobox-toggle {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  background: none;
  border: none;
  padding: 4px;
  cursor: pointer;
  color: var(--color-text-tertiary, #999);
  display: flex;
  align-items: center;
}

.combobox-toggle:hover {
  color: var(--color-text-secondary, #555);
}

.combobox-dropdown {
  max-width: min(720px, calc(100vw - 32px));
  max-height: 240px;
  overflow-y: auto;
  background: var(--color-bg-primary, #fff);
  border: 1px solid var(--color-border, #e0e0e0);
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
}

.combobox-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 14px;
  font-size: 13px;
  cursor: pointer;
  color: var(--color-text-primary, #1a1a1a);
}

.combobox-option:hover {
  background: var(--color-bg-secondary, #f5f5f5);
}

.combobox-option.active {
  background: color-mix(in srgb, var(--color-primary, #7c3aed) 12%, transparent);
  color: var(--color-primary, #7c3aed);
  font-weight: 500;
}

.combobox-option-meta {
  flex-shrink: 0;
  color: var(--color-text-tertiary, #999);
  font-size: 12px;
}

.api-key-input-wrapper {
  position: relative;
}

.api-key-input {
  padding-right: 40px !important;
}

.api-key-toggle {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  background: none;
  border: none;
  padding: 4px;
  cursor: pointer;
  color: var(--color-text-tertiary, #999);
  display: flex;
  align-items: center;
}

.api-key-toggle:hover {
  color: var(--color-text-secondary, #555);
}

.provider-mode-switch {
  display: inline-flex;
  width: 100%;
  gap: 8px;
}

.provider-mode-btn {
  flex: 1;
  min-height: 40px;
  border: 1px solid var(--color-border, #e0e0e0);
  border-radius: 8px;
  background: var(--color-bg-secondary, #f5f5f5);
  color: var(--color-text-secondary, #666);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.provider-mode-btn.active {
  border-color: var(--color-primary, #7c3aed);
  background: color-mix(in srgb, var(--color-primary, #7c3aed) 10%, white);
  color: var(--color-primary, #7c3aed);
}

.provider-model-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.provider-model-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.provider-model-actions {
  display: flex;
  gap: 8px;
}

.provider-model-action {
  width: 40px;
  height: 40px;
  flex-shrink: 0;
  border: 1px solid var(--color-border, #e0e0e0);
  border-radius: 8px;
  background: var(--color-bg-secondary, #f5f5f5);
  color: var(--color-text-secondary, #666);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s;
}

.provider-model-action:hover {
  border-color: var(--color-primary, #7c3aed);
  color: var(--color-primary, #7c3aed);
}

.provider-model-action--danger:hover {
  border-color: var(--color-danger, #ef4444);
  color: var(--color-danger, #ef4444);
}

.form-hint,
.form-error,
.model-loading,
.model-hint {
  font-size: 12px;
  margin-top: 4px;
}

.form-hint,
.model-loading,
.model-hint {
  color: var(--color-text-tertiary, #999);
}

.form-error {
  color: var(--color-danger, #ef4444);
}

.form-row {
  display: flex;
  gap: 16px;
}

.form-row .form-group {
  flex: 1;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 20px;
  border-top: 1px solid var(--color-border, #e0e0e0);
  background: var(--color-bg-secondary, #f5f5f5);
}

@media (max-width: 640px) {
  .modal-content {
    width: calc(100vw - 24px);
    max-height: calc(100vh - 24px);
  }

  .modal-header,
  .modal-body,
  .modal-footer {
    padding-left: 16px;
    padding-right: 16px;
  }

  .form-row {
    flex-direction: column;
    gap: 0;
  }
}
</style>
