import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { useNotificationStore } from '@/stores/notification'
import { useUIStore } from '@/stores/ui'
import { getErrorMessage } from '@/utils/api'
import { getOfficeFileType, OFFICE_FILE_TYPE_LABELS, type OfficeFileType, type OfficeViewerOpenInput } from '../types'
import { readBinaryFile, writeBinaryFile } from '../services/officeFileService'

export type ExportProvider = (() => Promise<Uint8Array | null>) | null

let _lastSnapshot: Record<string, unknown> | null = null

export function setCachedSnapshot(snapshot: Record<string, unknown> | null): void {
  _lastSnapshot = snapshot
}

export function getCachedSnapshot(): Record<string, unknown> | null {
  return _lastSnapshot
}

export const useOfficeViewerStore = defineStore('officeViewer', () => {
  const uiStore = useUIStore()
  const notificationStore = useNotificationStore()

  const activeProjectId = ref<string | null>(null)
  const activeProjectPath = ref<string | null>(null)
  const activeFilePath = ref<string | null>(null)

  const fileBuffer = ref<Uint8Array | null>(null)
  const originalBuffer = ref<Uint8Array | null>(null)
  const fileType = ref<OfficeFileType>('pdf')

  const isLoading = ref(false)
  const isSaving = ref(false)

  const exportProvider = ref<ExportProvider>(null)

  const fileName = computed(() => {
    if (!activeFilePath.value) return ''
    return activeFilePath.value.split(/[\\/]/).pop() ?? activeFilePath.value
  })

  const fileTypeLabel = computed(() => OFFICE_FILE_TYPE_LABELS[fileType.value])
  const hasActiveFile = computed(() => Boolean(activeFilePath.value))

  const resetState = (): void => {
    activeProjectId.value = null
    activeProjectPath.value = null
    activeFilePath.value = null
    fileBuffer.value = null
    originalBuffer.value = null
    fileType.value = 'pdf'
    isLoading.value = false
    isSaving.value = false
    exportProvider.value = null
  }

  const openFile = async (input: OfficeViewerOpenInput): Promise<boolean> => {
    isLoading.value = true
    activeProjectId.value = input.projectId
    activeProjectPath.value = input.projectPath
    activeFilePath.value = input.filePath
    fileBuffer.value = null
    originalBuffer.value = null
    fileType.value = getOfficeFileType(input.filePath)
    exportProvider.value = null
    _lastSnapshot = null

    uiStore.setMainContentMode('officeViewer')

    try {
      const buffer = await readBinaryFile(input.filePath)
      fileBuffer.value = buffer
      originalBuffer.value = new Uint8Array(buffer)
      return true
    } catch (error) {
      notificationStore.error('打开文件失败', getErrorMessage(error))
      return false
    } finally {
      isLoading.value = false
    }
  }

  const updateBuffer = (newBuffer: Uint8Array): void => {
    fileBuffer.value = newBuffer
  }

  const registerExportProvider = (provider: ExportProvider): void => {
    exportProvider.value = provider
  }

  const saveFile = async (): Promise<boolean> => {
    if (!activeFilePath.value || isSaving.value) {
      console.warn('[OfficeViewer] save: skipped, activeFilePath=', activeFilePath.value, 'isSaving=', isSaving.value)
      return false
    }

    isSaving.value = true
    try {
      let dataToSave: Uint8Array | null = fileBuffer.value
      console.log('[OfficeViewer] save: fileBuffer size =', dataToSave?.length ?? 'null')

      if (exportProvider.value) {
        console.log('[OfficeViewer] save: calling exportProvider...')
        const exported = await exportProvider.value()
        console.log('[OfficeViewer] save: exportProvider returned size =', exported?.length ?? 'null')
        if (exported) {
          dataToSave = exported
        }
      } else {
        console.log('[OfficeViewer] save: no exportProvider registered')
      }

      if (!dataToSave) {
        notificationStore.error('保存文件失败', '没有可保存的数据')
        return false
      }

      console.log('[OfficeViewer] save: writing', dataToSave.length, 'bytes to', activeFilePath.value)
      await writeBinaryFile(activeFilePath.value, dataToSave)
      console.log('[OfficeViewer] save: writeBinaryFile completed successfully')
      originalBuffer.value = new Uint8Array(dataToSave)
      fileBuffer.value = dataToSave
      notificationStore.success('保存成功', `文件 ${fileName.value} 已保存`)
      return true
    } catch (error) {
      console.error('[OfficeViewer] save: error =', error)
      notificationStore.error('保存文件失败', getErrorMessage(error))
      return false
    } finally {
      isSaving.value = false
    }
  }

  const switchBackToChat = (): void => {
    resetState()
    uiStore.setMainContentMode('chat')
  }

  return {
    activeProjectId,
    activeProjectPath,
    activeFilePath,
    fileBuffer,
    originalBuffer,
    fileType,
    isLoading,
    isSaving,
    fileName,
    fileTypeLabel,
    hasActiveFile,
    openFile,
    updateBuffer,
    registerExportProvider,
    saveFile,
    switchBackToChat,
    resetState,
  }
})
