<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api'
import { useThemeStore } from '@/stores/theme'
import { applyMonacoTheme, ensureMonacoSetup } from '../monaco/setup'
import type { CompletionEntry, CompletionKind, MonacoLanguageId } from '../types'

interface EditorHighlightRange {
  startLine: number
  endLine: number
  startColumn?: number
  endColumn?: number
  className?: string
  isWholeLine?: boolean
}

interface MonacoCodeEditorProps {
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

const props = withDefaults(defineProps<MonacoCodeEditorProps>(), {
  performanceMode: 'default',
  completions: () => [],
  readOnly: false,
  highlightedRanges: () => [],
  focusRange: null
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'save-shortcut': []
}>()

const themeStore = useThemeStore()

const containerRef = ref<HTMLDivElement | null>(null)
let editor: monaco.editor.IStandaloneCodeEditor | null = null
let model: monaco.editor.ITextModel | null = null
let completionProviderDisposable: monaco.IDisposable | null = null
let decorationCollection: monaco.editor.IEditorDecorationsCollection | null = null
let isSyncingFromOutside = false
let renderTimer: number | null = null

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

const resolveTheme = (): 'easy-agent-light' | 'easy-agent-dark' => {
  return themeStore.isDark ? 'easy-agent-dark' : 'easy-agent-light'
}

const buildEditorOptions = (): monaco.editor.IStandaloneEditorConstructionOptions => {
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

const registerCompletionProvider = (): void => {
  completionProviderDisposable?.dispose()
  completionProviderDisposable = null

  if (props.performanceMode === 'large' || !props.completions.length) {
    return
  }

  completionProviderDisposable = monaco.languages.registerCompletionItemProvider(props.language, {
    triggerCharacters: completionTriggerCharactersMap[props.language],
    provideCompletionItems(textModel, position) {
      const word = textModel.getWordUntilPosition(position)
      const range = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn)

      const suggestions: monaco.languages.CompletionItem[] = props.completions.map(item => ({
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

const updateEditorOptions = (): void => {
  if (!editor) {
    return
  }

  editor.updateOptions(buildEditorOptions())
}

const updateDecorations = (): void => {
  if (!editor || !model) {
    return
  }

  if (!decorationCollection) {
    decorationCollection = editor.createDecorationsCollection()
  }

  decorationCollection.set(props.highlightedRanges.map((range) => ({
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
}

const revealFocusRange = (): void => {
  if (!editor || !props.focusRange) {
    return
  }

  const range = new monaco.Range(
    props.focusRange.startLine,
    props.focusRange.startColumn ?? 1,
    props.focusRange.endLine,
    props.focusRange.endColumn ?? 1
  )
  editor.revealRangeInCenter(range, monaco.editor.ScrollType.Smooth)
  editor.setPosition({
    lineNumber: props.focusRange.startLine,
    column: props.focusRange.startColumn ?? 1
  })
}

const flushEditorRender = (): void => {
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

onMounted(() => {
  ensureMonacoSetup()
  applyMonacoTheme(themeStore.isDark)

  if (!containerRef.value) {
    return
  }

  model = monaco.editor.createModel(props.modelValue, props.language)
  monaco.editor.setModelLanguage(model, props.language)

  editor = monaco.editor.create(containerRef.value, {
    model,
    theme: resolveTheme(),
    ...buildEditorOptions()
  })

  decorationCollection = editor.createDecorationsCollection()

  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
    emit('save-shortcut')
  })

  editor.onDidChangeModelContent(() => {
    if (isSyncingFromOutside || !model) {
      return
    }

    emit('update:modelValue', model.getValue())
  })

  registerCompletionProvider()
  updateDecorations()
  revealFocusRange()
  scheduleEditorRender()
})

watch(() => props.modelValue, nextValue => {
  if (!model || nextValue === model.getValue()) {
    return
  }

  isSyncingFromOutside = true
  model.setValue(nextValue)
  isSyncingFromOutside = false
  scheduleEditorRender()
})

watch(() => props.language, nextLanguage => {
  if (!model) {
    return
  }

  monaco.editor.setModelLanguage(model, nextLanguage)
  registerCompletionProvider()
  scheduleEditorRender()
})

watch(() => props.completions, () => {
  registerCompletionProvider()
}, { deep: true })

watch(() => [props.fontSize, props.tabSize, props.wordWrap, props.readOnly, props.performanceMode], () => {
  updateEditorOptions()
  registerCompletionProvider()
})

watch(() => props.highlightedRanges, () => {
  updateDecorations()
  scheduleEditorRender()
}, { deep: true })

watch(() => props.focusRange, () => {
  revealFocusRange()
  scheduleEditorRender()
}, { deep: true })

watch(
  () => [themeStore.isDark, themeStore.currentThemeColorId] as const,
  ([isDark]) => {
    applyMonacoTheme(isDark)
  }
)

onUnmounted(() => {
  if (renderTimer !== null) {
    window.clearTimeout(renderTimer)
    renderTimer = null
  }

  decorationCollection?.clear()
  decorationCollection = null
  completionProviderDisposable?.dispose()
  completionProviderDisposable = null

  editor?.dispose()
  editor = null

  model?.dispose()
  model = null
})
</script>

<template>
  <div
    ref="containerRef"
    class="monaco-editor-wrapper"
  />
</template>

<style scoped>
.monaco-editor-wrapper {
  width: 100%;
  height: 100%;
  min-height: 0;
}

:global(.monaco-editor .ea-monaco-highlight) {
  background: linear-gradient(90deg, rgba(37, 99, 235, 0.16), rgba(37, 99, 235, 0.05));
  border-left: 3px solid rgba(37, 99, 235, 0.72);
}

:global([data-theme='dark'] .monaco-editor .ea-monaco-highlight),
:global(.dark .monaco-editor .ea-monaco-highlight) {
  background: linear-gradient(90deg, rgba(147, 197, 253, 0.18), rgba(147, 197, 253, 0.06));
  border-left-color: rgba(147, 197, 253, 0.82);
}
</style>
