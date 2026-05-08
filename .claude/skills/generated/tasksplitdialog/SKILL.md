---
name: tasksplitdialog
description: "Skill for the TaskSplitDialog area of easy-agent-pilot. 70 symbols across 7 files."
---

# TaskSplitDialog

70 symbols | 7 files | Cohesion: 74%

## When to Use

- Working with code in `src/`
- Understanding how normalizeTaskInstructionInput, materializeTaskMentions, parseTaskInstruction work
- Modifying tasksplitdialog-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | buildInstructionPrompt, executeParsedInstruction, parseLogMetadata, buildFormRequestContent, buildFormResponseContent (+44) |
| `src/utils/taskInstructionParser.ts` | normalizeTaskInstructionInput, materializeTaskMentions, extractFirstNumber, extractPromptSuffix, resolveByNumber (+3) |
| `src/utils/structuredContent.ts` | extractFormResponse, extractBalancedJsonRanges, extractTaskSplitResult, extractTaskSplitSummary, stripTaskSplitResultFromContent |
| `src/utils/runtimeNotice.ts` | isRuntimeNoticeLineLabel, buildRuntimeNoticeFromSystemContent, formatProcessingDuration, buildProcessingTimeNotice |
| `src/components/message/messageBubble/useMessageBubble.ts` | userFormResponse, resolvedFormResponsesById |
| `src/components/message/ExecutionTimeline.vue` | toRuntimeNotices |
| `src/components/message/messageList/useMessageList.ts` | clearMessageListSessionState |

## Entry Points

Start here when exploring this area:

- **`normalizeTaskInstructionInput`** (Function) — `src/utils/taskInstructionParser.ts:28`
- **`materializeTaskMentions`** (Function) — `src/utils/taskInstructionParser.ts:33`
- **`parseTaskInstruction`** (Function) — `src/utils/taskInstructionParser.ts:106`
- **`buildInstructionPrompt`** (Function) — `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts:1604`
- **`executeParsedInstruction`** (Function) — `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts:1612`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `normalizeTaskInstructionInput` | Function | `src/utils/taskInstructionParser.ts` | 28 |
| `materializeTaskMentions` | Function | `src/utils/taskInstructionParser.ts` | 33 |
| `parseTaskInstruction` | Function | `src/utils/taskInstructionParser.ts` | 106 |
| `buildInstructionPrompt` | Function | `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | 1604 |
| `executeParsedInstruction` | Function | `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | 1612 |
| `buildRuntimeNoticeFromSystemContent` | Function | `src/utils/runtimeNotice.ts` | 345 |
| `splitChatMessages` | Function | `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | 855 |
| `toRuntimeNotices` | Function | `src/components/message/ExecutionTimeline.vue` | 61 |
| `formatProcessingDuration` | Function | `src/utils/runtimeNotice.ts` | 520 |
| `buildProcessingTimeNotice` | Function | `src/utils/runtimeNotice.ts` | 543 |
| `extractFormResponse` | Function | `src/utils/structuredContent.ts` | 862 |
| `userFormResponse` | Function | `src/components/message/messageBubble/useMessageBubble.ts` | 167 |
| `resolvedFormResponsesById` | Function | `src/components/message/messageBubble/useMessageBubble.ts` | 443 |
| `submittedForms` | Function | `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | 858 |
| `matchedFormIndex` | Function | `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | 926 |
| `autoResizeInput` | Function | `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | 1223 |
| `updateInstructionSelection` | Function | `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | 1234 |
| `applyMentionSuggestion` | Function | `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | 1562 |
| `handleInstructionCaretChange` | Function | `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | 1595 |
| `handleInstructionInput` | Function | `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | 1599 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Register_shortcut → ExtractBalancedJsonRanges` | cross_community | 8 |
| `SplitChatMessages → IsRecord` | cross_community | 6 |
| `SplitChatMessages → NormalizeJsonLikeText` | cross_community | 6 |
| `SplitChatMessages → ExtractBalancedJsonRanges` | cross_community | 6 |
| `SplitChatMessages → PushMarkdownBlock` | cross_community | 6 |
| `SplitChatMessages → SetStructuredContentCache` | cross_community | 5 |
| `SplitChatMessages → ParseSummaryLineMap` | cross_community | 4 |
| `SplitChatMessages → ParseSummaryValue` | cross_community | 4 |
| `SplitChatMessages → TrimContent` | cross_community | 3 |
| `SplitChatMessages → CompareTimestamp` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| MessageBubble | 4 calls |
| Cluster_18 | 3 calls |
| Stores | 3 calls |
| Usage | 2 calls |
| Plan | 2 calls |
| Cluster_16 | 1 calls |
| TaskExecutionLog | 1 calls |

## How to Explore

1. `gitnexus_context({name: "normalizeTaskInstructionInput"})` — see callers and callees
2. `gitnexus_query({query: "tasksplitdialog"})` — find related execution flows
3. Read key files listed above for implementation details
