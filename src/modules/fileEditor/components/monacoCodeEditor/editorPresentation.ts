import * as monaco from 'monaco-editor/esm/vs/editor/editor.api'
import type { EditorHighlightRange } from './types'

/**
 * 统一维护编辑器装饰、高亮跳转和延迟重绘，避免这些 UI 细节散落在主 composable 中。
 */
export function updateEditorDecorations(
  editor: monaco.editor.IStandaloneCodeEditor,
  decorationCollection: monaco.editor.IEditorDecorationsCollection | null,
  highlightedRanges: EditorHighlightRange[] = []
): monaco.editor.IEditorDecorationsCollection {
  const nextCollection = decorationCollection ?? editor.createDecorationsCollection()

  nextCollection.set(highlightedRanges.map(range => ({
    range: new monaco.Range(
      range.startLine,
      range.startColumn ?? 1,
      range.endLine,
      range.endColumn ?? Number.MAX_SAFE_INTEGER
    ),
    options: {
      isWholeLine: range.isWholeLine ?? true,
      className: range.className ?? 'ea-monaco-highlight',
      minimap: {
        color: 'rgba(59, 130, 246, 0.45)',
        position: monaco.editor.MinimapPosition.Inline
      },
      overviewRuler: {
        color: 'rgba(59, 130, 246, 0.6)',
        position: monaco.editor.OverviewRulerLane.Center
      }
    }
  })))

  return nextCollection
}

export function revealEditorFocusRange(
  editor: monaco.editor.IStandaloneCodeEditor,
  focusRange: EditorHighlightRange | null | undefined
): void {
  if (!focusRange) {
    return
  }

  const range = new monaco.Range(
    focusRange.startLine,
    focusRange.startColumn ?? 1,
    focusRange.endLine,
    focusRange.endColumn ?? 1
  )
  editor.revealRangeInCenter(range, monaco.editor.ScrollType.Smooth)
  editor.setPosition({
    lineNumber: focusRange.startLine,
    column: focusRange.startColumn ?? 1
  })
}

export function createEditorRenderScheduler(
  getEditor: () => monaco.editor.IStandaloneCodeEditor | null
) {
  let renderTimer: number | null = null

  const flushEditorRender = (): void => {
    const editor = getEditor()
    editor?.layout()
    editor?.render(true)
  }

  const scheduleEditorRender = (): void => {
    requestAnimationFrame(() => {
      flushEditorRender()
    })
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        flushEditorRender()
      })
    })

    if (renderTimer !== null) {
      window.clearTimeout(renderTimer)
    }

    renderTimer = window.setTimeout(() => {
      flushEditorRender()
      renderTimer = null
    }, 80)
  }

  const cleanup = (): void => {
    if (renderTimer !== null) {
      window.clearTimeout(renderTimer)
      renderTimer = null
    }
  }

  return {
    cleanup,
    scheduleEditorRender
  }
}
