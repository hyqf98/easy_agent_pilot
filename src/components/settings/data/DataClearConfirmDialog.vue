<script setup lang="ts">
import { computed } from 'vue'
import { EaButton, EaIcon } from '@/components/common'
import { useI18n } from 'vue-i18n'

interface Props {
  visible: boolean
  confirmText: string
  clearing: boolean
  message: string
  success: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  'update:confirmText': [value: string]
  confirm: []
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
      <div class="modal-content modal-content--danger">
        <div class="modal-header">
          <h3 class="modal-title">
            {{ t('settings.data.clearConfirmTitle') }}
          </h3>
          <button
            class="modal-close"
            @click="dialogVisible = false"
          >
            <EaIcon
              name="x"
              :size="20"
            />
          </button>
        </div>
        <div class="modal-body">
          <p class="modal-warning">
            {{ t('settings.data.clearConfirmWarning') }}
          </p>
          <p class="modal-hint">
            {{ t('settings.data.clearConfirmHint') }}
          </p>
          <div class="confirm-input-group">
            <label for="clear-confirm-input">
              {{ t('settings.data.clearConfirmLabel') }}
            </label>
            <input
              id="clear-confirm-input"
              :value="confirmText"
              type="text"
              class="form-input"
              placeholder="CLEAR"
              :disabled="clearing"
              @input="emit('update:confirmText', ($event.target as HTMLInputElement).value)"
            >
          </div>
          <div
            v-if="message"
            class="clear-message"
            :class="{ 'clear-message--success': success, 'clear-message--error': !success }"
          >
            <EaIcon
              :name="success ? 'check' : 'x'"
              :size="16"
            />
            <span>{{ message }}</span>
          </div>
        </div>
        <div class="modal-footer">
          <EaButton
            type="secondary"
            size="medium"
            :disabled="clearing"
            @click="dialogVisible = false"
          >
            {{ t('common.cancel') }}
          </EaButton>
          <EaButton
            type="danger"
            size="medium"
            :disabled="clearing"
            :loading="clearing"
            @click="emit('confirm')"
          >
            {{ t('settings.data.clearConfirmButton') }}
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
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(0, 0, 0, 0.5);
}

.modal-content {
  width: 90%;
  max-width: 480px;
  border-radius: var(--radius-lg);
  background-color: var(--color-bg-primary);
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
}

.modal-content--danger {
  border: 1px solid var(--color-error-light);
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4) var(--spacing-5);
  border-bottom: 1px solid var(--color-border);
}

.modal-title {
  margin: 0;
  color: var(--color-error);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
}

.modal-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: var(--radius-md);
  background: none;
  color: var(--color-text-tertiary);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.modal-close:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.modal-body {
  padding: var(--spacing-5);
}

.modal-warning {
  margin: 0 0 var(--spacing-3);
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  line-height: 1.5;
}

.modal-hint {
  margin: 0 0 var(--spacing-4);
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

.confirm-input-group {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.confirm-input-group label {
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.form-input {
  height: 40px;
  padding: 0 var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background-color: var(--color-bg-secondary);
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-family: var(--font-family-mono);
  transition: all var(--transition-fast) var(--easing-default);
}

.form-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
}

.form-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.form-input::placeholder {
  color: var(--color-text-tertiary);
}

.clear-message {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  margin-top: var(--spacing-3);
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
}

.clear-message--success {
  background-color: var(--color-success-light);
  color: var(--color-success);
}

.clear-message--error {
  background-color: var(--color-error-light);
  color: var(--color-error);
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3);
  padding: var(--spacing-4) var(--spacing-5);
  border-top: 1px solid var(--color-border);
}
</style>
