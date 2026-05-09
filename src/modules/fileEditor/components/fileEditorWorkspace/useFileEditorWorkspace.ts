import { computed, onMounted } from 'vue'
import { useSessionFileReference } from '@/composables'
import { useSettingsStore } from '@/stores/settings'
import { createFileLineRangeMention } from '@/utils/composerFileMention'
import { prewarmMonacoEditor } from '../../monaco/setup'
import { useFileEditorStore } from '../../stores/fileEditor'
import type { MarkdownEditorMode } from '../../types'

const languageNameMap: Record<string, string> = {
  plaintext: 'Plain Text',
  javascript: 'JavaScript',
  typescript: 'TypeScript',
  json: 'JSON',
  markdown: 'Markdown',
  python: 'Python',
  java: 'Java',
  rust: 'Rust',
  html: 'HTML',
  css: 'CSS',
  shell: 'Shell',
  yaml: 'YAML'
}

/**
 * 文件编辑工作区视图状态。
 * 负责聚合工具栏展示数据和选中代码发送到会话的行为。
 */
export function useFileEditorWorkspace() {
  const fileEditorStore = useFileEditorStore()
  const settingsStore = useSettingsStore()
  const { sendFileReferencesToSession } = useSessionFileReference()

  const languageName = computed(() => languageNameMap[fileEditorStore.languageId] ?? fileEditorStore.languageId)

  const fileSizeLabel = computed(() => {
    if (!fileEditorStore.hasActiveFile) {
      return ''
    }

    const bytes = fileEditorStore.fileSizeBytes
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  })

  const saveStatusText = computed(() => {
    if (fileEditorStore.previewMode !== 'editor') {
      return fileEditorStore.previewMode === 'image' ? '图片预览' : '只读'
    }
    if (fileEditorStore.isLoading) return '正在加载文件...'
    if (fileEditorStore.isSaving) return '正在保存...'
    if (fileEditorStore.isDirty) return '未保存'
    if (!fileEditorStore.hasActiveFile) return '未打开文件'
    return '已保存'
  })

  const markdownModeText = computed(() => {
    if (fileEditorStore.effectiveMarkdownMode === 'rich') {
      return '所见即所得'
    }

    return '源码'
  })

  const handleSave = async (): Promise<void> => {
    await fileEditorStore.saveFile()
  }

  const handleBack = (): void => {
    fileEditorStore.switchBackToChat()
  }

  const handleSendSelectionToSession = async (payload: { startLine: number; endLine: number }): Promise<void> => {
    if (!fileEditorStore.activeProjectId || !fileEditorStore.activeFilePath) {
      return
    }

    await sendFileReferencesToSession({
      sourceProjectId: fileEditorStore.activeProjectId,
      mentions: [createFileLineRangeMention({
        fullPath: fileEditorStore.activeFilePath,
        fileName: fileEditorStore.fileName,
        startLine: payload.startLine,
        endLine: payload.endLine
      })]
    })
  }

  const handleMarkdownModeChange = (mode: MarkdownEditorMode): void => {
    fileEditorStore.setMarkdownMode(mode)
  }

  onMounted(() => {
    if (typeof window !== 'undefined') {
      const run = () => {
        void prewarmMonacoEditor()
      }

      if (typeof window.requestIdleCallback === 'function') {
        window.requestIdleCallback(() => run(), { timeout: 240 })
      } else {
        globalThis.setTimeout(run, 60)
      }
    }
  })

  return {
    fileEditorStore,
    fileSizeLabel,
    handleBack,
    handleMarkdownModeChange,
    handleSave,
    handleSendSelectionToSession,
    languageName,
    markdownModeText,
    saveStatusText,
    settingsStore
  }
}
