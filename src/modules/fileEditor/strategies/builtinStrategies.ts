import type { CompletionEntry } from '../types'
import type { LanguageStrategy } from './languageStrategy'

const toKeywordEntries = (keywords: string[], detail: string): CompletionEntry[] =>
  keywords.map(keyword => ({
    label: keyword,
    insertText: keyword,
    detail,
    kind: 'keyword'
  }))

const createStrategy = (strategy: LanguageStrategy): LanguageStrategy => strategy

const jsTsKeywords = [
  'const', 'let', 'var', 'function', 'return', 'if', 'else', 'switch', 'case', 'for', 'while', 'try', 'catch', 'finally',
  'class', 'extends', 'implements', 'interface', 'type', 'import', 'export', 'async', 'await', 'new'
]

const pythonKeywords = [
  'def', 'class', 'import', 'from', 'as', 'if', 'elif', 'else', 'for', 'while', 'try', 'except', 'finally',
  'with', 'lambda', 'return', 'yield', 'async', 'await', 'pass', 'break', 'continue'
]

const javaKeywords = [
  'public', 'private', 'protected', 'class', 'interface', 'extends', 'implements', 'import', 'package', 'if', 'else',
  'switch', 'case', 'for', 'while', 'try', 'catch', 'finally', 'return', 'new', 'static', 'void', 'final'
]

const rustKeywords = [
  'fn', 'let', 'mut', 'struct', 'enum', 'impl', 'trait', 'match', 'if', 'else', 'for', 'while', 'loop', 'return',
  'pub', 'use', 'mod', 'crate', 'async', 'await', 'Result', 'Option'
]

const markdownSnippets: CompletionEntry[] = [
  {
    label: 'heading',
    insertText: '# ${1:Heading}',
    detail: 'Markdown Heading',
    kind: 'snippet'
  },
  {
    label: 'code-block',
    insertText: '```$1\n$2\n```',
    detail: 'Markdown Code Block',
    kind: 'snippet'
  },
  {
    label: 'table',
    insertText: '| Col1 | Col2 |\n| --- | --- |\n| $1 | $2 |',
    detail: 'Markdown Table',
    kind: 'snippet'
  }
]

export const builtinLanguageStrategies: LanguageStrategy[] = [
  createStrategy({
    id: 'typescript',
    monacoLanguageId: 'typescript',
    supportsCompletion: true,
    match: ({ extension }) => ['ts', 'tsx', 'mts', 'cts', 'd.ts'].includes(extension),
    getCompletions: () => toKeywordEntries(jsTsKeywords, 'TypeScript Keyword')
  }),
  createStrategy({
    id: 'javascript',
    monacoLanguageId: 'javascript',
    supportsCompletion: true,
    match: ({ extension }) => ['js', 'jsx', 'mjs', 'cjs'].includes(extension),
    getCompletions: () => toKeywordEntries(jsTsKeywords, 'JavaScript Keyword')
  }),
  createStrategy({
    id: 'vue',
    monacoLanguageId: 'html',
    supportsCompletion: true,
    match: ({ extension }) => extension === 'vue',
    getCompletions: () => [
      {
        label: 'script-setup',
        insertText: '<script setup lang="ts">\n$1\n</script>\n\n<template>\n  $2\n</template>\n',
        detail: 'Vue SFC Template',
        kind: 'snippet'
      },
      {
        label: 'defineProps',
        insertText: 'const props = defineProps<{\n  $1\n}>()',
        detail: 'Vue defineProps',
        kind: 'snippet'
      }
    ]
  }),
  createStrategy({
    id: 'json',
    monacoLanguageId: 'json',
    supportsCompletion: true,
    match: ({ extension, fileName }) => extension === 'json' || fileName.endsWith('.jsonc'),
    getCompletions: () => [
      {
        label: 'property',
        insertText: '"${1:key}": ${2:value}',
        detail: 'JSON Property',
        kind: 'snippet'
      }
    ]
  }),
  createStrategy({
    id: 'markdown',
    monacoLanguageId: 'markdown',
    supportsCompletion: true,
    match: ({ extension }) => ['md', 'mdx'].includes(extension),
    getCompletions: () => markdownSnippets
  }),
  createStrategy({
    id: 'python',
    monacoLanguageId: 'python',
    supportsCompletion: true,
    match: ({ extension }) => extension === 'py',
    getCompletions: () => toKeywordEntries(pythonKeywords, 'Python Keyword')
  }),
  createStrategy({
    id: 'java',
    monacoLanguageId: 'java',
    supportsCompletion: true,
    match: ({ extension }) => extension === 'java',
    getCompletions: () => toKeywordEntries(javaKeywords, 'Java Keyword')
  }),
  createStrategy({
    id: 'rust',
    monacoLanguageId: 'rust',
    supportsCompletion: true,
    match: ({ extension }) => extension === 'rs',
    getCompletions: () => toKeywordEntries(rustKeywords, 'Rust Keyword')
  }),
  createStrategy({
    id: 'html',
    monacoLanguageId: 'html',
    supportsCompletion: true,
    match: ({ extension }) => ['html', 'htm', 'xml', 'jsp', 'jspf', 'tag'].includes(extension),
    getCompletions: () => [
      {
        label: 'div',
        insertText: '<div>$1</div>',
        detail: 'HTML div',
        kind: 'snippet'
      },
      {
        label: 'span',
        insertText: '<span>$1</span>',
        detail: 'HTML span',
        kind: 'snippet'
      },
      {
        label: 'a',
        insertText: '<a href="${1:#}">$2</a>',
        detail: 'HTML anchor',
        kind: 'snippet'
      },
      {
        label: 'html5',
        insertText: '<!doctype html>\n<html lang="en">\n<head>\n  <meta charset="UTF-8" />\n  <meta name="viewport" content="width=device-width, initial-scale=1.0" />\n  <title>${1:Document}</title>\n</head>\n<body>\n  $2\n</body>\n</html>',
        detail: 'HTML5 Boilerplate',
        kind: 'snippet'
      },
      {
        label: 'jsp-page',
        insertText: '<%@ page contentType="text/html; charset=UTF-8" pageEncoding="UTF-8" %>\n$1',
        detail: 'JSP page directive',
        kind: 'snippet'
      }
    ]
  }),
  createStrategy({
    id: 'css',
    monacoLanguageId: 'css',
    supportsCompletion: true,
    match: ({ extension }) => ['css', 'scss', 'sass', 'less'].includes(extension),
    getCompletions: () => [
      {
        label: 'media-query',
        insertText: '@media (max-width: ${1:768px}) {\n  $2\n}',
        detail: 'Media Query',
        kind: 'snippet'
      }
    ]
  }),
  createStrategy({
    id: 'shell',
    monacoLanguageId: 'shell',
    supportsCompletion: true,
    match: ({ extension, fileName }) => ['sh', 'bash', 'zsh'].includes(extension) || fileName === 'Dockerfile',
    getCompletions: () => toKeywordEntries(['echo', 'if', 'then', 'fi', 'for', 'do', 'done', 'export'], 'Shell Keyword')
  }),
  createStrategy({
    id: 'yaml',
    monacoLanguageId: 'yaml',
    supportsCompletion: true,
    match: ({ extension }) => ['yaml', 'yml'].includes(extension),
    getCompletions: () => [
      {
        label: 'key-value',
        insertText: '${1:key}: ${2:value}',
        detail: 'YAML Key Value',
        kind: 'snippet'
      }
    ]
  }),
  createStrategy({
    id: 'plaintext',
    monacoLanguageId: 'plaintext',
    supportsCompletion: false,
    match: () => true
  })
]
