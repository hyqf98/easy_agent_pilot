import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker'
import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker'
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'
import 'monaco-editor/min/vs/editor/editor.main.css'

import * as monaco from 'monaco-editor/esm/vs/editor/editor.api'
import 'monaco-editor/esm/vs/language/typescript/monaco.contribution'
import 'monaco-editor/esm/vs/language/json/monaco.contribution'
import 'monaco-editor/esm/vs/language/css/monaco.contribution'
import 'monaco-editor/esm/vs/language/html/monaco.contribution'
import 'monaco-editor/esm/vs/basic-languages/html/html.contribution'
import 'monaco-editor/esm/vs/basic-languages/css/css.contribution'
import 'monaco-editor/esm/vs/basic-languages/javascript/javascript.contribution'
import 'monaco-editor/esm/vs/basic-languages/typescript/typescript.contribution'
import 'monaco-editor/esm/vs/basic-languages/xml/xml.contribution'

import 'monaco-editor/esm/vs/basic-languages/python/python.contribution'
import 'monaco-editor/esm/vs/basic-languages/java/java.contribution'
import 'monaco-editor/esm/vs/basic-languages/rust/rust.contribution'
import 'monaco-editor/esm/vs/basic-languages/markdown/markdown.contribution'
import 'monaco-editor/esm/vs/basic-languages/yaml/yaml.contribution'
import 'monaco-editor/esm/vs/basic-languages/shell/shell.contribution'

let initialized = false

function getCssVar(name: string, fallback: string): string {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return value || fallback
}

function withAlpha(color: string, alpha: number, fallback = '#3b82f6'): string {
  const normalizeHex = (value: string): string | null => {
    const normalized = value.trim().replace('#', '')
    if (normalized.length === 3 && /^[0-9a-f]{3}$/i.test(normalized)) {
      return normalized.split('').map(char => char + char).join('')
    }
    if (normalized.length === 6 && /^[0-9a-f]{6}$/i.test(normalized)) {
      return normalized
    }
    return null
  }

  const rgbMatch = color.trim().match(/^rgba?\((\d+),\s*(\d+),\s*(\d+)/i)
  const hex = normalizeHex(color)
    ?? (rgbMatch
      ? [rgbMatch[1], rgbMatch[2], rgbMatch[3]]
          .map(value => Number.parseInt(value, 10).toString(16).padStart(2, '0'))
          .join('')
      : normalizeHex(fallback))

  if (!hex) {
    return '#3b82f624'
  }

  const alphaHex = Math.round(Math.min(Math.max(alpha, 0), 1) * 255)
    .toString(16)
    .padStart(2, '0')

  return `#${hex}${alphaHex}`
}

export function applyMonacoTheme(isDark: boolean): void {
  const primary = getCssVar('--color-primary', isDark ? '#93c5fd' : '#2563eb')
  const background = getCssVar('--color-surface', isDark ? '#1e293b' : '#ffffff')
  const surfaceHover = getCssVar('--color-surface-hover', isDark ? '#334155' : '#f8fafc')
  const surfaceActive = getCssVar('--color-surface-active', isDark ? '#475569' : '#f1f5f9')
  const border = getCssVar('--color-border', isDark ? '#334155' : '#e2e8f0')
  const foreground = getCssVar('--color-text-primary', isDark ? '#f1f5f9' : '#1e293b')
  const secondary = getCssVar('--color-text-secondary', isDark ? '#94a3b8' : '#64748b')
  const tertiary = getCssVar('--color-text-tertiary', isDark ? '#64748b' : '#94a3b8')
  const themeName = isDark ? 'easy-agent-dark' : 'easy-agent-light'

  monaco.editor.defineTheme(themeName, {
    base: isDark ? 'vs-dark' : 'vs',
    inherit: true,
    rules: [],
    colors: {
      'editor.background': background,
      'editor.foreground': foreground,
      'editor.lineHighlightBackground': withAlpha(primary, isDark ? 0.1 : 0.05),
      'editor.selectionBackground': withAlpha(primary, isDark ? 0.22 : 0.14),
      'editor.inactiveSelectionBackground': withAlpha(primary, isDark ? 0.14 : 0.08),
      'editorCursor.foreground': primary,
      'editorWhitespace.foreground': withAlpha(tertiary, isDark ? 0.28 : 0.22),
      'editorIndentGuide.background1': withAlpha(border, isDark ? 0.52 : 0.48),
      'editorIndentGuide.activeBackground1': withAlpha(primary, isDark ? 0.58 : 0.46),
      'editorLineNumber.foreground': tertiary,
      'editorLineNumber.activeForeground': secondary,
      'editorGutter.background': background,
      'editorWidget.background': surfaceHover,
      'editorWidget.border': border,
      'editorHoverWidget.background': surfaceHover,
      'editorHoverWidget.border': border,
      'editorSuggestWidget.background': surfaceHover,
      'editorSuggestWidget.border': border,
      'editorSuggestWidget.foreground': foreground,
      'editorSuggestWidget.selectedBackground': surfaceActive,
      'editorSuggestWidget.highlightForeground': primary,
      'editorOverviewRuler.border': border,
      'scrollbarSlider.background': withAlpha(secondary, isDark ? 0.22 : 0.16),
      'scrollbarSlider.hoverBackground': withAlpha(secondary, isDark ? 0.34 : 0.26),
      'scrollbarSlider.activeBackground': withAlpha(primary, isDark ? 0.42 : 0.3)
    }
  })

  monaco.editor.setTheme(themeName)
}

export function ensureMonacoSetup(): void {
  if (initialized) {
    return
  }

  ;(self as any).MonacoEnvironment = {
    getWorker(_: unknown, label: string): Worker {
      if (label === 'json') {
        return new jsonWorker()
      }
      if (label === 'css' || label === 'scss' || label === 'less') {
        return new cssWorker()
      }
      if (label === 'html' || label === 'handlebars' || label === 'razor') {
        return new htmlWorker()
      }
      if (label === 'typescript' || label === 'javascript') {
        return new tsWorker()
      }
      return new editorWorker()
    }
  }

  const ts = monaco.languages.typescript

  // Guard against partial Monaco language registration to avoid runtime crash.
  if (ts?.typescriptDefaults && ts?.javascriptDefaults) {
    ts.typescriptDefaults.setCompilerOptions({
      allowJs: true,
      target: ts.ScriptTarget.ES2020,
      moduleResolution: ts.ModuleResolutionKind.NodeJs,
      module: ts.ModuleKind.ESNext,
      strict: true,
      noEmit: true,
      jsx: ts.JsxEmit.Preserve
    })

    ts.javascriptDefaults.setCompilerOptions({
      allowJs: true,
      target: ts.ScriptTarget.ES2020,
      moduleResolution: ts.ModuleResolutionKind.NodeJs,
      module: ts.ModuleKind.ESNext,
      noEmit: true
    })
  }

  applyMonacoTheme(document.documentElement.getAttribute('data-theme') === 'dark')

  initialized = true
}
