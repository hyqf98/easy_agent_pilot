---
name: memoryauthoringdialog
description: "Skill for the MemoryAuthoringDialog area of easy-agent-pilot. 30 symbols across 4 files."
---

# MemoryAuthoringDialog

30 symbols | 4 files | Cohesion: 90%

## When to Use

- Working with code in `src/`
- Understanding how getLocale, availableExperts, effectiveDraftMarkdown work
- Modifying memoryauthoringdialog-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | translate, isChineseLocale, buildBuiltinMemoryExpertPrompt, buildMemoryAuthoringSystemPrompt, createMessage (+22) |
| `src/i18n.ts` | getLocale |
| `src/services/conversation/AgentExecutor.ts` | isSupported |
| `src/components/memory/MemoryAuthoringDialog.vue` | handleInstructionKeydown |

## Entry Points

Start here when exploring this area:

- **`getLocale`** (Function) — `src/i18n.ts:51`
- **`availableExperts`** (Function) — `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts:275`
- **`effectiveDraftMarkdown`** (Function) — `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts:296`
- **`resolveFallbackAgent`** (Function) — `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts:324`
- **`resolveSelectedRuntime`** (Function) — `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts:333`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `getLocale` | Function | `src/i18n.ts` | 51 |
| `availableExperts` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 275 |
| `effectiveDraftMarkdown` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 296 |
| `resolveFallbackAgent` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 324 |
| `resolveSelectedRuntime` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 333 |
| `ensureDependenciesLoaded` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 363 |
| `resetState` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 375 |
| `openDialog` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 389 |
| `appendTimelineEntry` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 400 |
| `syncSelectedRecordsFromStore` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 409 |
| `runAuthoring` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 425 |
| `generateInitialDraft` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 587 |
| `submitInstruction` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 606 |
| `stopGeneration` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 636 |
| `handleInstructionKeydown` | Function | `src/components/memory/MemoryAuthoringDialog.vue` | 45 |
| `isSupported` | Method | `src/services/conversation/AgentExecutor.ts` | 42 |
| `translate` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 37 |
| `isChineseLocale` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 41 |
| `buildBuiltinMemoryExpertPrompt` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 45 |
| `buildMemoryAuthoringSystemPrompt` | Function | `src/components/memory/memoryAuthoringDialog/useMemoryAuthoringDialog.ts` | 61 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `HandleFailure → IsSupported` | cross_community | 5 |
| `OnDone → IsSupported` | cross_community | 5 |
| `HandleConfirmCompress → IsSupported` | cross_community | 5 |
| `RetryMessage → IsSupported` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Stores | 4 calls |
| Conversation | 1 calls |

## How to Explore

1. `gitnexus_context({name: "getLocale"})` — see callers and callees
2. `gitnexus_query({query: "memoryauthoringdialog"})` — find related execution flows
3. Read key files listed above for implementation details
