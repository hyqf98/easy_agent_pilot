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

type CellRole = 'normal' | 'dragging' | 'ghost'

interface GridCell {
  key: string
  role: CellRole
  paneId: string | null
}

interface DisplayRow {
  key: string
  cells: GridCell[]
}

const DRAG_Y_SPLIT = 0.5

const { t } = useI18n()
const splitPaneStore = useSplitPaneStore()

const containerRef = ref<HTMLElement | null>(null)
const draggingPaneId = ref<string | null>(null)
const dropTarget = ref<DropTarget | null>(null)

let onMouseMove: ((e: MouseEvent) => void) | null = null
let onMouseUp: (() => void) | null = null
let rafId = 0
let pendingX = 0
let pendingY = 0
let hasPendingMove = false

function getPaneSessionId(paneId: string): string {
  const pane = splitPaneStore.getPaneById(paneId)
  return pane?.sessionId ?? ''
}

const displayRows = computed<DisplayRow[]>(() => {
  const grid = splitPaneStore.paneGrid
  const pid = draggingPaneId.value
  const dt = dropTarget.value
  const isDragging = !!pid

  if (!isDragging || !dt) {
    return grid.map((row, r) => ({
      key: `row-${r}`,
      cells: row.map(id => ({ key: id, role: 'normal' as CellRole, paneId: id }))
    }))
  }

  const ghost: GridCell = { key: 'ghost', role: 'ghost', paneId: null }
  const sourceRows: GridCell[][] = []

  for (let r = 0; r < grid.length; r++) {
    const row: GridCell[] = []
    for (let c = 0; c < grid[r].length; c++) {
      const id = grid[r][c]
      row.push({
        key: id,
        role: id === pid ? 'dragging' : 'normal',
        paneId: id
      })
    }
    sourceRows.push(row)
  }

  if (dt.position === 'new-row') {
    sourceRows.push([ghost])
  } else if (dt.position === 'new-row-below') {
    const insertAt = Math.min(dt.row + 1, sourceRows.length)
    sourceRows.splice(insertAt, 0, [ghost])
  } else {
    const targetRow = sourceRows[dt.row]
    if (targetRow) {
      let insertCol = dt.position === 'after' ? Math.min(dt.col + 1, targetRow.length) : dt.col
      if (insertCol < 0) insertCol = 0
      targetRow.splice(insertCol, 0, ghost)
    }
  }

  return sourceRows
    .filter(row => row.length > 0)
    .map((row, r) => ({ key: `row-${r}`, cells: row }))
})

function onPaneClose(paneId: string) {
  splitPaneStore.removePane(paneId)
  if (splitPaneStore.paneCount <= 1) {
    splitPaneStore.exitSplitMode()
  }
}

function flushMove() {
  if (hasPendingMove) {
    hasPendingMove = false
    updateDropTarget(pendingX, pendingY)
  }
}

function onPaneDragStart(paneId: string) {
  draggingPaneId.value = paneId
  dropTarget.value = null

  onMouseMove = (e: MouseEvent) => {
    if (!draggingPaneId.value) return
    pendingX = e.clientX
    pendingY = e.clientY
    if (!hasPendingMove) {
      hasPendingMove = true
      rafId = requestAnimationFrame(flushMove)
    }
  }

  onMouseUp = () => {
    finishDrag()
  }

  document.addEventListener('mousemove', onMouseMove, { passive: true })
  document.addEventListener('mouseup', onMouseUp)
  document.body.style.userSelect = 'none'
  document.body.style.cursor = 'grabbing'
}

function updateDropTarget(clientX: number, clientY: number) {
  const container = containerRef.value
  if (!container) return

  const allRows = container.querySelectorAll('.split-row')
  if (allRows.length === 0) return

  const grid = splitPaneStore.paneGrid

  const lastRowRect = allRows[allRows.length - 1].getBoundingClientRect()
  if (clientY > lastRowRect.bottom) {
    dropTarget.value = { row: grid.length, col: 0, position: 'new-row' }
    return
  }

  let hitRow = -1
  for (let r = 0; r < allRows.length; r++) {
    const rr = allRows[r].getBoundingClientRect()
    if (clientY >= rr.top && clientY <= rr.bottom) {
      hitRow = r
      break
    }
  }

  if (hitRow < 0) {
    dropTarget.value = null
    return
  }

  const rowWrappers = allRows[hitRow].querySelectorAll<HTMLElement>('.split-pane-wrapper')
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
  cancelAnimationFrame(rafId)
  hasPendingMove = false
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
  cancelAnimationFrame(rafId)
  if (onMouseMove) document.removeEventListener('mousemove', onMouseMove)
  if (onMouseUp) document.removeEventListener('mouseup', onMouseUp)
})
</script>

<template>
  <div
    ref="containerRef"
    class="split-container"
  >
    <div
      v-for="(row, rowIdx) in displayRows"
      :key="row.key"
      class="split-row"
    >
      <template v-for="(cell, colIdx) in row.cells" :key="cell.key">
        <div
          v-if="cell.role === 'ghost'"
          class="split-ghost"
        >
          <div class="split-ghost__inner">
            <EaIcon
              name="plus"
              :size="20"
            />
            <span class="split-ghost__label">{{ t('splitPane.newPane') }}</span>
          </div>
        </div>
        <div
          v-else
          class="split-pane-wrapper"
          :class="{ 'split-pane-wrapper--shrunk': cell.role === 'dragging' }"
          :data-pane-id="cell.paneId"
          :data-row="rowIdx"
          :data-col="colIdx"
        >
          <PaneWrapper
            :pane-id="cell.paneId!"
            :session-id="getPaneSessionId(cell.paneId!)"
            @close="onPaneClose"
            @dragstart="onPaneDragStart"
          />
        </div>
      </template>
    </div>

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
  flex: 1;
  display: flex;
  min-height: 0;
  overflow: hidden;
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
  opacity: 0.3;
  pointer-events: none;
  min-width: 60px;
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
}

.split-ghost + .split-pane-wrapper,
.split-pane-wrapper + .split-ghost {
  border-left: 2px solid var(--color-border);
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
