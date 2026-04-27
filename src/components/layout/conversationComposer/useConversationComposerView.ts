import { computed, nextTick, onMounted, onUnmounted, ref, watch, type ComponentPublicInstance } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useI18n } from 'vue-i18n'
import { useConversationComposer } from '@/composables/useConversationComposer'
import { useSettingsStore } from '@/stores/settings'
import { useThemeStore } from '@/stores/theme'
import type { SlashCommandPanelType } from '@/services/slashCommands'

export interface ConversationComposerProps {
  panelType: SlashCommandPanelType
  sessionId?: string | null
  workingDirectory?: string | null
  setWorkingDirectory?: (path: string) => Promise<string>
  defaultFileMentionScope?: 'project' | 'global'
  compact?: boolean
  showWorkingDirectory?: boolean
  hideStatusBar?: boolean
}

/**
 * 主会话输入区视图状态。
 * 负责装配发送框能力、拖拽监听、排队消息编辑和主题衍生状态。
 */
export function useConversationComposerView(props: Readonly<ConversationComposerProps>) {
  const { t } = useI18n()
  const settingsStore = useSettingsStore()
  const themeStore = useThemeStore()
  const rootRef = ref<HTMLElement | null>(null)
  const isDragOver = ref(false)
  const isQueueCollapsed = ref(true)
  const editingQueuedDraftId = ref<string | null>(null)
  const queuedDraftEditText = ref('')
  const queuedDraftEditorRefs = new Map<string, HTMLTextAreaElement>()
  let unlistenDragDrop: (() => void) | null = null

  const isMainPanel = computed(() => props.panelType === 'main')
  const isMiniPanel = computed(() => props.panelType === 'mini')
  const isDarkTheme = computed(() => themeStore.isDark)

  const composer = useConversationComposer({
    panelType: props.panelType,
    sessionId: computed(() => props.sessionId ?? null),
    projectPath: computed(() => props.workingDirectory || null),
    workingDirectory: computed(() => props.workingDirectory || null),
    setWorkingDirectory: props.setWorkingDirectory
  })

  const shouldUseRichTextOverlay = computed(() => (
    composer.parsedInputText.value.some(segment => segment.type === 'file' || segment.type === 'slash' || segment.type === 'memory')
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
        composer.insertFileMentions(event.payload.paths)
        composer.focusInput()
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
    void nextTick(() => {
      const editor = queuedDraftEditorRefs.get(draftId)
      if (!editor) {
        return
      }

      editor.focus()
      editor.setSelectionRange(editor.value.length, editor.value.length)
    })
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

    composer.updateQueuedMessage(draftId, {
      content: normalized,
      displayContent: normalized,
      memoryReferences: []
    })
    cancelQueuedMessageEdit()
  }

  const setQueuedDraftEditorRef = (draftId: string, element: Element | ComponentPublicInstance | null) => {
    if (!(element instanceof HTMLTextAreaElement)) {
      queuedDraftEditorRefs.delete(draftId)
      return
    }

    queuedDraftEditorRefs.set(draftId, element)
  }

  return {
    ...composer,
    composerSendShortcutHint,
    editingQueuedDraftId,
    isDarkTheme,
    isDragOver,
    isMainPanel,
    isMiniPanel,
    isQueueCollapsed,
    queuedDraftEditText,
    rootRef,
    saveQueuedMessageEdit,
    setQueuedDraftEditorRef,
    shouldUseRichTextOverlay,
    startQueuedMessageEdit,
    t,
    toggleQueueCollapsed,
    cancelQueuedMessageEdit
  }
}

export type ConversationComposerViewState = ReturnType<typeof useConversationComposerView>
