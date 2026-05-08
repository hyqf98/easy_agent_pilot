---
name: unattended
description: "Skill for the Unattended area of easy-agent-pilot. 103 symbols across 17 files."
---

# Unattended

103 symbols | 17 files | Cohesion: 66%

## When to Use

- Working with code in `src-tauri/`
- Understanding how process_structured_intent, process_unattended_structured_intent, list_projects work
- Modifying unattended-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src-tauri/src/unattended/structured.rs` | compact_text, extract_task_title_from_raw_text, has_task_updates, build_project_summary, build_follow_up_action (+13) |
| `src-tauri/src/unattended/repository.rs` | list_channels, upsert_thread, record_event, list_events, update_account_sync_cursor (+9) |
| `src-tauri/src/unattended/commands.rs` | process_unattended_structured_intent, list_unattended_channels, record_unattended_event, send_unattended_text, list_unattended_events (+8) |
| `src/services/unattended/UnattendedService.ts` | toCamelCase, transformResult, listChannels, createChannel, listAccounts (+6) |
| `src-tauri/src/unattended/runtime.rs` | restore_runtime, handle_incoming_message, send_text, run_weixin_loop, has_task (+4) |
| `src/services/unattended/intentParser.ts` | normalize, extractPlanHint, extractTaskPriority, extractTaskStatus, extractQuotedSegment (+4) |
| `src/services/unattended/contextBuilder.ts` | formatValue, formatTaskStatus, buildProjectSection, buildPlanSection, buildTaskSection (+1) |
| `src-tauri/src/unattended/channels/weixin/api.rs` | send_text_message, with_base_url, get_bot_qrcode, get_updates, get_qrcode_status |
| `src-tauri/src/commands/project.rs` | list_project_memory_library_ids, hidden_mini_panel_project_id, list_projects |
| `src/stores/unattended.ts` | applyPendingChannelPatches, loadAll, initialize |

## Entry Points

Start here when exploring this area:

- **`process_structured_intent`** (Function) — `src-tauri/src/unattended/structured.rs:415`
- **`process_unattended_structured_intent`** (Function) — `src-tauri/src/unattended/commands.rs:268`
- **`list_projects`** (Function) — `src-tauri/src/commands/project.rs:310`
- **`applyPendingChannelPatches`** (Function) — `src/stores/unattended.ts:269`
- **`loadAll`** (Function) — `src/stores/unattended.ts:298`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `process_structured_intent` | Function | `src-tauri/src/unattended/structured.rs` | 415 |
| `process_unattended_structured_intent` | Function | `src-tauri/src/unattended/commands.rs` | 268 |
| `list_projects` | Function | `src-tauri/src/commands/project.rs` | 310 |
| `applyPendingChannelPatches` | Function | `src/stores/unattended.ts` | 269 |
| `loadAll` | Function | `src/stores/unattended.ts` | 298 |
| `initialize` | Function | `src/stores/unattended.ts` | 318 |
| `init_runtime_logging` | Function | `src-tauri/src/logging.rs` | 133 |
| `write_log` | Function | `src-tauri/src/logging.rs` | 150 |
| `run` | Function | `src-tauri/src/lib.rs` | 7 |
| `restore_runtime` | Function | `src-tauri/src/unattended/runtime.rs` | 82 |
| `list_channels` | Function | `src-tauri/src/unattended/repository.rs` | 106 |
| `list_unattended_channels` | Function | `src-tauri/src/unattended/commands.rs` | 22 |
| `start_scheduler` | Function | `src-tauri/src/scheduler/plan_scheduler.rs` | 17 |
| `write_runtime_log_command` | Function | `src-tauri/src/commands/runtime_log.rs` | 32 |
| `init_persistence_dirs` | Function | `src-tauri/src/commands/mod.rs` | 94 |
| `register` | Function | `src-tauri/src/commands/conversation/executor.rs` | 24 |
| `init_registry` | Function | `src-tauri/src/commands/conversation/executor.rs` | 78 |
| `detectUnattendedIntent` | Function | `src/services/unattended/intentParser.ts` | 103 |
| `extractTaskCreateDraft` | Function | `src/services/unattended/intentParser.ts` | 231 |
| `extractTaskUpdateDraft` | Function | `src/services/unattended/intentParser.ts` | 247 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Stop_plan_split → Get_dev_default_persistence_path` | cross_community | 8 |
| `Stop_plan_split → Get_prod_default_persistence_path` | cross_community | 8 |
| `Stop_plan_split → Now` | cross_community | 7 |
| `Data_management_roundtrip_works_in_isolated_home → Get_dev_default_persistence_path` | cross_community | 7 |
| `Data_management_roundtrip_works_in_isolated_home → Get_prod_default_persistence_path` | cross_community | 7 |
| `Process_structured_intent → Get_dev_default_persistence_path` | cross_community | 6 |
| `Process_structured_intent → Get_prod_default_persistence_path` | cross_community | 6 |
| `Get_unattended_weixin_login_status → Get_dev_default_persistence_path` | cross_community | 6 |
| `Get_unattended_weixin_login_status → Get_prod_default_persistence_path` | cross_community | 6 |
| `Run → Now` | cross_community | 5 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Commands | 32 calls |
| Scheduler | 3 calls |

## How to Explore

1. `gitnexus_context({name: "process_structured_intent"})` — see callers and callees
2. `gitnexus_query({query: "unattended"})` — find related execution flows
3. Read key files listed above for implementation details
