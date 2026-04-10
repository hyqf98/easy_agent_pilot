import { ref } from 'vue'
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api'
import type { MonacoCodeEditorEmits } from './types'

export type SelectionPayload = { text: string; startLine: number; endLine: number } | null

interface SelectionContextController {
  contextMenuState: ReturnType<typeof ref<{ x: number; y: number } | null>>
  handleDocumentKeydown: (event: KeyboardEvent) => void
  handleDocumentPointerDown: (event: MouseEvent) => void
  handleEditorMouseDown: (
    contextKey: monaco.editor.IContextKey<boolean>,
    mouseEvent: monaco.editor.IEditorMouseEvent
  ) => void
  handleSelectionChange: (
    contextKey: monaco.editor.IContextKey<boolean>,
    payload: SelectionPayload
  ) => void
  handleSelectionContextMenu: (event: MouseEvent) => void
  handleSendSelectionFromContextMenu: () => void
  cleanup: () => void
}

/**
 * 封装编辑器选区和自定义右键菜单状态，保证 Monaco 菜单和页面菜单共享同一份选区语义。
 */
export function createSelectionContextController(
  emit: MonacoCodeEditorEmits,
  resolveSelectionPayload: () => SelectionPayload
): SelectionContextController {
  const contextMenuState = ref<{ x: number; y: number } | null>(null)
  let preserveSelectionForContextMenu = false
  let preserveSelectionTimer: number | null = null
  let lastSelectionPayload: SelectionPayload = null

  const hideSelectionContextMenu = (): void => {
    contextMenuState.value = null
  }

  const clearPreserveSelectionTimer = (): void => {
    if (preserveSelectionTimer !== null) {
      window.clearTimeout(preserveSelectionTimer)
      preserveSelectionTimer = null
    }
  }

  const schedulePreserveSelectionReset = (): void => {
    clearPreserveSelectionTimer()
    preserveSelectionTimer = window.setTimeout(() => {
      preserveSelectionForContextMenu = false
      preserveSelectionTimer = null
    }, 240)
  }

  const applySelectionPayload = (
    contextKey: monaco.editor.IContextKey<boolean>,
    payload: SelectionPayload
  ): void => {
    if (!payload) {
      if (preserveSelectionForContextMenu && lastSelectionPayload) {
        contextKey.set(true)
        emit('selection-change', lastSelectionPayload)
        return
      }

      lastSelectionPayload = null
      contextKey.set(false)
      emit('selection-change', null)
      return
    }

    lastSelectionPayload = payload
    contextKey.set(true)
    emit('selection-change', payload)
  }

  const handleSelectionContextMenu = (event: MouseEvent): void => {
    const payload = resolveSelectionPayload() ?? lastSelectionPayload
    if (!payload) {
      hideSelectionContextMenu()
      return
    }

    event.preventDefault()
    event.stopPropagation()
    preserveSelectionForContextMenu = true
    lastSelectionPayload = payload
    schedulePreserveSelectionReset()
    contextMenuState.value = { x: event.clientX, y: event.clientY }
  }

  const handleDocumentPointerDown = (event: MouseEvent): void => {
    const target = event.target
    if (contextMenuState.value && target instanceof Element && target.closest('.monaco-selection-context-menu')) {
      return
    }

    hideSelectionContextMenu()
  }

  const handleDocumentKeydown = (event: KeyboardEvent): void => {
    if (event.key === 'Escape') {
      hideSelectionContextMenu()
    }
  }

  const handleSendSelectionFromContextMenu = (): void => {
    const payload = resolveSelectionPayload() ?? lastSelectionPayload
    hideSelectionContextMenu()
    if (!payload) {
      return
    }

    emit('send-selection', payload)
    preserveSelectionForContextMenu = false
    clearPreserveSelectionTimer()
  }

  const handleEditorMouseDown = (
    contextKey: monaco.editor.IContextKey<boolean>,
    mouseEvent: monaco.editor.IEditorMouseEvent
  ): void => {
    if (!mouseEvent.event.rightButton) {
      preserveSelectionForContextMenu = false
      clearPreserveSelectionTimer()
      return
    }

    const payload = resolveSelectionPayload()
    if (!payload) {
      preserveSelectionForContextMenu = false
      clearPreserveSelectionTimer()
      return
    }

    preserveSelectionForContextMenu = true
    lastSelectionPayload = payload
    contextKey.set(true)
    schedulePreserveSelectionReset()
  }

  const cleanup = (): void => {
    clearPreserveSelectionTimer()
    preserveSelectionForContextMenu = false
    lastSelectionPayload = null
    hideSelectionContextMenu()
  }

  return {
    contextMenuState,
    handleDocumentKeydown,
    handleDocumentPointerDown,
    handleEditorMouseDown,
    handleSelectionChange: applySelectionPayload,
    handleSelectionContextMenu,
    handleSendSelectionFromContextMenu,
    cleanup
  }
}

export function resolveModelSelectionPayload(
  model: monaco.editor.ITextModel,
  selection: monaco.Selection | monaco.Range
): SelectionPayload {
  const text = model.getValueInRange(selection)
  if (!text.trim()) {
    return null
  }

  return {
    text,
    startLine: Math.min(selection.startLineNumber, selection.endLineNumber),
    endLine: Math.max(selection.startLineNumber, selection.endLineNumber)
  }
}
