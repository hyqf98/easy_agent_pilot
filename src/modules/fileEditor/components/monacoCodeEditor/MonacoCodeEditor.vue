<script setup lang="ts">
import {
  useMonacoCodeEditor,
  type MonacoCodeEditorEmits,
  type MonacoCodeEditorProps
} from './useMonacoCodeEditor'

const props = withDefaults(defineProps<MonacoCodeEditorProps>(), {
  performanceMode: 'default',
  completions: () => [],
  readOnly: false,
  highlightedRanges: () => [],
  focusRange: null
})

const emit = defineEmits<MonacoCodeEditorEmits>()

const {
  containerRef,
  contextMenuState,
  handleSendSelectionFromContextMenu
} = useMonacoCodeEditor(props, emit)
</script>

<template>
  <div class="monaco-editor-shell">
    <div
      ref="containerRef"
      class="monaco-editor-wrapper"
    />

    <Teleport to="body">
      <div
        v-if="contextMenuState"
        class="monaco-selection-context-menu"
        :style="{
          left: `${contextMenuState.x}px`,
          top: `${contextMenuState.y}px`
        }"
      >
        <button
          type="button"
          class="monaco-selection-context-menu__item"
          @click="handleSendSelectionFromContextMenu"
        >
          发送到会话
        </button>
      </div>
    </Teleport>
  </div>
</template>

<style scoped src="./styles.css"></style>
