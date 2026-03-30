<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ProviderProfile, CliType, CreateProviderProfileInput, UpdateProviderProfileInput } from '@/stores/providerProfile'
import { EaButton, EaIcon } from '@/components/common'
import { useOverlayDismiss } from '@/composables/useOverlayDismiss'

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
    reasoningModel: '',
    haikuModel: '',
    sonnetDefault: '',
    opusDefault: '',
    codexModel: ''
  }
}

// 填充表单（编辑模式）
function populateForm(profile: ProviderProfile) {
  form.value = {
    name: profile.name || '',
    apiKey: profile.apiKey || '',
    baseUrl: profile.baseUrl || '',
    providerName: profile.providerName || '',
    mainModel: profile.mainModel || '',
    reasoningModel: profile.reasoningModel || '',
    haikuModel: profile.haikuModel || '',
    sonnetDefault: profile.sonnetDefault || '',
    opusDefault: profile.opusDefault || '',
    codexModel: profile.codexModel || ''
  }
}

// 监听 profile 变化
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

// 关闭弹窗
function handleClose() {
  emit('update:visible', false)
  resetForm()
}

const { handleOverlayPointerDown, handleOverlayClick } = useOverlayDismiss(handleClose)

// 提交表单
async function handleSubmit() {
  if (!isCurrentConfig.value && !form.value.name.trim()) {
    return
  }

  saving.value = true

  try {
    if (isCurrentConfig.value) {
      const input: UpdateProviderProfileInput = {
        apiKey: form.value.apiKey || undefined,
        baseUrl: form.value.baseUrl || undefined,
        providerName: form.value.providerName || undefined,
        mainModel: form.value.mainModel || undefined,
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
              name="close"
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
                <input
                  v-model="form.apiKey"
                  type="password"
                  class="form-input"
                  :placeholder="t('settings.providerSwitch.form.apiKeyPlaceholder')"
                >
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
                <input
                  v-model="form.apiKey"
                  type="password"
                  class="form-input"
                  :placeholder="t('settings.providerSwitch.form.apiKeyPlaceholder')"
                >
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
            :disabled="!isCurrentConfig && !form.name.trim()"
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
