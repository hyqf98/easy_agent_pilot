<script setup lang="ts">
import { EaButton, EaIcon } from '@/components/common'
import MonacoCodeEditor from '../monacoCodeEditor/MonacoCodeEditor.vue'
import { useFileEditorWorkspace } from './useFileEditorWorkspace'

const {
  fileEditorStore,
  fileSizeLabel,
  handleBack,
  handleSave,
  handleSendSelectionToSession,
  languageName,
  saveStatusText,
  settingsStore
} = useFileEditorWorkspace()
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
        @selection-change="fileEditorStore.updateSelection"
        @send-selection="handleSendSelectionToSession"
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

<style scoped src="./styles.css"></style>
