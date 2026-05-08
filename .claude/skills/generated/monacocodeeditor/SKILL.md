---
name: monacocodeeditor
description: "Skill for the MonacoCodeEditor area of easy-agent-pilot. 27 symbols across 5 files."
---

# MonacoCodeEditor

27 symbols | 5 files | Cohesion: 86%

## When to Use

- Working with code in `src/`
- Understanding how useMonacoCodeEditor, resolveTheme, registerCompletionProvider work
- Modifying monacocodeeditor-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | createSelectionContextController, resolveModelSelectionPayload, hideSelectionContextMenu, clearPreserveSelectionTimer, schedulePreserveSelectionReset (+6) |
| `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts` | useMonacoCodeEditor, resolveTheme, registerCompletionProvider, updateEditorOptions, resolveSelectionPayload (+3) |
| `src/modules/fileEditor/components/monacoCodeEditor/editorPresentation.ts` | createEditorRenderScheduler, updateEditorDecorations, revealEditorFocusRange, resolveEditorSearchRange, flushEditorRender (+1) |
| `src/modules/fileEditor/components/monacoCodeEditor/editorOptions.ts` | buildEditorOptions |
| `src/modules/fileEditor/components/monacoCodeEditor/completionProvider.ts` | createCompletionProvider |

## Entry Points

Start here when exploring this area:

- **`useMonacoCodeEditor`** (Function) — `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts:23`
- **`resolveTheme`** (Function) — `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts:41`
- **`registerCompletionProvider`** (Function) — `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts:47`
- **`updateEditorOptions`** (Function) — `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts:52`
- **`resolveSelectionPayload`** (Function) — `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts:92`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `useMonacoCodeEditor` | Function | `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts` | 23 |
| `resolveTheme` | Function | `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts` | 41 |
| `registerCompletionProvider` | Function | `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts` | 47 |
| `updateEditorOptions` | Function | `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts` | 52 |
| `resolveSelectionPayload` | Function | `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts` | 92 |
| `createSelectionContextController` | Function | `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | 26 |
| `resolveModelSelectionPayload` | Function | `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | 160 |
| `createEditorRenderScheduler` | Function | `src/modules/fileEditor/components/monacoCodeEditor/editorPresentation.ts` | 94 |
| `buildEditorOptions` | Function | `src/modules/fileEditor/components/monacoCodeEditor/editorOptions.ts` | 6 |
| `createCompletionProvider` | Function | `src/modules/fileEditor/components/monacoCodeEditor/completionProvider.ts` | 30 |
| `hideSelectionContextMenu` | Function | `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | 35 |
| `clearPreserveSelectionTimer` | Function | `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | 39 |
| `schedulePreserveSelectionReset` | Function | `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | 46 |
| `handleSelectionContextMenu` | Function | `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | 76 |
| `handleDocumentPointerDown` | Function | `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | 91 |
| `handleDocumentKeydown` | Function | `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | 100 |
| `handleSendSelectionFromContextMenu` | Function | `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | 106 |
| `handleEditorMouseDown` | Function | `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | 118 |
| `cleanup` | Function | `src/modules/fileEditor/components/monacoCodeEditor/selectionContextMenu.ts` | 141 |
| `updateDecorations` | Function | `src/modules/fileEditor/components/monacoCodeEditor/useMonacoCodeEditor.ts` | 56 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `UseMonacoCodeEditor → NormalizeHex` | cross_community | 5 |
| `UseMonacoCodeEditor → GetCssVar` | cross_community | 4 |
| `UseMonacoCodeEditor → RegisterVueLanguage` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Monaco | 2 calls |
| Stores | 1 calls |

## How to Explore

1. `gitnexus_context({name: "useMonacoCodeEditor"})` — see callers and callees
2. `gitnexus_query({query: "monacocodeeditor"})` — find related execution flows
3. Read key files listed above for implementation details
