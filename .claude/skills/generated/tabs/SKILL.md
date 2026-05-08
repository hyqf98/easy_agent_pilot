---
name: tabs
description: "Skill for the Tabs area of easy-agent-pilot. 51 symbols across 7 files."
---

# Tabs

51 symbols | 7 files | Cohesion: 84%

## When to Use

- Working with code in `src/`
- Understanding how countReferences, setSelectedExpert, emptyForm work
- Modifying tabs-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/components/settings/tabs/ProviderProfileForm.vue` | formatInvokeError, loadOpenCodeModels, loadOpenCodeProviderApiKey, handleOpenCodeProviderChange, selectOpenCodeProvider (+21) |
| `src/components/settings/tabs/LogSettings.vue` | scrollContentToLatest, loadLogFile, loadLogs, handleManualRefresh, handleStartListening (+4) |
| `src/components/settings/tabs/AgentTeamsSettings.vue` | emptyForm, applyExpertToForm, selectExpert, handleCreate, handleCopy (+1) |
| `src/components/settings/tabs/ProviderSwitch.vue` | handleSwitch, handleDelete, showSuccess, showError |
| `src/stores/agentTeams.ts` | countReferences, setSelectedExpert |
| `src/utils/fsWatcher.ts` | isUnavailableError, startFsWatcher |
| `src/components/skill-config/tabs/McpConfigTab.vue` | goBackToList, handleSave |

## Entry Points

Start here when exploring this area:

- **`countReferences`** (Function) — `src/stores/agentTeams.ts:210`
- **`setSelectedExpert`** (Function) — `src/stores/agentTeams.ts:228`
- **`emptyForm`** (Function) — `src/components/settings/tabs/AgentTeamsSettings.vue:38`
- **`applyExpertToForm`** (Function) — `src/components/settings/tabs/AgentTeamsSettings.vue:115`
- **`selectExpert`** (Function) — `src/components/settings/tabs/AgentTeamsSettings.vue:146`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `countReferences` | Function | `src/stores/agentTeams.ts` | 210 |
| `setSelectedExpert` | Function | `src/stores/agentTeams.ts` | 228 |
| `emptyForm` | Function | `src/components/settings/tabs/AgentTeamsSettings.vue` | 38 |
| `applyExpertToForm` | Function | `src/components/settings/tabs/AgentTeamsSettings.vue` | 115 |
| `selectExpert` | Function | `src/components/settings/tabs/AgentTeamsSettings.vue` | 146 |
| `handleCreate` | Function | `src/components/settings/tabs/AgentTeamsSettings.vue` | 151 |
| `handleCopy` | Function | `src/components/settings/tabs/AgentTeamsSettings.vue` | 158 |
| `handleDelete` | Function | `src/components/settings/tabs/AgentTeamsSettings.vue` | 291 |
| `scrollContentToLatest` | Function | `src/components/settings/tabs/LogSettings.vue` | 86 |
| `loadLogFile` | Function | `src/components/settings/tabs/LogSettings.vue` | 111 |
| `loadLogs` | Function | `src/components/settings/tabs/LogSettings.vue` | 132 |
| `handleManualRefresh` | Function | `src/components/settings/tabs/LogSettings.vue` | 235 |
| `handleStartListening` | Function | `src/components/settings/tabs/LogSettings.vue` | 239 |
| `handleClearLogs` | Function | `src/components/settings/tabs/LogSettings.vue` | 252 |
| `startFsWatcher` | Function | `src/utils/fsWatcher.ts` | 13 |
| `stopWatchingLogs` | Function | `src/components/settings/tabs/LogSettings.vue` | 94 |
| `refreshLogsSilently` | Function | `src/components/settings/tabs/LogSettings.vue` | 171 |
| `bindLogWatcher` | Function | `src/components/settings/tabs/LogSettings.vue` | 203 |
| `formatInvokeError` | Function | `src/components/settings/tabs/ProviderProfileForm.vue` | 109 |
| `loadOpenCodeModels` | Function | `src/components/settings/tabs/ProviderProfileForm.vue` | 255 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `HandleExpandChange → IsUnavailableError` | cross_community | 6 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Stores | 4 calls |

## How to Explore

1. `gitnexus_context({name: "countReferences"})` — see callers and callees
2. `gitnexus_query({query: "tabs"})` — find related execution flows
3. Read key files listed above for implementation details
