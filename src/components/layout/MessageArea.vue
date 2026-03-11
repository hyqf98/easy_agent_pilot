<script setup lang="ts">
import { nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import { MessageList } from '@/components/message'
import CompressionConfirmDialog from '@/components/common/CompressionConfirmDialog.vue'
import TokenProgressBar from '@/components/common/TokenProgressBar.vue'
import type { Message } from '@/stores/message'
import { useSessionStore } from '@/stores/session'
import { useMessageStore } from '@/stores/message'
import { useMessageComposer } from '@/composables'
import FileMentionDropdown from './FileMentionDropdown.vue'

const { t } = useI18n()
const sessionStore = useSessionStore()
const messageStore = useMessageStore()
const {
  agentDropdownRef,
  agentOptions,
  closeFileMention,
  currentAgent,
  currentAgentId,
  currentAgentName,
  fileMentionPosition,
  getModelLabel,
  handleCancelCompress,
  handleConfirmCompress,
  handleFileSelect,
  handleInput,
  handleKeyDown,
  handleMessageFormSubmit,
  handleOpenCompress,
  handleSend,
  inputPlaceholder,
  inputText,
  isAgentDropdownOpen,
  isCompressing,
  isModelDropdownOpen,
  isSending,
  mentionSearchText,
  mentionStart,
  modelDropdownRef,
  parsedInputText,
  presetModelOptions,
  renderLayerRef,
  selectedModelId,
  selectAgent,
  selectModel,
  shouldShowCompressButton,
  showCompressionDialog,
  showFileMention,
  syncScroll,
  textareaRef,
  toggleAgentDropdown,
  toggleModelDropdown,
  tokenUsage,
  messageCount
} = useMessageComposer()

// 处理消息重试
const handleRetry = async (message: Message) => {
  const sessionId = sessionStore.currentSessionId
  if (!sessionId || isSending.value) return

  // 如果是用户消息的重试，将内容填回输入框
  if (message.role === 'user') {
    inputText.value = message.content
    await nextTick()
    textareaRef.value?.focus()
    return
  }

  // 如果是 AI 消息的重试，找到对应的用户消息并重新发送
  if (message.role === 'assistant') {
    // 获取当前会话的所有消息
    const messages = messageStore.messagesBySession(sessionId)
    const messageIndex = messages.findIndex(m => m.id === message.id)

    // 找到这条 AI 消息之前的用户消息
    for (let i = messageIndex - 1; i >= 0; i--) {
      if (messages[i].role === 'user') {
        // 删除 AI 消息
        await messageStore.deleteMessage(message.id)
        // 将用户消息内容填入输入框并重新发送
        inputText.value = messages[i].content
        // 删除用户消息
        await messageStore.deleteMessage(messages[i].id)
        // 重新发送
        await handleSend()
        return
      }
    }
  }
}

</script>

<template>
  <div class="message-area">
    <!-- 消息列表 -->
    <MessageList
      v-if="sessionStore.currentSessionId"
      class="message-area__list"
      @retry="handleRetry"
      @form-submit="handleMessageFormSubmit"
    />

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

    <!-- 底部输入区域 -->
    <div
      v-if="sessionStore.currentSessionId"
      class="message-area__bottom"
    >
      <!-- Todo 待办 + Token 进度条（同一行） -->
      <div class="bottom-status-bar">
        <div class="bottom-status-bar__left" />
        <div class="bottom-status-bar__center">
          <TokenProgressBar
            :show-compress-button="true"
            @compress="handleOpenCompress"
          />
        </div>
        <div class="bottom-status-bar__right" />
      </div>

      <!-- 输入框容器 -->
      <div class="message-input">
        <!-- 顶部工具栏：智能体选择器 + MCP工具选择器 -->
        <div class="message-input__toolbar message-input__toolbar--top">
          <!-- 智能体选择器 -->
          <div
            ref="agentDropdownRef"
            class="input-chip"
            :class="{ 'input-chip--open': isAgentDropdownOpen }"
          >
            <button
              class="input-chip__btn"
              @click="toggleAgentDropdown"
            >
              <EaIcon
                :name="currentAgent?.type === 'cli' ? 'terminal' : 'code'"
                :size="12"
              />
              <span>{{ currentAgentName }}</span>
              <EaIcon
                :name="isAgentDropdownOpen ? 'chevron-up' : 'chevron-down'"
                :size="10"
              />
            </button>
            <Transition name="dropdown">
              <div
                v-if="isAgentDropdownOpen"
                class="input-chip__menu"
              >
                <div
                  v-for="option in agentOptions"
                  :key="option.value"
                  class="input-chip__option"
                  :class="{ 'input-chip__option--selected': option.value === currentAgentId }"
                  @click="selectAgent(option.value)"
                >
                  <EaIcon
                    :name="option.type === 'cli' ? 'terminal' : 'code'"
                    :size="12"
                  />
                  <span>{{ option.label }}</span>
                  <span class="input-chip__tag">{{ option.type === 'cli' ? 'CLI' : 'SDK' }}</span>
                </div>
                <div
                  v-if="agentOptions.length === 0"
                  class="input-chip__empty"
                >
                  {{ t('settings.agentConfig.noAgents') }}
                </div>
              </div>
            </Transition>
          </div>

        </div>

        <!-- 输入框容器 -->
        <div class="message-input__editor">
          <!-- 渲染层 - 显示带样式的文件标签 -->
          <div ref="renderLayerRef" class="message-input__render">
            <template v-if="parsedInputText.length > 0">
              <template v-for="(segment, index) in parsedInputText" :key="index">
                <span v-if="segment.type === 'text'" class="message-input__text">{{ segment.content }}</span>
                <span v-else class="message-input__file-tag" :title="segment.fullPath">@{{ segment.fullPath }}</span>
              </template>
            </template>
            <span v-else-if="!inputText" class="message-input__placeholder">{{ inputPlaceholder }}</span>
          </div>
          <!-- 输入层 - 透明的 textarea -->
          <textarea
            ref="textareaRef"
            v-model="inputText"
            class="message-input__textarea"
            rows="4"
            :disabled="!sessionStore.currentSessionId || isSending"
            @input="handleInput"
            @keydown="handleKeyDown"
            @scroll="syncScroll"
          />
        </div>

        <!-- 底部工具栏：模型选择器 -->
        <div class="message-input__toolbar message-input__toolbar--bottom">
          <!-- 压缩按钮 -->
          <button
            v-if="shouldShowCompressButton"
            class="input-chip__btn input-chip__btn--compress"
            :disabled="isCompressing || isSending"
            @click="handleOpenCompress"
          >
            <EaIcon
              name="archive"
              :size="12"
              :class="{ 'input-chip__icon--loading': isCompressing }"
            />
            <span>{{ isCompressing ? t('compression.processing') : t('token.compress') }}</span>
          </button>

          <div
            v-if="currentAgent"
            ref="modelDropdownRef"
            class="input-chip"
            :class="{ 'input-chip--open': isModelDropdownOpen }"
          >
            <button
              class="input-chip__btn"
              @click="toggleModelDropdown"
            >
              <EaIcon
                name="cpu"
                :size="12"
              />
              <span>{{ getModelLabel(selectedModelId) }}</span>
              <EaIcon
                :name="isModelDropdownOpen ? 'chevron-up' : 'chevron-down'"
                :size="10"
              />
            </button>
            <Transition name="dropdown">
              <div
                v-if="isModelDropdownOpen"
                class="input-chip__menu input-chip__menu--right"
              >
                <div
                  v-for="model in presetModelOptions"
                  :key="model.value"
                  class="input-chip__option"
                  :class="{ 'input-chip__option--selected': model.value === selectedModelId }"
                  @click="selectModel(model.value)"
                >
                  {{ model.label }}
                </div>
                <div
                  v-if="presetModelOptions.length === 0"
                  class="input-chip__empty"
                >
                  {{ t('settings.agent.selectModel') }}
                </div>
              </div>
            </Transition>
          </div>
        </div>
      </div>
    </div>

    <!-- 压缩确认对话框 -->
    <CompressionConfirmDialog
      v-model:visible="showCompressionDialog"
      :token-usage="tokenUsage"
      :message-count="messageCount"
      :loading="isCompressing"
      @confirm="handleConfirmCompress"
      @cancel="handleCancelCompress"
    />

    <!-- 文件引用选择器 -->
    <FileMentionDropdown
      :visible="showFileMention"
      :position="fileMentionPosition"
      :search-text="mentionSearchText"
      :mention-start="mentionStart"
      @select="handleFileSelect"
      @close="closeFileMention"
    />
  </div>
</template>

<style scoped>
.message-area {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background-color: var(--color-bg-primary);
}

/* 消息列表区域 - 可滚动 */
.message-area__list {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

/* 空状态 */
.message-area__empty {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
}

/* 底部区域 */
.message-area__bottom {
  flex-shrink: 0;
  background-color: var(--color-bg-primary);
}

/* 底部状态栏：Todo + 进度条 同一行 */
.bottom-status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-1) var(--spacing-4);
  min-height: 28px;
}

.bottom-status-bar__left {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: flex-start;
}

.bottom-status-bar__center {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.bottom-status-bar__right {
  flex: 1;
}

.message-area__empty-icon {
  color: var(--color-text-tertiary);
  margin-bottom: var(--spacing-4);
}

.message-area__empty-text {
  font-size: var(--font-size-lg);
  color: var(--color-text-secondary);
  margin-bottom: var(--spacing-2);
}

.message-area__empty-hint {
  font-size: var(--font-size-sm);
  color: var(--color-text-tertiary);
}

/* 输入区域 */
.message-input {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  padding-bottom: calc(var(--spacing-2) + env(safe-area-inset-bottom, 0px));
  margin: var(--spacing-2) var(--spacing-4);
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  transition: all var(--transition-fast) var(--easing-default);
}

.message-input:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-light);
}

/* 工具栏 */
.message-input__toolbar {
  display: flex;
  align-items: center;
  min-height: 24px;
}

.message-input__toolbar--top {
  justify-content: flex-start;
  gap: var(--spacing-2);
}

.message-input__toolbar--bottom {
  justify-content: flex-end;
}

/* 输入框编辑器容器 */
.message-input__editor {
  position: relative;
  flex: 1;
  min-height: calc(1.5em * 4); /* 4 行 */
  max-height: calc(1.5em * 6); /* 6 行 */
}

/* 渲染层 - 显示带样式的文件标签 */
.message-input__render {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  padding: var(--spacing-1) 0;
  font-size: var(--font-size-sm);
  font-family: inherit;
  line-height: 1.5;
  color: var(--color-text-primary);
  white-space: pre-wrap;
  word-wrap: break-word;
  overflow-y: auto;
  overflow-x: hidden;
  pointer-events: none;
  scrollbar-width: none;
}

.message-input__render::-webkit-scrollbar {
  display: none;
}

.message-input__placeholder {
  color: var(--color-text-tertiary);
  font-style: italic;
  opacity: 0.7;
  transition: opacity var(--transition-fast) var(--easing-default);
}

.message-input__editor:focus-within .message-input__placeholder {
  opacity: 0;
}

/* 文件标签样式 - 保持字符宽度基本一致 */
.message-input__file-tag {
  color: var(--color-primary);
  font-weight: 500;
  background-color: color-mix(in srgb, var(--color-primary) 10%, transparent);
  border-radius: 2px;
}

.message-input__text {
  white-space: pre-wrap;
}

.message-input__textarea {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  width: 100%;
  height: 100%;
  padding: var(--spacing-1) 0;
  background: none;
  border: none;
  font-size: var(--font-size-sm);
  font-family: inherit;
  line-height: 1.5;
  color: transparent;
  -webkit-text-fill-color: transparent;
  text-shadow: none;
  caret-color: var(--color-primary);
  resize: none;
  outline: none;
  overflow-y: auto;
}

.message-input__textarea::selection {
  background: var(--color-primary-light);
}

.message-input__textarea::-moz-selection {
  background: var(--color-primary-light);
}

.message-input__textarea:focus {
  outline: none;
  border: none;
  box-shadow: none;
}

.message-input__textarea:disabled {
  cursor: not-allowed;
}

.message-input__editor:has(.message-input__textarea:disabled) .message-input__render {
  opacity: 0.6;
}

/* 小芯片选择器 */
.input-chip {
  position: relative;
  flex-shrink: 0;
}

.input-chip__btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  background-color: var(--color-bg-tertiary);
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
  white-space: nowrap;
  max-width: 120px;
}

.input-chip__btn:hover {
  background-color: var(--color-surface-hover);
}

/* 压缩按钮样式 */
.input-chip__btn--compress {
  background-color: var(--color-warning-light);
}

.input-chip__btn--compress span {
  color: var(--color-warning-dark);
}

.input-chip__btn--compress svg {
  color: var(--color-warning);
}

.input-chip__btn--compress:hover:not(:disabled) {
  background-color: var(--color-warning);
}

.input-chip__btn--compress:hover:not(:disabled) span,
.input-chip__btn--compress:hover:not(:disabled) svg {
  color: white;
}

.input-chip__btn--compress:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.input-chip__icon--loading {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.input-chip--open .input-chip__btn {
  background-color: var(--color-primary-light);
}

.input-chip__btn span {
  font-size: 11px;
  font-weight: 500;
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.input-chip__btn svg {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.input-chip--open .input-chip__btn span,
.input-chip--open .input-chip__btn svg {
  color: var(--color-primary);
}

.input-chip__menu {
  position: absolute;
  bottom: calc(100% + 6px);
  left: 0;
  min-width: 180px;
  max-height: 280px;
  overflow-y: auto;
  background-color: var(--color-surface-elevated);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  z-index: 1000;
  padding: var(--spacing-1);
}

.input-chip__menu--right {
  left: auto;
  right: 0;
}

.input-chip__option {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  font-size: 13px;
  color: var(--color-text-primary);
  cursor: pointer;
  border-radius: var(--radius-md);
  transition: all var(--transition-fast) var(--easing-default);
}

.input-chip__option svg {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.input-chip__option:hover {
  background-color: var(--color-surface-hover);
}

.input-chip__option--selected {
  background-color: var(--color-primary-light);
}

.input-chip__option--selected span {
  color: var(--color-primary);
  font-weight: 500;
}

.input-chip__option--selected svg {
  color: var(--color-primary);
}

.input-chip__tag {
  margin-left: auto;
  font-size: 10px;
  font-weight: 600;
  padding: 2px 6px;
  background-color: var(--color-primary-light);
  color: var(--color-primary);
  border-radius: var(--radius-sm);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.input-chip__empty {
  padding: var(--spacing-3);
  font-size: 13px;
  color: var(--color-text-tertiary);
  text-align: center;
}

/* 下拉框动画 */
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all var(--transition-fast) var(--easing-default);
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
