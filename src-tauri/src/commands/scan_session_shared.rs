use std::fs;
use std::path::{Path, PathBuf};

fn is_separator_only_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }

    let mut has_separator = false;
    for ch in trimmed.chars() {
        match ch {
            '-' | '=' | '_' => has_separator = true,
            '|' | ':' | ' ' | '\t' => {}
            _ => return false,
        }
    }

    has_separator
}

pub(crate) fn clean_display_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_local_command = trimmed
        .strip_prefix("<local-command-stdout>")
        .and_then(|value| value.strip_suffix("</local-command-stdout>"))
        .unwrap_or(trimmed)
        .trim();

    let normalized_lines: Vec<&str> = without_local_command
        .lines()
        .map(str::trim_end)
        .filter(|line| !is_separator_only_line(line))
        .collect();

    let normalized = normalized_lines.join("\n");
    let normalized = normalized.trim();

    if normalized.is_empty() {
        return None;
    }

    Some(normalized.to_string())
}

fn extract_wrapped_user_prompt(text: &str) -> Option<String> {
    let cleaned = clean_display_text(text)?;
    let trimmed = cleaned.trim();

    if !(trimmed.starts_with("system:") || trimmed.starts_with("developer:")) {
        return Some(cleaned);
    }

    for marker in ["\nuser:\n", "\nuser: ", "user:\n", "user: "] {
        if let Some(index) = cleaned.rfind(marker) {
            let tail = &cleaned[index + marker.len()..];
            if let Some(unwrapped) = clean_display_text(tail) {
                return Some(unwrapped);
            }
        }
    }

    Some(cleaned)
}

fn format_json_string(text: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        return format_json_value(&value);
    }

    clean_display_text(text)
}

pub(crate) fn format_json_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(text) => clean_display_text(text),
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => Some(value.to_string()),
        serde_json::Value::Array(items) => {
            let parts: Vec<String> = items.iter().filter_map(format_content_block).collect();

            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n\n"))
            }
        }
        serde_json::Value::Object(_) => serde_json::to_string_pretty(value)
            .ok()
            .and_then(|text| clean_display_text(&text)),
    }
}

fn format_content_block(value: &serde_json::Value) -> Option<String> {
    if !value.is_object() {
        return format_json_value(value);
    }

    let block_type = value.get("type").and_then(|v| v.as_str());

    match block_type {
        Some("input_text") | Some("output_text") | Some("text") => value
            .get("text")
            .and_then(|v| v.as_str())
            .and_then(clean_display_text),
        Some("thinking") => value
            .get("thinking")
            .and_then(|v| v.as_str())
            .and_then(clean_display_text)
            .map(|text| format!("[Thinking]\n{}", text)),
        Some("tool_use") => {
            let tool_name = value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let tool_input = value
                .get("input")
                .and_then(format_json_value)
                .unwrap_or_default();

            if tool_input.is_empty() {
                Some(format!("[Tool Use] {}", tool_name))
            } else {
                Some(format!("[Tool Use] {}\n{}", tool_name, tool_input))
            }
        }
        Some("tool_result") => {
            let tool_use_id = value
                .get("tool_use_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let result = value
                .get("content")
                .and_then(format_json_value)
                .unwrap_or_default();

            if result.is_empty() {
                Some(format!("[Tool Result] {}", tool_use_id))
            } else {
                Some(format!("[Tool Result] {}\n{}", tool_use_id, result))
            }
        }
        _ => {
            if let Some(text) = value.get("text").and_then(|v| v.as_str()) {
                return clean_display_text(text);
            }

            if let Some(content) = value.get("content") {
                return format_json_value(content);
            }

            serde_json::to_string_pretty(value)
                .ok()
                .and_then(|text| clean_display_text(&text))
        }
    }
}

fn extract_payload_message_content(
    top_level_type: &str,
    payload: &serde_json::Value,
) -> Option<String> {
    let payload_type = payload
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    match (top_level_type, payload_type) {
        ("response_item", "message") => payload.get("content").and_then(format_json_value),
        ("response_item", "function_call") => {
            let tool_name = payload
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let arguments = payload
                .get("arguments")
                .and_then(|v| v.as_str())
                .and_then(format_json_string)
                .unwrap_or_default();

            if arguments.is_empty() {
                Some(format!("[Tool Use] {}", tool_name))
            } else {
                Some(format!("[Tool Use] {}\n{}", tool_name, arguments))
            }
        }
        ("response_item", "function_call_output") => {
            let call_id = payload
                .get("call_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let output = payload
                .get("output")
                .and_then(format_json_value)
                .unwrap_or_default();

            if output.is_empty() {
                Some(format!("[Tool Result] {}", call_id))
            } else {
                Some(format!("[Tool Result] {}\n{}", call_id, output))
            }
        }
        ("response_item", "reasoning") => {
            if let Some(summary) = payload.get("summary").and_then(format_json_value) {
                Some(format!("[Reasoning]\n{}", summary))
            } else {
                Some("[Reasoning] 内容不可直接展示".to_string())
            }
        }
        ("event_msg", "agent_message") | ("event_msg", "user_message") => payload
            .get("message")
            .and_then(|v| v.as_str())
            .and_then(clean_display_text),
        ("event_msg", "token_count") => Some("[Token Count] 使用统计已更新".to_string()),
        ("event_msg", "task_started") => Some("[Task Started] 任务已开始".to_string()),
        ("event_msg", "agent_reasoning") => payload
            .get("text")
            .and_then(|v| v.as_str())
            .and_then(clean_display_text)
            .or_else(|| Some("[Reasoning] 正在分析".to_string())),
        ("session_meta", _) => {
            let cwd = payload
                .get("cwd")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let cli_version = payload
                .get("cli_version")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let mut summary = String::from("[Session Meta]");
            if !cwd.is_empty() {
                summary.push_str(&format!("\n{}", cwd));
            }
            if !cli_version.is_empty() {
                summary.push_str(&format!("\nCLI {}", cli_version));
            }
            Some(summary)
        }
        ("turn_context", _) => payload
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|cwd| format!("[Turn Context]\n{}", cwd))
            .or_else(|| Some("[Turn Context] 上下文已初始化".to_string())),
        _ => payload
            .get("content")
            .and_then(format_json_value)
            .or_else(|| payload.get("output").and_then(format_json_value))
            .or_else(|| {
                payload
                    .get("message")
                    .and_then(|v| v.as_str())
                    .and_then(clean_display_text)
            })
            .or_else(|| {
                serde_json::to_string_pretty(payload)
                    .ok()
                    .and_then(|text| clean_display_text(&text))
            }),
    }
}

pub(crate) fn extract_jsonl_project_path(json: &serde_json::Value) -> Option<String> {
    json.get("cwd")
        .and_then(|v| v.as_str())
        .or_else(|| {
            json.get("payload")
                .and_then(|payload| payload.get("cwd"))
                .and_then(|v| v.as_str())
        })
        .map(|s| s.to_string())
}

pub(crate) fn extract_jsonl_role(json: &serde_json::Value) -> Option<String> {
    let top_level_type = json
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    if matches!(top_level_type, "user" | "assistant" | "system") {
        return Some(top_level_type.to_string());
    }

    json.get("payload")
        .and_then(|payload| payload.get("role"))
        .and_then(|v| v.as_str())
        .or_else(|| json.get("role").and_then(|v| v.as_str()))
        .or_else(|| {
            json.get("message")
                .and_then(|m| m.get("role"))
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            json.get("payload")
                .and_then(|payload| payload.get("type"))
                .and_then(|v| v.as_str())
                .and_then(|payload_type| match payload_type {
                    "user_message" => Some("user"),
                    "agent_message" => Some("assistant"),
                    _ => None,
                })
        })
        .map(|s| s.to_string())
}

pub(crate) fn extract_jsonl_message_type(json: &serde_json::Value) -> String {
    let top_level_type = json
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    match top_level_type {
        "response_item" => {
            if let Some(payload) = json.get("payload") {
                match payload.get("type").and_then(|v| v.as_str()) {
                    Some("message") => match payload.get("role").and_then(|v| v.as_str()) {
                        Some("developer") => "system".to_string(),
                        Some(role) => role.to_string(),
                        None => "message".to_string(),
                    },
                    Some("function_call") => "tool_use".to_string(),
                    Some("function_call_output") => "tool_result".to_string(),
                    Some("reasoning") => "reasoning".to_string(),
                    Some(other) => other.to_string(),
                    None => "response_item".to_string(),
                }
            } else {
                "response_item".to_string()
            }
        }
        "event_msg" => {
            if let Some(payload) = json.get("payload") {
                match payload.get("type").and_then(|v| v.as_str()) {
                    Some("agent_message") | Some("user_message") | Some("token_count") => {
                        "progress".to_string()
                    }
                    Some("task_started") => "system".to_string(),
                    Some("agent_reasoning") => "reasoning".to_string(),
                    Some(other) => other.to_string(),
                    None => "event_msg".to_string(),
                }
            } else {
                "event_msg".to_string()
            }
        }
        "session_meta" | "turn_context" => "system".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn extract_jsonl_message_content(json: &serde_json::Value) -> Option<String> {
    let message_type = extract_jsonl_message_type(json);

    if let Some(payload) = json.get("payload") {
        let top_level_type = json
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        if let Some(text) = extract_payload_message_content(top_level_type, payload) {
            if message_type == "user" {
                return extract_wrapped_user_prompt(&text);
            }
            return Some(text);
        }
    }

    if let Some(message) = json.get("message") {
        if let Some(content) = message.get("content") {
            if let Some(text) = format_json_value(content) {
                if message_type == "user" {
                    return extract_wrapped_user_prompt(&text);
                }
                return Some(text);
            }
        }
    }

    if let Some(content) = json.get("content") {
        if let Some(text) = format_json_value(content) {
            if message_type == "user" {
                return extract_wrapped_user_prompt(&text);
            }
            return Some(text);
        }
    }

    if let Some(result) = json.get("result") {
        if let Some(text) = format_json_value(result) {
            return Some(text);
        }
    }

    if let Some(data) = json.get("data") {
        let progress_type = data
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("progress");
        let hook_event = data
            .get("hookEvent")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let hook_name = data
            .get("hookName")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let command = data
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let mut summary = format!("[{}]", progress_type);
        if !hook_event.is_empty() {
            summary.push_str(&format!(" {}", hook_event));
        }
        if !hook_name.is_empty() {
            summary.push_str(&format!(" ({})", hook_name));
        }
        if !command.is_empty() {
            summary.push_str(&format!("\n{}", command));
        }
        return Some(summary);
    }

    match json.get("type").and_then(|v| v.as_str()) {
        Some("file-history-snapshot") => Some("[File Snapshot] 历史文件快照已更新".to_string()),
        Some(other) => Some(format!("[{}]", other)),
        None => None,
    }
}

fn cleanup_empty_parent_dirs(path: &Path) {
    if let Some(parent) = path.parent() {
        if let Ok(entries) = fs::read_dir(parent) {
            if entries.count() == 0 {
                let _ = fs::remove_dir(parent);

                if let Some(grandparent) = parent.parent() {
                    if grandparent
                        .file_name()
                        .map(|name| name == "projects")
                        .unwrap_or(false)
                    {
                        return;
                    }

                    if let Ok(entries) = fs::read_dir(grandparent) {
                        if entries.count() == 0 {
                            let _ = fs::remove_dir(grandparent);
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn delete_cli_session_path(path: &Path, cleanup_empty_dirs: bool) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("会话文件不存在: {}", path.display()));
    }

    fs::remove_file(path).map_err(|e| format!("无法删除会话文件: {}", e))?;

    if cleanup_empty_dirs {
        cleanup_empty_parent_dirs(path);
    }

    Ok(())
}

pub(crate) fn collect_jsonl_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_jsonl_files(&path, files);
            } else if path
                .extension()
                .map(|extension| extension == "jsonl")
                .unwrap_or(false)
            {
                files.push(path);
            }
        }
    }
}
