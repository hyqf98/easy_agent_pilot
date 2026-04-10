import * as monaco from 'monaco-editor/esm/vs/editor/editor.api'
import type { CompletionKind, MonacoLanguageId } from '../../types'
import type { MonacoCodeEditorProps } from './types'

const completionKindMap: Record<CompletionKind, monaco.languages.CompletionItemKind> = {
  keyword: monaco.languages.CompletionItemKind.Keyword,
  function: monaco.languages.CompletionItemKind.Function,
  snippet: monaco.languages.CompletionItemKind.Snippet,
  variable: monaco.languages.CompletionItemKind.Variable,
  class: monaco.languages.CompletionItemKind.Class,
  property: monaco.languages.CompletionItemKind.Property
}

const completionTriggerCharactersMap: Partial<Record<MonacoLanguageId, string[]>> = {
  html: ['<', '/', ' ', '"', "'", ':'],
  css: ['.', '#', ':', '-'],
  javascript: ['.', '"', "'", '/', '@'],
  typescript: ['.', '"', "'", '/', '@', ':'],
  json: ['"', ':'],
  markdown: ['#', '`', '['],
  shell: ['$', '-', '.'],
  python: ['.', '_', '@'],
  java: ['.', '@', ':'],
  rust: ['.', ':', '#'],
  yaml: [':', '-']
}

/**
 * 注册当前语言的补全提供器。大文件模式或无补全项时直接返回空。
 */
export function createCompletionProvider(
  props: Readonly<MonacoCodeEditorProps>
): monaco.IDisposable | null {
  if (props.performanceMode === 'large' || !(props.completions?.length)) {
    return null
  }

  return monaco.languages.registerCompletionItemProvider(props.language, {
    triggerCharacters: completionTriggerCharactersMap[props.language],
    provideCompletionItems(textModel, position) {
      const word = textModel.getWordUntilPosition(position)
      const range = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn)

      const suggestions: monaco.languages.CompletionItem[] = props.completions!.map(item => ({
        label: item.label,
        kind: completionKindMap[item.kind ?? 'keyword'],
        insertText: item.insertText,
        detail: item.detail,
        documentation: item.documentation,
        range
      }))

      return { suggestions }
    }
  })
}
