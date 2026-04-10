<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import TaskSplitPreview from '../TaskSplitPreview.vue'
import ExecutionTimeline from '@/components/message/ExecutionTimeline.vue'
import { useTaskSplitDialog } from './useTaskSplitDialog'
const { t } = useI18n()
 
const {
  planStore,
  taskSplitStore,
  isDarkTheme,
  isConfirming,
  messagesContainerRef,
  userInstruction,
  instructionInputRef,
  showMentionSuggestions,
  mentionSuggestions,
  selectedMentionOptionIndex,
  showPreview,
  refinementMode,
  hasPendingRefinement,
  isSubSplitActive,
  subSplitTargetTitle,
  previewActionsDisabled,
  canApplyRefinement,
  isSessionRunning,
  showStopButton,
  showLoadingIndicator,
  canRetrySplit,
  canContinueSplit,
  retryActionLabel,
  primaryActionLabel,
  footerHint,
  runningStatusText,
  timelineEntries,
  restartSplit,
  handleTimelineFormSubmit,
  confirmSplit,
  closeDialog,
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
          <button
            class="btn-close"
            @click="closeDialog"
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

        <div class="dialog-body">
          <div class="split-content">
            <div class="conversation-pane">
              <div
                ref="messagesContainerRef"
                class="messages-container"
              >
                <ExecutionTimeline
                  :entries="timelineEntries"
                  group-tool-calls
                  :form-cancel-text="t('taskSplit.hide')"
                  @form-submit="handleTimelineFormSubmit"
                  @form-cancel="closeDialog"
                  @message-form-submit="(_formId, values) => handleTimelineFormSubmit('', values)"
                />

                <div
                  v-if="showLoadingIndicator"
                  class="message assistant"
                >
                  <div class="message-content loading">
                    <span class="dot" />
                    <span class="dot" />
                    <span class="dot" />
                  </div>
                  <div class="message-loading-status">
                    {{ runningStatusText }}
                  </div>
                </div>
              </div>

              <div
                v-if="showPreview"
                class="conversation-input-area"
              >
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
                      :disabled="isSessionRunning || isConfirming"
                      :placeholder="t('taskSplit.instructionPlaceholder')"
                      rows="1"
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

        <div class="dialog-footer">
          <template v-if="showPreview">
            <div class="footer-actions footer-actions--confirm">
              <button
                v-if="hasPendingRefinement"
                class="btn btn-secondary"
                @click="closeDialog"
              >
                {{ t(refinementMode === 'list_optimize' ? 'taskSplit.discardOptimize' : 'taskSplit.discardResplit') }}
              </button>
              <button
                v-if="showStopButton"
                class="btn btn-danger"
                @click="stopSplitTask"
              >
                {{ t('taskSplit.stopTask') }}
              </button>
              <button
                v-if="canRetrySplit"
                class="btn btn-secondary btn-retry"
                @click="retrySplitTask"
              >
                {{ retryActionLabel }}
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
                :disabled="isConfirming || isSessionRunning"
                @click="closeDialog"
              >
                {{ t('taskSplit.close') }}
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
          </template>

          <template v-else>
            <div class="footer-bar">
              <span
                class="idle-hint"
                :class="{ 'idle-hint--error': canRetrySplit }"
              >
                {{ footerHint }}
              </span>
              <div class="footer-actions">
                <button
                  v-if="canRetrySplit"
                  class="btn btn-secondary btn-retry"
                  @click="retrySplitTask"
                >
                  {{ retryActionLabel }}
                </button>
                <button
                  v-if="canContinueSplit"
                  class="btn btn-secondary btn-continue"
                  @click="continueSplitTask"
                >
                  {{ t('taskSplit.continueSplit') }}
                </button>
                <button
                  v-if="showStopButton"
                  class="btn btn-danger"
                  @click="stopSplitTask"
                >
                  {{ t('taskSplit.stopTask') }}
                </button>
                <button
                  class="btn btn-secondary"
                  @click="closeDialog"
                >
                  {{ t('taskSplit.hide') }}
                </button>
              </div>
            </div>
          </template>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped src="./styles.css"></style>
