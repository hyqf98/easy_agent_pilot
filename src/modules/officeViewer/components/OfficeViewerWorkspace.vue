<script setup lang="ts">
import { defineAsyncComponent, computed, onMounted, onBeforeUnmount } from 'vue'
import { EaButton, EaIcon } from '@/components/common'
import { useOfficeViewerStore } from '../stores/officeViewer'
import { useUIStore } from '@/stores/ui'
import { OFFICE_FILE_TYPE_LABELS } from '../types'

const PdfViewer = defineAsyncComponent(() => import('../components/PdfViewer.vue'))
const SheetEditor = defineAsyncComponent(() => import('../components/SheetEditor.vue'))
const DocxEditor = defineAsyncComponent(() => import('../components/DocxEditor.vue'))
const SlideEditor = defineAsyncComponent(() => import('../components/SlideEditor.vue'))

const store = useOfficeViewerStore()
const uiStore = useUIStore()

const typeLabel = computed(() => OFFICE_FILE_TYPE_LABELS[store.fileType])

const viewerComponent = computed(() => {
  switch (store.fileType) {
    case 'pdf':
      return PdfViewer
    case 'xlsx':
      return SheetEditor
    case 'docx':
      return DocxEditor
    case 'pptx':
      return SlideEditor
    default:
      return null
  }
})

const isDirty = computed(() => {
  if (!store.fileBuffer || !store.originalBuffer) return false
  if (store.fileBuffer.length !== store.originalBuffer.length) return true
  for (let i = 0; i < store.fileBuffer.length; i++) {
    if (store.fileBuffer[i] !== store.originalBuffer[i]) return true
  }
  return false
})

const isEditable = computed(() => store.fileType === 'xlsx' || store.fileType === 'docx')

const saveStatusText = computed(() => {
  if (store.isLoading) return '正在加载文件...'
  if (store.isSaving) return '正在保存...'
  if (isDirty.value) return '未保存'
  if (!store.hasActiveFile) return '未打开文件'
  return '已保存'
})

const handleBack = (): void => {
  store.switchBackToChat()
}

const handleSave = async (): Promise<void> => {
  await store.saveFile()
}

const handleLoading = (loading: boolean): void => {
  void loading
}

const handleKeydown = (event: KeyboardEvent) => {
  if (uiStore.mainContentMode !== 'officeViewer') return
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
    event.preventDefault()
    event.stopPropagation()
    handleSave()
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown, true)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleKeydown, true)
})
</script>

<template>
  <div class="office-viewer-workspace">
    <div class="office-viewer-workspace__toolbar">
      <div class="office-viewer-workspace__toolbar-left">
        <EaButton
          type="ghost"
          size="small"
          @click="handleBack"
        >
          <EaIcon
            name="arrow-left"
            :size="14"
          />
          返回聊天
        </EaButton>

        <div class="office-viewer-workspace__file-meta">
          <span class="office-viewer-workspace__file-name">{{ store.fileName || '未选择文件' }}</span>
          <span class="office-viewer-workspace__divider">•</span>
          <span class="office-viewer-workspace__type-badge">{{ typeLabel }}</span>
          <span class="office-viewer-workspace__divider">•</span>
          <span
            class="office-viewer-workspace__status"
            :class="{ 'office-viewer-workspace__status--dirty': isDirty }"
          >
            {{ saveStatusText }}
          </span>
        </div>
      </div>

      <div class="office-viewer-workspace__toolbar-right">
        <span
          v-if="!isEditable"
          class="office-viewer-workspace__readonly-badge"
        >
          只读预览
        </span>
        <EaButton
          v-if="isEditable"
          type="primary"
          size="small"
          :loading="store.isSaving"
          :disabled="!store.hasActiveFile"
          @click="handleSave"
        >
          <EaIcon
            name="save"
            :size="14"
          />
          保存 (Ctrl/Cmd+S)
        </EaButton>
      </div>
    </div>

    <div
      v-if="store.hasActiveFile && viewerComponent"
      class="office-viewer-workspace__content"
    >
      <component
        :is="viewerComponent"
        :buffer="store.fileBuffer"
        @loading="handleLoading"
      />
    </div>

    <div
      v-else-if="!store.hasActiveFile"
      class="office-viewer-workspace__empty"
    >
      <EaIcon
        name="file-text"
        :size="22"
      />
      <span>请选择左侧文件预览</span>
    </div>
  </div>
</template>

<style scoped>
.office-viewer-workspace {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background-color: var(--color-surface);
}

.office-viewer-workspace__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  height: 44px;
  padding: 0 var(--spacing-3);
  border-bottom: 1px solid var(--color-border);
}

.office-viewer-workspace__toolbar-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  min-width: 0;
}

.office-viewer-workspace__toolbar-right {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  flex-shrink: 0;
}

.office-viewer-workspace__file-meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  min-width: 0;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.office-viewer-workspace__file-name {
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text-primary);
}

.office-viewer-workspace__divider {
  color: var(--color-text-tertiary);
}

.office-viewer-workspace__type-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 12%, transparent);
  color: var(--color-primary);
  font-size: 11px;
  font-weight: 600;
}

.office-viewer-workspace__status {
  color: var(--color-text-tertiary);
}

.office-viewer-workspace__status--dirty {
  color: var(--color-warning, #f59e0b);
}

.office-viewer-workspace__readonly-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 10px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-text-tertiary) 15%, transparent);
  color: var(--color-text-tertiary);
  font-size: 11px;
  font-weight: 500;
}

.office-viewer-workspace__content {
  flex: 1;
  min-height: 0;
  min-width: 0;
  display: flex;
  overflow: hidden;
}

.office-viewer-workspace__content > :deep(*) {
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.office-viewer-workspace__empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  color: var(--color-text-tertiary);
  font-size: var(--font-size-sm);
}
</style>
