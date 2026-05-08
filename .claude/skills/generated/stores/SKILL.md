---
name: stores
description: "Skill for the Stores area of easy-agent-pilot. 993 symbols across 122 files."
---

# Stores

993 symbols | 122 files | Cohesion: 80%

## When to Use

- Working with code in `src/`
- Understanding how resolveSessionAgent, resolveSessionAgentId, buildCliEnvironmentNotice work
- Modifying stores-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/stores/taskSplit.ts` | buildSplitSystemPrompt, useTaskSplitStore, startBackgroundSession, initSession, startSubSplit (+60) |
| `src/stores/taskExecution.ts` | useTaskExecutionStore, finalizeRunningToolCalls, normalizeCliSessionProvider, isMissingRecordError, sleep (+57) |
| `src/stores/sessionExecution.ts` | useSessionExecutionStore, getExecutionState, getInputText, getIsQueueDraining, getIsAwaitingRetry (+50) |
| `src/stores/soloExecution.ts` | transformStep, buildSoloBindingKey, collectMountedMemoryLibraryIds, useSoloExecutionStore, setSteps (+42) |
| `src/stores/unattended.ts` | compactText, useUnattendedStore, createWeixinChannel, appendThreadTranscript, ensureThreadSession (+38) |
| `src/services/conversation/ConversationService.ts` | finalizePendingToolCalls, extractRawMemoryCaptureContent, resolveRequestedUsageModel, markInjectedSystemMessages, filterSessionScopedInjectedMessages (+35) |
| `src/stores/message.ts` | parseRuntimeNoticeNumber, useMessageStore, loadMessages, reconcilePersistedUsageDisplay, serializeToolCalls (+30) |
| `src/stores/skillConfig.ts` | getCliInventoryCacheKey, invalidateCliInventory, selectAgent, loadSdkConfigs, loadCliConfigs (+27) |
| `src/stores/agentConfig.ts` | useAgentConfigStore, loadRemoteModels, transformMcpConfig, transformSkillsConfig, transformPluginsConfig (+22) |
| `src/stores/session.ts` | useSessionStore, deleteSession, saveOpenSessions, transformSession, replaceProjectSessions (+20) |

## Entry Points

Start here when exploring this area:

- **`resolveSessionAgent`** (Function) — `src/utils/sessionAgent.ts:7`
- **`resolveSessionAgentId`** (Function) — `src/utils/sessionAgent.ts:39`
- **`buildCliEnvironmentNotice`** (Function) — `src/utils/runtimeNotice.ts:278`
- **`buildContextStrategyNotice`** (Function) — `src/utils/runtimeNotice.ts:378`
- **`upsertRuntimeNotice`** (Function) — `src/utils/runtimeNotice.ts:557`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `FileTraceCollector` | Class | `src/services/conversation/fileTraceCollector.ts` | 337 |
| `AppUpdateService` | Class | `src/services/appUpdate/AppUpdateService.ts` | 11 |
| `TauriUpdaterAdapter` | Class | `src/services/appUpdate/tauriUpdaterAdapter.ts` | 118 |
| `resolveSessionAgent` | Function | `src/utils/sessionAgent.ts` | 7 |
| `resolveSessionAgentId` | Function | `src/utils/sessionAgent.ts` | 39 |
| `buildCliEnvironmentNotice` | Function | `src/utils/runtimeNotice.ts` | 278 |
| `buildContextStrategyNotice` | Function | `src/utils/runtimeNotice.ts` | 378 |
| `upsertRuntimeNotice` | Function | `src/utils/runtimeNotice.ts` | 557 |
| `loadAgentMcpServers` | Function | `src/utils/mcpServerConfig.ts` | 90 |
| `formatAgentModelLabel` | Function | `src/utils/agentModelLabel.ts` | 8 |
| `useUnattendedStore` | Function | `src/stores/unattended.ts` | 246 |
| `createWeixinChannel` | Function | `src/stores/unattended.ts` | 385 |
| `appendThreadTranscript` | Function | `src/stores/unattended.ts` | 1012 |
| `ensureThreadSession` | Function | `src/stores/unattended.ts` | 1202 |
| `useTokenStore` | Function | `src/stores/token.ts` | 132 |
| `restorePersistedSessionTokens` | Function | `src/stores/token.ts` | 135 |
| `getTokenUsage` | Function | `src/stores/token.ts` | 200 |
| `updateRealtimeTokens` | Function | `src/stores/token.ts` | 292 |
| `clearRealtimeTokens` | Function | `src/stores/token.ts` | 310 |
| `hardClearSessionTokens` | Function | `src/stores/token.ts` | 314 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Register_shortcut → ExtractBalancedJsonRanges` | cross_community | 8 |
| `Register_shortcut → ParseJson` | cross_community | 8 |
| `Register_shortcut → ParseStreamPayloadMetadata` | cross_community | 8 |
| `HandleBulkSwitchRuntime → FormatPrimitiveError` | cross_community | 7 |
| `HandleBulkSwitchRuntime → IsRecord` | cross_community | 7 |
| `Register_shortcut → NormalizeFormSchemasForRendering` | cross_community | 7 |
| `HandleSave → FormatPrimitiveError` | cross_community | 6 |
| `HandleSave → IsRecord` | cross_community | 6 |
| `Timer → GetCurrentAiMessage` | cross_community | 6 |
| `Timer → Now` | cross_community | 6 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Unattended | 30 calls |
| Conversation | 23 calls |
| Usage | 22 calls |
| Composables | 13 calls |
| Plan | 12 calls |
| Compression | 12 calls |
| Memory | 6 calls |
| TaskSplitDialog | 5 calls |

## How to Explore

1. `gitnexus_context({name: "resolveSessionAgent"})` — see callers and callees
2. `gitnexus_query({query: "stores"})` — find related execution flows
3. Read key files listed above for implementation details
