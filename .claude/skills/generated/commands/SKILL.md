---
name: commands
description: "Skill for the Commands area of easy-agent-pilot. 872 symbols across 56 files."
---

# Commands

872 symbols | 56 files | Cohesion: 70%

## When to Use

- Working with code in `src-tauri/`
- Understanding how record_plan_split_event, mark_plan_split_failed, start_plan_split work
- Modifying commands-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src-tauri/src/commands/scan.rs` | extract_session_project_path, extract_session_info, read_cli_session_detail, read_cli_session_detail_impl, infer_transport_from_fields (+71) |
| `src-tauri/src/commands/plan_split.rs` | is_terminal_session_status, parse_json_vec, read_session, insert_or_update_session, normalize_task_count_mode (+47) |
| `src-tauri/src/commands/agent_config.rs` | get_model_agent_id, update_agent_mcp_config, update_agent_skills_config, update_agent_plugins_config, update_agent_model (+43) |
| `src-tauri/src/commands/project.rs` | normalize_path_key, invalidate_project_file_cache_for_path, invalidate_project_file_cache_for_paths, resolve_path, scan_single_directory (+38) |
| `src-tauri/src/commands/memory.rs` | normalize_search_text, build_search_candidates, sanitize_fts_term, build_fts_match_query, build_matched_terms (+35) |
| `src-tauri/src/commands/provider_profile.rs` | update_provider_profile, fetch_all_opencode_provider_ids, list_opencode_models, map_provider_profile_row, refresh_cli_runtime_state (+34) |
| `src-tauri/src/commands/cli_config.rs` | json_to_toml_value, json_map_to_toml_table, read_opencode_config, sync_cli_items, new (+33) |
| `src-tauri/src/commands/task.rs` | get_task_runtime_binding_internal, get_task_runtime_binding, upsert_task_runtime_binding, delete_task_runtime_binding, get_split_session (+32) |
| `src-tauri/src/commands/cli_installer.rs` | detect_package_managers, detect_cli_installed, get_cli_install_options, extract_version_token, normalize_version_identity (+25) |
| `src-tauri/src/commands/data.rs` | get_db_path, query_count, query_size, table_exists, get_data_management_stats (+21) |

## Entry Points

Start here when exploring this area:

- **`record_plan_split_event`** (Function) — `src-tauri/src/commands/plan_split.rs:1436`
- **`mark_plan_split_failed`** (Function) — `src-tauri/src/commands/plan_split.rs:1537`
- **`start_plan_split`** (Function) — `src-tauri/src/commands/plan_split.rs:1817`
- **`update_plan_split_result`** (Function) — `src-tauri/src/commands/plan_split.rs:1877`
- **`submit_plan_split_form`** (Function) — `src-tauri/src/commands/plan_split.rs:1909`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `record_plan_split_event` | Function | `src-tauri/src/commands/plan_split.rs` | 1436 |
| `mark_plan_split_failed` | Function | `src-tauri/src/commands/plan_split.rs` | 1537 |
| `start_plan_split` | Function | `src-tauri/src/commands/plan_split.rs` | 1817 |
| `update_plan_split_result` | Function | `src-tauri/src/commands/plan_split.rs` | 1877 |
| `submit_plan_split_form` | Function | `src-tauri/src/commands/plan_split.rs` | 1909 |
| `stop_plan_split` | Function | `src-tauri/src/commands/plan_split.rs` | 1983 |
| `reset_plan_split_turn_for_restart` | Function | `src-tauri/src/commands/plan_split.rs` | 2029 |
| `is_execution_session_active_internal` | Function | `src-tauri/src/commands/conversation/executor.rs` | 110 |
| `is_execution_session_active` | Function | `src-tauri/src/commands/conversation/executor.rs` | 117 |
| `emit_cli_event` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 12 |
| `list_threads` | Function | `src-tauri/src/unattended/repository.rs` | 484 |
| `list_unattended_threads` | Function | `src-tauri/src/unattended/commands.rs` | 214 |
| `update_task_execution_log` | Function | `src-tauri/src/commands/task_execution.rs` | 437 |
| `list_task_execution_logs` | Function | `src-tauri/src/commands/task_execution.rs` | 455 |
| `clear_task_execution_logs` | Function | `src-tauri/src/commands/task_execution.rs` | 487 |
| `get_task_execution_log_stats` | Function | `src-tauri/src/commands/task_execution.rs` | 768 |
| `clear_plan_execution_results` | Function | `src-tauri/src/commands/task_execution.rs` | 804 |
| `get_task_runtime_binding` | Function | `src-tauri/src/commands/task.rs` | 1056 |
| `upsert_task_runtime_binding` | Function | `src-tauri/src/commands/task.rs` | 1066 |
| `delete_task_runtime_binding` | Function | `src-tauri/src/commands/task.rs` | 1097 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Stop_plan_split → Get_dev_default_persistence_path` | cross_community | 8 |
| `Stop_plan_split → Get_prod_default_persistence_path` | cross_community | 8 |
| `Register_shortcut → ExtractBalancedJsonRanges` | cross_community | 8 |
| `Register_shortcut → ParseJson` | cross_community | 8 |
| `Register_shortcut → ParseStreamPayloadMetadata` | cross_community | 8 |
| `Search_memory_suggestions → Get_dev_default_persistence_path` | cross_community | 7 |
| `Search_memory_suggestions → Get_prod_default_persistence_path` | cross_community | 7 |
| `Batch_delete_raw_memory_records → Get_dev_default_persistence_path` | cross_community | 7 |
| `Batch_delete_raw_memory_records → Get_prod_default_persistence_path` | cross_community | 7 |
| `Create_memory_library → Get_dev_default_persistence_path` | cross_community | 7 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Scheduler | 25 calls |
| Unattended | 10 calls |
| Conversation | 6 calls |
| Strategies | 5 calls |
| Stores | 2 calls |

## How to Explore

1. `gitnexus_context({name: "record_plan_split_event"})` — see callers and callees
2. `gitnexus_query({query: "commands"})` — find related execution flows
3. Read key files listed above for implementation details
