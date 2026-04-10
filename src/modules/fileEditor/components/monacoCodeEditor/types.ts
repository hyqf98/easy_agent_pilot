import type { CompletionEntry, MonacoLanguageId } from '../../types'

export interface EditorHighlightRange {
  startLine: number
  endLine: number
  startColumn?: number
  endColumn?: number
  className?: string
  isWholeLine?: boolean
}

export interface MonacoCodeEditorProps {
  modelValue: string
  language: MonacoLanguageId
  fontSize: number
  tabSize: number
  wordWrap: boolean
  performanceMode?: 'default' | 'large'
  completions?: CompletionEntry[]
  readOnly?: boolean
  highlightedRanges?: EditorHighlightRange[]
  focusRange?: EditorHighlightRange | null
}

export type MonacoCodeEditorEmits = {
  (event: 'update:modelValue', value: string): void
  (event: 'save-shortcut'): void
  (event: 'selection-change', value: { text: string; startLine: number; endLine: number } | null): void
  (event: 'send-selection', value: { text: string; startLine: number; endLine: number }): void
}
