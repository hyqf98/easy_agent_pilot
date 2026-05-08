---
name: messagebubble
description: "Skill for the MessageBubble area of easy-agent-pilot. 33 symbols across 3 files."
---

# MessageBubble

33 symbols | 3 files | Cohesion: 77%

## When to Use

- Working with code in `src/`
- Understanding how summarizeRuntimeNotice, getUsageNoticeSummary, resolveRuntimeNoticeModel work
- Modifying messagebubble-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/utils/runtimeNotice.ts` | parseRuntimeNoticeLines, isModelLabel, isInputLabel, isOutputLabel, formatCompactNumber (+15) |
| `src/components/message/messageBubble/useMessageBubble.ts` | toolCallModelLabel, assistantElapsedLabel, processingTimeNotice, visibleRuntimeNotices, userFormResponseDisplay (+3) |
| `src/utils/structuredContent.ts` | pushMarkdownBlock, tryParseStructuredBlocks, parseInlineStructuredBlocks, setStructuredContentCache, parseStructuredContent |

## Entry Points

Start here when exploring this area:

- **`summarizeRuntimeNotice`** (Function) — `src/utils/runtimeNotice.ts:413`
- **`getUsageNoticeSummary`** (Function) — `src/utils/runtimeNotice.ts:426`
- **`resolveRuntimeNoticeModel`** (Function) — `src/utils/runtimeNotice.ts:471`
- **`toolCallModelLabel`** (Function) — `src/components/message/messageBubble/useMessageBubble.ts:371`
- **`isContextRuntimeNotice`** (Function) — `src/utils/runtimeNotice.ts:256`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `summarizeRuntimeNotice` | Function | `src/utils/runtimeNotice.ts` | 413 |
| `getUsageNoticeSummary` | Function | `src/utils/runtimeNotice.ts` | 426 |
| `resolveRuntimeNoticeModel` | Function | `src/utils/runtimeNotice.ts` | 471 |
| `toolCallModelLabel` | Function | `src/components/message/messageBubble/useMessageBubble.ts` | 371 |
| `isContextRuntimeNotice` | Function | `src/utils/runtimeNotice.ts` | 256 |
| `isProcessingTimeRuntimeNotice` | Function | `src/utils/runtimeNotice.ts` | 262 |
| `isEnvironmentRuntimeNotice` | Function | `src/utils/runtimeNotice.ts` | 268 |
| `usageNotice` | Function | `src/utils/runtimeNotice.ts` | 476 |
| `getProcessingTimeNoticeSummary` | Function | `src/utils/runtimeNotice.ts` | 577 |
| `assistantElapsedLabel` | Function | `src/components/message/messageBubble/useMessageBubble.ts` | 315 |
| `processingTimeNotice` | Function | `src/components/message/messageBubble/useMessageBubble.ts` | 321 |
| `visibleRuntimeNotices` | Function | `src/components/message/messageBubble/useMessageBubble.ts` | 339 |
| `parseStructuredContent` | Function | `src/utils/structuredContent.ts` | 722 |
| `userFormResponseDisplay` | Function | `src/components/message/messageBubble/useMessageBubble.ts` | 172 |
| `assistantStructuredBlocks` | Function | `src/components/message/messageBubble/useMessageBubble.ts` | 418 |
| `sortedToolCalls` | Function | `src/components/message/messageBubble/useMessageBubble.ts` | 389 |
| `weight` | Function | `src/components/message/messageBubble/useMessageBubble.ts` | 399 |
| `parseRuntimeNoticeLines` | Function | `src/utils/runtimeNotice.ts` | 83 |
| `isModelLabel` | Function | `src/utils/runtimeNotice.ts` | 136 |
| `isInputLabel` | Function | `src/utils/runtimeNotice.ts` | 141 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `SplitChatMessages → ExtractBalancedJsonRanges` | cross_community | 6 |
| `SplitChatMessages → PushMarkdownBlock` | cross_community | 6 |
| `SplitChatMessages → SetStructuredContentCache` | cross_community | 5 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_16 | 1 calls |
| Cluster_18 | 1 calls |
| TaskSplitDialog | 1 calls |
| Stores | 1 calls |
| Usage | 1 calls |
| Commands | 1 calls |

## How to Explore

1. `gitnexus_context({name: "summarizeRuntimeNotice"})` — see callers and callees
2. `gitnexus_query({query: "messagebubble"})` — find related execution flows
3. Read key files listed above for implementation details
