---
name: compression
description: "Skill for the Compression area of easy-agent-pilot. 25 symbols across 3 files."
---

# Compression

25 symbols | 3 files | Cohesion: 70%

## When to Use

- Working with code in `src/`
- Understanding how useTracePreviewStore, getSessionRuntimeBinding, CompressionService work
- Modifying compression-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/services/compression/CompressionService.ts` | t, syncTemporarySessionPreview, renderCompressionExecutionFlow, buildCompressionTriggerMessage, buildFileSummaryLines (+18) |
| `src/stores/tracePreview.ts` | useTracePreviewStore |
| `src/services/conversation/runtimeBindings.ts` | getSessionRuntimeBinding |

## Entry Points

Start here when exploring this area:

- **`useTracePreviewStore`** (Function) — `src/stores/tracePreview.ts:13`
- **`getSessionRuntimeBinding`** (Function) — `src/services/conversation/runtimeBindings.ts:31`
- **`CompressionService`** (Class) — `src/services/compression/CompressionService.ts:60`
- **`t`** (Method) — `src/services/compression/CompressionService.ts:75`
- **`syncTemporarySessionPreview`** (Method) — `src/services/compression/CompressionService.ts:79`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `CompressionService` | Class | `src/services/compression/CompressionService.ts` | 60 |
| `useTracePreviewStore` | Function | `src/stores/tracePreview.ts` | 13 |
| `getSessionRuntimeBinding` | Function | `src/services/conversation/runtimeBindings.ts` | 31 |
| `t` | Method | `src/services/compression/CompressionService.ts` | 75 |
| `syncTemporarySessionPreview` | Method | `src/services/compression/CompressionService.ts` | 79 |
| `renderCompressionExecutionFlow` | Method | `src/services/compression/CompressionService.ts` | 93 |
| `buildCompressionTriggerMessage` | Method | `src/services/compression/CompressionService.ts` | 260 |
| `buildFileSummaryLines` | Method | `src/services/compression/CompressionService.ts` | 651 |
| `formatFileGroupLine` | Method | `src/services/compression/CompressionService.ts` | 669 |
| `generateSimpleSummary` | Method | `src/services/compression/CompressionService.ts` | 350 |
| `buildSummaryPrompt` | Method | `src/services/compression/CompressionService.ts` | 488 |
| `resolveAssistantSummary` | Method | `src/services/compression/CompressionService.ts` | 556 |
| `resolvePendingWork` | Method | `src/services/compression/CompressionService.ts` | 569 |
| `buildPromptFileLines` | Method | `src/services/compression/CompressionService.ts` | 660 |
| `compactText` | Method | `src/services/compression/CompressionService.ts` | 730 |
| `extractFileGroups` | Method | `src/services/compression/CompressionService.ts` | 585 |
| `appendTraceFile` | Method | `src/services/compression/CompressionService.ts` | 632 |
| `extractPathReferencesFromUnknown` | Method | `src/services/compression/CompressionService.ts` | 676 |
| `walkPathReferences` | Method | `src/services/compression/CompressionService.ts` | 682 |
| `extractPathReferencesFromText` | Method | `src/services/compression/CompressionService.ts` | 702 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `HandleSend → UniqueStrings` | cross_community | 5 |
| `HandleConfirmCompress → UniqueStrings` | cross_community | 5 |
| `HandleSend → AppendTraceFile` | cross_community | 4 |
| `HandleConfirmCompress → AppendTraceFile` | cross_community | 4 |
| `HandleSend → T` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Stores | 4 calls |
| Cluster_15 | 2 calls |

## How to Explore

1. `gitnexus_context({name: "useTracePreviewStore"})` — see callers and callees
2. `gitnexus_query({query: "compression"})` — find related execution flows
3. Read key files listed above for implementation details
