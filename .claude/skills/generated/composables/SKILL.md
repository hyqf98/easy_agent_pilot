---
name: composables
description: "Skill for the Composables area of easy-agent-pilot. 149 symbols across 19 files."
---

# Composables

149 symbols | 19 files | Cohesion: 79%

## When to Use

- Working with code in `src/`
- Understanding how createComposerMemoryReference, setActiveMemorySuggestionIndex, moveActiveMemorySuggestion work
- Modifying composables-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/composables/useConversationComposer.ts` | buildMemoryReferenceToken, createComposerMemoryReference, setActiveMemorySuggestionIndex, moveActiveMemorySuggestion, clearMemoryPreview (+74) |
| `src/composables/useMessageComposer.ts` | parsedInputText, toPendingImage, sanitizeComposerText, closeFileMention, formatMentionInsertText (+13) |
| `src/composables/useConfirmDialog.ts` | defaultState, resetState, handleConfirm, handleCancel, handleVisibleChange (+4) |
| `src/composables/useMiniPanelShortcut.ts` | normalizePermissionErrorMessage, useMiniPanelShortcut, setRegistrationError, unregisterCurrentShortcut, unregisterWindowsOverrideShortcut (+1) |
| `src/utils/fileMention.ts` | isGlobalMentionPath, getMentionDisplayName, getMentionDisplayText, getMentionTitle |
| `src/composables/useDefaultCliConfigEditor.ts` | readDefaultCliEditorFile, loadDefaultConfigFile, openConfigEditor, reloadConfigEditor |
| `src/utils/attachmentPreview.ts` | buildCacheKey, getFileSrc, resolveAttachmentPreviewUrl, getAttachmentPreviewUrl |
| `src/composables/useTypewriterText.ts` | useTypewriterText, stopAnimation, syncImmediately, tick |
| `src/composables/useSessionFileReference.ts` | appendMentionsToDraft, resolveTargetSession, ensureTargetSession, sendFileReferencesToSession |
| `src/composables/useAsyncOperation.ts` | error, execute, setProgress, handleProgress |

## Entry Points

Start here when exploring this area:

- **`createComposerMemoryReference`** (Function) — `src/composables/useConversationComposer.ts:437`
- **`setActiveMemorySuggestionIndex`** (Function) — `src/composables/useConversationComposer.ts:710`
- **`moveActiveMemorySuggestion`** (Function) — `src/composables/useConversationComposer.ts:721`
- **`clearMemoryPreview`** (Function) — `src/composables/useConversationComposer.ts:757`
- **`clearMemorySuggestionAutoHideTimer`** (Function) — `src/composables/useConversationComposer.ts:794`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `createComposerMemoryReference` | Function | `src/composables/useConversationComposer.ts` | 437 |
| `setActiveMemorySuggestionIndex` | Function | `src/composables/useConversationComposer.ts` | 710 |
| `moveActiveMemorySuggestion` | Function | `src/composables/useConversationComposer.ts` | 721 |
| `clearMemoryPreview` | Function | `src/composables/useConversationComposer.ts` | 757 |
| `clearMemorySuggestionAutoHideTimer` | Function | `src/composables/useConversationComposer.ts` | 794 |
| `isMemorySuggestionKeyboardActive` | Function | `src/composables/useConversationComposer.ts` | 808 |
| `hideMemorySuggestionPanel` | Function | `src/composables/useConversationComposer.ts` | 812 |
| `armMemorySuggestionAutoHide` | Function | `src/composables/useConversationComposer.ts` | 824 |
| `markMemorySuggestionKeyboardInteraction` | Function | `src/composables/useConversationComposer.ts` | 845 |
| `handleMemorySuggestionPointerEnter` | Function | `src/composables/useConversationComposer.ts` | 852 |
| `handleMemorySuggestionPointerLeave` | Function | `src/composables/useConversationComposer.ts` | 859 |
| `insertMemoryReference` | Function | `src/composables/useConversationComposer.ts` | 1366 |
| `dismissMemorySuggestion` | Function | `src/composables/useConversationComposer.ts` | 1422 |
| `removeMemoryReferenceFromDraft` | Function | `src/composables/useConversationComposer.ts` | 1442 |
| `handleKeyDown` | Function | `src/composables/useConversationComposer.ts` | 2292 |
| `executeSlashCommand` | Function | `src/services/slashCommands.ts` | 183 |
| `closeFileMention` | Function | `src/composables/useConversationComposer.ts` | 1280 |
| `closeSlashCommand` | Function | `src/composables/useConversationComposer.ts` | 1287 |
| `closeCdPathSuggestions` | Function | `src/composables/useConversationComposer.ts` | 1293 |
| `openFileMention` | Function | `src/composables/useConversationComposer.ts` | 1299 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `InsertFileMentions → IsGlobalMentionPath` | cross_community | 6 |
| `InsertFileMentions → GetMentionDisplayName` | cross_community | 6 |
| `HandleSend → UniqueStrings` | cross_community | 5 |
| `HandleFileSelect → IsGlobalMentionPath` | cross_community | 5 |
| `HandleFileSelect → GetMentionDisplayName` | cross_community | 5 |
| `HandleRetry → GetAttachmentKind` | cross_community | 5 |
| `HandleKeyDown → ClearMemorySuggestionAutoHideTimer` | intra_community | 5 |
| `HandleKeyDown → ClearMemoryPreview` | intra_community | 5 |
| `InsertFileMentions → FormatMentionLiteral` | cross_community | 5 |
| `HandleSend → SetLocale` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Stores | 14 calls |
| FileTree | 3 calls |
| Services | 2 calls |
| Cluster_61 | 1 calls |
| Cluster_26 | 1 calls |
| General | 1 calls |

## How to Explore

1. `gitnexus_context({name: "createComposerMemoryReference"})` — see callers and callees
2. `gitnexus_query({query: "composables"})` — find related execution flows
3. Read key files listed above for implementation details
