<script setup lang="ts">
import { computed } from 'vue'
import { EaButton, EaIcon } from '@/components/common'
import { useFileEditorStore } from '../stores/fileEditor'
import { useSettingsStore } from '@/stores/settings'
import MonacoCodeEditor from './MonacoCodeEditor.vue'

const fileEditorStore = useFileEditorStore()
const settingsStore = useSettingsStore()

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

const languageName = computed(() => languageNameMap[fileEditorStore.languageId] ?? fileEditorStore.languageId)

const fileSizeLabel = computed(() => {
  if (!fileEditorStore.hasActiveFile) return ''

  const bytes = fileEditorStore.fileSizeBytes
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
})

const saveStatusText = computed(() => {
  if (fileEditorStore.isLoading) return '正在加载文件...'
  if (fileEditorStore.isSaving) return '正在保存...'
  if (fileEditorStore.isDirty) return '未保存'
  if (!fileEditorStore.hasActiveFile) return '未打开文件'
  return '已保存'
})

const handleSave = async (): Promise<void> => {
  await fileEditorStore.saveFile()
}

const handleBack = (): void => {
  fileEditorStore.switchBackToChat()
}
</script>

<template>
  <div class="file-editor-workspace">
    <div class="file-editor-workspace__toolbar">
      <div class="file-editor-workspace__toolbar-left">
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

        <div class="file-editor-workspace__file-meta">
          <span class="file-editor-workspace__file-name">{{ fileEditorStore.fileName || '未选择文件' }}</span>
          <span class="file-editor-workspace__divider">•</span>
          <span class="file-editor-workspace__language">{{ languageName }}</span>
          <span class="file-editor-workspace__divider">•</span>
          <span class="file-editor-workspace__metrics">{{ fileSizeLabel }} / {{ fileEditorStore.lineCount }} 行</span>
          <span class="file-editor-workspace__divider">•</span>
          <span
            class="file-editor-workspace__status"
            :class="{ 'file-editor-workspace__status--dirty': fileEditorStore.isDirty }"
          >
            {{ saveStatusText }}
          </span>
          <span
            v-if="fileEditorStore.isLargeFile"
            class="file-editor-workspace__badge"
          >
            大文件模式
          </span>
        </div>
      </div>

      <div class="file-editor-workspace__toolbar-right">
        <EaButton
          type="primary"
          size="small"
          :loading="fileEditorStore.isSaving"
          :disabled="!fileEditorStore.hasActiveFile || !fileEditorStore.isDirty"
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
      v-if="fileEditorStore.loadError"
      class="file-editor-workspace__error"
    >
      <EaIcon
        name="alert-circle"
        :size="18"
      />
      <span>{{ fileEditorStore.loadError }}</span>
    </div>

    <div
      v-if="fileEditorStore.hasActiveFile"
      class="file-editor-workspace__content"
    >
      <MonacoCodeEditor
        :model-value="fileEditorStore.content"
        :language="fileEditorStore.languageId"
        :font-size="settingsStore.settings.editorFontSize"
        :tab-size="settingsStore.settings.editorTabSize"
        :word-wrap="settingsStore.settings.editorWordWrap"
        :completions="fileEditorStore.completionEntries"
        :performance-mode="fileEditorStore.isLargeFile ? 'large' : 'default'"
        :read-only="fileEditorStore.isLoading"
        @update:model-value="fileEditorStore.updateContent"
        @save-shortcut="handleSave"
      />
    </div>

    <div
      v-else
      class="file-editor-workspace__empty"
    >
      <EaIcon
        name="file-text"
        :size="22"
      />
      <span>请选择左侧文件开始编辑</span>
    </div>
  </div>
</template>

<style scoped>
.file-editor-workspace {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background-color: var(--color-surface);
}

.file-editor-workspace__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  height: 44px;
  padding: 0 var(--spacing-3);
  border-bottom: 1px solid var(--color-border);
}

.file-editor-workspace__toolbar-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  min-width: 0;
}

.file-editor-workspace__toolbar-right {
  flex-shrink: 0;
}

.file-editor-workspace__file-meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  min-width: 0;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.file-editor-workspace__file-name {
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text-primary);
}

.file-editor-workspace__language {
  color: var(--color-text-tertiary);
}

.file-editor-workspace__metrics {
  color: var(--color-text-tertiary);
}

.file-editor-workspace__divider {
  color: var(--color-text-tertiary);
}

.file-editor-workspace__status {
  color: var(--color-text-secondary);
}

.file-editor-workspace__status--dirty {
  color: var(--color-warning);
}

.file-editor-workspace__badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-warning) 12%, transparent);
  color: var(--color-warning);
  font-size: 11px;
  font-weight: 600;
}

.file-editor-workspace__error {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  color: var(--color-error);
  font-size: var(--font-size-xs);
  border-bottom: 1px solid var(--color-border);
}

.file-editor-workspace__content {
  flex: 1;
  min-height: 0;
}

.file-editor-workspace__empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  color: var(--color-text-tertiary);
  font-size: var(--font-size-sm);
}

@media (max-width: 960px) {
  .file-editor-workspace__file-name {
    max-width: 180px;
  }

  .file-editor-workspace__language,
  .file-editor-workspace__divider {
    display: none;
  }
}
</style>
