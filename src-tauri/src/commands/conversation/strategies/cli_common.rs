use tauri::{AppHandle, Emitter};

use crate::commands::conversation::types::CliStreamEvent;
use crate::commands::plan_split::{record_plan_split_event, SplitStreamRecord};

pub fn emit_cli_event(
    app: &AppHandle,
    event_name: &str,
    plan_id: Option<&String>,
    event: &CliStreamEvent,
) {
    let _ = app.emit(event_name, event);

    if let Some(plan_id) = plan_id {
        let _ = record_plan_split_event(
            app,
            plan_id,
            &event.session_id,
            SplitStreamRecord {
                event_type: event.event_type.clone(),
                content: event.content.clone(),
                tool_name: event.tool_name.clone(),
                tool_call_id: event.tool_call_id.clone(),
                tool_input: event.tool_input.clone(),
                tool_result: event.tool_result.clone(),
                error: event.error.clone(),
            },
        );
    }
}

pub fn build_content_event(session_id: &str, content: String) -> CliStreamEvent {
    CliStreamEvent {
        event_type: "content".to_string(),
        session_id: session_id.to_string(),
        content: Some(content),
        tool_name: None,
        tool_call_id: None,
        tool_input: None,
        tool_result: None,
        error: None,
        input_tokens: None,
        output_tokens: None,
        model: None,
    }
}

pub fn build_error_event(session_id: &str, error: String) -> CliStreamEvent {
    CliStreamEvent {
        event_type: "error".to_string(),
        session_id: session_id.to_string(),
        content: None,
        tool_name: None,
        tool_call_id: None,
        tool_input: None,
        tool_result: None,
        error: Some(error),
        input_tokens: None,
        output_tokens: None,
        model: None,
    }
}

pub fn shell_escape(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub fn preview_text(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }

    normalized.chars().take(max_chars).collect::<String>() + "..."
}

pub fn parse_json_blob_with_fallback(
    output: &str,
) -> std::result::Result<serde_json::Value, serde_json::Error> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(output) {
        return Ok(value);
    }

    for line in output.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            return Ok(value);
        }
    }

    let mut parse_error: Option<serde_json::Error> = None;
    let snippets = extract_balanced_json_snippets(output);
    for snippet in snippets.iter().rev() {
        match serde_json::from_str::<serde_json::Value>(snippet) {
            Ok(value) => return Ok(value),
            Err(error) => parse_error = Some(error),
        }
    }

    if let Some(error) = parse_error {
        return Err(error);
    }

    serde_json::from_str::<serde_json::Value>(output)
}

pub fn extract_balanced_json_snippets(text: &str) -> Vec<String> {
    let mut snippets = Vec::new();
    let mut stack: Vec<char> = Vec::new();
    let mut start: Option<usize> = None;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            continue;
        }

        if ch == '{' || ch == '[' {
            if stack.is_empty() {
                start = Some(index);
            }
            stack.push(ch);
            continue;
        }

        if ch == '}' {
            if let Some('{') = stack.last() {
                stack.pop();
                if stack.is_empty() {
                    if let Some(s) = start {
                        snippets.push(text[s..=index].to_string());
                    }
                    start = None;
                }
            }
            continue;
        }

        if ch == ']' {
            if let Some('[') = stack.last() {
                stack.pop();
                if stack.is_empty() {
                    if let Some(s) = start {
                        snippets.push(text[s..=index].to_string());
                    }
                    start = None;
                }
            }
            continue;
        }
    }

    snippets
}

pub fn extract_structured_output_from_json_blob(parsed: &serde_json::Value) -> Option<String> {
    if parsed.get("structured_output").is_some() {
        return serde_json::to_string(parsed).ok();
    }

    extract_tool_output_from_result(parsed)
}

pub fn extract_error_from_json_blob(parsed: &serde_json::Value) -> Option<String> {
    if let Some(error) = parsed.get("error") {
        if let Some(error_str) = error.as_str() {
            return Some(error_str.to_string());
        }
        if let Some(error_obj) = error.as_object() {
            if let Some(message) = error_obj.get("message").and_then(|m| m.as_str()) {
                return Some(message.to_string());
            }
        }
    }

    if let Some(result) = parsed.get("result") {
        if let Some(content) = result.get("content") {
            if let Some(content_array) = content.as_array() {
                for item in content_array {
                    if item.get("type").and_then(|t| t.as_str()) == Some("error") {
                        if let Some(error_text) = item.get("error").and_then(|e| e.as_str()) {
                            return Some(error_text.to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

pub fn extract_result_content_from_json_blob(parsed: &serde_json::Value) -> Option<String> {
    parsed
        .get("result")
        .and_then(|result| result.as_str())
        .map(ToString::to_string)
}

fn extract_tool_output_from_result(parsed: &serde_json::Value) -> Option<String> {
    let result = parsed.get("result")?;
    let content = result.get("content")?;
    let content_array = content.as_array()?;

    let mut output_parts = Vec::new();

    for item in content_array {
        let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match item_type {
            "text" => {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    output_parts.push(text.to_string());
                }
            }
            "tool_use" => {
                if let Some(tool_name) = item.get("name").and_then(|n| n.as_str()) {
                    let tool_input = item
                        .get("input")
                        .and_then(|i| serde_json::to_string(i).ok())
                        .unwrap_or_else(|| "{}".to_string());
                    output_parts.push(format!("[Tool: {}]\n{}", tool_name, tool_input));
                }
            }
            _ => {}
        }
    }

    if output_parts.is_empty() {
        return None;
    }

    Some(output_parts.join("\n\n"))
}
