---
name: conversation
description: "Skill for the Conversation area of easy-agent-pilot. 105 symbols across 21 files."
---

# Conversation

105 symbols | 21 files | Cohesion: 78%

## When to Use

- Working with code in `src/`
- Understanding how mergeToolInputArguments, registerTraceTask, markMetric work
- Modifying conversation-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/services/conversation/ConversationService.ts` | resolveCodexContextWindowOccupancy, normalizeUsageNumber, resolveRuntimeContextWindowOccupancy, removeRuntimeNoticeById, isSnapshotProneContentRuntime (+27) |
| `src/services/conversation/fileTraceCollector.ts` | splitLines, clampLine, computeDiffRange, findSnippetRange, buildSnippet (+14) |
| `src/services/conversation/runtimeProfiles.ts` | resolveAllowedTools, resolveCliOutputFormat, normalizeRuntimeProvider, normalizeRuntimeKey, normalizeCliModelId (+4) |
| `src-tauri/src/commands/conversation/mod.rs` | execute_compat_request, execute_claude_cli, execute_codex_cli, execute_claude_sdk, execute_codex_sdk (+2) |
| `src-tauri/src/commands/conversation/abort.rs` | get_abort_flag, set_abort_flag, should_abort, write_abort_log, register_session_pid (+1) |
| `src/services/conversation/buildConversationMessages.ts` | normalizeContent, createSyntheticMessage, buildCompressionSummaryMessage, dedupeSystemMessages, buildConversationMessages |
| `src/utils/toolInput.ts` | isRecord, readRawValue, mergeToolInputObjects, mergeToolInputArguments |
| `src/services/conversation/AgentExecutor.ts` | constructor, registerStrategy, getSupportedStrategy, execute |
| `src/services/plan/TaskSplitOrchestrator.ts` | executeTurn, abort, cleanup |
| `src-tauri/src/commands/conversation/executor.rs` | execute, get_registry, execute_agent |

## Entry Points

Start here when exploring this area:

- **`mergeToolInputArguments`** (Function) — `src/utils/toolInput.ts:52`
- **`registerTraceTask`** (Function) — `src/services/conversation/ConversationService.ts:943`
- **`markMetric`** (Function) — `src/services/conversation/ConversationService.ts:981`
- **`normalizeBufferedMessageUpdate`** (Function) — `src/services/conversation/ConversationService.ts:1006`
- **`flushPendingUiUpdate`** (Function) — `src/services/conversation/ConversationService.ts:1028`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `CodexCliStrategy` | Class | `src/services/conversation/strategies/CodexCliStrategy.ts` | 2 |
| `ClaudeCliStrategy` | Class | `src/services/conversation/strategies/ClaudeCliStrategy.ts` | 2 |
| `ConversationService` | Class | `src/services/conversation/ConversationService.ts` | 255 |
| `mergeToolInputArguments` | Function | `src/utils/toolInput.ts` | 52 |
| `registerTraceTask` | Function | `src/services/conversation/ConversationService.ts` | 943 |
| `markMetric` | Function | `src/services/conversation/ConversationService.ts` | 981 |
| `normalizeBufferedMessageUpdate` | Function | `src/services/conversation/ConversationService.ts` | 1006 |
| `flushPendingUiUpdate` | Function | `src/services/conversation/ConversationService.ts` | 1028 |
| `scheduleUiFlush` | Function | `src/services/conversation/ConversationService.ts` | 1043 |
| `bufferMessageUpdate` | Function | `src/services/conversation/ConversationService.ts` | 1063 |
| `syncRealtimeUsageNotice` | Function | `src/services/conversation/ConversationService.ts` | 1137 |
| `clearRetryPresentationOnRecoveredStream` | Function | `src/services/conversation/ConversationService.ts` | 1233 |
| `onContent` | Function | `src/services/conversation/ConversationService.ts` | 1378 |
| `onThinking` | Function | `src/services/conversation/ConversationService.ts` | 1386 |
| `onThinkingStart` | Function | `src/services/conversation/ConversationService.ts` | 1395 |
| `onToolUse` | Function | `src/services/conversation/ConversationService.ts` | 1401 |
| `onToolInputDelta` | Function | `src/services/conversation/ConversationService.ts` | 1433 |
| `onToolResult` | Function | `src/services/conversation/ConversationService.ts` | 1448 |
| `onFileEdit` | Function | `src/services/conversation/ConversationService.ts` | 1479 |
| `onUsage` | Function | `src/services/conversation/ConversationService.ts` | 1490 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Stop_plan_split → Get_dev_default_persistence_path` | cross_community | 8 |
| `Stop_plan_split → Get_prod_default_persistence_path` | cross_community | 8 |
| `Stop_plan_split → Now` | cross_community | 7 |
| `Timer → GetCurrentAiMessage` | cross_community | 6 |
| `Timer → Now` | cross_community | 6 |
| `Timer → IsSnapshotProneContentRuntime` | cross_community | 6 |
| `ExecuteConversation → FindTextOverlapLength` | cross_community | 5 |
| `ExecuteConversation → GetCurrentAiMessage` | cross_community | 5 |
| `HandleStreamEvent → ClearScheduledUiFlush` | cross_community | 5 |
| `Timer → HashString` | cross_community | 5 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Stores | 10 calls |
| Commands | 4 calls |
| Strategies | 4 calls |
| Unattended | 2 calls |
| Usage | 2 calls |
| Compression | 1 calls |
| Scheduler | 1 calls |
| TaskSplitDialog | 1 calls |

## How to Explore

1. `gitnexus_context({name: "mergeToolInputArguments"})` — see callers and callees
2. `gitnexus_query({query: "conversation"})` — find related execution flows
3. Read key files listed above for implementation details
