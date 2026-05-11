<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AgentConfig, AgentType, AgentProvider } from '@/stores/agent'
import { EaButton, EaSelect } from '@/components/common'
import { validateUrl } from '@/utils/validation'

export interface AgentConfigFormProps {
  agent?: AgentConfig | null
}

const props = defineProps<AgentConfigFormProps>()

const emit = defineEmits<{
  submit: [data: Omit<AgentConfig, 'id' | 'createdAt' | 'updatedAt' | 'status'>]
  cancel: []
}>()

const { t } = useI18n()

function createDefaultForm() {
  return {
    name: '',
    type: 'cli' as AgentType,
    provider: 'claude' as AgentProvider,
    apiKey: '',
    baseUrl: ''
  }
}

const form = ref(createDefaultForm())

function createDefaultFieldErrors() {
  return {
    name: '',
    baseUrl: ''
  }
}

const fieldErrors = ref(createDefaultFieldErrors())

const isValidating = ref({
  baseUrl: false
})

const errorMessage = ref('')
const isSubmitting = ref(false)

const isEditing = computed(() => !!props.agent)

function resetForm() {
  form.value = createDefaultForm()
  fieldErrors.value = createDefaultFieldErrors()
  errorMessage.value = ''
  isValidating.value = {
    baseUrl: false
  }
}

// 编辑模式下填充表单
watch(() => props.agent, (agent) => {
  if (agent) {
    form.value = {
      name: agent.name,
      type: agent.type,
      provider: agent.provider || 'claude',
      apiKey: agent.apiKey || '',
      baseUrl: agent.baseUrl || ''
    }
    fieldErrors.value = createDefaultFieldErrors()
    errorMessage.value = ''
  } else {
    resetForm()
  }
}, { immediate: true })

// 监听名称输入，清除错误
watch(() => form.value.name, () => {
  if (fieldErrors.value.name) {
    fieldErrors.value.name = ''
  }
  if (errorMessage.value) {
    errorMessage.value = ''
  }
})

// 监听 baseUrl 输入，清除错误
watch(() => form.value.baseUrl, () => {
  if (fieldErrors.value.baseUrl) {
    fieldErrors.value.baseUrl = ''
  }
  if (errorMessage.value) {
    errorMessage.value = ''
  }
})

const typeOptions = computed(() => (
  isEditing.value
    ? [
        { value: 'cli', label: t('settings.agent.modeCli') },
        { value: 'sdk', label: t('settings.agent.modeApi') }
      ]
    : [
        { value: 'cli', label: t('settings.agent.modeCli') }
      ]
))

const providerOptions = computed(() => {
  if (form.value.type === 'cli') {
    return [
      { value: 'claude', label: t('settings.agent.providerClaudeCli') },
      { value: 'codex', label: t('settings.agent.providerCodexCli') },
      { value: 'opencode', label: t('settings.agent.providerOpencodeCli') }
    ]
  } else {
    return [
      { value: 'claude', label: t('settings.agent.providerClaudeSdk') },
      { value: 'codex', label: t('settings.agent.providerCodexSdk') }
    ]
  }
})

const showSdkFields = computed(() => form.value.type === 'sdk')

// 验证 URL 格式（即时验证）
const validateBaseUrlFormat = () => {
  if (!form.value.baseUrl.trim()) {
    fieldErrors.value.baseUrl = ''
    return true
  }

  const result = validateUrl(form.value.baseUrl.trim())
  if (!result.valid && result.error) {
    // 使用 i18n 翻译错误消息
    if (result.error.includes('协议')) {
      fieldErrors.value.baseUrl = t('settings.agent.validation.urlProtocolRequired')
    } else {
      fieldErrors.value.baseUrl = t('settings.agent.validation.urlInvalid')
    }
    return false
  }

  fieldErrors.value.baseUrl = ''
  return true
}

// 表单有效性校验
const isFormValid = computed(() => {
  // 名称必填
  if (!form.value.name.trim()) return false

  // 有字段级错误时禁用
  if (fieldErrors.value.name || fieldErrors.value.baseUrl) {
    return false
  }

  // 正在验证时禁用
  if (isValidating.value.baseUrl) {
    return false
  }

  // SDK 模式下 Base URL 必填
  if (form.value.type === 'sdk' && !form.value.baseUrl.trim()) return false

  return true
})

const validateForm = async (): Promise<boolean> => {
  // 名称必填
  if (!form.value.name.trim()) {
    fieldErrors.value.name = t('settings.agent.nameRequired')
    return false
  }

  // SDK 模式验证
  if (form.value.type === 'sdk') {
    if (!form.value.baseUrl.trim()) {
      fieldErrors.value.baseUrl = t('settings.agent.baseUrlRequired')
      return false
    }

    // 验证 URL 格式
    if (!validateBaseUrlFormat()) {
      return false
    }
  }

  return true
}

const handleSubmit = async () => {
  if (!(await validateForm())) return

  isSubmitting.value = true
  try {
    emit('submit', {
      name: form.value.name.trim(),
      type: form.value.type,
      provider: form.value.provider,
      apiKey: form.value.apiKey || undefined,
      baseUrl: form.value.baseUrl || undefined,
      cliPath: form.value.type === 'cli' ? form.value.provider : undefined
    })
  } finally {
    isSubmitting.value = false
  }
}

const handleCancel = () => {
  emit('cancel')
}

const handleBaseUrlBlur = () => {
  validateBaseUrlFormat()
}
</script>

<template>
  <div class="agent-form">
    <div class="agent-form__header">
      <h3 class="agent-form__title">
        {{ isEditing ? t('settings.agent.editAgent') : t('settings.agent.addAgent') }}
      </h3>
    </div>

    <form
      class="agent-form__body"
      @submit.prevent="handleSubmit"
    >
      <!-- 全局错误提示 -->
      <div
        v-if="errorMessage"
        class="form-error"
      >
        {{ errorMessage }}
      </div>

      <div class="form-group">
        <label class="form-label">
          {{ t('settings.agent.name') }} <span class="form-label__required">*</span>
        </label>
        <input
          v-model="form.name"
          type="text"
          class="form-input"
          :class="{ 'form-input--error': fieldErrors.name }"
          :placeholder="t('settings.agent.namePlaceholder')"
        >
        <span
          v-if="fieldErrors.name"
          class="form-field-error"
        >
          {{ fieldErrors.name }}
        </span>
      </div>

      <div class="form-row">
        <div
          v-if="isEditing"
          class="form-group"
        >
          <label class="form-label">
            {{ t('settings.agent.type') }} <span class="form-label__required">*</span>
          </label>
          <EaSelect
            v-model="form.type"
            :options="typeOptions"
          />
        </div>

        <div class="form-group">
          <label class="form-label">
            {{ t('settings.agent.provider') }} <span class="form-label__required">*</span>
          </label>
          <EaSelect
            v-model="form.provider"
            :options="providerOptions"
          />
        </div>
      </div>

      <!-- SDK 模式字段 -->
      <template v-if="showSdkFields">
        <div class="form-group">
          <label class="form-label">{{ t('settings.agent.apiKey') }}</label>
          <input
            v-model="form.apiKey"
            type="password"
            class="form-input"
            :placeholder="t('settings.agent.apiKeyPlaceholder')"
          >
        </div>

        <div class="form-group">
          <label class="form-label">
            {{ t('settings.agent.baseUrl') }} <span class="form-label__required">*</span>
          </label>
          <input
            v-model="form.baseUrl"
            type="url"
            class="form-input"
            :class="{ 'form-input--error': fieldErrors.baseUrl }"
            :placeholder="t('settings.agent.baseUrlPlaceholder')"
            @blur="handleBaseUrlBlur"
          >
          <span
            v-if="fieldErrors.baseUrl"
            class="form-field-error"
          >
            {{ fieldErrors.baseUrl }}
          </span>
        </div>
      </template>

      <div class="agent-form__actions">
        <EaButton
          type="secondary"
          @click="handleCancel"
        >
          {{ t('common.cancel') }}
        </EaButton>
        <EaButton
          type="primary"
          :disabled="!isFormValid || isSubmitting"
          @click="handleSubmit"
        >
          {{ isEditing ? t('common.save') : t('common.create') }}
        </EaButton>
      </div>
    </form>
  </div>
</template>

<style scoped>
.agent-form {
  display: flex;
  flex-direction: column;
}

.agent-form__header {
  padding: var(--spacing-4) var(--spacing-5);
  border-bottom: 1px solid var(--color-border);
}

.agent-form__title {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.agent-form__body {
  padding: var(--spacing-5);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.form-row {
  display: flex;
  gap: var(--spacing-4);
}

.form-row .form-group {
  flex: 1;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.form-label {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
}

.form-label__required {
  color: var(--color-error, #ef4444);
  margin-left: 2px;
}

.form-input,
.form-select {
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  transition: border-color var(--transition-fast) var(--easing-default);
}

.form-input:focus,
.form-select:focus {
  border-color: var(--color-primary);
  outline: none;
}

.form-input::placeholder {
  color: var(--color-text-tertiary);
}

.form-input--error {
  border-color: var(--color-error, #ef4444);
}

.form-input--error:focus {
  border-color: var(--color-error, #ef4444);
}

.form-field-error {
  font-size: var(--font-size-xs);
  color: var(--color-error, #ef4444);
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
}

.form-error {
  padding: var(--spacing-3);
  background-color: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--color-error, #ef4444);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-error, #ef4444);
}

.agent-form__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3);
  padding-top: var(--spacing-4);
  border-top: 1px solid var(--color-border);
  margin-top: var(--spacing-2);
}
</style>
