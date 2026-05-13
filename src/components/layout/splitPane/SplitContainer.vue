<script setup lang="ts">
import { computed, ref, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import PaneWrapper from './PaneWrapper.vue'
import { useSplitPaneStore } from '@/stores/splitPane'

interface DropTarget {
  row: number
  col: number
  position: 'before' | 'after' | 'new-row-below' | 'new-row'
}

type PreviewCell =
  | { type: 'pane'; paneId: string }
  | { type: 'ghost' }

const DRAG_Y_SPLIT = 0.5

const { t } = useI18n()
const splitPaneStore = useSplitPaneStore()

const containerRef = ref<HTMLElement | null>(null)
const draggingPaneId = ref<string | null>(null)
const dropTarget = ref<DropTarget | null>(null)

let onMouseMove: ((e: MouseEvent) => void) | null = null
let onMouseUp: (() => void) | null = null

const paneGrid = computed(() => splitPaneStore.paneGrid)
const rowCount = computed(() => paneGrid.value.length)
const rowHeight = computed(() => {
  if (rowCount.value === 0) return 100
  return 100 / rowCount.value
})

function getPaneSessionId(paneId: string): string {
  const pane = splitPaneStore.getPaneById(paneId)
  return pane?.sessionId ?? ''
}

const previewGrid = computed(() => {
  if (!draggingPaneId.value || !dropTarget.value) return null

  const pid = draggingPaneId.value
  const dt = dropTarget.value
  const grid = splitPaneStore.paneGrid

  const result: PreviewCell[][] = []
  for (let r = 0; r < grid.length; r++) {
    const row: PreviewCell[] = []
    for (let c = 0; c < grid[r].length; c++) {
      const id = grid[r][c]
      if (id === pid) continue
      row.push({ type: 'pane', paneId: id })
    }
    result.push(row)
  }

  const ghost: PreviewCell = { type: 'ghost' }

  if (dt.position === 'new-row') {
    result.push([ghost])
  } else if (dt.position === 'new-row-below') {
    const insertRowAt = dt.row + 1
    const targetRow = result[dt.row]
    if (targetRow) {
      result.splice(insertRowAt, 0, [ghost])
    } else {
      result.push([ghost])
    }
  } else {
    let targetRow = result[dt.row]
    if (!targetRow) {
      targetRow = []
      result[dt.row] = targetRow
    }

    let insertCol = dt.col
    const actualCount = targetRow.length
    if (dt.position === 'after') {
      insertCol = Math.min(dt.col + 1, actualCount)
    }
    if (insertCol < 0) insertCol = 0
    if (insertCol > actualCount) insertCol = actualCount

    targetRow.splice(insertCol, 0, ghost)
  }

  return result.filter(row => row.length > 0)
})

const previewRowCount = computed(() => {
  if (!previewGrid.value) return rowCount.value
  return previewGrid.value.length
})

const previewRowHeight = computed(() => {
  const count = previewRowCount.value
  if (count === 0) return 100
  return 100 / count
})

function onPaneClose(paneId: string) {
  splitPaneStore.removePane(paneId)
  if (splitPaneStore.paneCount <= 1) {
    splitPaneStore.exitSplitMode()
  }
}

function onPaneDragStart(paneId: string) {
  draggingPaneId.value = paneId
  dropTarget.value = null

  onMouseMove = (e: MouseEvent) => {
    if (!draggingPaneId.value) return
    updateDropTarget(e.clientX, e.clientY)
  }

  onMouseUp = () => {
    finishDrag()
  }

  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
  document.body.style.userSelect = 'none'
  document.body.style.cursor = 'grabbing'
}

function updateDropTarget(clientX: number, clientY: number) {
  const container = containerRef.value
  if (!container) return

  const rows = container.querySelectorAll('.split-row')
  if (rows.length === 0) return

  const grid = splitPaneStore.paneGrid

  const lastRowRect = rows[rows.length - 1].getBoundingClientRect()
  if (clientY > lastRowRect.bottom) {
    dropTarget.value = { row: grid.length, col: 0, position: 'new-row' }
    return
  }

  let hitRow = -1
  for (let r = 0; r < rows.length; r++) {
    const rr = rows[r].getBoundingClientRect()
    if (clientY >= rr.top && clientY <= rr.bottom) {
      hitRow = r
      break
    }
  }

  if (hitRow < 0) {
    dropTarget.value = null
    return
  }

  const rowWrappers = rows[hitRow].querySelectorAll<HTMLElement>('.split-pane-wrapper')
  const paneCount = rowWrappers.length

  let hitCol = -1
  let hitRect: DOMRect | null = null

  for (let c = 0; c < paneCount; c++) {
    const pid = rowWrappers[c].dataset.paneId
    if (pid === draggingPaneId.value) continue
    const wr = rowWrappers[c].getBoundingClientRect()
    if (clientX >= wr.left && clientX <= wr.right) {
      hitCol = c
      hitRect = wr
      break
    }
  }

  if (hitCol < 0 && paneCount > 0) {
    const firstW = rowWrappers[0].getBoundingClientRect()
    const lastW = rowWrappers[paneCount - 1].getBoundingClientRect()
    if (clientX < firstW.left) {
      hitCol = 0
      hitRect = firstW
    } else if (clientX > lastW.right) {
      hitCol = paneCount - 1
      hitRect = lastW
    }
  }

  if (hitCol < 0 || !hitRect) {
    dropTarget.value = null
    return
  }

  const hitPaneId = rowWrappers[hitCol].dataset.paneId
  if (hitPaneId === draggingPaneId.value) {
    dropTarget.value = null
    return
  }

  const yRatio = (clientY - hitRect.top) / hitRect.height

  if (yRatio > DRAG_Y_SPLIT) {
    const srcPos = splitPaneStore.getPanePosition(draggingPaneId.value!)
    const isLastInRow = srcPos && srcPos.row === hitRow && srcPos.col === rowWrappers.length - 1
    if (isLastInRow && paneCount <= 1) {
      dropTarget.value = null
      return
    }
    const actualCol = parseInt(rowWrappers[hitCol].dataset.col || '0')
    dropTarget.value = { row: hitRow, col: actualCol, position: 'new-row-below' }
  } else {
    const midX = (hitRect.left + hitRect.right) / 2
    const pos = clientX < midX ? 'before' : 'after'
    const actualCol = parseInt(rowWrappers[hitCol].dataset.col || '0')
    dropTarget.value = { row: hitRow, col: actualCol, position: pos }
  }
}

function finishDrag() {
  if (onMouseMove) document.removeEventListener('mousemove', onMouseMove)
  if (onMouseUp) document.removeEventListener('mouseup', onMouseUp)
  document.body.style.userSelect = ''
  document.body.style.cursor = ''

  if (draggingPaneId.value && dropTarget.value) {
    const dt = dropTarget.value
    const pid = draggingPaneId.value
    const grid = splitPaneStore.paneGrid

    if (dt.position === 'new-row' || dt.position === 'new-row-below') {
      splitPaneStore.movePaneToNewRow(pid)
    } else if (dt.position === 'before') {
      const targetId = grid[dt.row]?.[dt.col]
      if (targetId && targetId !== pid) {
        splitPaneStore.movePaneBefore(pid, targetId)
      }
    } else if (dt.position === 'after') {
      const targetId = grid[dt.row]?.[dt.col]
      if (targetId && targetId !== pid) {
        splitPaneStore.movePaneAfter(pid, targetId)
      }
    }
  }

  draggingPaneId.value = null
  dropTarget.value = null
  onMouseMove = null
  onMouseUp = null
}

onUnmounted(() => {
  if (onMouseMove) document.removeEventListener('mousemove', onMouseMove)
  if (onMouseUp) document.removeEventListener('mouseup', onMouseUp)
})
</script>

<template>
  <div
    ref="containerRef"
    class="split-container"
  >
    <!-- Dragging: show preview grid with ghost -->
    <template v-if="previewGrid && draggingPaneId">
      <div
        v-for="(row, rowIdx) in previewGrid"
        :key="'preview-row-' + rowIdx"
        class="split-row split-row--preview"
        :style="{ height: previewRowHeight + '%' }"
      >
        <template v-for="(cell, colIdx) in row" :key="'preview-' + rowIdx + '-' + colIdx">
          <div
            v-if="cell.type === 'ghost'"
            class="split-ghost"
          >
            <div class="split-ghost__inner">
              <EaIcon
                name="plus"
                :size="20"
              />
              <span class="split-ghost__label">{{ getPaneSessionId(draggingPaneId!) ? '' : t('splitPane.newPane') }}</span>
            </div>
          </div>
          <div
            v-else
            class="split-pane-wrapper split-pane-wrapper--shrunk"
            :data-pane-id="cell.paneId"
          >
            <PaneWrapper
              :pane-id="cell.paneId"
              :session-id="getPaneSessionId(cell.paneId)"
              @close="onPaneClose"
              @dragstart="onPaneDragStart"
            />
          </div>
        </template>
      </div>
    </template>

    <!-- Not dragging: normal grid -->
    <template v-else>
      <div
        v-for="(row, rowIdx) in paneGrid"
        :key="'row-' + rowIdx"
        class="split-row"
        :style="{ height: rowHeight + '%' }"
      >
        <div
          v-for="(paneId, colIdx) in row"
          :key="paneId"
          class="split-pane-wrapper"
          :data-pane-id="paneId"
          :data-row="rowIdx"
          :data-col="colIdx"
        >
          <PaneWrapper
            :pane-id="paneId"
            :session-id="getPaneSessionId(paneId)"
            @close="onPaneClose"
            @dragstart="onPaneDragStart"
          />
        </div>
      </div>
    </template>

    <!-- Full-overlay to capture mouse events -->
    <div
      v-if="draggingPaneId"
      class="drag-overlay"
    />
  </div>
</template>

<style scoped>
.split-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  position: relative;
}

.split-row {
  display: flex;
  min-height: 0;
  overflow: hidden;
}

.split-row--preview {
  transition: height 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.split-row + .split-row {
  border-top: 2px solid var(--color-border);
}

.split-pane-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 200px;
  min-height: 0;
  overflow: hidden;
  position: relative;
}

.split-pane-wrapper + .split-pane-wrapper {
  border-left: 2px solid var(--color-border);
}

.split-pane-wrapper--shrunk {
  transition: flex 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.split-pane-wrapper--shrunk + .split-pane-wrapper--shrunk,
.split-ghost + .split-pane-wrapper--shrunk,
.split-pane-wrapper--shrunk + .split-ghost {
  border-left: 2px solid var(--color-border);
}

.split-ghost {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 200px;
  min-height: 0;
  overflow: hidden;
  position: relative;
  padding: 8px;
  transition: flex 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.split-ghost__inner {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  border: 2px dashed var(--color-primary);
  border-radius: var(--radius-lg);
  color: var(--color-primary);
  animation: ghost-pulse 1.5s ease-in-out infinite;
}

.split-ghost__label {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

@keyframes ghost-pulse {
  0%, 100% {
    background: color-mix(in srgb, var(--color-primary) 8%, transparent);
    border-color: color-mix(in srgb, var(--color-primary) 50%, transparent);
  }
  50% {
    background: color-mix(in srgb, var(--color-primary) 15%, transparent);
    border-color: var(--color-primary);
  }
}

.drag-overlay {
  position: fixed;
  inset: 0;
  z-index: 999;
  cursor: grabbing;
}
</style>
