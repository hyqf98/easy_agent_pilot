---
name: taskexecutionlog
description: "Skill for the TaskExecutionLog area of easy-agent-pilot. 29 symbols across 6 files."
---

# TaskExecutionLog

29 symbols | 6 files | Cohesion: 91%

## When to Use

- Working with code in `src/`
- Understanding how resolveKnownContextWindow, findConfiguredModel, matchById work
- Modifying taskexecutionlog-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/utils/todoToolCall.ts` | normalizeToolName, normalizeTodoStatus, resolveTodoStatus, resolveTodoArray, resolveTodoContent (+4) |
| `src/utils/configuredModelContext.ts` | normalizeModelId, collectModelIdAliases, modelIdsMatch, matchConfiguredModel, findConfiguredModel (+2) |
| `src/components/plan/taskExecutionLog/useTaskExecutionLog.ts` | tokenContextLimit, todoSnapshot, timelineEntries, buildRuntimeFallbackUsage, sortedTodoItems (+2) |
| `src/utils/modelContextWindow.ts` | normalizeModelId, collectModelAliases, resolveKnownContextWindow |
| `src/utils/structuredContent.ts` | stripExecutionResultFromContent, containsFormSchema |
| `src/utils/taskExecutionStatus.ts` | getTaskExecutionStatusMeta |

## Entry Points

Start here when exploring this area:

- **`resolveKnownContextWindow`** (Function) — `src/utils/modelContextWindow.ts:30`
- **`findConfiguredModel`** (Function) — `src/utils/configuredModelContext.ts:54`
- **`matchById`** (Function) — `src/utils/configuredModelContext.ts:63`
- **`resolveConfiguredContextWindow`** (Function) — `src/utils/configuredModelContext.ts:72`
- **`tokenContextLimit`** (Function) — `src/components/plan/taskExecutionLog/useTaskExecutionLog.ts:110`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `resolveKnownContextWindow` | Function | `src/utils/modelContextWindow.ts` | 30 |
| `findConfiguredModel` | Function | `src/utils/configuredModelContext.ts` | 54 |
| `matchById` | Function | `src/utils/configuredModelContext.ts` | 63 |
| `resolveConfiguredContextWindow` | Function | `src/utils/configuredModelContext.ts` | 72 |
| `tokenContextLimit` | Function | `src/components/plan/taskExecutionLog/useTaskExecutionLog.ts` | 110 |
| `extractTodoSnapshotFromToolCalls` | Function | `src/utils/todoToolCall.ts` | 158 |
| `todoSnapshot` | Function | `src/components/plan/taskExecutionLog/useTaskExecutionLog.ts` | 184 |
| `stripExecutionResultFromContent` | Function | `src/utils/structuredContent.ts` | 778 |
| `containsFormSchema` | Function | `src/utils/structuredContent.ts` | 854 |
| `timelineEntries` | Function | `src/components/plan/taskExecutionLog/useTaskExecutionLog.ts` | 303 |
| `buildRuntimeFallbackUsage` | Function | `src/components/plan/taskExecutionLog/useTaskExecutionLog.ts` | 311 |
| `sortTodoItems` | Function | `src/utils/todoToolCall.ts` | 204 |
| `weight` | Function | `src/utils/todoToolCall.ts` | 205 |
| `sortedTodoItems` | Function | `src/components/plan/taskExecutionLog/useTaskExecutionLog.ts` | 192 |
| `getTaskExecutionStatusMeta` | Function | `src/utils/taskExecutionStatus.ts` | 52 |
| `statusText` | Function | `src/components/plan/taskExecutionLog/useTaskExecutionLog.ts` | 236 |
| `statusColor` | Function | `src/components/plan/taskExecutionLog/useTaskExecutionLog.ts` | 241 |
| `normalizeModelId` | Function | `src/utils/modelContextWindow.ts` | 11 |
| `collectModelAliases` | Function | `src/utils/modelContextWindow.ts` | 15 |
| `normalizeModelId` | Function | `src/utils/configuredModelContext.ts` | 5 |

## Connected Areas

| Area | Connections |
|------|-------------|
| MessageBubble | 2 calls |
| Cluster_5 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "resolveKnownContextWindow"})` — see callers and callees
2. `gitnexus_query({query: "taskexecutionlog"})` — find related execution flows
3. Read key files listed above for implementation details
