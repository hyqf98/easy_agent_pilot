import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useSessionStore } from './session'

export interface PaneInfo {
  id: string
  sessionId: string
}

let paneSeq = 0

export const useSplitPaneStore = defineStore('splitPane', () => {
  const activePanes = ref<PaneInfo[]>([])
  const paneGrid = ref<string[][]>([])
  const focusedPaneId = ref<string | null>(null)

  const isSplitActive = computed(() => activePanes.value.length > 1)

  const paneCount = computed(() => activePanes.value.length)

  const rowCount = computed(() => paneGrid.value.length)

  const focusedPane = computed(() =>
    activePanes.value.find(p => p.id === focusedPaneId.value) ?? null
  )

  function getPaneById(paneId: string): PaneInfo | undefined {
    return activePanes.value.find(p => p.id === paneId)
  }

  function getPanePosition(paneId: string): { row: number; col: number } | null {
    for (let r = 0; r < paneGrid.value.length; r++) {
      for (let c = 0; c < paneGrid.value[r].length; c++) {
        if (paneGrid.value[r][c] === paneId) {
          return { row: r, col: c }
        }
      }
    }
    return null
  }

  function addPane(sessionId: string): PaneInfo {
    paneSeq++
    const pane: PaneInfo = {
      id: `pane-${paneSeq}`,
      sessionId
    }
    activePanes.value.push(pane)

    if (paneGrid.value.length === 0) {
      paneGrid.value.push([])
    }
    paneGrid.value[paneGrid.value.length - 1].push(pane.id)
    focusedPaneId.value = pane.id
    return pane
  }

  function addPaneToNewRow(sessionId: string): PaneInfo {
    paneSeq++
    const pane: PaneInfo = {
      id: `pane-${paneSeq}`,
      sessionId
    }
    activePanes.value.push(pane)
    paneGrid.value.push([pane.id])
    focusedPaneId.value = pane.id
    return pane
  }

  function removePane(paneId: string) {
    const idx = activePanes.value.findIndex(p => p.id === paneId)
    if (idx < 0) return

    activePanes.value.splice(idx, 1)

    for (let r = 0; r < paneGrid.value.length; r++) {
      for (let c = paneGrid.value[r].length - 1; c >= 0; c--) {
        if (paneGrid.value[r][c] === paneId) {
          paneGrid.value[r].splice(c, 1)
          break
        }
      }
    }
    paneGrid.value = paneGrid.value.filter(row => row.length > 0)

    if (focusedPaneId.value === paneId) {
      if (activePanes.value.length > 0) {
        const nextIdx = Math.min(idx, activePanes.value.length - 1)
        focusedPaneId.value = activePanes.value[nextIdx].id
      } else {
        focusedPaneId.value = null
      }
    }
  }

  function movePaneToNewRow(paneId: string) {
    const pos = getPanePosition(paneId)
    if (!pos) return
    paneGrid.value[pos.row].splice(pos.col, 1)
    if (paneGrid.value[pos.row].length === 0) {
      paneGrid.value.splice(pos.row, 1)
    }
    paneGrid.value.push([paneId])
    paneGrid.value = paneGrid.value.filter(row => row.length > 0)
    focusedPaneId.value = paneId
  }

  function movePaneBefore(targetPaneId: string, beforePaneId: string) {
    if (targetPaneId === beforePaneId) return
    const srcPos = getPanePosition(targetPaneId)
    if (!srcPos) return
    paneGrid.value[srcPos.row].splice(srcPos.col, 1)

    const destPos = getPanePosition(beforePaneId)
    if (!destPos) {
      if (paneGrid.value[srcPos.row].length === 0) {
        paneGrid.value.splice(srcPos.row, 1)
      }
      paneGrid.value.push([targetPaneId])
      focusedPaneId.value = targetPaneId
      return
    }

    let insertCol = destPos.col
    if (srcPos.row === destPos.row && srcPos.col < destPos.col) {
      insertCol = destPos.col - 1
    }

    if (paneGrid.value[srcPos.row].length === 0) {
      paneGrid.value.splice(srcPos.row, 1)
      if (srcPos.row < destPos.row) {
        destPos.row--
      }
    }
    paneGrid.value[destPos.row].splice(insertCol, 0, targetPaneId)
    paneGrid.value = paneGrid.value.filter(row => row.length > 0)
    focusedPaneId.value = targetPaneId
  }

  function movePaneAfter(targetPaneId: string, afterPaneId: string) {
    if (targetPaneId === afterPaneId) return
    const srcPos = getPanePosition(targetPaneId)
    if (!srcPos) return
    paneGrid.value[srcPos.row].splice(srcPos.col, 1)

    const destPos = getPanePosition(afterPaneId)
    if (!destPos) {
      if (paneGrid.value[srcPos.row].length === 0) {
        paneGrid.value.splice(srcPos.row, 1)
      }
      paneGrid.value.push([targetPaneId])
      focusedPaneId.value = targetPaneId
      return
    }

    let insertCol = destPos.col + 1
    if (srcPos.row === destPos.row && srcPos.col <= destPos.col) {
      insertCol = destPos.col
    }

    if (paneGrid.value[srcPos.row].length === 0) {
      paneGrid.value.splice(srcPos.row, 1)
      if (srcPos.row < destPos.row) {
        destPos.row--
      }
    }
    paneGrid.value[destPos.row].splice(insertCol, 0, targetPaneId)
    paneGrid.value = paneGrid.value.filter(row => row.length > 0)
    focusedPaneId.value = targetPaneId
  }

  function focusPane(paneId: string) {
    const pane = getPaneById(paneId)
    if (pane) {
      focusedPaneId.value = paneId
      const sessionStore = useSessionStore()
      sessionStore.setCurrentSession(pane.sessionId)
    }
  }

  function updatePaneSession(paneId: string, sessionId: string) {
    const pane = getPaneById(paneId)
    if (pane) {
      pane.sessionId = sessionId
    }
  }

  function exitSplitMode() {
    const focused = focusedPane.value
    if (focused) {
      const sessionStore = useSessionStore()
      sessionStore.setCurrentSession(focused.sessionId)
    }
    activePanes.value = []
    paneGrid.value = []
    focusedPaneId.value = null
  }

  function enterSplitMode(sessionId: string, secondSessionId?: string) {
    const sessionStore = useSessionStore()
    const s1 = secondSessionId
      ? sessionId
      : (sessionStore.currentSessionId ?? sessionId)
    const s2 = secondSessionId ?? sessionId

    activePanes.value = []
    paneGrid.value = []

    addPane(s1)
    addPane(s2)
  }

  return {
    activePanes,
    paneGrid,
    focusedPaneId,
    isSplitActive,
    paneCount,
    rowCount,
    focusedPane,
    getPaneById,
    getPanePosition,
    addPane,
    addPaneToNewRow,
    removePane,
    movePaneToNewRow,
    movePaneBefore,
    movePaneAfter,
    focusPane,
    updatePaneSession,
    exitSplitMode,
    enterSplitMode
  }
})
