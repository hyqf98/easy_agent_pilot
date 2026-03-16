import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { useNotificationStore } from '@/stores/notification'
import { useUIStore } from '@/stores/ui'
import { getErrorMessage } from '@/utils/api'
import { getLanguageStrategy } from '../strategies/registry'
import { readProjectFile, writeProjectFile } from '../services/fileEditorService'
import { activateLspForFile } from '../services/lspService'
import type { CompletionEntry, FileEditorOpenInput, MonacoLanguageId } from '../types'

const UNSAVED_CHANGES_CONFIRM = '当前文件有未保存修改，确认放弃这些修改吗？'
const LARGE_FILE_SIZE_BYTES = 512 * 1024
const LARGE_FILE_LINE_COUNT = 8000

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

  const isLoading = ref(false)
  const isSaving = ref(false)
  const loadError = ref<string | null>(null)

  const fileName = computed(() => {
    if (!activeFilePath.value) return ''
    const normalized = activeFilePath.value.replace(/\\/g, '/')
    return normalized.split('/').pop() ?? normalized
  })

  const isDirty = computed(() => content.value !== originalContent.value)
  const hasActiveFile = computed(() => Boolean(activeFilePath.value))
  const isLargeFile = computed(() =>
    fileSizeBytes.value >= LARGE_FILE_SIZE_BYTES || lineCount.value >= LARGE_FILE_LINE_COUNT
  )

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
    isLoading.value = false
    isSaving.value = false
    loadError.value = null
  }

  const resolveLanguageState = async (filePath: string): Promise<{
    strategyId: string
    languageId: MonacoLanguageId
    completionEntries: CompletionEntry[]
  }> => {
    const strategy = getLanguageStrategy(filePath)
    let resolvedLanguageId: MonacoLanguageId = strategy.monacoLanguageId
    const resolvedCompletionEntries: CompletionEntry[] = strategy.getCompletions?.() ?? []

    try {
      const lspResult = await activateLspForFile(filePath)

      if (lspResult.activated) {
        resolvedLanguageId = lspResult.monacoLanguageId as MonacoLanguageId
      }
    } catch {
      // 静默回退到内置语言策略，不提示 LSP 激活信息。
    }

    return {
      strategyId: strategy.id,
      languageId: resolvedLanguageId,
      completionEntries: resolvedCompletionEntries
    }
  }

  const refreshActiveFileLanguage = async (): Promise<void> => {
    if (!activeFilePath.value) {
      return
    }

    const languageState = await resolveLanguageState(activeFilePath.value)
    strategyId.value = languageState.strategyId
    languageId.value = languageState.languageId
    completionEntries.value = languageState.completionEntries
  }

  const openFile = async (input: FileEditorOpenInput): Promise<boolean> => {
    if (
      activeFilePath.value === input.filePath &&
      activeProjectPath.value === input.projectPath &&
      hasActiveFile.value
    ) {
      const languageState = await resolveLanguageState(input.filePath)
      strategyId.value = languageState.strategyId
      languageId.value = languageState.languageId
      completionEntries.value = languageState.completionEntries

      uiStore.setMainContentMode('fileEditor')
      return true
    }

    if (!canSwitchFile()) {
      return false
    }

    isLoading.value = true
    loadError.value = null

    try {
      const filePayload = await readProjectFile(input.projectPath, input.filePath)
      const languageState = await resolveLanguageState(input.filePath)

      activeProjectId.value = input.projectId
      activeProjectPath.value = input.projectPath
      activeFilePath.value = input.filePath
      content.value = filePayload.content
      originalContent.value = filePayload.content
      strategyId.value = languageState.strategyId
      languageId.value = languageState.languageId
      completionEntries.value = languageState.completionEntries
      // 大文件场景把文件规模暴露给编辑器组件，便于主动关闭高成本能力。
      fileSizeBytes.value = filePayload.sizeBytes
      lineCount.value = filePayload.lineCount

      uiStore.setMainContentMode('fileEditor')
      return true
    } catch (error) {
      const message = getErrorMessage(error)
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
    isLargeFile,
    openFile,
    updateContent,
    saveFile,
    switchBackToChat,
    closeEditor,
    refreshActiveFileLanguage,
    resetEditorState
  }
})
