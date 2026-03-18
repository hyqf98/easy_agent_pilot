use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliTimeoutKind {
    Startup,
    Idle,
    Hard,
}

impl CliTimeoutKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Startup => "startup_timeout",
            Self::Idle => "idle_timeout",
            Self::Hard => "hard_timeout",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CliTimeoutConfig {
    pub startup: Duration,
    pub idle: Duration,
    pub hard: Duration,
}

impl Default for CliTimeoutConfig {
    fn default() -> Self {
        Self {
            startup: Duration::from_secs(60),
            idle: Duration::from_secs(180),
            hard: Duration::from_secs(900),
        }
    }
}

pub fn timeout_config_for_execution_mode(execution_mode: Option<&str>) -> CliTimeoutConfig {
    if matches!(execution_mode, Some("task_split")) {
        return CliTimeoutConfig {
            startup: Duration::from_secs(180),
            idle: Duration::from_secs(240),
            hard: Duration::from_secs(900),
        };
    }

    CliTimeoutConfig::default()
}

#[derive(Debug, Clone, Copy)]
pub struct CliExecutionSnapshot {
    pub started_at: Instant,
    pub first_meaningful_event_at: Option<Instant>,
    pub last_activity_at: Option<Instant>,
    pub process_exited_at: Option<Instant>,
    pub stderr_warning_count: u32,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct CliExecutionMonitor {
    state: Arc<Mutex<CliExecutionSnapshot>>,
}

impl CliExecutionMonitor {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(CliExecutionSnapshot {
                started_at: Instant::now(),
                first_meaningful_event_at: None,
                last_activity_at: None,
                process_exited_at: None,
                stderr_warning_count: 0,
                exit_code: None,
            })),
        }
    }

    pub fn note_activity(&self, meaningful: bool) {
        let now = Instant::now();
        let mut state = self.state.lock().expect("cli monitor poisoned");
        state.last_activity_at = Some(now);
        if meaningful && state.first_meaningful_event_at.is_none() {
            state.first_meaningful_event_at = Some(now);
        }
    }

    pub fn note_stderr_warning(&self) {
        let now = Instant::now();
        let mut state = self.state.lock().expect("cli monitor poisoned");
        state.last_activity_at = Some(now);
        state.stderr_warning_count += 1;
    }

    pub fn note_process_exit(&self, exit_code: Option<i32>) {
        let mut state = self.state.lock().expect("cli monitor poisoned");
        state.process_exited_at = Some(Instant::now());
        state.exit_code = exit_code;
    }

    pub fn snapshot(&self) -> CliExecutionSnapshot {
        *self.state.lock().expect("cli monitor poisoned")
    }
}

pub fn detect_cli_timeout(
    snapshot: &CliExecutionSnapshot,
    config: CliTimeoutConfig,
    now: Instant,
) -> Option<CliTimeoutKind> {
    if now.duration_since(snapshot.started_at) >= config.hard {
        return Some(CliTimeoutKind::Hard);
    }

    if snapshot.first_meaningful_event_at.is_none()
        && now.duration_since(snapshot.started_at) >= config.startup
    {
        return Some(CliTimeoutKind::Startup);
    }

    if snapshot.first_meaningful_event_at.is_some() {
        let last_activity_at = snapshot.last_activity_at.unwrap_or(snapshot.started_at);
        if now.duration_since(last_activity_at) >= config.idle {
            return Some(CliTimeoutKind::Idle);
        }
    }

    None
}

pub fn build_timeout_error_message(
    provider: &str,
    timeout_kind: CliTimeoutKind,
    snapshot: &CliExecutionSnapshot,
    now: Instant,
) -> String {
    let total_secs = now.duration_since(snapshot.started_at).as_secs_f64();
    let first_event_secs = snapshot
        .first_meaningful_event_at
        .map(|ts| ts.duration_since(snapshot.started_at).as_secs_f64());
    let idle_secs = snapshot
        .last_activity_at
        .map(|ts| now.duration_since(ts).as_secs_f64())
        .unwrap_or(total_secs);

    format!(
        "{provider} CLI {timeout_kind} after {total_secs:.1}s (first_meaningful={first_event}, idle={idle_secs:.1}s, stderr_warnings={}, exit_code={:?})",
        snapshot.stderr_warning_count,
        snapshot.exit_code,
        timeout_kind = timeout_kind.as_str(),
        first_event = first_event_secs
            .map(|secs| format!("{secs:.1}s"))
            .unwrap_or_else(|| "none".to_string())
    )
}

pub fn build_execution_summary(snapshot: &CliExecutionSnapshot, finished_at: Instant) -> String {
    let total_secs = finished_at
        .duration_since(snapshot.started_at)
        .as_secs_f64();
    let first_event_secs = snapshot
        .first_meaningful_event_at
        .map(|ts| ts.duration_since(snapshot.started_at).as_secs_f64());
    let last_activity_secs = snapshot
        .last_activity_at
        .map(|ts| ts.duration_since(snapshot.started_at).as_secs_f64());
    let process_exit_secs = snapshot
        .process_exited_at
        .map(|ts| ts.duration_since(snapshot.started_at).as_secs_f64());

    format!(
        "elapsed={total_secs:.2}s, first_meaningful={}, last_activity={}, process_exit={}, stderr_warnings={}, exit_code={:?}",
        first_event_secs
            .map(|secs| format!("{secs:.2}s"))
            .unwrap_or_else(|| "none".to_string()),
        last_activity_secs
            .map(|secs| format!("{secs:.2}s"))
            .unwrap_or_else(|| "none".to_string()),
        process_exit_secs
            .map(|secs| format!("{secs:.2}s"))
            .unwrap_or_else(|| "none".to_string()),
        snapshot.stderr_warning_count,
        snapshot.exit_code
    )
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

#[cfg(test)]
mod tests {
    use super::{
        build_timeout_error_message, detect_cli_timeout, CliExecutionSnapshot, CliTimeoutConfig,
        CliTimeoutKind,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn detects_startup_timeout_before_any_meaningful_output() {
        let started_at = Instant::now();
        let snapshot = CliExecutionSnapshot {
            started_at,
            first_meaningful_event_at: None,
            last_activity_at: None,
            process_exited_at: None,
            stderr_warning_count: 0,
            exit_code: None,
        };

        let timeout = detect_cli_timeout(
            &snapshot,
            CliTimeoutConfig {
                startup: Duration::from_secs(5),
                idle: Duration::from_secs(30),
                hard: Duration::from_secs(60),
            },
            started_at + Duration::from_secs(6),
        );

        assert_eq!(timeout, Some(CliTimeoutKind::Startup));
    }

    #[test]
    fn detects_idle_timeout_after_meaningful_output() {
        let started_at = Instant::now();
        let first_event_at = started_at + Duration::from_secs(2);
        let snapshot = CliExecutionSnapshot {
            started_at,
            first_meaningful_event_at: Some(first_event_at),
            last_activity_at: Some(first_event_at),
            process_exited_at: None,
            stderr_warning_count: 1,
            exit_code: None,
        };

        let timeout = detect_cli_timeout(
            &snapshot,
            CliTimeoutConfig {
                startup: Duration::from_secs(5),
                idle: Duration::from_secs(10),
                hard: Duration::from_secs(60),
            },
            first_event_at + Duration::from_secs(11),
        );

        assert_eq!(timeout, Some(CliTimeoutKind::Idle));
    }

    #[test]
    fn timeout_error_message_contains_diagnostics() {
        let started_at = Instant::now();
        let first_event_at = started_at + Duration::from_secs(1);
        let snapshot = CliExecutionSnapshot {
            started_at,
            first_meaningful_event_at: Some(first_event_at),
            last_activity_at: Some(first_event_at + Duration::from_secs(2)),
            process_exited_at: None,
            stderr_warning_count: 3,
            exit_code: None,
        };

        let message = build_timeout_error_message(
            "Codex",
            CliTimeoutKind::Idle,
            &snapshot,
            started_at + Duration::from_secs(20),
        );

        assert!(message.contains("Codex CLI idle_timeout"));
        assert!(message.contains("stderr_warnings=3"));
        assert!(message.contains("first_meaningful=1.0s"));
    }
}
