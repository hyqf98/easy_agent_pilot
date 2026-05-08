<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import EaConfirmDialog from '@/components/common/EaConfirmDialog.vue'
import TaskSplitPreview from '../TaskSplitPreview.vue'
import MessageList from '@/components/message/messageList/MessageList.vue'
import { useTaskSplitDialog } from './useTaskSplitDialog'

const { t } = useI18n()

const {
  planStore,
  splitDialogTabs,
  activeSplitPlanId,
  taskSplitStore,
  isDarkTheme,
  isConfirming,
  userInstruction,
  instructionInputRef,
  showMentionSuggestions,
  mentionSuggestions,
  selectedMentionOptionIndex,
  showPreview,
  hasPendingRefinement,
  isSubSplitActive,
  subSplitTargetTitle,
  previewActionsDisabled,
  canApplyRefinement,
  isSessionRunning,
  canRetrySplit,
  canContinueSplit,
  retryButtonLabel,
  isAutoRetryPending,
  primaryActionLabel,
  splitChatSessionId,
  splitChatMessages,
  splitCurrentStreamingMessageId,
  splitChatScrollToken,
  isSplitHistoryLoading,
  restartSplit,
  handleActiveFormSubmit,
  confirmSplit,
  handleCloseClick,
  cancelCloseConfirm,
  confirmCloseAndStop,
  handleMinimizeClick,
  showCloseConfirm,
  stopSplitTask,
  retrySplitTask,
  continueSplitTask,
  handleInstructionInput,
  handleInstructionKeydown,
  handleInstructionCaretChange,
  applyMentionSuggestion,
  handleOverlayPointerDown,
  handleOverlayClick
} = useTaskSplitDialog()
</script>

<template>
  <Teleport to="body">
    <div
      v-if="planStore.splitDialogVisible"
      class="split-dialog-overlay"
      :class="{ 'split-dialog-overlay--dark': isDarkTheme }"
      @pointerdown.capture="handleOverlayPointerDown"
      @click.self="handleOverlayClick"
    >
      <div
        class="split-dialog"
        :class="{ 'split-dialog--dark': isDarkTheme }"
      >
        <div class="dialog-header">
          <h4>
            <span class="dialog-icon">✂️</span>
            {{ t('taskSplit.dialogTitle') }}
          </h4>
          <div class="dialog-header-actions">
            <button
              class="btn-close"
              :title="t('taskSplit.hide')"
              @click="handleMinimizeClick"
            >
              <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <path d="M5 12h14" />
              </svg>
            </button>
            <button
              class="btn-close"
              :title="t('taskSplit.close')"
              @click="handleCloseClick"
            >
              <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <path d="M18 6L6 18M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <div
          v-if="splitDialogTabs.length > 1"
          class="dialog-tabs"
        >
          <button
            v-for="tab in splitDialogTabs"
            :key="tab.planId"
            type="button"
            class="dialog-tab"
            :class="{ 'dialog-tab--active': tab.planId === activeSplitPlanId }"
            @click.stop="planStore.switchSplitDialogTab(tab.planId)"
          >
            {{ tab.label }}
          </button>
        </div>

        <div class="dialog-body">
          <div class="split-content">
            <div class="conversation-pane">
              <div class="messages-container">
                <MessageList
                  :session-id="splitChatSessionId"
                  :messages="splitChatMessages"
                  :external-is-sending="isSessionRunning || isSplitHistoryLoading"
                  :current-streaming-message-id="splitCurrentStreamingMessageId"
                  :force-scroll-to-bottom-token="splitChatScrollToken"
                  hide-context-strategy-notice
                  @form-submit="handleActiveFormSubmit"
                  @stop="stopSplitTask"
                  @retry="retrySplitTask"
                />
              </div>

              <div class="conversation-input-area">
                <div
                  v-if="isSubSplitActive && isSessionRunning"
                  class="footer-resplit-hint"
                >
                  <span class="resplit-hint-spinner" />
                  <span>{{ t('taskSplit.resplitInProgressHint', { title: subSplitTargetTitle }) }}</span>
                </div>
                <div class="pane-input-bar">
                  <div class="input-wrapper">
                    <textarea
                      ref="instructionInputRef"
                      v-model="userInstruction"
                      class="instruction-input"
                      :disabled="isConfirming"
                      :placeholder="t('taskSplit.instructionPlaceholder')"
                      rows="2"
                      @keydown="handleInstructionKeydown"
                      @input="handleInstructionInput"
                      @click="handleInstructionCaretChange"
                      @keyup="handleInstructionCaretChange"
                      @select="handleInstructionCaretChange"
                    />
                    <div
                      v-if="showMentionSuggestions"
                      class="instruction-mentions"
                    >
                      <button
                        v-for="(option, index) in mentionSuggestions"
                        :key="option.index"
                        type="button"
                        class="instruction-mentions__item"
                        :class="{ 'instruction-mentions__item--active': index === selectedMentionOptionIndex }"
                        @mousedown.prevent="applyMentionSuggestion(index)"
                      >
                        <span class="instruction-mentions__badge">@{{ option.index + 1 }}</span>
                        <span class="instruction-mentions__title">{{ option.title || t('taskBoard.emptyNoTasks') }}</span>
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div
              v-if="showPreview"
              class="preview-pane"
            >
              <div
                v-if="isSubSplitActive && isSessionRunning"
                class="preview-resplit-overlay"
              >
                <div class="resplit-overlay-spinner" />
                <span class="resplit-overlay-text">{{ t('taskSplit.resplitInProgress', { title: subSplitTargetTitle }) }}</span>
              </div>
              <TaskSplitPreview
                :tasks="taskSplitStore.splitResult!"
                :disable-actions="previewActionsDisabled"
                @update="taskSplitStore.updateSplitTask"
                @remove="taskSplitStore.removeSplitTask"
                @add="taskSplitStore.addSplitTask"
              />
            </div>
          </div>
        </div>

        <div
          v-if="showPreview"
          class="dialog-footer"
        >
          <div class="footer-actions footer-actions--confirm">
            <button
              v-if="canRetrySplit"
              class="btn btn-secondary btn-retry"
              :class="{ 'btn-retry--pending': isAutoRetryPending }"
              @click="retrySplitTask"
            >
              <span
                v-if="isAutoRetryPending"
                class="btn-retry__pulse"
              />
              {{ retryButtonLabel }}
            </button>
            <button
              v-if="canContinueSplit"
              class="btn btn-secondary btn-continue"
              @click="continueSplitTask"
            >
              {{ t('taskSplit.continueSplit') }}
            </button>
            <button
              class="btn btn-secondary"
              :disabled="isSessionRunning"
              @click="restartSplit"
            >
              {{ t('taskSplit.restart') }}
            </button>
            <button
              class="btn btn-primary"
              :disabled="isConfirming || isSessionRunning || (hasPendingRefinement && !canApplyRefinement)"
              @click="confirmSplit"
            >
              {{ primaryActionLabel }}
            </button>
          </div>
        </div>

        <EaConfirmDialog
          v-model:visible="showCloseConfirm"
          type="warning"
          :title="t('taskSplit.closeConfirmTitle')"
          :message="t('taskSplit.closeConfirmMessage')"
          :confirm-label="t('taskSplit.closeConfirmStop')"
          :cancel-label="t('common.cancel')"
          confirm-button-type="danger"
          @cancel="cancelCloseConfirm"
          @confirm="confirmCloseAndStop"
        />
      </div>
    </div>
  </Teleport>
</template>

<style scoped src="./styles.css"></style>
