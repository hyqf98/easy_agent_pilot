import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getLanguageStrategy } from '@/modules/file-editor/strategies/registry'
import { readProjectFile } from '@/modules/file-editor/services/fileEditorService'
import type { MonacoLanguageId } from '@/modules/file-editor/types'
import type { FileEditChangeType, FileEditRange, FileEditTrace } from '@/types/fileTrace'

interface OpenTracePreviewInput {
  projectId: string
  projectPath: string
  trace: FileEditTrace
}

export const useTracePreviewStore = defineStore('tracePreview', () => {
  const activeProjectId = ref<string | null>(null)
  const activeProjectPath = ref<string | null>(null)
  const activeFilePath = ref<string | null>(null)
  const activeRelativePath = ref<string | null>(null)
  const currentTraceId = ref<string | null>(null)
  const changeType = ref<FileEditChangeType>('modify')
  const content = ref('')
  const beforeContent = ref('')
  const afterContent = ref('')
  const languageId = ref<MonacoLanguageId>('plaintext')
  const highlightedRange = ref<FileEditRange | null>(null)
  const isLoading = ref(false)
  const loadError = ref<string | null>(null)

  async function openTracePreview(input: OpenTracePreviewInput): Promise<void> {
    activeProjectId.value = input.projectId
    activeProjectPath.value = input.projectPath
    activeFilePath.value = input.trace.filePath
    activeRelativePath.value = input.trace.relativePath
    currentTraceId.value = input.trace.id
    changeType.value = input.trace.changeType
    highlightedRange.value = input.trace.range
    loadError.value = null
    beforeContent.value = input.trace.preview?.beforeContent ?? ''
    afterContent.value = input.trace.preview?.afterContent ?? ''

    if (input.trace.changeType === 'delete') {
      content.value = beforeContent.value || (input.trace.preview?.beforeSnippet ?? '')
      languageId.value = getLanguageStrategy(input.trace.filePath).monacoLanguageId
      isLoading.value = false
      return
    }

    if (afterContent.value) {
      content.value = afterContent.value
      languageId.value = getLanguageStrategy(input.trace.filePath).monacoLanguageId
      isLoading.value = false
      return
    }

    isLoading.value = true

    try {
      const nextContent = await readProjectFile(input.projectPath, input.trace.filePath)
      content.value = nextContent.content
      languageId.value = getLanguageStrategy(input.trace.filePath).monacoLanguageId
    } catch (error) {
      content.value = input.trace.preview?.afterSnippet ?? ''
      languageId.value = getLanguageStrategy(input.trace.filePath).monacoLanguageId
      loadError.value = error instanceof Error ? error.message : String(error)
    } finally {
      isLoading.value = false
    }
  }

  function clear(): void {
    activeProjectId.value = null
    activeProjectPath.value = null
    activeFilePath.value = null
    activeRelativePath.value = null
    currentTraceId.value = null
    changeType.value = 'modify'
    content.value = ''
    beforeContent.value = ''
    afterContent.value = ''
    languageId.value = 'plaintext'
    highlightedRange.value = null
    isLoading.value = false
    loadError.value = null
  }

  return {
    activeProjectId,
    activeProjectPath,
    activeFilePath,
    activeRelativePath,
    currentTraceId,
    changeType,
    content,
    beforeContent,
    afterContent,
    languageId,
    highlightedRange,
    isLoading,
    loadError,
    openTracePreview,
    clear
  }
})
