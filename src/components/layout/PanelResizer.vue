<script setup lang="ts">
import { ref, onUnmounted } from 'vue'

export interface PanelResizerProps {
  direction?: 'left' | 'right'
  minWidth?: number
  maxWidth?: number
  disabled?: boolean
  currentWidth?: number
}

const props = withDefaults(defineProps<PanelResizerProps>(), {
  direction: 'right',
  minWidth: 100,
  maxWidth: 400,
  disabled: false,
  currentWidth: 0
})

const emit = defineEmits<{
  resize: [delta: number]
  resizeEnd: [width: number]
  resizeStart: []
}>()

const isDragging = ref(false)
const startX = ref(0)
const startWidth = ref(0)
let rafId = 0
let pendingDelta = 0
let hasPending = false

function flushDelta() {
  if (hasPending) {
    emit('resize', pendingDelta)
    hasPending = false
  }
}

const handleMouseDown = (e: MouseEvent) => {
  if (props.disabled) return

  e.preventDefault()
  isDragging.value = true
  startX.value = e.clientX
  startWidth.value = props.currentWidth
  emit('resizeStart')
  document.addEventListener('mousemove', handleMouseMove, { passive: true })
  document.addEventListener('mouseup', handleMouseUp)
}

const handleMouseMove = (e: MouseEvent) => {
  if (!isDragging.value) return

  const delta = props.direction === 'right'
    ? e.clientX - startX.value
    : startX.value - e.clientX

  pendingDelta = delta
  if (!hasPending) {
    hasPending = true
    rafId = requestAnimationFrame(flushDelta)
  }
}

const handleMouseUp = (e: MouseEvent) => {
  if (!isDragging.value) return

  cancelAnimationFrame(rafId)
  hasPending = false
  isDragging.value = false

  const delta = props.direction === 'right'
    ? e.clientX - startX.value
    : startX.value - e.clientX

  emit('resizeEnd', startWidth.value + delta)
  document.removeEventListener('mousemove', handleMouseMove)
  document.removeEventListener('mouseup', handleMouseUp)
}

onUnmounted(() => {
  cancelAnimationFrame(rafId)
  document.removeEventListener('mousemove', handleMouseMove)
  document.removeEventListener('mouseup', handleMouseUp)
})
</script>

<template>
  <div
    v-if="!disabled"
    :class="['panel-resizer', { 'panel-resizer--dragging': isDragging }]"
    @mousedown="handleMouseDown"
  >
    <div class="panel-resizer__line" />
  </div>
</template>

<style scoped>
.panel-resizer {
  width: 4px;
  height: 100%;
  cursor: col-resize;
  flex-shrink: 0;
  position: relative;
  z-index: 10;
}

.panel-resizer:hover .panel-resizer__line,
.panel-resizer--dragging .panel-resizer__line {
  opacity: 1;
}

.panel-resizer--dragging {
  cursor: col-resize;
}

.panel-resizer__line {
  position: absolute;
  left: 50%;
  top: 0;
  bottom: 0;
  width: 2px;
  background-color: var(--color-primary);
  transform: translateX(-50%);
  opacity: 0;
  transition: opacity var(--transition-fast) var(--easing-default);
}
</style>
