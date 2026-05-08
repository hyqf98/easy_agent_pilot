---
name: strategies
description: "Skill for the Strategies area of easy-agent-pilot. 189 symbols across 18 files."
---

# Strategies

189 symbols | 18 files | Cohesion: 82%

## When to Use

- Working with code in `src-tauri/`
- Understanding how build_content_event, classify_cli_completion, buildNonImageAttachmentPrompt work
- Modifying strategies-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src-tauri/src/commands/conversation/strategies/cli_common.rs` | build_content_event, build_error_event, build_system_event, extract_runtime_system_notice, find_runtime_notice_payload (+41) |
| `src-tauri/src/commands/conversation/strategies/codex_cli.rs` | parse_codex_json_blob_output, build_tool_input_delta_event, build_usage_event, extract_external_session_id, attach_external_session_id (+23) |
| `src-tauri/src/commands/conversation/strategies/opencode_cli.rs` | extract_external_session_id, attach_external_session_id, stringify_json_value, extract_textish_value, collect_textish_parts (+21) |
| `src-tauri/src/commands/conversation/strategies/abnormal_completion.rs` | classify_cli_completion, build_failure, normalize_text, is_retryable_failure, has_retryable_http_status (+20) |
| `src-tauri/src/commands/conversation/strategies/claude_cli.rs` | parse_claude_json_blob_output, build_thinking_event, build_thinking_start_event, build_tool_input_delta_event, extract_textish_value (+18) |
| `src/services/conversation/strategies/BaseAgentStrategy.ts` | parseToolPayload, validateContext, execute, buildRequest, toMessageInputs (+11) |
| `src-tauri/src/commands/conversation/strategies/codex_sdk.rs` | parse_openai_sse_event, parse_openai_stream_event, extract_textish_value, attachment_to_openai_content, build_openai_message_content |
| `src-tauri/src/commands/conversation/strategies/claude_sdk.rs` | parse_sse_event, parse_anthropic_stream_event, extract_textish_value, attachment_to_anthropic_content, build_anthropic_message_content |
| `src/services/conversation/runtimeProfiles.ts` | validate, validateAgentRuntime, getAgentRuntimeProfile |
| `src/modules/fileEditor/strategies/registry.ts` | normalizeExtension, createContext, getLanguageStrategy |

## Entry Points

Start here when exploring this area:

- **`build_content_event`** (Function) — `src-tauri/src/commands/conversation/strategies/cli_common.rs:46`
- **`classify_cli_completion`** (Function) — `src-tauri/src/commands/conversation/strategies/abnormal_completion.rs:125`
- **`buildNonImageAttachmentPrompt`** (Function) — `src/utils/attachmentMeta.ts:63`
- **`validateAgentRuntime`** (Function) — `src/services/conversation/runtimeProfiles.ts:144`
- **`build_error_event`** (Function) — `src-tauri/src/commands/conversation/strategies/cli_common.rs:67`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `OpenCodeCliStrategy` | Class | `src/services/conversation/strategies/OpenCodeCliStrategy.ts` | 2 |
| `CodexSdkStrategy` | Class | `src/services/conversation/strategies/CodexSdkStrategy.ts` | 2 |
| `ClaudeSdkStrategy` | Class | `src/services/conversation/strategies/ClaudeSdkStrategy.ts` | 2 |
| `BaseAgentStrategy` | Class | `src/services/conversation/strategies/BaseAgentStrategy.ts` | 56 |
| `build_content_event` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 46 |
| `classify_cli_completion` | Function | `src-tauri/src/commands/conversation/strategies/abnormal_completion.rs` | 125 |
| `buildNonImageAttachmentPrompt` | Function | `src/utils/attachmentMeta.ts` | 63 |
| `validateAgentRuntime` | Function | `src/services/conversation/runtimeProfiles.ts` | 144 |
| `build_error_event` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 67 |
| `build_system_event` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 88 |
| `extract_runtime_system_notice` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 707 |
| `parse_json_blob_with_fallback` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 974 |
| `extract_balanced_json_snippets` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 1007 |
| `extract_structured_output_from_json_blob` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 1073 |
| `extract_error_from_json_blob` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 1081 |
| `extract_result_content_from_json_blob` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 1110 |
| `preview_text` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 712 |
| `extract_file_paths` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 582 |
| `extract_image_paths` | Function | `src-tauri/src/commands/conversation/strategies/cli_common.rs` | 610 |
| `lookup_claude_tool_use_usage` | Function | `src-tauri/src/commands/conversation/strategies/claude_cli.rs` | 494 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Execute → Validate` | intra_community | 4 |
| `Execute → GetAgentRuntimeProfile` | cross_community | 3 |
| `Execute → ParseToolPayload` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Stores | 2 calls |
| Commands | 1 calls |
| Conversation | 1 calls |

## How to Explore

1. `gitnexus_context({name: "build_content_event"})` — see callers and callees
2. `gitnexus_query({query: "strategies"})` — find related execution flows
3. Read key files listed above for implementation details
