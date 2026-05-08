---
name: usage
description: "Skill for the Usage area of easy-agent-pilot. 28 symbols across 7 files."
---

# Usage

28 symbols | 7 files | Cohesion: 64%

## When to Use

- Working with code in `src/`
- Understanding how recordPlanSplitUsage, resolveRecordedTaskUsage, recordTaskUsageOnce work
- Modifying usage-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/services/usage/agentCliUsageRecorder.ts` | normalizeModelId, splitModelTokens, areEquivalentModelIds, resolveRecordedModelId, buildUsagePayload (+4) |
| `src/utils/runtimeUsage.ts` | normalizeTokenValue, resolveCountForDisplay, isCumulativeUsageRuntime, normalizeRuntimeUsage, mergeResponseUsageCounts (+1) |
| `src/services/usage/cliSessionUsageSnapshot.ts` | normalizeProvider, resolveRuntimeKeyForProvider, normalizeCount, normalizeSnapshot, resolveSessionCliSnapshotTarget (+1) |
| `src/stores/taskExecution.ts` | resolveRecordedTaskUsage, recordTaskUsageOnce, updateTaskTokenUsage |
| `src/components/message/ExecutionTimeline.vue` | resolveTimelineEntriesModel, resolveToolGroupModelLabel |
| `src/stores/taskSplit.ts` | recordPlanSplitUsage |
| `src/services/conversation/ConversationService.ts` | recordUsageOnce |

## Entry Points

Start here when exploring this area:

- **`recordPlanSplitUsage`** (Function) — `src/stores/taskSplit.ts:886`
- **`resolveRecordedTaskUsage`** (Function) — `src/stores/taskExecution.ts:483`
- **`recordTaskUsageOnce`** (Function) — `src/stores/taskExecution.ts:842`
- **`resolveRecordedModelId`** (Function) — `src/services/usage/agentCliUsageRecorder.ts:65`
- **`recordAgentCliUsage`** (Function) — `src/services/usage/agentCliUsageRecorder.ts:171`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `recordPlanSplitUsage` | Function | `src/stores/taskSplit.ts` | 886 |
| `resolveRecordedTaskUsage` | Function | `src/stores/taskExecution.ts` | 483 |
| `recordTaskUsageOnce` | Function | `src/stores/taskExecution.ts` | 842 |
| `resolveRecordedModelId` | Function | `src/services/usage/agentCliUsageRecorder.ts` | 65 |
| `recordAgentCliUsage` | Function | `src/services/usage/agentCliUsageRecorder.ts` | 171 |
| `recordAgentCliUsageInBackground` | Function | `src/services/usage/agentCliUsageRecorder.ts` | 198 |
| `recordUsageOnce` | Function | `src/services/conversation/ConversationService.ts` | 1115 |
| `resolveTimelineEntriesModel` | Function | `src/components/message/ExecutionTimeline.vue` | 112 |
| `resolveToolGroupModelLabel` | Function | `src/components/message/ExecutionTimeline.vue` | 123 |
| `normalizeTokenValue` | Function | `src/utils/runtimeUsage.ts` | 32 |
| `resolveCountForDisplay` | Function | `src/utils/runtimeUsage.ts` | 40 |
| `isCumulativeUsageRuntime` | Function | `src/utils/runtimeUsage.ts` | 44 |
| `normalizeRuntimeUsage` | Function | `src/utils/runtimeUsage.ts` | 49 |
| `mergeResponseUsageCounts` | Function | `src/utils/runtimeUsage.ts` | 138 |
| `mergeFinalUsageSnapshotCounts` | Function | `src/utils/runtimeUsage.ts` | 163 |
| `updateTaskTokenUsage` | Function | `src/stores/taskExecution.ts` | 448 |
| `findLatestUsageSnapshot` | Function | `src/services/usage/agentCliUsageRecorder.ts` | 104 |
| `resolveSessionCliSnapshotTarget` | Function | `src/services/usage/cliSessionUsageSnapshot.ts` | 72 |
| `readCliSessionUsageSnapshot` | Function | `src/services/usage/cliSessionUsageSnapshot.ts` | 104 |
| `normalizeModelId` | Function | `src/services/usage/agentCliUsageRecorder.ts` | 23 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `HandleStreamEvent → NormalizeTokenValue` | cross_community | 5 |
| `HandleStreamEvent → NormalizeModelId` | cross_community | 5 |
| `HandleStreamEvent → SplitModelTokens` | cross_community | 5 |
| `HandleStreamEvent → IsCumulativeUsageRuntime` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Stores | 3 calls |

## How to Explore

1. `gitnexus_context({name: "recordPlanSplitUsage"})` — see callers and callees
2. `gitnexus_query({query: "usage"})` — find related execution flows
3. Read key files listed above for implementation details
