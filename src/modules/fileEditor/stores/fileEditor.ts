import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { useNotificationStore } from '@/stores/notification'
import { useUIStore } from '@/stores/ui'
import { getErrorMessage } from '@/utils/api'
import { readProjectFile, writeProjectFile } from '../services/fileEditorService'
import type { CompletionEntry, FileEditorOpenInput, MarkdownEditorMode, MonacoLanguageId } from '../types'
import { resolveFileEditorLanguageState } from './languageState'
import {
  isLargeEditorFile,
  normalizeSelectionPayload,
  resolveFileEditorName,
  type FileEditorSelectionRange,
  UNSAVED_CHANGES_CONFIRM
} from './storeShared'

export const useFileEditorStore = defineStore('fileEditor', () => {
  const uiStore = useUIStore()
  const notificationStore = useNotificationStore()

  const activeProjectId = ref<string | null>(null)
  const activeProjectPath = ref<string | null>(null)
  const activeFilePath = ref<string | null>(null)

  const content = ref('')
  const originalContent = ref('')
  const languageId = ref<MonacoLanguageId>('plaintext')
  const strategyId = ref('plaintext')
  const completionEntries = ref<CompletionEntry[]>([])
  const fileSizeBytes = ref(0)
  const lineCount = ref(0)
  const selectedText = ref('')
  const selectionRange = ref<FileEditorSelectionRange | null>(null)
  const markdownMode = ref<MarkdownEditorMode>('source')

  const isLoading = ref(false)
  const isSaving = ref(false)
  const loadError = ref<string | null>(null)

  const fileName = computed(() => resolveFileEditorName(activeFilePath.value))

  const isDirty = computed(() => content.value !== originalContent.value)
  const hasActiveFile = computed(() => Boolean(activeFilePath.value))
  const hasSelection = computed(() => Boolean(selectedText.value.trim()) && selectionRange.value !== null)
  const isLargeFile = computed(() => isLargeEditorFile(fileSizeBytes.value, lineCount.value))
  const isMarkdownFile = computed(() => languageId.value === 'markdown')
  const effectiveMarkdownMode = computed<MarkdownEditorMode>(() => {
    if (!isMarkdownFile.value || isLargeFile.value) {
      return 'source'
    }

    return markdownMode.value
  })

  const canSwitchFile = (): boolean => {
    if (!isDirty.value) {
      return true
    }

    return window.confirm(UNSAVED_CHANGES_CONFIRM)
  }

  const resetEditorState = (): void => {
    activeProjectId.value = null
    activeProjectPath.value = null
    activeFilePath.value = null
    content.value = ''
    originalContent.value = ''
    languageId.value = 'plaintext'
    strategyId.value = 'plaintext'
    completionEntries.value = []
    fileSizeBytes.value = 0
    lineCount.value = 0
    selectedText.value = ''
    selectionRange.value = null
    markdownMode.value = 'source'
    isLoading.value = false
    isSaving.value = false
    loadError.value = null
  }

  const refreshActiveFileLanguage = async (): Promise<void> => {
    if (!activeFilePath.value) {
      return
    }

    const languageState = await resolveFileEditorLanguageState(activeFilePath.value)
    strategyId.value = languageState.strategyId
    languageId.value = languageState.languageId
    completionEntries.value = languageState.completionEntries
    if (languageState.languageId !== 'markdown') {
      markdownMode.value = 'source'
    }
  }

  const openFile = async (input: FileEditorOpenInput): Promise<boolean> => {
    if (
      activeFilePath.value === input.filePath &&
      activeProjectPath.value === input.projectPath &&
      hasActiveFile.value
    ) {
      const languageState = await resolveFileEditorLanguageState(input.filePath)
      strategyId.value = languageState.strategyId
      languageId.value = languageState.languageId
      completionEntries.value = languageState.completionEntries
      if (languageState.languageId !== 'markdown') {
        markdownMode.value = 'source'
      }

      uiStore.setMainContentMode('fileEditor')
      return true
    }

    if (!canSwitchFile()) {
      return false
    }

    const previousState = {
      activeProjectId: activeProjectId.value,
      activeProjectPath: activeProjectPath.value,
      activeFilePath: activeFilePath.value,
      content: content.value,
      originalContent: originalContent.value,
      languageId: languageId.value,
      strategyId: strategyId.value,
      completionEntries: [...completionEntries.value],
      fileSizeBytes: fileSizeBytes.value,
      lineCount: lineCount.value,
      markdownMode: markdownMode.value,
      selectedText: selectedText.value,
      selectionRange: selectionRange.value
    }

    isLoading.value = true
    loadError.value = null
    activeProjectId.value = input.projectId
    activeProjectPath.value = input.projectPath
    activeFilePath.value = input.filePath
    content.value = ''
    originalContent.value = ''
    fileSizeBytes.value = 0
    lineCount.value = 0
    selectedText.value = ''
    selectionRange.value = null
    uiStore.setMainContentMode('fileEditor')

    try {
      const filePayload = await readProjectFile(input.projectPath, input.filePath)
      const languageState = await resolveFileEditorLanguageState(input.filePath)

      content.value = filePayload.content
      originalContent.value = filePayload.content
      strategyId.value = languageState.strategyId
      languageId.value = languageState.languageId
      completionEntries.value = languageState.completionEntries
      // 大文件场景把文件规模暴露给编辑器组件，便于主动关闭高成本能力。
      fileSizeBytes.value = filePayload.sizeBytes
      lineCount.value = filePayload.lineCount
      selectedText.value = ''
      selectionRange.value = null
      markdownMode.value = languageState.languageId === 'markdown' ? 'rich' : 'source'
      return true
    } catch (error) {
      const message = getErrorMessage(error)
      activeProjectId.value = previousState.activeProjectId
      activeProjectPath.value = previousState.activeProjectPath
      activeFilePath.value = previousState.activeFilePath
      content.value = previousState.content
      originalContent.value = previousState.originalContent
      languageId.value = previousState.languageId
      strategyId.value = previousState.strategyId
      completionEntries.value = previousState.completionEntries
      fileSizeBytes.value = previousState.fileSizeBytes
      lineCount.value = previousState.lineCount
      markdownMode.value = previousState.markdownMode
      selectedText.value = previousState.selectedText
      selectionRange.value = previousState.selectionRange
      loadError.value = message
      notificationStore.error('打开文件失败', message)
      return false
    } finally {
      isLoading.value = false
    }
  }

  const updateContent = (nextContent: string): void => {
    content.value = nextContent
  }

  const replaceContentSnapshot = (nextContent: string): void => {
    content.value = nextContent
    originalContent.value = nextContent
  }

  const setMarkdownMode = (nextMode: MarkdownEditorMode): void => {
    markdownMode.value = nextMode
  }

  /**
   * 同步编辑器当前选区，供外部执行“引用选中内容”等动作。
   */
  const updateSelection = (payload: { text: string; startLine: number; endLine: number } | null): void => {
    const nextSelection = normalizeSelectionPayload(payload)
    selectedText.value = nextSelection.selectedText
    selectionRange.value = nextSelection.selectionRange
  }

  const saveFile = async (): Promise<boolean> => {
    if (!activeFilePath.value || !activeProjectPath.value || isSaving.value) {
      return false
    }

    isSaving.value = true

    try {
      await writeProjectFile({
        projectPath: activeProjectPath.value,
        filePath: activeFilePath.value,
        content: content.value
      })
      originalContent.value = content.value
      return true
    } catch (error) {
      notificationStore.error('保存文件失败', getErrorMessage(error))
      return false
    } finally {
      isSaving.value = false
    }
  }

  const switchBackToChat = (): boolean => {
    if (!canSwitchFile()) {
      return false
    }

    uiStore.setMainContentMode('chat')
    return true
  }

  const closeEditor = (): boolean => {
    if (!canSwitchFile()) {
      return false
    }

    resetEditorState()
    uiStore.setMainContentMode('chat')
    return true
  }

  return {
    activeProjectId,
    activeProjectPath,
    activeFilePath,
    content,
    originalContent,
    languageId,
    strategyId,
    completionEntries,
    isLoading,
    isSaving,
    loadError,
    fileName,
    isDirty,
    hasActiveFile,
    fileSizeBytes,
    lineCount,
    selectedText,
    selectionRange,
    isLargeFile,
    hasSelection,
    isMarkdownFile,
    markdownMode,
    effectiveMarkdownMode,
    openFile,
    updateContent,
    replaceContentSnapshot,
    setMarkdownMode,
    updateSelection,
    saveFile,
    switchBackToChat,
    closeEditor,
    refreshActiveFileLanguage,
    resetEditorState
  }
})
