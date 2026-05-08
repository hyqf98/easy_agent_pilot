---
name: memory
description: "Skill for the Memory area of easy-agent-pilot. 47 symbols across 7 files."
---

# Memory

47 symbols | 7 files | Cohesion: 76%

## When to Use

- Working with code in `src/`
- Understanding how normalizeText, serializeParagraph, serializeBlockquote work
- Modifying memory-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/components/memory/MemoryMarkdownEditor.vue` | normalizeText, serializeParagraph, serializeBlockquote, serializeListItem, serializeList (+28) |
| `src/services/memory/MemoryMergeService.ts` | createMessage, stripCodeFence, buildPrompt, mergeLibrary |
| `src/services/memory/projectMemoryPrompt.ts` | normalizeContent, truncateContent, buildProjectMemorySystemPrompt |
| `src/services/memory/mountedMemoryPrompt.ts` | uniqueLibraryIds, buildPromptCacheKey, loadMountedMemoryPrompt |
| `src/components/memory/MemoryBatchDeleteModal.vue` | toIsoString, handleConfirm |
| `src/services/conversation/ConversationService.ts` | buildProjectMemoryPrompt |
| `src/services/compression/CompressionService.ts` | buildProjectMemoryPrompt |

## Entry Points

Start here when exploring this area:

- **`normalizeText`** (Function) — `src/components/memory/MemoryMarkdownEditor.vue:29`
- **`serializeParagraph`** (Function) — `src/components/memory/MemoryMarkdownEditor.vue:112`
- **`serializeBlockquote`** (Function) — `src/components/memory/MemoryMarkdownEditor.vue:116`
- **`serializeListItem`** (Function) — `src/components/memory/MemoryMarkdownEditor.vue:130`
- **`serializeList`** (Function) — `src/components/memory/MemoryMarkdownEditor.vue:146`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `normalizeText` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 29 |
| `serializeParagraph` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 112 |
| `serializeBlockquote` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 116 |
| `serializeListItem` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 130 |
| `serializeList` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 146 |
| `serializeTable` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 155 |
| `serializePre` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 172 |
| `serializeBlock` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 180 |
| `placeCaretAtEnd` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 278 |
| `placeCaretAtOffset` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 290 |
| `activateBlock` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 439 |
| `buildProjectMemorySystemPrompt` | Function | `src/services/memory/projectMemoryPrompt.ts` | 25 |
| `loadMountedMemoryPrompt` | Function | `src/services/memory/mountedMemoryPrompt.ts` | 26 |
| `isElementNode` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 33 |
| `escapeInlineText` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 37 |
| `serializeInline` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 74 |
| `getBlockAncestor` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 238 |
| `getCurrentBlockElement` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 249 |
| `isInsideList` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 391 |
| `shouldShowRaw` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 426 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `RunLoop → UseNotificationStore` | cross_community | 5 |
| `RunLoop → NormalizeContent` | cross_community | 5 |
| `HandleEditorKeydown → SerializeEditor` | cross_community | 4 |
| `HandleEditorKeydown → IsElementNode` | cross_community | 4 |
| `ExecuteStepTurn → UseNotificationStore` | cross_community | 4 |
| `ExecuteStepTurn → NormalizeContent` | cross_community | 4 |
| `ExecuteStepTurn → TruncateContent` | cross_community | 4 |
| `ExecuteControlTurn → TruncateContent` | cross_community | 4 |
| `HandleDocumentSelectionChange → IsElementNode` | cross_community | 4 |
| `HandleDocumentSelectionChange → NormalizeText` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Message | 1 calls |
| Conversation | 1 calls |
| Stores | 1 calls |

## How to Explore

1. `gitnexus_context({name: "normalizeText"})` — see callers and callees
2. `gitnexus_query({query: "memory"})` — find related execution flows
3. Read key files listed above for implementation details
