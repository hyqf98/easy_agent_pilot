import type * as monaco from 'monaco-editor/esm/vs/editor/editor.api'
import type { MonacoCodeEditorProps } from './types'

/**
 * 根据编辑器当前模式生成 Monaco 配置，避免大文件模式下启用高成本能力。
 */
export function buildEditorOptions(
  props: Readonly<MonacoCodeEditorProps>
): monaco.editor.IStandaloneEditorConstructionOptions {
  const isLargeFile = props.performanceMode === 'large'

  return {
    fontSize: props.fontSize,
    tabSize: props.tabSize,
    wordWrap: props.wordWrap ? 'on' : 'off',
    readOnly: props.readOnly,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    automaticLayout: true,
    smoothScrolling: !isLargeFile,
    quickSuggestions: isLargeFile
      ? false
      : {
          other: true,
          comments: true,
          strings: true
        },
    suggestOnTriggerCharacters: !isLargeFile,
    snippetSuggestions: isLargeFile ? 'none' : 'inline',
    occurrencesHighlight: isLargeFile ? 'off' : 'singleFile',
    selectionHighlight: !isLargeFile,
    folding: !isLargeFile,
    guides: {
      bracketPairs: !isLargeFile,
      indentation: !isLargeFile
    }
  }
}
