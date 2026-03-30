<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import CompressionConfirmDialog from '@/components/common/CompressionConfirmDialog.vue'
import { useConversationComposer } from '@/composables/useConversationComposer'
import { ConversationTodoPanel } from '@/components/message'
import { useSettingsStore } from '@/stores/settings'
import { useThemeStore } from '@/stores/theme'
import type { SlashCommandPanelType } from '@/services/slashCommands'
import CdPathDropdown from './CdPathDropdown.vue'
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

const { t } = useI18n()
const settingsStore = useSettingsStore()
const themeStore = useThemeStore()
const rootRef = ref<HTMLElement | null>(null)
const isDragOver = ref(false)
const isQueueCollapsed = ref(true)
const editingQueuedDraftId = ref<string | null>(null)
const queuedDraftEditText = ref('')
let unlistenDragDrop: (() => void) | null = null
const isMainPanel = computed(() => props.panelType === 'main')
const isMiniPanel = computed(() => props.panelType === 'mini')
const isDarkTheme = computed(() => themeStore.isDark)

const {
  agentDropdownRef,
  agentOptions,
  buildQueuedMessagePreview,
  cdPathPosition,
  cdPathQuery,
  closeFileMention,
  closeCdPathSuggestions,
  closeSlashCommand,
  currentAgent,
  currentAgentId,
  currentAgentName,
  currentMemoryPreview,
  currentMemoryReferences,
  currentProjectPath,
  currentWorkingDirectory,
  clearMemoryPreview,
  dismissMemorySuggestion,
  fileInputRef,
  fileMentionPosition,
  focusInput,
  getModelLabel,
  handleCancelCompress,
  handleCdPathSelect,
  handleConfirmCompress,
  handleCompositionEnd,
  handleCompositionStart,
  handleFileSelect,
  handleImageFileChange,
  handleInput,
  handleKeyDown,
  handleMessageFormSubmit,
  handleOpenCompress,
  handlePaste,
  hasVisibleMemorySuggestions,
  insertMemoryReference,
  handleSlashCommandSelect,
  inputPlaceholder,
  inputText,
  insertFileMentions,
  isActiveMemorySuggestion,
  isAgentDropdownOpen,
  isCompressing,
  isMemorySuggestionLoading,
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
  previewMemoryReference,
  previewMemorySuggestion,
  presetModelOptions,
  queuedMessages,
  removeImage,
  removeMemoryReferenceFromDraft,
  removeQueuedMessage,
  renderLayerRef,
  retryMessage,
  retryQueuedMessage,
  selectedModelId,
  selectAgent,
  selectModel,
  shouldShowCompressButton,
  showCompressionDialog,
  showCdPathSuggestions,
  showFileMention,
  showSlashCommand,
  slashCommandPosition,
  slashCommandQuery,
  slashCommands,
  shouldShowMemorySuggestionEmptyState,
  shouldShowMemorySuggestionIdleHint,
  shouldShowMemorySuggestions,
  syncScroll,
  textareaRef,
  toggleAgentDropdown,
  toggleModelDropdown,
  tokenUsage,
  updateQueuedMessage,
  visibleMemorySuggestions
} = useConversationComposer({
  panelType: props.panelType,
  sessionId: computed(() => props.sessionId),
  projectPath: computed(() => props.workingDirectory || null),
  workingDirectory: computed(() => props.workingDirectory || null),
  setWorkingDirectory: props.setWorkingDirectory
})

const shouldUseRichTextOverlay = computed(() => (
  parsedInputText.value.some(segment => segment.type !== 'text')
))

const composerSendShortcutHint = computed(() => (
  settingsStore.settings.sendOnEnter
    ? t('message.shortcutEnter')
    : t('message.shortcutModifierEnter')
))

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

watch(() => props.sessionId, () => {
  isQueueCollapsed.value = true
  editingQueuedDraftId.value = null
  queuedDraftEditText.value = ''
})

const toggleQueueCollapsed = () => {
  isQueueCollapsed.value = !isQueueCollapsed.value
}

const startQueuedMessageEdit = (draftId: string, content: string) => {
  editingQueuedDraftId.value = draftId
  queuedDraftEditText.value = content
}

const cancelQueuedMessageEdit = () => {
  editingQueuedDraftId.value = null
  queuedDraftEditText.value = ''
}

const saveQueuedMessageEdit = (draftId: string) => {
  const normalized = queuedDraftEditText.value.trim()
  if (!normalized) {
    return
  }

  updateQueuedMessage(draftId, {
    content: normalized,
    displayContent: normalized,
    memoryReferences: []
  })
  cancelQueuedMessageEdit()
}

defineExpose({
  focusInput,
  handleMessageFormSubmit,
  retryMessage,
  openCompressionDialog: handleOpenCompress
})
</script>

<template>
  <div
    ref="rootRef"
    class="conversation-composer"
    :data-drag-text="t('message.dropImages')"
    :class="{
      'conversation-composer--main': isMainPanel,
      'conversation-composer--mini': isMiniPanel,
      'conversation-composer--compact': compact,
      'conversation-composer--drag-over': isDragOver,
      'conversation-composer--dark': isDarkTheme
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
          <span>{{ isCompressing ? t('compression.processing') : t('token.compress') }}</span>
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

        <div
          v-if="currentAgent"
          ref="modelDropdownRef"
          class="composer-chip composer-chip--dropdown"
          :class="{ 'composer-chip--open': isModelDropdownOpen }"
        >
          <button
            class="composer-chip__button"
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

        <button
          class="composer-chip composer-chip--image"
          :disabled="isUploadingImages"
          @click="openImagePicker"
        >
          <EaIcon
            name="image-up"
            :size="12"
          />
          <span>{{ isUploadingImages ? t('message.uploadingImages') : t('message.selectImages') }}</span>
        </button>
      </div>
    </div>

    <div class="conversation-composer__panel">
      <ConversationTodoPanel
        v-if="sessionId"
        :session-id="sessionId"
        :default-collapsed="true"
      />

      <div
        v-if="isMainPanel && pendingImages.length > 0"
        class="conversation-composer__attachments conversation-composer__attachments--main"
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

          <!-- 模型选择器，移到智能体旁边 -->
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
            <span>{{ isUploadingImages ? t('message.uploadingImages') : t('message.selectImages') }}</span>
          </button>

          <div
            v-if="queuedMessages.length > 0"
            class="conversation-composer__queue-pill conversation-composer__queue-pill--main"
          >
            <EaIcon
              name="clock-3"
              :size="12"
            />
            <span>{{ t('message.queueCount', { count: queuedMessages.length }) }}</span>
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
        v-if="!isMainPanel && pendingImages.length > 0"
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
        <button
          v-if="isMainPanel"
          type="button"
          class="conversation-composer__queue-head"
          :aria-expanded="!isQueueCollapsed"
          @click="toggleQueueCollapsed"
        >
          <span class="conversation-composer__queue-head-title">
            <EaIcon
              name="clock-3"
              :size="13"
            />
            <span>{{ t('message.queueCount', { count: queuedMessages.length }) }}</span>
          </span>
          <EaIcon
            :name="isQueueCollapsed ? 'chevron-down' : 'chevron-up'"
            :size="14"
          />
        </button>

        <div
          v-for="(draft, index) in queuedMessages"
          v-show="!isMainPanel || !isQueueCollapsed"
          :key="draft.id"
          class="conversation-composer__queue-item"
        >
          <div class="conversation-composer__queue-index">
            {{ index + 1 }}
          </div>
          <div class="conversation-composer__queue-body">
            <div class="conversation-composer__queue-top">
              <span>{{ draft.status === 'failed' ? t('message.pendingFailed') : t('message.pendingLabel') }}</span>
              <span v-if="draft.attachments.length > 0">{{ t('message.queueImages', { count: draft.attachments.length }) }}</span>
            </div>
            <div class="conversation-composer__queue-preview">
              <textarea
                v-if="editingQueuedDraftId === draft.id"
                v-model="queuedDraftEditText"
                class="conversation-composer__queue-editor"
                rows="3"
              />
              <template v-else>
                {{ buildQueuedMessagePreview(draft) || t('message.pendingEmpty') }}
              </template>
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
              v-if="editingQueuedDraftId !== draft.id"
              class="conversation-composer__queue-action"
              @click="startQueuedMessageEdit(draft.id, draft.displayContent || draft.content)"
            >
              <EaIcon
                name="pencil"
                :size="12"
              />
            </button>
            <button
              v-else
              class="conversation-composer__queue-action"
              @click="saveQueuedMessageEdit(draft.id)"
            >
              <EaIcon
                name="check"
                :size="12"
              />
            </button>
            <button
              v-if="editingQueuedDraftId === draft.id"
              class="conversation-composer__queue-action"
              @click="cancelQueuedMessageEdit"
            >
              <EaIcon
                name="x"
                :size="12"
              />
            </button>
            <button
              v-else-if="draft.status === 'failed'"
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
        v-if="isMainPanel && currentMemoryReferences.length > 0"
        class="conversation-composer__memory-tray"
      >
        <div class="conversation-composer__memory-tray-label">
          {{ t('message.memoryReferencesTitle') }}
        </div>
        <div class="conversation-composer__memory-chips">
          <button
            v-for="reference in currentMemoryReferences"
            :key="`${reference.sourceType}:${reference.sourceId}`"
            class="conversation-composer__memory-chip"
            type="button"
            :title="reference.fullContent"
            @mouseenter="previewMemoryReference(reference)"
            @mouseleave="clearMemoryPreview"
            @focus="previewMemoryReference(reference)"
            @blur="clearMemoryPreview"
            @click="removeMemoryReferenceFromDraft(reference)"
          >
            <span class="conversation-composer__memory-chip-type">
              {{ reference.sourceType === 'library_chunk' ? t('message.memorySourceLibrary') : t('message.memorySourceRaw') }}
            </span>
            <span class="conversation-composer__memory-chip-text">{{ reference.title }}</span>
            <EaIcon
              name="x"
              :size="12"
            />
          </button>
        </div>
      </div>

      <div
        v-if="isMainPanel && currentMemoryPreview && !shouldShowMemorySuggestions"
        class="conversation-composer__memory-preview"
        :class="{
          'conversation-composer__memory-preview--library': currentMemoryPreview.sourceType === 'library_chunk',
          'conversation-composer__memory-preview--raw': currentMemoryPreview.sourceType === 'raw_record'
        }"
      >
        <div class="conversation-composer__memory-preview-header">
          <span class="conversation-composer__memory-preview-label">
            {{ t('message.memoryPreviewTitle') }}
          </span>
          <span class="conversation-composer__memory-preview-source">
            {{ currentMemoryPreview.sourceLabel }}
          </span>
        </div>
        <div class="conversation-composer__memory-preview-name">
          {{ currentMemoryPreview.title }}
        </div>
        <pre class="conversation-composer__memory-preview-content">{{ currentMemoryPreview.fullContent }}</pre>
      </div>

      <div
        v-if="isMainPanel && shouldShowMemorySuggestions"
        class="conversation-composer__memory-panel"
      >
        <div class="conversation-composer__memory-panel-header">
          <div>
            <div class="conversation-composer__memory-eyebrow">
              {{ t('message.memorySuggestionEyebrow') }}
            </div>
            <div class="conversation-composer__memory-title">
              {{ hasVisibleMemorySuggestions ? t('message.memorySuggestionTitle') : t('message.memorySearchingActive') }}
            </div>
            <div
              v-if="hasVisibleMemorySuggestions"
              class="conversation-composer__memory-keyboard-hint"
            >
              {{ t('message.memoryKeyboardHint') }}
            </div>
          </div>
          <div
            v-if="isMemorySuggestionLoading"
            class="conversation-composer__memory-loading"
          >
            <span class="conversation-composer__memory-spinner" />
            <span>{{ t('message.memorySearchingActive') }}</span>
          </div>
        </div>

        <div
          v-if="currentMemoryPreview"
          class="conversation-composer__memory-preview"
          :class="{
            'conversation-composer__memory-preview--library': currentMemoryPreview.sourceType === 'library_chunk',
            'conversation-composer__memory-preview--raw': currentMemoryPreview.sourceType === 'raw_record'
          }"
        >
          <div class="conversation-composer__memory-preview-header">
            <span class="conversation-composer__memory-preview-label">
              {{ t('message.memoryPreviewTitle') }}
            </span>
            <span class="conversation-composer__memory-preview-source">
              {{ currentMemoryPreview.sourceLabel }}
            </span>
          </div>
          <div class="conversation-composer__memory-preview-name">
            {{ currentMemoryPreview.title }}
          </div>
          <pre class="conversation-composer__memory-preview-content">{{ currentMemoryPreview.fullContent }}</pre>
        </div>

        <div
          v-if="shouldShowMemorySuggestionEmptyState"
          class="conversation-composer__memory-empty"
        >
          <div class="conversation-composer__memory-empty-title">
            {{ t('message.memoryNoMatches') }}
          </div>
          <div class="conversation-composer__memory-empty-hint">
            {{ t('message.memoryKeepTypingHint') }}
          </div>
        </div>

        <div
          v-else-if="shouldShowMemorySuggestionIdleHint"
          class="conversation-composer__memory-empty conversation-composer__memory-empty--subtle"
        >
          <div class="conversation-composer__memory-empty-hint">
            {{ t('message.memorySearchSettling') }}
          </div>
        </div>

        <div
          v-if="hasVisibleMemorySuggestions && visibleMemorySuggestions.librarySuggestions.length > 0"
          class="conversation-composer__memory-group"
        >
          <div class="conversation-composer__memory-group-title">
            {{ t('message.memorySourceLibrary') }}
          </div>
          <div class="conversation-composer__memory-list">
            <article
              v-for="suggestion in visibleMemorySuggestions.librarySuggestions"
              :key="`${suggestion.sourceType}:${suggestion.sourceId}`"
              class="conversation-composer__memory-card conversation-composer__memory-card--library"
              :class="{
                'conversation-composer__memory-card--active': isActiveMemorySuggestion(suggestion)
              }"
              role="option"
              :aria-selected="isActiveMemorySuggestion(suggestion)"
              :title="suggestion.fullContent"
              @mouseenter="previewMemorySuggestion(suggestion)"
              @mouseleave="clearMemoryPreview"
            >
              <div class="conversation-composer__memory-card-body">
                <div class="conversation-composer__memory-card-top">
                  <span class="conversation-composer__memory-badge">
                    {{ t('message.memorySourceLibrary') }}
                  </span>
                  <span class="conversation-composer__memory-card-title">{{ suggestion.title }}</span>
                </div>
                <p class="conversation-composer__memory-card-snippet">
                  {{ suggestion.snippet || suggestion.fullContent }}
                </p>
              </div>
              <div class="conversation-composer__memory-card-actions">
                <button
                  class="conversation-composer__memory-action conversation-composer__memory-action--ghost"
                  type="button"
                  @click="dismissMemorySuggestion(suggestion)"
                >
                  {{ t('message.memoryDismiss') }}
                </button>
                <button
                  class="conversation-composer__memory-action conversation-composer__memory-action--primary"
                  type="button"
                  @click="insertMemoryReference(suggestion)"
                >
                  {{ t('message.memoryInsert') }}
                </button>
              </div>
            </article>
          </div>
        </div>

        <div
          v-if="hasVisibleMemorySuggestions && visibleMemorySuggestions.rawSuggestions.length > 0"
          class="conversation-composer__memory-group"
        >
          <div class="conversation-composer__memory-group-title">
            {{ t('message.memorySourceRaw') }}
          </div>
          <div class="conversation-composer__memory-list">
            <article
              v-for="suggestion in visibleMemorySuggestions.rawSuggestions"
              :key="`${suggestion.sourceType}:${suggestion.sourceId}`"
              class="conversation-composer__memory-card conversation-composer__memory-card--raw"
              :class="{
                'conversation-composer__memory-card--active': isActiveMemorySuggestion(suggestion)
              }"
              role="option"
              :aria-selected="isActiveMemorySuggestion(suggestion)"
              :title="suggestion.fullContent"
              @mouseenter="previewMemorySuggestion(suggestion)"
              @mouseleave="clearMemoryPreview"
            >
              <div class="conversation-composer__memory-card-body">
                <div class="conversation-composer__memory-card-top">
                  <span class="conversation-composer__memory-badge conversation-composer__memory-badge--raw">
                    {{ t('message.memorySourceRaw') }}
                  </span>
                  <span class="conversation-composer__memory-card-title">{{ suggestion.title }}</span>
                </div>
                <p class="conversation-composer__memory-card-snippet">
                  {{ suggestion.snippet || suggestion.fullContent }}
                </p>
              </div>
              <div class="conversation-composer__memory-card-actions">
                <button
                  class="conversation-composer__memory-action conversation-composer__memory-action--ghost"
                  type="button"
                  @click="dismissMemorySuggestion(suggestion)"
                >
                  {{ t('message.memoryDismiss') }}
                </button>
                <button
                  class="conversation-composer__memory-action conversation-composer__memory-action--primary"
                  type="button"
                  @click="insertMemoryReference(suggestion)"
                >
                  {{ t('message.memoryInsert') }}
                </button>
              </div>
            </article>
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
          :class="{
            'conversation-composer__render--hidden': !shouldUseRichTextOverlay
          }"
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
                v-else-if="segment.type === 'file'"
                class="conversation-composer__file-tag"
                :title="segment.titleContent"
              >{{ segment.displayContent || segment.content }}</span>
              <span
                v-else-if="segment.type === 'memory'"
                class="conversation-composer__memory-tag"
                :title="segment.titleContent"
              >{{ segment.displayContent || segment.content }}</span>
              <span
                v-else
                class="conversation-composer__slash-tag"
              >{{ segment.content }}</span>
            </template>
          </template>
        </div>

        <div
          v-if="isMainPanel && !inputText"
          class="conversation-composer__ghost-hints"
        >
          <span class="conversation-composer__ghost-hint-pill">
            <EaIcon
              name="image-up"
              :size="11"
            />
            <span>{{ t('message.ghostHintImages') }}</span>
          </span>
          <span class="conversation-composer__ghost-hint-pill">
            <EaIcon
              name="at-sign"
              :size="11"
            />
            <span>{{ t('message.ghostHintFiles') }}</span>
          </span>
          <span class="conversation-composer__ghost-hint-pill">
            <EaIcon
              name="corner-down-left"
              :size="11"
            />
            <span>{{ t('message.ghostHintSend', { shortcut: composerSendShortcutHint }) }}</span>
          </span>
        </div>

        <textarea
          ref="textareaRef"
          v-model="inputText"
          class="conversation-composer__textarea"
          :class="{
            'conversation-composer__textarea--overlay': shouldUseRichTextOverlay
          }"
          rows="4"
          :disabled="!sessionId"
          :placeholder="shouldUseRichTextOverlay ? '' : (inputPlaceholder || t('message.inputPlaceholder', { shortcut: t('message.shortcutEnter') }))"
          @compositionstart="handleCompositionStart"
          @compositionend="handleCompositionEnd"
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

    <CdPathDropdown
      :visible="showCdPathSuggestions"
      :position="cdPathPosition"
      :query="cdPathQuery"
      :current-directory="currentWorkingDirectory"
      @select="handleCdPathSelect"
      @close="closeCdPathSuggestions"
    />
  </div>
</template>

<style scoped>
.conversation-composer {
  --composer-chip-bg: rgba(255, 255, 255, 0.9);
  --composer-chip-border: color-mix(in srgb, var(--color-border) 78%, transparent);
  --composer-chip-text: var(--color-text-primary);
  --composer-chip-hover-bg: rgba(248, 250, 252, 0.98);
  --composer-menu-bg: rgba(255, 255, 255, 0.98);
  --composer-menu-border: color-mix(in srgb, var(--color-border) 72%, transparent);
  --composer-menu-shadow: 0 18px 38px rgba(15, 23, 42, 0.16);
  --composer-menu-option-hover: color-mix(in srgb, var(--color-primary-light) 56%, white);
  --composer-menu-tag-text: var(--color-text-tertiary);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  gap: 10px;
  padding: 14px;
  border-top: none;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.94), rgba(248, 250, 252, 0.98));
  position: relative;
}

.conversation-composer--main {
  gap: 0;
  padding: 10px 16px 8px;
  border-top: none;
  background: var(--color-bg-primary);
}

.conversation-composer--compact {
  padding: 12px;
  gap: 8px;
}

.conversation-composer--drag-over::after {
  content: attr(data-drag-text);
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
  border: 1px solid var(--composer-chip-border);
  border-radius: 999px;
  background: var(--composer-chip-bg);
  color: var(--composer-chip-text);
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
  padding-bottom: calc(4px + env(safe-area-inset-bottom, 0px));
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
  background: var(--composer-chip-hover-bg);
}

.composer-chip__menu {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 0;
  top: auto;
  min-width: 220px;
  padding: 8px;
  border: 1px solid var(--composer-menu-border);
  border-radius: 16px;
  background: var(--composer-menu-bg);
  box-shadow: var(--composer-menu-shadow);
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
  background: var(--composer-menu-option-hover);
}

.composer-chip__tag {
  margin-left: auto;
  font-size: 11px;
  color: var(--composer-menu-tag-text);
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

.conversation-composer__attachments--main {
  align-items: flex-start;
  padding-bottom: 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--color-border) 68%, transparent);
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

.conversation-composer__queue-head {
  display: inline-flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  width: 100%;
  padding: 8px 10px;
  border: 1px solid rgba(226, 232, 240, 0.94);
  border-radius: 14px;
  background: rgba(248, 250, 252, 0.92);
  color: var(--color-text-secondary);
  font-size: 12px;
  font-weight: 600;
}

.conversation-composer__queue-head-title {
  display: inline-flex;
  align-items: center;
  gap: 8px;
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

.conversation-composer__queue-editor {
  width: 100%;
  min-height: 72px;
  padding: 8px 10px;
  border: 1px solid color-mix(in srgb, var(--color-primary) 18%, var(--color-border));
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.96);
  color: var(--color-text-primary);
  font: inherit;
  line-height: 1.5;
  resize: vertical;
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

.conversation-composer--main .conversation-composer__editor-shell:focus-within {
  border-color: var(--color-border);
  box-shadow: none;
}

.conversation-composer__render,
.conversation-composer__textarea {
  width: 100%;
  min-height: 122px;
  padding: 16px 18px;
  box-sizing: border-box;
  font-family: inherit;
  font-weight: inherit;
  letter-spacing: normal;
  word-spacing: normal;
  font-size: var(--font-size-sm);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  overflow-wrap: anywhere;
}

.conversation-composer--main .conversation-composer__render,
.conversation-composer--main .conversation-composer__textarea {
  min-height: 160px;
  padding: 14px 16px 48px;
  font-family: inherit;
  line-height: 1.6;
}

.conversation-composer__render {
  position: relative;
  z-index: 0;
  pointer-events: none;
  color: var(--color-text-primary);
  text-align: start;
  text-indent: 0;
}

.conversation-composer__render--hidden {
  opacity: 0;
}

.conversation-composer__textarea {
  position: absolute;
  inset: 0;
  z-index: 1;
  resize: none;
  background: transparent;
  color: var(--color-text-primary);
  -webkit-text-fill-color: currentColor;
  caret-color: var(--color-text-primary);
  text-align: start;
  text-indent: 0;
}

.conversation-composer__textarea--overlay {
  color: transparent;
  -webkit-text-fill-color: transparent;
}

.conversation-composer__textarea::placeholder {
  color: var(--color-text-tertiary);
  -webkit-text-fill-color: currentColor;
  opacity: 1;
}

.conversation-composer__textarea::selection {
  background: color-mix(in srgb, var(--color-primary) 22%, transparent);
  color: inherit;
  -webkit-text-fill-color: currentColor;
}

.conversation-composer__textarea::-moz-selection {
  background: color-mix(in srgb, var(--color-primary) 22%, transparent);
  color: transparent;
}

.conversation-composer__textarea--overlay::selection {
  color: transparent;
  -webkit-text-fill-color: transparent;
}

.conversation-composer__textarea--overlay::-moz-selection {
  color: transparent;
}

.conversation-composer__placeholder {
  color: var(--color-text-tertiary);
}

.conversation-composer--main .conversation-composer__placeholder {
  color: var(--color-text-tertiary);
}

.conversation-composer__ghost-hints {
  position: absolute;
  left: 14px;
  right: 14px;
  bottom: 10px;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  pointer-events: none;
}

.conversation-composer__ghost-hint-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 24px;
  padding: 0 10px;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
  background: color-mix(in srgb, var(--color-bg-secondary) 84%, white);
  color: var(--color-text-tertiary);
  font-size: 11px;
  line-height: 1;
  white-space: nowrap;
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

.conversation-composer__slash-tag {
  display: inline;
  color: color-mix(in srgb, var(--color-warning) 82%, var(--color-text-primary));
  font: inherit;
  background:
    linear-gradient(
      180deg,
      transparent 0%,
      transparent 18%,
      color-mix(in srgb, var(--color-warning) 12%, transparent) 18%,
      color-mix(in srgb, var(--color-warning) 12%, transparent) 100%
    );
  text-decoration: underline;
  text-decoration-color: color-mix(in srgb, var(--color-warning) 34%, transparent);
  text-decoration-thickness: 1px;
  text-underline-offset: 2px;
  -webkit-box-decoration-break: clone;
  box-decoration-break: clone;
}

.conversation-composer__memory-tag {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin: 0 2px;
  padding: 2px 10px;
  border-radius: 999px;
  background: linear-gradient(135deg, rgba(14, 165, 233, 0.12), rgba(16, 185, 129, 0.16));
  color: color-mix(in srgb, var(--color-primary) 70%, var(--color-text-primary));
  font-weight: var(--font-weight-medium);
  box-decoration-break: clone;
  -webkit-box-decoration-break: clone;
}

.conversation-composer__memory-tray {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.conversation-composer__memory-tray-label,
.conversation-composer__memory-group-title,
.conversation-composer__memory-eyebrow {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
}

.conversation-composer__memory-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.conversation-composer__memory-chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--color-primary) 18%, var(--color-border));
  background: color-mix(in srgb, var(--color-primary-light) 35%, white);
  color: var(--color-text-primary);
  font-size: var(--font-size-xs);
  transition: transform 0.16s ease, border-color 0.16s ease, background-color 0.16s ease;
}

.conversation-composer__memory-chip:hover {
  transform: translateY(-1px);
  border-color: color-mix(in srgb, var(--color-primary) 32%, var(--color-border));
}

.conversation-composer__memory-chip-type {
  padding: 2px 7px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.82);
  color: var(--color-text-secondary);
}

.conversation-composer__memory-chip-text {
  max-width: min(30vw, 240px);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.conversation-composer__memory-panel {
  display: flex;
  flex-direction: column;
  gap: clamp(10px, 1.2vw, 12px);
  padding: clamp(10px, 1.6vw, 14px);
  border: 1px solid color-mix(in srgb, var(--color-primary) 16%, var(--color-border));
  border-radius: clamp(14px, 2vw, 18px);
  background:
    radial-gradient(circle at top left, rgba(14, 165, 233, 0.08), transparent 36%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(248, 250, 252, 0.98));
  box-shadow: 0 16px 38px rgba(15, 23, 42, 0.08);
}

.conversation-composer__memory-panel-header,
.conversation-composer__memory-card-top,
.conversation-composer__memory-card-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.conversation-composer__memory-title {
  font-size: clamp(13px, 0.82rem + 0.16vw, var(--font-size-sm));
  font-weight: 700;
  color: var(--color-text-primary);
}

.conversation-composer__memory-keyboard-hint {
  margin-top: 4px;
  font-size: clamp(10px, 0.68rem + 0.12vw, 11px);
  color: var(--color-text-secondary);
}

.conversation-composer__memory-loading {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: clamp(11px, 0.72rem + 0.1vw, var(--font-size-xs));
  color: var(--color-text-secondary);
}

.conversation-composer__memory-spinner {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  border: 2px solid color-mix(in srgb, var(--color-primary) 18%, rgba(255, 255, 255, 0.36));
  border-top-color: var(--color-primary);
  border-radius: 999px;
  animation: conversation-composer-memory-spin 0.72s linear infinite;
}

.conversation-composer__memory-preview {
  display: flex;
  flex-direction: column;
  gap: clamp(6px, 1vw, 8px);
  padding: clamp(10px, 1.4vw, 14px);
  border: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
  border-radius: clamp(12px, 1.8vw, 16px);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.94), rgba(255, 255, 255, 0.88)),
    var(--color-surface);
}

.conversation-composer__memory-preview--library {
  border-color: rgba(14, 165, 233, 0.22);
}

.conversation-composer__memory-preview--raw {
  border-color: rgba(16, 185, 129, 0.22);
}

.conversation-composer__memory-preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.conversation-composer__memory-preview-label {
  font-size: clamp(10px, 0.68rem + 0.12vw, 11px);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
}

.conversation-composer__memory-preview-source {
  font-size: clamp(10px, 0.68rem + 0.12vw, 11px);
  font-weight: 600;
  color: var(--color-text-secondary);
}

.conversation-composer__memory-preview-name {
  font-size: clamp(13px, 0.82rem + 0.16vw, var(--font-size-sm));
  font-weight: 700;
  color: var(--color-text-primary);
}

.conversation-composer__memory-preview-content {
  margin: 0;
  max-height: clamp(112px, 18vh, 168px);
  overflow: auto;
  padding: clamp(8px, 1vw, 12px);
  border-radius: 12px;
  background: color-mix(in srgb, var(--color-bg-secondary) 84%, white);
  color: var(--color-text-primary);
  font-family: inherit;
  font-size: clamp(12px, 0.78rem + 0.14vw, var(--font-size-sm));
  line-height: 1.58;
  white-space: pre-wrap;
  word-break: break-word;
}

.conversation-composer__memory-empty {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 14px;
  border: 1px dashed color-mix(in srgb, var(--color-primary) 18%, var(--color-border));
  border-radius: 14px;
  background: color-mix(in srgb, var(--color-primary-light) 18%, white);
}

.conversation-composer__memory-empty--subtle {
  background: color-mix(in srgb, var(--color-bg-secondary) 72%, white);
  border-style: solid;
}

.conversation-composer__memory-empty-title {
  font-size: clamp(12px, 0.78rem + 0.14vw, var(--font-size-sm));
  font-weight: 700;
  color: var(--color-text-primary);
}

.conversation-composer__memory-empty-hint {
  font-size: clamp(11px, 0.72rem + 0.1vw, var(--font-size-xs));
  line-height: 1.5;
  color: var(--color-text-secondary);
}

.conversation-composer__memory-group {
  display: flex;
  flex-direction: column;
  gap: clamp(6px, 0.9vw, 8px);
  min-height: 0;
}

.conversation-composer__memory-list {
  display: grid;
  gap: clamp(8px, 1vw, 10px);
  max-height: clamp(140px, 26vh, 268px);
  overflow-y: auto;
  padding-right: 4px;
  scrollbar-gutter: stable;
}

.conversation-composer__memory-card {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: clamp(10px, 1.3vw, 14px);
  padding: clamp(10px, 1.4vw, 14px);
  border-radius: clamp(12px, 1.8vw, 16px);
  border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
  background: rgba(255, 255, 255, 0.86);
}

.conversation-composer__memory-card--active {
  border-color: color-mix(in srgb, var(--color-primary) 40%, var(--color-border));
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--color-primary-light) 36%, white), rgba(255, 255, 255, 0.94));
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-primary) 16%, transparent);
}

.conversation-composer__memory-card--library {
  border-color: rgba(14, 165, 233, 0.18);
}

.conversation-composer__memory-card--raw {
  border-color: rgba(16, 185, 129, 0.18);
}

.conversation-composer__memory-card-body {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  gap: clamp(6px, 0.9vw, 8px);
}

.conversation-composer__memory-badge {
  padding: 3px clamp(6px, 0.9vw, 8px);
  border-radius: 999px;
  background: rgba(14, 165, 233, 0.12);
  color: #0369a1;
  font-size: clamp(10px, 0.68rem + 0.12vw, 11px);
  font-weight: 700;
}

.conversation-composer__memory-badge--raw {
  background: rgba(16, 185, 129, 0.12);
  color: #047857;
}

.conversation-composer__memory-card-title {
  min-width: 0;
  font-size: clamp(12px, 0.78rem + 0.16vw, var(--font-size-sm));
  font-weight: 600;
  color: var(--color-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.conversation-composer__memory-card-snippet {
  margin: 0;
  font-size: clamp(12px, 0.78rem + 0.14vw, var(--font-size-sm));
  line-height: 1.55;
  color: var(--color-text-secondary);
}

.conversation-composer__memory-card-actions {
  flex-shrink: 0;
  flex-wrap: wrap;
}

.conversation-composer__memory-action {
  min-height: clamp(28px, 4vh, 32px);
  padding: 0 clamp(10px, 1.1vw, 12px);
  border-radius: 999px;
  font-size: clamp(11px, 0.72rem + 0.1vw, var(--font-size-xs));
  font-weight: 600;
  transition: transform 0.16s ease, background-color 0.16s ease, border-color 0.16s ease;
}

.conversation-composer__memory-action:hover {
  transform: translateY(-1px);
}

.conversation-composer__memory-action--ghost {
  border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
  background: transparent;
  color: var(--color-text-secondary);
}

.conversation-composer__memory-action--primary {
  border: none;
  background: linear-gradient(135deg, #0ea5e9, #14b8a6);
  color: white;
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
  padding: 4px 0 0;
  border-top: none;
}

@media (max-width: 900px) {
  .conversation-composer--main {
    padding: 8px 12px 6px;
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

  .conversation-composer__memory-card {
    flex-direction: column;
  }

  .conversation-composer__memory-card-actions {
    width: 100%;
    justify-content: flex-end;
  }

  .conversation-composer__memory-list {
    max-height: min(24vh, 220px);
  }

  .conversation-composer__memory-preview-content {
    max-height: min(16vh, 140px);
  }

  .conversation-composer--main .conversation-composer__render,
  .conversation-composer--main .conversation-composer__textarea {
    min-height: 140px;
    padding: 16px 16px 10px;
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

  .conversation-composer__memory-chip {
    padding: 7px 10px;
  }

  .conversation-composer__memory-chip-text {
    max-width: min(40vw, 180px);
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

@keyframes conversation-composer-memory-spin {
  to {
    transform: rotate(360deg);
  }
}

:global([data-theme='dark']) .conversation-composer--main,
:global(.dark) .conversation-composer--main {
  background: var(--color-bg-primary);
}

:global([data-theme='dark']) .conversation-composer--main .conversation-composer__panel,
:global(.dark) .conversation-composer--main .conversation-composer__panel {
  background: var(--color-surface);
  border-color: var(--color-border);
}

:global([data-theme='dark']) .conversation-composer--main .conversation-composer__editor-shell,
:global(.dark) .conversation-composer--main .conversation-composer__editor-shell {
  background: var(--color-bg-secondary);
  border-color: var(--color-border);
}

:global([data-theme='dark']) .conversation-composer--main .conversation-composer__editor-shell:focus-within,
:global(.dark) .conversation-composer--main .conversation-composer__editor-shell:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(30, 58, 95, 0.5);
}

:global([data-theme='dark']) .conversation-composer--main .composer-chip,
:global(.dark) .conversation-composer--main .composer-chip,
:global([data-theme='dark']) .conversation-composer--main .conversation-composer__send,
:global(.dark) .conversation-composer--main .conversation-composer__send {
  background: var(--color-surface);
  border-color: var(--color-border);
}

:global([data-theme='dark']) .conversation-composer--main .conversation-composer__send--main,
:global(.dark) .conversation-composer--main .conversation-composer__send--main {
  background: var(--color-primary);
}

:global([data-theme='dark']) .conversation-composer__path,
:global(.dark) .conversation-composer__path,
:global([data-theme='dark']) .conversation-composer__queue-pill,
:global(.dark) .conversation-composer__queue-pill,
:global([data-theme='dark']) .composer-chip,
:global(.dark) .composer-chip,
:global([data-theme='dark']) .conversation-composer__send,
:global(.dark) .conversation-composer__send {
  --composer-chip-bg: rgba(30, 41, 59, 0.94);
  --composer-chip-border: rgba(71, 85, 105, 0.82);
  --composer-chip-text: #e2e8f0;
  --composer-chip-hover-bg: rgba(51, 65, 85, 0.94);
  --composer-menu-bg: rgba(15, 23, 42, 0.98);
  --composer-menu-border: rgba(71, 85, 105, 0.82);
  --composer-menu-shadow: 0 18px 38px rgba(2, 6, 23, 0.34);
  --composer-menu-option-hover: rgba(51, 65, 85, 0.88);
  --composer-menu-tag-text: #94a3b8;
  background: rgba(30, 41, 59, 0.94);
  border-color: rgba(71, 85, 105, 0.82);
  color: #e2e8f0;
}

:global([data-theme='dark']) .conversation-composer__queue-pill--main,
:global(.dark) .conversation-composer__queue-pill--main {
  background: rgba(30, 64, 175, 0.2);
  border-color: rgba(96, 165, 250, 0.28);
  color: #bfdbfe;
}

:global([data-theme='dark']) .conversation-composer__attachments--main,
:global(.dark) .conversation-composer__attachments--main {
  border-bottom-color: rgba(71, 85, 105, 0.72);
}

:global([data-theme='dark']) .conversation-composer__attachment,
:global(.dark) .conversation-composer__attachment,
:global([data-theme='dark']) .conversation-composer__queue-item,
:global(.dark) .conversation-composer__queue-item {
  border-color: rgba(71, 85, 105, 0.72);
  background: rgba(15, 23, 42, 0.9);
  box-shadow: none;
}

:global([data-theme='dark']) .conversation-composer__queue-index,
:global(.dark) .conversation-composer__queue-index {
  background: rgba(51, 65, 85, 0.92);
  color: #cbd5e1;
}

:global([data-theme='dark']) .conversation-composer__attachment-remove,
:global(.dark) .conversation-composer__attachment-remove {
  background: rgba(2, 6, 23, 0.8);
}

:global([data-theme='dark']) .composer-chip__menu,
:global(.dark) .composer-chip__menu {
  background: rgba(15, 23, 42, 0.98) !important;
  border-color: rgba(71, 85, 105, 0.82) !important;
  box-shadow: 0 18px 38px rgba(2, 6, 23, 0.34) !important;
  color: #e2e8f0;
}

:global([data-theme='dark']) .composer-chip__option:hover,
:global(.dark) .composer-chip__option:hover,
:global([data-theme='dark']) .composer-chip__option--selected,
:global(.dark) .composer-chip__option--selected {
  background: var(--composer-menu-option-hover);
}

:global([data-theme='dark']) .composer-chip__option,
:global(.dark) .composer-chip__option,
:global([data-theme='dark']) .composer-chip__button,
:global(.dark) .composer-chip__button {
  color: #e2e8f0;
}

:global([data-theme='dark']) .composer-chip__tag,
:global(.dark) .composer-chip__tag {
  color: #94a3b8;
}

:global([data-theme='dark']) .conversation-composer__slash-tag,
:global(.dark) .conversation-composer__slash-tag,
.conversation-composer--dark .conversation-composer__slash-tag {
  color: #fbbf24;
  background:
    linear-gradient(
      180deg,
      transparent 0%,
      transparent 18%,
      rgba(245, 158, 11, 0.14) 18%,
      rgba(245, 158, 11, 0.14) 100%
    );
  text-decoration-color: rgba(245, 158, 11, 0.34);
}

:global([data-theme='dark']) .conversation-composer__memory-tag,
:global(.dark) .conversation-composer__memory-tag,
.conversation-composer--dark .conversation-composer__memory-tag {
  background: linear-gradient(135deg, rgba(14, 165, 233, 0.18), rgba(16, 185, 129, 0.22));
  color: #bae6fd;
}

:global([data-theme='dark']) .conversation-composer__memory-panel,
:global(.dark) .conversation-composer__memory-panel,
.conversation-composer--dark .conversation-composer__memory-panel {
  border-color: rgba(56, 189, 248, 0.18);
  background:
    radial-gradient(circle at top left, rgba(14, 165, 233, 0.12), transparent 36%),
    linear-gradient(180deg, rgba(15, 23, 42, 0.94), rgba(15, 23, 42, 0.98));
  box-shadow: 0 16px 38px rgba(2, 6, 23, 0.24);
}

:global([data-theme='dark']) .conversation-composer__memory-keyboard-hint,
:global(.dark) .conversation-composer__memory-keyboard-hint,
.conversation-composer--dark .conversation-composer__memory-keyboard-hint {
  color: #94a3b8;
}

:global([data-theme='dark']) .conversation-composer__memory-empty,
:global(.dark) .conversation-composer__memory-empty,
.conversation-composer--dark .conversation-composer__memory-empty {
  border-color: rgba(56, 189, 248, 0.18);
  background: rgba(15, 23, 42, 0.68);
}

:global([data-theme='dark']) .conversation-composer__memory-empty-hint,
:global(.dark) .conversation-composer__memory-empty-hint,
.conversation-composer--dark .conversation-composer__memory-empty-hint {
  color: #94a3b8;
}

:global([data-theme='dark']) .conversation-composer__ghost-hint-pill,
:global(.dark) .conversation-composer__ghost-hint-pill,
.conversation-composer--dark .conversation-composer__ghost-hint-pill {
  border-color: rgba(148, 163, 184, 0.18);
  background: rgba(15, 23, 42, 0.58);
  color: #94a3b8;
}

:global([data-theme='dark']) .conversation-composer__memory-card,
:global(.dark) .conversation-composer__memory-card,
.conversation-composer--dark .conversation-composer__memory-card {
  background: rgba(15, 23, 42, 0.82);
  border-color: rgba(71, 85, 105, 0.82);
}

:global([data-theme='dark']) .conversation-composer__memory-card--active,
:global(.dark) .conversation-composer__memory-card--active,
.conversation-composer--dark .conversation-composer__memory-card--active {
  border-color: rgba(56, 189, 248, 0.44);
  background:
    linear-gradient(180deg, rgba(14, 165, 233, 0.16), rgba(15, 23, 42, 0.92));
  box-shadow: 0 0 0 1px rgba(56, 189, 248, 0.16);
}

:global([data-theme='dark']) .conversation-composer__memory-chip,
:global(.dark) .conversation-composer__memory-chip,
.conversation-composer--dark .conversation-composer__memory-chip {
  background: rgba(30, 41, 59, 0.94);
  border-color: rgba(56, 189, 248, 0.18);
  color: #e2e8f0;
}

:global([data-theme='dark']) .conversation-composer__memory-chip-type,
:global(.dark) .conversation-composer__memory-chip-type,
.conversation-composer--dark .conversation-composer__memory-chip-type {
  background: rgba(51, 65, 85, 0.92);
  color: #cbd5e1;
}

:global([data-theme='dark']) .conversation-composer__memory-preview,
:global(.dark) .conversation-composer__memory-preview,
.conversation-composer--dark .conversation-composer__memory-preview {
  border-color: rgba(71, 85, 105, 0.82);
  background:
    linear-gradient(180deg, rgba(30, 41, 59, 0.98), rgba(15, 23, 42, 0.94)),
    var(--color-surface);
}

:global([data-theme='dark']) .conversation-composer__memory-preview-label,
:global(.dark) .conversation-composer__memory-preview-label,
.conversation-composer--dark .conversation-composer__memory-preview-label {
  color: #94a3b8;
}

:global([data-theme='dark']) .conversation-composer__memory-preview-source,
:global(.dark) .conversation-composer__memory-preview-source,
.conversation-composer--dark .conversation-composer__memory-preview-source {
  color: #cbd5e1;
}

:global([data-theme='dark']) .conversation-composer__memory-preview-name,
:global(.dark) .conversation-composer__memory-preview-name,
.conversation-composer--dark .conversation-composer__memory-preview-name {
  color: #f8fafc;
}

:global([data-theme='dark']) .conversation-composer__memory-preview-content,
:global(.dark) .conversation-composer__memory-preview-content,
.conversation-composer--dark .conversation-composer__memory-preview-content {
  background: rgba(15, 23, 42, 0.9);
  color: #e2e8f0;
}

.conversation-composer--dark.conversation-composer--main .composer-chip,
.conversation-composer--dark.conversation-composer--main .conversation-composer__send,
.conversation-composer--dark .conversation-composer__path,
.conversation-composer--dark .conversation-composer__queue-pill {
  background: rgba(30, 41, 59, 0.94) !important;
  border-color: rgba(71, 85, 105, 0.82) !important;
  color: #e2e8f0 !important;
}

.conversation-composer--dark.conversation-composer--main .composer-chip:hover,
.conversation-composer--dark.conversation-composer--main .composer-chip--open,
.conversation-composer--dark.conversation-composer--main .composer-chip--dropdown:hover {
  background: rgba(51, 65, 85, 0.94) !important;
  border-color: rgba(100, 116, 139, 0.88) !important;
}

.conversation-composer--dark .composer-chip__button,
.conversation-composer--dark .composer-chip__button span,
.conversation-composer--dark .composer-chip__button svg,
.conversation-composer--dark .composer-chip__option {
  color: #e2e8f0 !important;
}

.conversation-composer--dark .composer-chip__menu {
  background: rgba(15, 23, 42, 0.98) !important;
  border-color: rgba(71, 85, 105, 0.82) !important;
  box-shadow: 0 18px 38px rgba(2, 6, 23, 0.34) !important;
  color: #e2e8f0 !important;
}

.conversation-composer--dark .composer-chip__option:hover,
.conversation-composer--dark .composer-chip__option--selected {
  background: rgba(51, 65, 85, 0.88) !important;
}

.conversation-composer--dark .composer-chip__tag {
  color: #94a3b8 !important;
}
</style>
