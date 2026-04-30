<script setup lang="ts">
import { EaButton, EaIcon } from '@/components/common'
import {
  useSessionPanelDialogs,
  type SessionPanelDialogsEmits,
  type SessionPanelDialogsProps,
} from './useSessionPanelDialogs'

defineProps<SessionPanelDialogsProps>()
const emit = defineEmits<SessionPanelDialogsEmits>()

const { t } = useSessionPanelDialogs()
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
              {{ deletingSessionCount > 1 ? t('session.confirmBatchDeleteTitle') : t('session.confirmDeleteTitle') }}
            </h4>
            <p class="confirm-dialog__message">
              {{
                deletingSessionCount > 1
                  ? t('session.confirmBatchDeleteMessage', { count: deletingSessionCount })
                  : t('session.confirmDeleteMessage', { name: deletingSession?.name })
              }}
            </p>
          </div>
          <div class="confirm-dialog__actions">
            <EaButton
              type="secondary"
              :disabled="isDeletingSessions"
              @click="emit('closeDelete')"
            >
              {{ t('common.cancel') }}
            </EaButton>
            <EaButton
              type="primary"
              :loading="isDeletingSessions"
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

<style scoped src="./styles.css"></style>
