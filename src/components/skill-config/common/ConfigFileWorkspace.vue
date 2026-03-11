<script setup lang="ts">
import { computed } from 'vue'
import { EaIcon } from '@/components/common'
import MarkdownRenderer from '@/components/message/MarkdownRenderer.vue'

interface WorkspaceFile {
  name: string
  path: string
  content: string
  fileType: string
}

const props = withDefaults(defineProps<{
  loading?: boolean
  editing?: boolean
  file: WorkspaceFile | null
  editContent: string
  editPlaceholder?: string
  emptyText: string
  maxWidth?: string
  padding?: string
}>(), {
  loading: false,
  editing: false,
  editPlaceholder: '',
  maxWidth: '960px',
  padding: 'var(--spacing-6)',
})

const emit = defineEmits<{
  (e: 'update:editContent', value: string): void
}>()

const isMarkdown = computed(() => props.file?.fileType === 'markdown')

const contentStyle = computed(() => ({
  '--config-file-workspace-max-width': props.maxWidth,
  '--config-file-workspace-padding': props.padding,
}))

function handleInput(event: Event): void {
  const target = event.target as HTMLTextAreaElement
  emit('update:editContent', target.value)
}
</script>

<template>
  <div
    class="config-file-workspace"
    :style="contentStyle"
  >
    <div
      v-if="loading"
      class="config-file-workspace__loading"
    >
      <EaIcon
        name="lucide:loader-2"
        class="config-file-workspace__spinner"
      />
      <slot name="loading">
        Loading...
      </slot>
    </div>

    <div
      v-else-if="editing && file"
      class="config-file-workspace__editor"
    >
      <textarea
        class="config-file-workspace__textarea"
        :value="editContent"
        :placeholder="editPlaceholder"
        @input="handleInput"
      />
    </div>

    <div
      v-else-if="file && isMarkdown"
      class="config-file-workspace__markdown"
    >
      <MarkdownRenderer :content="file.content" />
    </div>

    <div
      v-else-if="file"
      class="config-file-workspace__code"
    >
      <pre class="config-file-workspace__code-content"><code>{{ file.content }}</code></pre>
    </div>

    <div
      v-else
      class="config-file-workspace__empty"
    >
      <EaIcon
        name="lucide:file-x"
        class="config-file-workspace__empty-icon"
      />
      <p>{{ emptyText }}</p>
    </div>
  </div>
</template>

<style scoped>
.config-file-workspace {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.config-file-workspace__loading,
.config-file-workspace__empty {
  display: flex;
  flex: 1;
  min-height: 0;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: var(--spacing-3);
  padding: var(--spacing-6);
  color: var(--color-text-tertiary);
}

.config-file-workspace__spinner {
  width: 20px;
  height: 20px;
  animation: config-file-workspace-spin 1s linear infinite;
}

.config-file-workspace__editor {
  flex: 1;
  display: flex;
  min-height: 0;
  width: 100%;
  max-width: var(--config-file-workspace-max-width);
  margin: 0 auto;
}

.config-file-workspace__textarea {
  flex: 1;
  width: 100%;
  min-height: 100%;
  padding: var(--config-file-workspace-padding);
  border: none;
  background: var(--color-surface);
  color: var(--color-text);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  line-height: 1.7;
  resize: none;
  outline: none;
}

.config-file-workspace__textarea:focus {
  box-shadow: inset 0 0 0 2px var(--color-primary);
}

.config-file-workspace__markdown,
.config-file-workspace__code {
  flex: 1;
  min-height: 0;
  overflow: auto;
  width: 100%;
  max-width: var(--config-file-workspace-max-width);
  margin: 0 auto;
  padding: var(--config-file-workspace-padding);
}

.config-file-workspace__code {
  background: var(--color-background-secondary);
}

.config-file-workspace__code-content {
  margin: 0;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}

.config-file-workspace__empty-icon {
  width: 48px;
  height: 48px;
  opacity: 0.5;
}

@keyframes config-file-workspace-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
