<script setup lang="ts">
import { computed } from 'vue'
import { EaButton, EaIcon, EaModal } from '@/components/common'
import MonacoCodeEditor from '@/modules/fileEditor/components/monacoCodeEditor/MonacoCodeEditor.vue'
import { useSettingsStore } from '@/stores/settings'
import type { CliType } from '@/stores/providerProfile'
import type { MonacoLanguageId } from '@/modules/fileEditor/types'
import type { DefaultCliConfigLocateTarget } from '@/composables/useDefaultCliConfigEditor'

interface ConfigEditorFile {
  cliType: CliType
  path: string
  content: string
  fileType: 'json' | 'toml'
}

const props = defineProps<{
  visible: boolean
  loading: boolean
  saving: boolean
  file: ConfigEditorFile | null
  content: string
  dirty: boolean
  locateTarget?: DefaultCliConfigLocateTarget | null
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  'update:content': [value: string]
  reload: []
  format: []
  save: []
}>()

const settingsStore = useSettingsStore()

const languageId = computed<MonacoLanguageId>(() => {
  if (props.file?.fileType === 'json') {
    return 'json'
  }
  return 'plaintext'
})

const fileTypeLabel = computed(() => {
  if (props.file?.fileType === 'json') return 'JSON'
  if (props.file?.fileType === 'toml') return 'TOML'
  return 'TEXT'
})

const title = computed(() => {
  switch (props.file?.cliType) {
    case 'claude':
      return 'Claude 默认配置文件'
    case 'codex':
      return 'Codex 默认配置文件'
    case 'opencode':
      return 'OpenCode 默认配置文件'
    default:
      return '默认配置文件'
  }
})

const locateLabel = computed(() => props.locateTarget?.label?.trim() || props.locateTarget?.query?.trim() || '')
</script>

<template>
  <EaModal
    :visible="visible"
    content-class="provider-config-editor-modal"
    overlay-class="provider-config-editor-modal__overlay"
    @update:visible="emit('update:visible', $event)"
  >
    <template #header>
      <div class="provider-config-editor-modal__header">
        <div class="provider-config-editor-modal__header-copy">
          <h3>{{ title }}</h3>
          <p>{{ file?.path || '加载配置文件中…' }}</p>
          <p
            v-if="locateLabel"
            class="provider-config-editor-modal__hint"
          >
            已定位到：{{ locateLabel }}
          </p>
        </div>
        <div class="provider-config-editor-modal__header-meta">
          <span class="provider-config-editor-modal__badge">{{ fileTypeLabel }}</span>
          <span
            class="provider-config-editor-modal__status"
            :class="{ 'provider-config-editor-modal__status--dirty': dirty }"
          >
            {{ dirty ? '未保存' : '已同步' }}
          </span>
        </div>
      </div>
    </template>

    <div
      v-if="loading"
      class="provider-config-editor-modal__loading"
    >
      <EaIcon
        name="loading"
        spin
        :size="22"
      />
      <span>正在加载配置文件…</span>
    </div>

    <div
      v-else-if="file"
      class="provider-config-editor-modal__body"
    >
      <MonacoCodeEditor
        :model-value="content"
        :language="languageId"
        :font-size="settingsStore.settings.editorFontSize"
        :tab-size="settingsStore.settings.editorTabSize"
        :word-wrap="settingsStore.settings.editorWordWrap"
        :read-only="saving"
        :search-target="locateTarget ?? null"
        @update:model-value="emit('update:content', $event)"
        @save-shortcut="emit('save')"
      />
    </div>

    <div
      v-else
      class="provider-config-editor-modal__loading"
    >
      <EaIcon
        name="file-text"
        :size="22"
      />
      <span>没有可编辑的配置文件</span>
    </div>

    <template #footer>
      <div class="provider-config-editor-modal__footer">
        <EaButton
          type="ghost"
          size="small"
          :disabled="loading || saving"
          @click="emit('reload')"
        >
          <EaIcon
            name="refresh-cw"
            :size="14"
          />
          重新加载
        </EaButton>
        <EaButton
          type="secondary"
          size="small"
          :disabled="loading || saving || !file"
          @click="emit('format')"
        >
          <EaIcon
            name="wand-2"
            :size="14"
          />
          格式化
        </EaButton>
        <EaButton
          type="ghost"
          size="small"
          :disabled="saving"
          @click="emit('update:visible', false)"
        >
          关闭
        </EaButton>
        <EaButton
          size="small"
          :loading="saving"
          :disabled="loading || saving || !file || !dirty"
          @click="emit('save')"
        >
          <EaIcon
            name="save"
            :size="14"
          />
          保存
        </EaButton>
      </div>
    </template>
  </EaModal>
</template>

<style scoped>
.provider-config-editor-modal__overlay {
  backdrop-filter: blur(10px);
}

:global(.ea-modal.provider-config-editor-modal) {
  width: min(1080px, calc(100vw - 48px));
  max-width: 1080px;
  min-width: min(1080px, calc(100vw - 48px));
  height: min(82vh, 760px);
  max-height: 82vh;
}

:global(.ea-modal.provider-config-editor-modal .ea-modal__body) {
  padding: 0;
  display: flex;
  min-height: 0;
  overflow: hidden;
}

.provider-config-editor-modal__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.provider-config-editor-modal__header-copy {
  min-width: 0;
}

.provider-config-editor-modal__header-copy h3 {
  margin: 0;
  color: var(--color-text-primary);
  font-size: 18px;
}

.provider-config-editor-modal__header-copy p {
  margin: 6px 0 0;
  color: var(--color-text-secondary);
  font-size: 12px;
  line-height: 1.6;
  word-break: break-all;
}

.provider-config-editor-modal__hint {
  color: var(--color-primary) !important;
  word-break: break-word;
}

.provider-config-editor-modal__header-meta {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}

.provider-config-editor-modal__badge,
.provider-config-editor-modal__status {
  display: inline-flex;
  align-items: center;
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 12px;
  white-space: nowrap;
}

.provider-config-editor-modal__badge {
  background: color-mix(in srgb, var(--color-primary) 12%, transparent);
  color: var(--color-primary);
}

.provider-config-editor-modal__status {
  background: color-mix(in srgb, var(--color-surface-hover) 86%, transparent);
  color: var(--color-text-secondary);
}

.provider-config-editor-modal__status--dirty {
  background: color-mix(in srgb, #f59e0b 16%, transparent);
  color: #b45309;
}

.provider-config-editor-modal__body,
.provider-config-editor-modal__loading {
  flex: 1;
  min-height: 0;
}

.provider-config-editor-modal__loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--color-text-secondary);
}

.provider-config-editor-modal__footer {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

@media (max-width: 960px) {
  :global(.ea-modal.provider-config-editor-modal) {
    width: calc(100vw - 20px);
    min-width: calc(100vw - 20px);
    height: calc(100vh - 32px);
    max-height: calc(100vh - 32px);
  }

  .provider-config-editor-modal__header,
  .provider-config-editor-modal__footer {
    flex-direction: column;
    align-items: stretch;
  }

  .provider-config-editor-modal__header-meta {
    justify-content: flex-start;
  }
}
</style>
