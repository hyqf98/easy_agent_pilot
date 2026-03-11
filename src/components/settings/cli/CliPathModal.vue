<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon } from '@/components/common'

interface CliNameOption {
  value: string
  label: string
}

interface ValidationResult {
  valid: boolean
  version: string | null
}

interface Props {
  visible: boolean
  mode: 'add' | 'edit'
  name: string
  path: string
  nameOptions: CliNameOption[]
  formError: string
  isValidating: boolean
  validationResult: ValidationResult | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  'update:name': [value: string]
  'update:path': [value: string]
  browse: []
  submit: []
  validate: []
}>()

const { t } = useI18n()

const dialogVisible = computed({
  get: () => props.visible,
  set: (value: boolean) => emit('update:visible', value)
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="dialogVisible"
      class="modal-overlay"
      @click.self="dialogVisible = false"
    >
      <div class="modal">
        <div class="modal__header">
          <h3 class="modal__title">
            {{ mode === 'add' ? t('settings.cli.addCliTitle') : t('settings.cli.editCliTitle') }}
          </h3>
          <button
            class="modal__close"
            @click="dialogVisible = false"
          >
            <EaIcon
              name="x"
              :size="20"
            />
          </button>
        </div>

        <div class="modal__body">
          <div class="form-group">
            <label class="form-label">{{ t('settings.cli.cliName') }}</label>
            <select
              :value="name"
              class="form-select"
              @change="emit('update:name', ($event.target as HTMLSelectElement).value)"
            >
              <option
                value=""
                disabled
              >
                {{ t('settings.cli.selectCli') }}
              </option>
              <option
                v-for="option in nameOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </div>

          <div class="form-group">
            <label class="form-label">{{ t('settings.cli.executablePath') }}</label>
            <div class="form-input-group">
              <input
                :value="path"
                type="text"
                class="form-input"
                :placeholder="t('settings.cli.pathPlaceholder')"
                @input="emit('update:path', ($event.target as HTMLInputElement).value)"
                @blur="emit('validate')"
              >
              <EaButton
                type="secondary"
                size="small"
                @click="emit('browse')"
              >
                {{ t('settings.cli.browse') }}
              </EaButton>
            </div>
          </div>

          <div
            v-if="isValidating"
            class="validation-status validation-status--loading"
          >
            <div class="validation-spinner">
              <svg
                viewBox="0 0 24 24"
                class="animate-spin"
              >
                <circle
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="3"
                  fill="none"
                  stroke-dasharray="31.416"
                  stroke-dashoffset="10"
                />
              </svg>
            </div>
            <span>{{ t('settings.cli.validating') }}</span>
          </div>

          <div
            v-else-if="validationResult"
            class="validation-status"
            :class="validationResult.valid ? 'validation-status--success' : 'validation-status--error'"
          >
            <EaIcon
              :name="validationResult.valid ? 'check-circle' : 'x-circle'"
              :size="16"
            />
            <span v-if="validationResult.valid">{{ t('settings.cli.validationSuccess', { version: validationResult.version }) }}</span>
            <span v-else>{{ t('settings.cli.validationFailed') }}</span>
          </div>

          <div
            v-if="formError"
            class="form-error"
          >
            {{ formError }}
          </div>
        </div>

        <div class="modal__footer">
          <EaButton
            type="secondary"
            @click="dialogVisible = false"
          >
            {{ t('common.cancel') }}
          </EaButton>
          <EaButton
            type="primary"
            :disabled="!name || !path"
            @click="emit('submit')"
          >
            {{ mode === 'add' ? t('common.create') : t('common.save') }}
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
  z-index: 1060;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(0, 0, 0, 0.5);
}

.modal {
  width: 100%;
  max-width: 480px;
  margin: var(--spacing-4);
  border-radius: var(--radius-lg);
  background-color: var(--color-bg-primary);
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
}

.modal__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4) var(--spacing-5);
  border-bottom: 1px solid var(--color-border);
}

.modal__title {
  margin: 0;
  color: var(--color-text-primary);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
}

.modal__close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: var(--radius-sm);
  background-color: transparent;
  color: var(--color-text-tertiary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.modal__close:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.modal__body {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  padding: var(--spacing-5);
}

.modal__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3);
  padding: var(--spacing-4) var(--spacing-5);
  border-top: 1px solid var(--color-border);
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.form-label {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.form-input,
.form-select {
  height: 36px;
  padding: 0 var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background-color: var(--color-surface);
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  outline: none;
  transition: border-color var(--transition-fast);
}

.form-input:focus,
.form-select:focus {
  border-color: var(--color-primary);
}

.form-input::placeholder {
  color: var(--color-text-tertiary);
}

.form-select {
  cursor: pointer;
}

.form-input-group {
  display: flex;
  gap: var(--spacing-2);
}

.form-input-group .form-input {
  flex: 1;
}

.validation-status {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
}

.validation-status--loading {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

.validation-status--success {
  background-color: var(--color-success-light, rgba(34, 197, 94, 0.1));
  color: var(--color-success);
}

.validation-status--error {
  background-color: var(--color-error-light, rgba(239, 68, 68, 0.1));
  color: var(--color-error, #ef4444);
}

.validation-spinner {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
}

.validation-spinner svg {
  width: 100%;
  height: 100%;
}

.form-error {
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-md);
  background-color: var(--color-error-light, rgba(239, 68, 68, 0.1));
  color: var(--color-error, #ef4444);
  font-size: var(--font-size-sm);
}

.animate-spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(360deg);
  }
}
</style>
