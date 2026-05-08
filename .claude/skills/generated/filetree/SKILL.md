---
name: filetree
description: "Skill for the FileTree area of easy-agent-pilot. 99 symbols across 6 files."
---

# FileTree

99 symbols | 6 files | Cohesion: 82%

## When to Use

- Working with code in `src/`
- Understanding how useFileTree, normalizeComparablePath, uniquePaths work
- Modifying filetree-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/components/fileTree/useFileTree.ts` | cloneTreeNodes, useFileTree, normalizeComparablePath, uniquePaths, resolveParentPath (+83) |
| `src/utils/fileIcon.ts` | inferExtensionFromName, normalizeExtension, getAllRules, resolveFileIcon |
| `src/utils/composerFileMention.ts` | formatMentionLiteral, createComposerFileMention, createFileLineRangeMention |
| `src/components/layout/projectPanel/useProjectPanel.ts` | handleRefresh, handleToggleExpand |
| `src/components/fileTree/composables/useFileOperations.ts` | useFileOperations |
| `src/modules/fileEditor/components/fileEditorWorkspace/useFileEditorWorkspace.ts` | handleSendSelectionToSession |

## Entry Points

Start here when exploring this area:

- **`useFileTree`** (Function) — `src/components/fileTree/useFileTree.ts:195`
- **`normalizeComparablePath`** (Function) — `src/components/fileTree/useFileTree.ts:256`
- **`uniquePaths`** (Function) — `src/components/fileTree/useFileTree.ts:267`
- **`resolveParentPath`** (Function) — `src/components/fileTree/useFileTree.ts:288`
- **`currentCache`** (Function) — `src/components/fileTree/useFileTree.ts:306`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `useFileTree` | Function | `src/components/fileTree/useFileTree.ts` | 195 |
| `normalizeComparablePath` | Function | `src/components/fileTree/useFileTree.ts` | 256 |
| `uniquePaths` | Function | `src/components/fileTree/useFileTree.ts` | 267 |
| `resolveParentPath` | Function | `src/components/fileTree/useFileTree.ts` | 288 |
| `currentCache` | Function | `src/components/fileTree/useFileTree.ts` | 306 |
| `buildWatchTargets` | Function | `src/components/fileTree/useFileTree.ts` | 310 |
| `hasSameWatchTargets` | Function | `src/components/fileTree/useFileTree.ts` | 325 |
| `syncExpandedKeysToCache` | Function | `src/components/fileTree/useFileTree.ts` | 333 |
| `resolveDirectoryPath` | Function | `src/components/fileTree/useFileTree.ts` | 341 |
| `resolveParentDirectoryPath` | Function | `src/components/fileTree/useFileTree.ts` | 356 |
| `buildAncestorDirectories` | Function | `src/components/fileTree/useFileTree.ts` | 362 |
| `resolveScrollElement` | Function | `src/components/fileTree/useFileTree.ts` | 378 |
| `saveScrollPosition` | Function | `src/components/fileTree/useFileTree.ts` | 382 |
| `restoreScrollPosition` | Function | `src/components/fileTree/useFileTree.ts` | 391 |
| `clearDragState` | Function | `src/components/fileTree/useFileTree.ts` | 412 |
| `clearSelectionState` | Function | `src/components/fileTree/useFileTree.ts` | 423 |
| `loadTreeData` | Function | `src/components/fileTree/useFileTree.ts` | 486 |
| `restoreExpandedDirectories` | Function | `src/components/fileTree/useFileTree.ts` | 532 |
| `queueDirectoryRefresh` | Function | `src/components/fileTree/useFileTree.ts` | 553 |
| `stopFileWatcher` | Function | `src/components/fileTree/useFileTree.ts` | 568 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `UseFileTree → BuildCacheKey` | cross_community | 6 |
| `UseFileTree → CreateRuntimeCache` | cross_community | 6 |
| `HandleDocumentKeydown → NormalizeComparablePath` | cross_community | 6 |
| `InsertFileMentions → IsGlobalMentionPath` | cross_community | 6 |
| `InsertFileMentions → GetMentionDisplayName` | cross_community | 6 |
| `HandleExpandChange → IsUnavailableError` | cross_community | 6 |
| `HandleExpandChange → NormalizeComparablePath` | intra_community | 6 |
| `HandleExpandChange → FindTreeNodeByKey` | intra_community | 6 |
| `HandleDrop → BuildCacheKey` | cross_community | 6 |
| `HandleDrop → CreateRuntimeCache` | cross_community | 6 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Composables | 2 calls |
| Stores | 2 calls |
| Tabs | 1 calls |

## How to Explore

1. `gitnexus_context({name: "useFileTree"})` — see callers and callees
2. `gitnexus_query({query: "filetree"})` — find related execution flows
3. Read key files listed above for implementation details
