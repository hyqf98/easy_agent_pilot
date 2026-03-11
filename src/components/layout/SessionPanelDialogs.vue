<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Session } from '@/stores/session'
import { EaButton, EaIcon } from '@/components/common'

interface Props {
  sessionCreateModalVisible: boolean
  newSessionName: string
  isNewSessionFormValid: boolean
  showDeleteConfirm: boolean
  deletingSession: Session | null
  showClearMessagesConfirm: boolean
  isClearingMessages: boolean
  showErrorModal: boolean
  errorSession: Session | null
  showSummaryModal: boolean
  summarySession: Session | null
}

defineProps<Props>()

const emit = defineEmits<{
  closeCreate: []
  submitCreate: []
  updateNewSessionName: [value: string]
  closeDelete: []
  confirmDelete: []
  closeClearMessages: []
  confirmClearMessages: []
  closeError: []
  retryError: []
  closeSummary: []
  rerunSummary: []
}>()

const { t } = useI18n()
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="sessionCreateModalVisible"
        class="modal-overlay"
        @click="emit('closeCreate')"
      >
        <div
          class="modal-container"
          @click.stop
        >
          <div class="session-form">
            <div class="session-form__header">
              <h3 class="session-form__title">
                {{ t('session.createSession') }}
              </h3>
            </div>
            <form
              class="session-form__body"
              @submit.prevent="emit('submitCreate')"
            >
              <div class="form-group">
                <label class="form-label">
                  {{ t('session.sessionName') }} <span class="form-label__required">*</span>
                </label>
                <input
                  :value="newSessionName"
                  name="sessionName"
                  type="text"
                  class="form-input"
                  :placeholder="t('session.enterSessionName')"
                  required
                  autofocus
                  @input="emit('updateNewSessionName', ($event.target as HTMLInputElement).value)"
                >
              </div>
              <div class="session-form__actions">
                <EaButton
                  type="secondary"
                  @click="emit('closeCreate')"
                >
                  {{ t('common.cancel') }}
                </EaButton>
                <EaButton
                  type="primary"
                  native-type="submit"
                  :disabled="!isNewSessionFormValid"
                >
                  {{ t('common.create') }}
                </EaButton>
              </div>
            </form>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>

  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="showDeleteConfirm"
        class="modal-overlay"
        @click="emit('closeDelete')"
      >
        <div
          class="confirm-dialog"
          @click.stop
        >
          <div class="confirm-dialog__content">
            <EaIcon
              name="alert-triangle"
              :size="24"
              class="confirm-dialog__icon"
            />
            <h4 class="confirm-dialog__title">
              {{ t('session.confirmDeleteTitle') }}
            </h4>
            <p class="confirm-dialog__message">
              {{ t('session.confirmDeleteMessage', { name: deletingSession?.name }) }}
            </p>
          </div>
          <div class="confirm-dialog__actions">
            <EaButton
              type="secondary"
              @click="emit('closeDelete')"
            >
              {{ t('common.cancel') }}
            </EaButton>
            <EaButton
              type="primary"
              @click="emit('confirmDelete')"
            >
              {{ t('common.confirmDelete') }}
            </EaButton>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>

  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="showClearMessagesConfirm"
        class="modal-overlay"
        @click="emit('closeClearMessages')"
      >
        <div
          class="confirm-dialog"
          @click.stop
        >
          <div class="confirm-dialog__content">
            <EaIcon
              name="eraser"
              :size="24"
              class="confirm-dialog__icon confirm-dialog__icon--warning"
            />
            <h4 class="confirm-dialog__title">
              {{ t('message.clearMessages') }}
            </h4>
            <p class="confirm-dialog__message">
              {{ t('message.clearMessagesConfirm') }}
            </p>
          </div>
          <div class="confirm-dialog__actions">
            <EaButton
              type="secondary"
              :disabled="isClearingMessages"
              @click="emit('closeClearMessages')"
            >
              {{ t('common.cancel') }}
            </EaButton>
            <EaButton
              type="primary"
              :loading="isClearingMessages"
              @click="emit('confirmClearMessages')"
            >
              {{ t('common.confirm') }}
            </EaButton>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>

  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="showErrorModal"
        class="modal-overlay"
        @click="emit('closeError')"
      >
        <div
          class="error-dialog"
          @click.stop
        >
          <div class="error-dialog__content">
            <EaIcon
              name="alert-circle"
              :size="24"
              class="error-dialog__icon"
            />
            <h4 class="error-dialog__title">
              {{ t('session.errorDetails') }}
            </h4>
            <p class="error-dialog__session-name">
              {{ errorSession?.name }}
            </p>
            <div class="error-dialog__message-box">
              <pre class="error-dialog__message">{{ errorSession?.errorMessage || t('session.noErrorMessage') }}</pre>
            </div>
          </div>
          <div class="error-dialog__actions">
            <EaButton
              type="secondary"
              @click="emit('closeError')"
            >
              {{ t('common.close') }}
            </EaButton>
            <EaButton
              type="primary"
              @click="emit('retryError')"
            >
              <EaIcon
                name="refresh-cw"
                :size="14"
              />
              {{ t('common.retry') }}
            </EaButton>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>

  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="showSummaryModal"
        class="modal-overlay"
        @click="emit('closeSummary')"
      >
        <div
          class="summary-dialog"
          @click.stop
        >
          <div class="summary-dialog__content">
            <EaIcon
              name="check-circle"
              :size="24"
              class="summary-dialog__icon"
            />
            <h4 class="summary-dialog__title">
              {{ t('session.executionSummary') }}
            </h4>
            <p class="summary-dialog__session-name">
              {{ summarySession?.name }}
            </p>
            <div class="summary-dialog__info">
              <div class="summary-dialog__stats">
                <div class="summary-stat">
                  <span class="summary-stat__label">{{ t('session.summaryContent') }}</span>
                  <span class="summary-stat__value">{{ summarySession?.messageCount || 0 }} {{ t('common.messages', { n: '' }).trim() }}</span>
                </div>
              </div>
            </div>
            <div class="summary-dialog__message-box">
              <pre class="summary-dialog__message">{{ summarySession?.lastMessage || t('session.noSummaryAvailable') }}</pre>
            </div>
          </div>
          <div class="summary-dialog__actions">
            <EaButton
              type="secondary"
              @click="emit('closeSummary')"
            >
              {{ t('common.close') }}
            </EaButton>
            <EaButton
              type="primary"
              @click="emit('rerunSummary')"
            >
              <EaIcon
                name="rotate-ccw"
                :size="14"
              />
              {{ t('session.rerun') }}
            </EaButton>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(15, 23, 42, 0.38);
  backdrop-filter: blur(8px);
}

.modal-container,
.confirm-dialog,
.error-dialog,
.summary-dialog {
  background: var(--color-surface);
  border: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
}

.modal-container {
  width: min(420px, 92vw);
}

.session-form {
  display: flex;
  flex-direction: column;
}

.session-form__header,
.session-form__body,
.confirm-dialog__content,
.confirm-dialog__actions,
.error-dialog__content,
.error-dialog__actions,
.summary-dialog__content,
.summary-dialog__actions {
  padding-left: var(--spacing-5);
  padding-right: var(--spacing-5);
}

.session-form__header {
  padding-top: var(--spacing-4);
  padding-bottom: var(--spacing-4);
  border-bottom: 1px solid var(--color-border);
}

.session-form__title,
.confirm-dialog__title,
.error-dialog__title,
.summary-dialog__title {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.session-form__body {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  padding-top: var(--spacing-5);
  padding-bottom: var(--spacing-5);
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
  margin-left: 2px;
  color: var(--color-error);
}

.form-input {
  padding: var(--spacing-2) var(--spacing-3);
  background: color-mix(in srgb, var(--color-surface) 84%, var(--color-bg-tertiary));
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
}

.form-input:focus {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary-light) 62%, transparent);
  outline: none;
}

.session-form__actions,
.confirm-dialog__actions,
.error-dialog__actions,
.summary-dialog__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3);
  padding-top: var(--spacing-4);
  padding-bottom: var(--spacing-4);
  border-top: 1px solid var(--color-border);
}

.confirm-dialog {
  width: min(420px, 92vw);
}

.confirm-dialog__content,
.error-dialog__content,
.summary-dialog__content {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding-top: var(--spacing-6);
  padding-bottom: var(--spacing-6);
  text-align: center;
}

.confirm-dialog__icon,
.error-dialog__icon,
.summary-dialog__icon {
  margin-bottom: var(--spacing-4);
}

.confirm-dialog__icon {
  color: var(--color-warning);
}

.confirm-dialog__icon--warning {
  color: var(--color-warning);
}

.confirm-dialog__message,
.error-dialog__session-name,
.summary-dialog__session-name {
  margin: var(--spacing-2) 0 0;
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  line-height: 1.6;
}

.error-dialog,
.summary-dialog {
  width: min(520px, 92vw);
}

.error-dialog__icon {
  color: var(--color-error);
}

.summary-dialog__icon {
  color: var(--color-success);
}

.error-dialog__message-box,
.summary-dialog__message-box {
  width: 100%;
  max-height: 220px;
  overflow: auto;
  margin-top: var(--spacing-4);
  background: color-mix(in srgb, var(--color-bg-tertiary) 88%, transparent);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  text-align: left;
}

.error-dialog__message,
.summary-dialog__message {
  margin: 0;
  padding: var(--spacing-3);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}

.error-dialog__message {
  color: var(--color-error);
}

.summary-dialog__info {
  width: 100%;
  margin-top: var(--spacing-4);
}

.summary-dialog__stats {
  display: flex;
  justify-content: center;
}

.summary-stat {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--spacing-1);
  padding: var(--spacing-3) var(--spacing-4);
  background: color-mix(in srgb, var(--color-bg-tertiary) 72%, transparent);
  border-radius: var(--radius-lg);
}

.summary-stat__label {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.summary-stat__value {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity var(--transition-normal) var(--easing-default);
}

.modal-enter-active .modal-container,
.modal-enter-active .confirm-dialog,
.modal-enter-active .error-dialog,
.modal-enter-active .summary-dialog,
.modal-leave-active .modal-container,
.modal-leave-active .confirm-dialog,
.modal-leave-active .error-dialog,
.modal-leave-active .summary-dialog {
  transition:
    transform var(--transition-normal) var(--easing-default),
    opacity var(--transition-normal) var(--easing-default);
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal-container,
.modal-enter-from .confirm-dialog,
.modal-enter-from .error-dialog,
.modal-enter-from .summary-dialog,
.modal-leave-to .modal-container,
.modal-leave-to .confirm-dialog,
.modal-leave-to .error-dialog,
.modal-leave-to .summary-dialog {
  transform: translateY(10px) scale(0.98);
  opacity: 0;
}
</style>
