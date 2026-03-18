<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { EaIcon } from '@/components/common'
import CompressionConfirmDialog from '@/components/common/CompressionConfirmDialog.vue'
import { useConversationComposer } from '@/composables/useConversationComposer'
import type { SlashCommandPanelType } from '@/services/slashCommands'
import FileMentionDropdown from './FileMentionDropdown.vue'
import SlashCommandDropdown from './SlashCommandDropdown.vue'

const props = withDefaults(defineProps<{
  panelType: SlashCommandPanelType
  sessionId?: string | null
  workingDirectory?: string | null
  setWorkingDirectory?: (path: string) => Promise<string>
  defaultFileMentionScope?: 'project' | 'global'
  compact?: boolean
  showWorkingDirectory?: boolean
  hideStatusBar?: boolean
}>(), {
  sessionId: null,
  workingDirectory: null,
  setWorkingDirectory: undefined,
  defaultFileMentionScope: 'project',
  compact: false,
  showWorkingDirectory: false,
  hideStatusBar: false
})

const emit = defineEmits<{
  focus: []
}>()

const rootRef = ref<HTMLElement | null>(null)
const isDragOver = ref(false)
let unlistenDragDrop: (() => void) | null = null
const isMainPanel = computed(() => props.panelType === 'main')
const isMiniPanel = computed(() => props.panelType === 'mini')

const {
  agentDropdownRef,
  agentOptions,
  buildQueuedMessagePreview,
  closeFileMention,
  closeSlashCommand,
  currentAgent,
  currentAgentId,
  currentAgentName,
  currentProjectPath,
  currentWorkingDirectory,
  fileInputRef,
  fileMentionPosition,
  focusInput,
  getModelLabel,
  handleCancelCompress,
  handleConfirmCompress,
  handleFileSelect,
  handleImageFileChange,
  handleInput,
  handleKeyDown,
  handleMessageFormSubmit,
  handleOpenCompress,
  handlePaste,
  handleSlashCommandSelect,
  inputPlaceholder,
  inputText,
  insertFileMentions,
  isAgentDropdownOpen,
  isCompressing,
  isModelDropdownOpen,
  isSending,
  isUploadingImages,
  mentionSearchText,
  mentionStart,
  messageCount,
  modelDropdownRef,
  openImagePicker,
  parsedInputText,
  pendingImages,
  presetModelOptions,
  queuedMessages,
  removeImage,
  removeQueuedMessage,
  renderLayerRef,
  retryQueuedMessage,
  selectedModelId,
  selectAgent,
  selectModel,
  shouldShowCompressButton,
  showCompressionDialog,
  showFileMention,
  showSlashCommand,
  slashCommandPosition,
  slashCommandQuery,
  slashCommands,
  syncScroll,
  textareaRef,
  toggleAgentDropdown,
  toggleModelDropdown,
  tokenUsage
} = useConversationComposer({
  panelType: props.panelType,
  sessionId: computed(() => props.sessionId),
  projectPath: computed(() => props.workingDirectory || null),
  workingDirectory: computed(() => props.workingDirectory || null),
  setWorkingDirectory: props.setWorkingDirectory
})

function isWithinComposer(position: { x: number, y: number }) {
  if (!rootRef.value) {
    return false
  }

  const rect = rootRef.value.getBoundingClientRect()
  const x = position.x / window.devicePixelRatio
  const y = position.y / window.devicePixelRatio
  return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
}

onMounted(async () => {
  const appWindow = getCurrentWindow()
  unlistenDragDrop = await appWindow.onDragDropEvent((event) => {
    if (event.payload.type === 'leave') {
      isDragOver.value = false
      return
    }

    const inside = isWithinComposer(event.payload.position)
    isDragOver.value = inside

    if (inside && event.payload.type === 'drop') {
      insertFileMentions(event.payload.paths)
      focusInput()
      isDragOver.value = false
    }
  })
})

onUnmounted(() => {
  unlistenDragDrop?.()
})

defineExpose({
  focusInput,
  handleMessageFormSubmit,
  openCompressionDialog: handleOpenCompress
})
</script>

<template>
  <div
    ref="rootRef"
    class="conversation-composer"
    :class="{
      'conversation-composer--main': isMainPanel,
      'conversation-composer--mini': isMiniPanel,
      'conversation-composer--compact': compact,
      'conversation-composer--drag-over': isDragOver
    }"
  >
    <div
      v-if="hideStatusBar && showWorkingDirectory && currentWorkingDirectory"
      class="conversation-composer__path-row"
    >
      <div
        class="conversation-composer__path"
        :title="currentWorkingDirectory"
      >
        <EaIcon
          name="folder-open"
          :size="12"
        />
        <span>{{ currentWorkingDirectory }}</span>
      </div>
    </div>

    <div
      v-if="!isMainPanel && !hideStatusBar"
      class="conversation-composer__status"
    >
      <div class="conversation-composer__status-left">
        <div
          v-if="queuedMessages.length > 0"
          class="conversation-composer__queue-pill"
        >
          <EaIcon
            name="clock-3"
            :size="12"
          />
          <span>{{ queuedMessages.length }}</span>
        </div>

        <div
          v-if="showWorkingDirectory && currentWorkingDirectory"
          class="conversation-composer__path"
          :title="currentWorkingDirectory"
        >
          <EaIcon
            name="folder-open"
            :size="12"
          />
          <span>{{ currentWorkingDirectory }}</span>
        </div>
      </div>

      <div class="conversation-composer__status-right">
        <button
          v-if="shouldShowCompressButton"
          class="composer-chip composer-chip--compress"
          :disabled="isCompressing || isSending"
          @click="handleOpenCompress"
        >
          <EaIcon
            name="archive"
            :size="12"
          />
          <span>{{ isCompressing ? '压缩中' : '压缩' }}</span>
        </button>

        <div
          ref="agentDropdownRef"
          class="composer-chip composer-chip--dropdown"
          :class="{ 'composer-chip--open': isAgentDropdownOpen }"
        >
          <button
            class="composer-chip__button"
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
              class="composer-chip__menu"
            >
              <div
                v-for="option in agentOptions"
                :key="option.value"
                class="composer-chip__option"
                :class="{ 'composer-chip__option--selected': option.value === currentAgentId }"
                @click="selectAgent(option.value)"
              >
                <EaIcon
                  :name="option.type === 'cli' ? 'terminal' : 'code'"
                  :size="12"
                />
                <span>{{ option.label }}</span>
                <span class="composer-chip__tag">{{ option.type === 'cli' ? 'CLI' : 'SDK' }}</span>
              </div>
            </div>
          </Transition>
        </div>
      </div>
    </div>

    <div class="conversation-composer__panel">
      <div
        v-if="isMainPanel"
        class="conversation-composer__main-header"
      >
        <div class="conversation-composer__main-header-left">
          <div
            ref="agentDropdownRef"
            class="composer-chip composer-chip--dropdown"
            :class="{
              'composer-chip--main': isMainPanel,
              'composer-chip--open': isAgentDropdownOpen
            }"
          >
            <button
              class="composer-chip__button"
              @click="toggleAgentDropdown"
            >
              <EaIcon
                :name="currentAgent?.type === 'cli' ? 'terminal' : 'code'"
                :size="11"
              />
              <span>{{ currentAgentName }}</span>
              <EaIcon
                :name="isAgentDropdownOpen ? 'chevron-up' : 'chevron-down'"
                :size="9"
              />
            </button>
            <Transition name="dropdown">
              <div
                v-if="isAgentDropdownOpen"
                class="composer-chip__menu"
              >
                <div
                  v-for="option in agentOptions"
                  :key="option.value"
                  class="composer-chip__option"
                  :class="{ 'composer-chip__option--selected': option.value === currentAgentId }"
                  @click="selectAgent(option.value)"
                >
                  <EaIcon
                    :name="option.type === 'cli' ? 'terminal' : 'code'"
                    :size="12"
                  />
                  <span>{{ option.label }}</span>
                  <span class="composer-chip__tag">{{ option.type === 'cli' ? 'CLI' : 'SDK' }}</span>
                </div>
              </div>
            </Transition>
          </div>

          <!-- 模型选择器 - 移到智能体旁边 -->
          <div
            v-if="currentAgent"
            ref="modelDropdownRef"
            class="composer-chip composer-chip--dropdown"
            :class="{
              'composer-chip--main': isMainPanel,
              'composer-chip--open': isModelDropdownOpen
            }"
          >
            <button
              class="composer-chip__button"
              @click="toggleModelDropdown"
            >
              <EaIcon
                name="cpu"
                :size="11"
              />
              <span>{{ getModelLabel(selectedModelId) }}</span>
              <EaIcon
                :name="isModelDropdownOpen ? 'chevron-up' : 'chevron-down'"
                :size="9"
              />
            </button>
            <Transition name="dropdown">
              <div
                v-if="isModelDropdownOpen"
                class="composer-chip__menu"
              >
                <div
                  v-for="model in presetModelOptions"
                  :key="model.value"
                  class="composer-chip__option"
                  :class="{ 'composer-chip__option--selected': model.value === selectedModelId }"
                  @click="selectModel(model.value)"
                >
                  {{ model.label }}
                </div>
              </div>
            </Transition>
          </div>

          <!-- 图片按钮移到顶部 -->
          <button
            class="composer-chip composer-chip--image"
            :class="{ 'composer-chip--main': isMainPanel }"
            :disabled="isUploadingImages"
            @click="openImagePicker"
          >
            <EaIcon
              name="image-up"
              :size="12"
            />
            <span>{{ isUploadingImages ? '上传中…' : '图片' }}</span>
          </button>

          <div
            v-if="queuedMessages.length > 0"
            class="conversation-composer__queue-pill conversation-composer__queue-pill--main"
          >
            <EaIcon
              name="clock-3"
              :size="12"
            />
            <span>待发送 {{ queuedMessages.length }}</span>
          </div>
        </div>
      </div>

      <input
        ref="fileInputRef"
        type="file"
        class="conversation-composer__file-input"
        accept="image/*"
        multiple
        @change="handleImageFileChange"
      >

      <div
        v-if="pendingImages.length > 0"
        class="conversation-composer__attachments"
      >
        <div
          v-for="image in pendingImages"
          :key="image.id"
          class="conversation-composer__attachment"
        >
          <img
            :src="image.previewUrl"
            :alt="image.name"
            class="conversation-composer__attachment-image"
          >
          <button
            class="conversation-composer__attachment-remove"
            @click="removeImage(image.id)"
          >
            <EaIcon
              name="x"
              :size="12"
            />
          </button>
        </div>
      </div>

      <div
        v-if="queuedMessages.length > 0"
        class="conversation-composer__queue"
      >
        <div
          v-for="(draft, index) in queuedMessages"
          :key="draft.id"
          class="conversation-composer__queue-item"
        >
          <div class="conversation-composer__queue-index">
            {{ index + 1 }}
          </div>
          <div class="conversation-composer__queue-body">
            <div class="conversation-composer__queue-top">
              <span>{{ draft.status === 'failed' ? '失败' : '待发送' }}</span>
              <span v-if="draft.attachments.length > 0">{{ draft.attachments.length }} 图</span>
            </div>
            <div class="conversation-composer__queue-preview">
              {{ buildQueuedMessagePreview(draft) || '空消息' }}
            </div>
            <div
              v-if="draft.status === 'failed' && draft.errorMessage"
              class="conversation-composer__queue-error"
            >
              {{ draft.errorMessage }}
            </div>
          </div>
          <div class="conversation-composer__queue-actions">
            <button
              v-if="draft.status === 'failed'"
              class="conversation-composer__queue-action"
              @click="retryQueuedMessage(draft.id)"
            >
              <EaIcon
                name="refresh-cw"
                :size="12"
              />
            </button>
            <button
              class="conversation-composer__queue-action"
              @click="removeQueuedMessage(draft.id)"
            >
              <EaIcon
                name="x"
                :size="12"
              />
            </button>
          </div>
        </div>
      </div>

      <div
        class="conversation-composer__editor-shell"
        @contextmenu.prevent
      >
        <div
          ref="renderLayerRef"
          class="conversation-composer__render"
        >
          <template v-if="parsedInputText.length > 0">
            <template
              v-for="(segment, index) in parsedInputText"
              :key="index"
            >
              <span
                v-if="segment.type === 'text'"
                class="conversation-composer__text"
              >{{ segment.content }}</span>
              <span
                v-else
                class="conversation-composer__file-tag"
                :title="segment.titleContent"
              >{{ segment.displayContent || segment.content }}</span>
            </template>
          </template>
          <span
            v-else
            class="conversation-composer__placeholder"
          >
            {{ inputPlaceholder || '输入消息，按 Enter 发送' }}
          </span>
          <span
            v-if="isMainPanel && !inputText"
            class="conversation-composer__hint"
          >
            按 Enter 发送，Shift+Enter 换行
          </span>
        </div>

        <textarea
          ref="textareaRef"
          v-model="inputText"
          class="conversation-composer__textarea"
          rows="4"
          :disabled="!sessionId"
          @input="handleInput"
          @keydown="handleKeyDown"
          @paste="handlePaste"
          @scroll="syncScroll"
          @focus="emit('focus')"
        />
      </div>
    </div>

    <CompressionConfirmDialog
      v-model:visible="showCompressionDialog"
      :token-usage="tokenUsage"
      :message-count="messageCount"
      :loading="isCompressing"
      @confirm="handleConfirmCompress"
      @cancel="handleCancelCompress"
    />

    <FileMentionDropdown
      :visible="showFileMention"
      :position="fileMentionPosition"
      :search-text="mentionSearchText"
      :mention-start="mentionStart"
      :project-path="workingDirectory || currentProjectPath || undefined"
      :default-scope="defaultFileMentionScope"
      @select="handleFileSelect"
      @close="closeFileMention"
    />

    <SlashCommandDropdown
      :visible="showSlashCommand"
      :position="slashCommandPosition"
      :query="slashCommandQuery"
      :commands="slashCommands"
      @select="handleSlashCommandSelect"
      @close="closeSlashCommand"
    />
  </div>
</template>

<style scoped>
.conversation-composer {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  border-top: none;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.94), rgba(248, 250, 252, 0.98));
  position: relative;
}

.conversation-composer--main {
  gap: 0;
  padding: 10px 16px 20px;
  border-top: none;
  background: var(--color-bg-primary);
}

.conversation-composer--compact {
  padding: 12px;
  gap: 8px;
}

.conversation-composer--drag-over::after {
  content: '拖拽文件到这里即可插入路径引用';
  position: absolute;
  inset: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1.5px dashed color-mix(in srgb, var(--color-primary) 62%, transparent);
  border-radius: 18px;
  background: color-mix(in srgb, var(--color-primary-light) 42%, white);
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  backdrop-filter: blur(8px);
  z-index: 2;
}

.conversation-composer__status,
.conversation-composer__toolbar,
.conversation-composer__status-left,
.conversation-composer__status-right,
.conversation-composer__toolbar-left,
.conversation-composer__toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.conversation-composer__panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.conversation-composer__status,
.conversation-composer__toolbar {
  justify-content: space-between;
}

.conversation-composer__path-row {
  display: flex;
  align-items: center;
  justify-content: flex-start;
}

.conversation-composer__status-left,
.conversation-composer__toolbar-left {
  flex-wrap: wrap;
}

.conversation-composer__path,
.conversation-composer__queue-pill,
.composer-chip,
.conversation-composer__send {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 32px;
  padding: 0 12px;
  border: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.9);
  color: var(--color-text-primary);
}

.conversation-composer__path {
  max-width: 320px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  font-size: var(--font-size-xs);
}

.conversation-composer__queue-pill {
  font-size: var(--font-size-xs);
}

.conversation-composer__queue-pill--main {
  border: 1px solid rgba(125, 211, 252, 0.36);
  background: rgba(239, 246, 255, 0.9);
  color: #0369a1;
  font-weight: 600;
}

.conversation-composer--main .conversation-composer__panel {
  width: 100%;
  gap: 10px;
  padding: 10px 14px;
  padding-bottom: calc(10px + env(safe-area-inset-bottom, 0px));
  background: var(--color-surface);
  border: none;
  border-radius: 0;
  box-shadow: none;
}

.conversation-composer__main-header {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: 10px;
  padding: 0 0 6px;
  border-bottom: none;
}

.conversation-composer__main-header-left {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.composer-chip {
  position: relative;
  padding: 0;
}

.conversation-composer--main .composer-chip,
.conversation-composer--main .conversation-composer__send {
  min-height: 28px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-surface);
  color: var(--color-text-primary);
  box-shadow: none;
}

.composer-chip__button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 28px;
  padding: 0 10px;
  font-size: 12px;
}

.conversation-composer--main .composer-chip__button {
  min-height: 28px;
  gap: 4px;
  padding: 0 9px;
  font-size: 12px;
  white-space: nowrap;
}

.conversation-composer--main .composer-chip__button > span {
  line-height: 1;
}

.composer-chip--image {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-width: 68px;
  padding: 0 10px;
  font-size: 12px;
  line-height: 1;
  white-space: nowrap;
}

.conversation-composer--main .composer-chip--image {
  min-height: 28px;
}

.composer-chip--image:disabled {
  cursor: wait;
  opacity: 0.72;
}

.conversation-composer--main .composer-chip:hover,
.conversation-composer--main .conversation-composer__send--main:hover {
  border-color: rgba(148, 163, 184, 0.88);
  background: rgba(248, 250, 252, 0.98);
}

.composer-chip__menu {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 0;
  top: auto;
  min-width: 220px;
  padding: 8px;
  border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.98);
  box-shadow: 0 18px 38px rgba(15, 23, 42, 0.16);
  z-index: var(--z-dropdown);
}

.composer-chip__menu--right {
  left: auto;
  right: 0;
}

.composer-chip__option {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 12px;
  cursor: pointer;
}

.composer-chip__option:hover,
.composer-chip__option--selected {
  background: color-mix(in srgb, var(--color-primary-light) 56%, white);
}

.composer-chip__tag {
  margin-left: auto;
  font-size: 11px;
  color: var(--color-text-tertiary);
}

.conversation-composer__file-input {
  display: none;
}

.conversation-composer__attachments,
.conversation-composer__queue {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.conversation-composer--main .conversation-composer__attachments {
  gap: 8px;
  padding: 2px 2px 0;
}

.conversation-composer__attachment {
  position: relative;
  width: 74px;
  height: 74px;
  border-radius: 16px;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
}

.conversation-composer--main .conversation-composer__attachment {
  width: 64px;
  height: 64px;
  border-radius: 16px;
  border: 1px solid rgba(226, 232, 240, 0.94);
  background: rgba(248, 250, 252, 0.94);
  box-shadow: 0 10px 22px rgba(15, 23, 42, 0.06);
}

.conversation-composer__attachment-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.conversation-composer__attachment-remove {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 22px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.72);
  color: white;
}

.conversation-composer--main .conversation-composer__attachment-remove {
  top: 4px;
  right: 4px;
  width: 18px;
  height: 18px;
}

.conversation-composer__queue {
  flex-direction: column;
}

.conversation-composer--main .conversation-composer__queue {
  gap: 8px;
  padding: 2px 2px 0;
}

.conversation-composer__queue-item {
  display: grid;
  grid-template-columns: 28px minmax(0, 1fr) auto;
  gap: 10px;
  align-items: start;
  padding: 10px 12px;
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.84);
  border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
}

.conversation-composer--main .conversation-composer__queue-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  border-radius: 16px;
  border: 1px solid rgba(226, 232, 240, 0.94);
  background: linear-gradient(180deg, rgba(248, 250, 252, 0.98), rgba(255, 255, 255, 0.98));
}

.conversation-composer__queue-body {
  flex: 1;
  min-width: 0;
}

.conversation-composer__queue-index,
.conversation-composer__queue-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.conversation-composer__queue-index {
  width: 28px;
  height: 28px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary-light) 66%, white);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
}

.conversation-composer--main .conversation-composer__queue-index {
  min-width: 24px;
  width: 24px;
  height: 24px;
  background: rgba(226, 232, 240, 0.86);
  color: var(--color-text-secondary);
  font-size: 11px;
  font-weight: 700;
}

.conversation-composer__queue-top,
.conversation-composer__queue-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.conversation-composer__queue-top {
  justify-content: space-between;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  white-space: nowrap;
}

.conversation-composer__queue-preview {
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  display: -webkit-box;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  word-break: break-word;
}

.conversation-composer__queue-error {
  font-size: var(--font-size-xs);
  color: var(--color-danger);
  display: -webkit-box;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  line-clamp: 2;
}

.conversation-composer__queue-action {
  width: 28px;
  height: 28px;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--color-border) 76%, transparent);
}

.conversation-composer--main .conversation-composer__queue-action {
  width: 24px;
  height: 24px;
  border: none;
  background: rgba(241, 245, 249, 0.96);
}

.conversation-composer__editor-shell {
  position: relative;
  min-height: 122px;
  border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
  border-radius: 22px;
  background: rgba(255, 255, 255, 0.96);
  overflow: hidden;
}

.conversation-composer--main .conversation-composer__editor-shell {
  min-height: 160px;
  max-height: 320px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-bg-primary);
  box-shadow: none;
}

/* 移除编辑器激活样式 */
.conversation-composer--main .conversation-composer__editor-shell:focus-within {
  border-color: var(--color-border);
  box-shadow: none;
}

.conversation-composer__render,
.conversation-composer__textarea {
  width: 100%;
  min-height: 122px;
  padding: 16px 18px;
  font-size: var(--font-size-sm);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}

.conversation-composer--main .conversation-composer__render,
.conversation-composer--main .conversation-composer__textarea {
  min-height: 160px;
  padding: 14px 16px;
  font-family: inherit;
  line-height: 1.6;
}

.conversation-composer__render {
  pointer-events: none;
  color: var(--color-text-primary);
}

.conversation-composer__textarea {
  position: absolute;
  inset: 0;
  resize: none;
  background: transparent;
  color: transparent;
  caret-color: var(--color-text-primary);
  -webkit-text-fill-color: transparent;
}

/* 隐藏 textarea 的文本选择背景 */
.conversation-composer__textarea::selection {
  background: transparent;
  color: transparent;
  -webkit-text-fill-color: transparent;
}

.conversation-composer__textarea::-moz-selection {
  background: transparent;
  color: transparent;
}

.conversation-composer__placeholder {
  color: var(--color-text-tertiary);
}

.conversation-composer--main .conversation-composer__placeholder {
  color: var(--color-text-tertiary);
}

.conversation-composer__hint {
  position: absolute;
  bottom: 8px;
  right: 14px;
  font-size: 10px;
  color: var(--color-text-quaternary);
  pointer-events: none;
}

.conversation-composer__inline-note {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.conversation-composer__file-tag {
  display: inline;
  color: var(--color-primary);
  font-weight: var(--font-weight-medium);
  text-decoration: underline;
  text-decoration-color: var(--color-primary-light);
  text-underline-offset: 2px;
}

.conversation-composer__send {
  background: var(--color-primary);
  color: white;
}

.conversation-composer--main .conversation-composer__send--main {
  min-height: 30px;
  padding: 0 14px;
  border: none;
  background: var(--color-primary);
  color: white;
  box-shadow: 0 2px 6px rgba(37, 99, 235, 0.16);
}

.conversation-composer--main .conversation-composer__send--main:hover {
  background: var(--color-primary-hover);
}

.conversation-composer--main .conversation-composer__toolbar--bottom {
  padding: 6px 0 0;
  border-top: none;
}

@media (max-width: 900px) {
  .conversation-composer--main {
    padding: 8px 12px 14px;
  }

  .conversation-composer__main-header {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }

  .conversation-composer__main-header-right {
    flex-basis: auto;
    justify-content: flex-start;
  }

  .conversation-composer__main-progress {
    width: 100%;
  }

  .conversation-composer--main .conversation-composer__toolbar--bottom {
    flex-wrap: wrap;
  }

  .conversation-composer--main .conversation-composer__panel {
    padding: 12px;
    border-radius: 22px;
  }

  .conversation-composer--main .conversation-composer__editor-shell {
    min-height: 140px;
  }

  .conversation-composer--main .conversation-composer__render,
  .conversation-composer--main .conversation-composer__textarea {
    min-height: 140px;
    padding: 16px 16px 18px;
  }
}

@media (max-width: 720px) {
  .conversation-composer__main-header-left {
    flex-wrap: nowrap;
    overflow-x: auto;
    overflow-y: hidden;
    padding-bottom: 2px;
    scrollbar-width: none;
  }

  .conversation-composer__main-header-left::-webkit-scrollbar {
    display: none;
  }

  .conversation-composer--main .composer-chip__button {
    padding: 0 8px;
  }

  .composer-chip--image {
    min-width: 60px;
    padding: 0 8px;
  }

  .conversation-composer--main .conversation-composer__queue-item {
    padding: 9px 10px;
    gap: 8px;
  }
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.14s ease, transform 0.14s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

/* 暗色模式优化 */
[data-theme='dark'] .conversation-composer--main {
  background: var(--color-bg-primary);
}

[data-theme='dark'] .conversation-composer--main .conversation-composer__panel {
  background: var(--color-surface);
  border-color: var(--color-border);
}

[data-theme='dark'] .conversation-composer--main .conversation-composer__editor-shell {
  background: var(--color-bg-secondary);
  border-color: var(--color-border);
}

[data-theme='dark'] .conversation-composer--main .conversation-composer__editor-shell:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(30, 58, 95, 0.5);
}

[data-theme='dark'] .conversation-composer--main .composer-chip,
[data-theme='dark'] .conversation-composer--main .conversation-composer__send {
  background: var(--color-surface);
  border-color: var(--color-border);
}

[data-theme='dark'] .conversation-composer--main .conversation-composer__send--main {
  background: var(--color-primary);
}
</style>
