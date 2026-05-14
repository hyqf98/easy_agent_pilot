<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import TokenProgressBar from '@/components/common/TokenProgressBar.vue'
import CompressionConfirmDialog from '@/components/common/CompressionConfirmDialog.vue'
import { MessageList } from '@/components/message'
import AiEditTracePane from './AiEditTracePane.vue'
import PanelResizer from '../PanelResizer.vue'
import ConversationComposer from '../conversationComposer/ConversationComposer.vue'
import { useMessageArea } from './useMessageArea'
const { t } = useI18n()
 
const {
  sessionStore,
  showCompressionDialog,
  isCompressing,
  workspaceRef,
  composerRef,
  TRACE_PANE_MIN_WIDTH,
  handleRetry,
  currentTraceState,
  currentTokenUsage,
  currentMessageCount,
  showDesktopTraceHandle,
  showDesktopTracePane,
  showMobileTraceDrawer,
  showMobileTraceButton,
  handleHideTracePane,
  handleShowTracePane,
  handleOpenMobileTrace,
  handleOpenEditTrace,
  handleComposerFocus,
  handleOpenCompress,
  handleConfirmCompress,
  handleCancelCompress,
  handleMessageFormSubmit,
  handleTraceOverlayPointerDown,
  handleTraceOverlayClick,
  getTracePaneMaxWidth,
  handleTracePaneResize,
  handleTracePaneResizeEnd
} = useMessageArea()
</script>

<template>
  <div class="message-area">
    <template v-if="sessionStore.currentSessionId">
      <div
        ref="workspaceRef"
        class="message-area__workspace"
        :class="{ 'message-area__workspace--trace-active': showDesktopTracePane }"
      >
        <div
          v-if="showDesktopTracePane"
          class="message-area__trace-pane"
          :style="{ width: `${currentTraceState?.paneWidth ?? 640}px` }"
        >
          <AiEditTracePane
            :session-id="sessionStore.currentSessionId"
            @close="handleHideTracePane"
          />
        </div>

        <PanelResizer
          v-if="showDesktopTracePane"
          class="message-area__trace-resizer"
          :current-width="currentTraceState?.paneWidth ?? 640"
          :min-width="TRACE_PANE_MIN_WIDTH"
          :max-width="getTracePaneMaxWidth()"
          @resize="handleTracePaneResize"
          @resize-end="handleTracePaneResizeEnd"
        />

        <button
          v-if="showDesktopTraceHandle"
          class="message-area__trace-handle"
          @click="handleShowTracePane"
        >
          <EaIcon
            name="panel-left-open"
            :size="16"
          />
          <span>文件追踪</span>
          <span
            v-if="currentTraceState && currentTraceState.unseenCount > 0"
            class="message-area__trace-handle-badge"
          >
            {{ currentTraceState.unseenCount }}
          </span>
        </button>

        <div
          class="message-area__conversation"
          :class="{ 'message-area__conversation--trace-active': showDesktopTracePane }"
        >
          <!-- Token 进度条，固定在会话面板顶部 -->
          <div
            class="message-area__token-bar"
            :class="`message-area__token-bar--${currentTokenUsage.level}`"
          >
            <div class="message-area__token-peek">
              <div
                class="message-area__token-peek-fill"
                :style="{ width: currentTokenUsage.percentage > 0 && currentTokenUsage.percentage < 1 ? '1%' : `${Math.min(100, currentTokenUsage.percentage)}%` }"
              />
            </div>
            <div class="message-area__token-full">
              <TokenProgressBar
                :session-id="sessionStore.currentSessionId"
                :show-compress-button="true"
                @compress="handleOpenCompress"
              />
            </div>
          </div>

          <MessageList
            class="message-area__list"
            :hide-context-strategy-notice="true"
            :top-safe-inset="56"
            @retry="handleRetry"
            @form-submit="handleMessageFormSubmit"
            @open-edit-trace="handleOpenEditTrace"
          />

          <ConversationComposer
            ref="composerRef"
            :session-id="sessionStore.currentSessionId"
            panel-type="main"
            @focus="handleComposerFocus"
            @open-compress="handleOpenCompress"
          />
        </div>
      </div>

      <Transition name="trace-drawer">
        <div
          v-if="showMobileTraceDrawer"
          class="message-area__trace-drawer-backdrop"
          @pointerdown.capture="handleTraceOverlayPointerDown"
          @click.self="handleTraceOverlayClick"
        >
          <div class="message-area__trace-drawer">
            <AiEditTracePane
              :session-id="sessionStore.currentSessionId"
              mobile
              @close="handleHideTracePane"
            />
          </div>
        </div>
      </Transition>

      <button
        v-if="showMobileTraceButton"
        class="message-area__trace-fab"
        @click="handleOpenMobileTrace"
      >
        <EaIcon
          name="file-code"
          :size="16"
        />
        <span>文件变更</span>
        <span
          v-if="currentTraceState && currentTraceState.unseenCount > 0"
          class="message-area__trace-badge"
        >
          {{ currentTraceState.unseenCount }}
        </span>
      </button>
    </template>

    <!-- 空状态 -->
    <div
      v-else
      class="message-area__empty"
    >
      <EaIcon
        name="message-circle"
        :size="48"
        class="message-area__empty-icon"
      />
      <p class="message-area__empty-text">
        {{ t('message.noSessionSelected') }}
      </p>
      <p class="message-area__empty-hint">
        {{ t('message.startConversation') }}
      </p>
    </div>

    <!-- 压缩确认对话框 -->
    <CompressionConfirmDialog
      v-model:visible="showCompressionDialog"
      :token-usage="currentTokenUsage"
      :message-count="currentMessageCount"
      :loading="isCompressing"
      @confirm="handleConfirmCompress"
      @cancel="handleCancelCompress"
    />
  </div>
</template>

<style scoped src="./styles.css"></style>
