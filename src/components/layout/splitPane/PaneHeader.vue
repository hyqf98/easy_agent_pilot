<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'

defineProps<{
  title: string
  isFocused: boolean
  canClose: boolean
}>()

defineEmits<{
  close: []
  focus: []
  dragstart: [e: DragEvent]
  dragend: []
}>()

const { t } = useI18n()
</script>

<template>
  <div
    :class="['pane-header', { 'pane-header--focused': isFocused }]"
    @click="$emit('focus')"
  >
    <div
      class="pane-header__left"
      draggable="true"
      @dragstart="$emit('dragstart', $event)"
      @dragend="$emit('dragend')"
    >
      <span class="pane-header__drag-handle">
        <EaIcon
          name="grip-vertical"
          :size="12"
        />
      </span>
      <span class="pane-header__title">{{ title || t('splitPane.newPane') }}</span>
    </div>
    <div
      class="pane-header__right"
      @mousedown.stop
    >
      <button
        v-if="canClose"
        class="pane-header__close"
        :title="t('splitPane.closePane')"
        @click.stop="$emit('close')"
      >
        <EaIcon
          name="x"
          :size="12"
        />
      </button>
    </div>
  </div>
</template>

<style scoped>
.pane-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px;
  height: 30px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface);
  user-select: none;
  cursor: pointer;
}

.pane-header--focused {
  border-bottom-color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary-light, var(--color-surface)) 40%, var(--color-surface));
}

.pane-header__left {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  flex: 1;
  cursor: grab;
}

.pane-header__left:active {
  cursor: grabbing;
}

.pane-header__drag-handle {
  display: flex;
  align-items: center;
  color: var(--color-text-quaternary);
  flex-shrink: 0;
  opacity: 0.4;
  transition: opacity var(--transition-fast) var(--easing-default);
}

.pane-header__drag-handle:hover {
  opacity: 1;
}

.pane-header__title {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  pointer-events: none;
}

.pane-header--focused .pane-header__title {
  color: var(--color-primary);
}

.pane-header__right {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

.pane-header__close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-tertiary);
  cursor: pointer;
  opacity: 0;
  transition: all var(--transition-fast) var(--easing-default);
}

.pane-header:hover .pane-header__close {
  opacity: 1;
}

.pane-header__close:hover {
  background: var(--color-surface-hover);
  color: var(--color-text-primary);
}
</style>
