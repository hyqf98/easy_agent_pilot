---
name: richmarkdowneditor
description: "Skill for the RichMarkdownEditor area of easy-agent-pilot. 29 symbols across 1 files."
---

# RichMarkdownEditor

29 symbols | 1 files | Cohesion: 82%

## When to Use

- Working with code in `src/`
- Understanding how normalizeText, isElementNode, escapeInlineText work
- Modifying richmarkdowneditor-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | normalizeText, isElementNode, escapeInlineText, serializeInline, serializeParagraph (+24) |

## Entry Points

Start here when exploring this area:

- **`normalizeText`** (Function) — `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue:32`
- **`isElementNode`** (Function) — `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue:36`
- **`escapeInlineText`** (Function) — `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue:40`
- **`serializeInline`** (Function) — `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue:79`
- **`serializeParagraph`** (Function) — `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue:118`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `normalizeText` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 32 |
| `isElementNode` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 36 |
| `escapeInlineText` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 40 |
| `serializeInline` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 79 |
| `serializeParagraph` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 118 |
| `serializeBlockquote` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 122 |
| `serializeListItem` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 138 |
| `serializeList` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 156 |
| `serializeTable` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 165 |
| `serializePre` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 185 |
| `serializeBlock` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 193 |
| `isInsideList` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 380 |
| `createParagraphElement` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 51 |
| `getBlockAncestor` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 254 |
| `getCurrentBlockElement` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 265 |
| `placeCaretAtStart` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 275 |
| `parseShortcut` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 302 |
| `createShortcutBlock` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 334 |
| `handleListEnter` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 400 |
| `handleDocumentSelectionChange` | Function | `src/modules/fileEditor/components/richMarkdownEditor/RichMarkdownEditor.vue` | 416 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `HandleEditorKeydown → IsElementNode` | cross_community | 4 |
| `HandleEditorKeydown → SerializeEditor` | cross_community | 4 |
| `HandleEditorKeydown → NormalizeText` | cross_community | 3 |
| `HandleEditorKeydown → CreateParagraphElement` | intra_community | 3 |
| `HandleEditorKeydown → PlaceCaretAtStart` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Message | 1 calls |

## How to Explore

1. `gitnexus_context({name: "normalizeText"})` — see callers and callees
2. `gitnexus_query({query: "richmarkdowneditor"})` — find related execution flows
3. Read key files listed above for implementation details
