use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

use crate::commands::conversation::types::CliStreamEvent;
use crate::commands::conversation::types::MessageInput;
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
                input_tokens: event.input_tokens,
                output_tokens: event.output_tokens,
                raw_input_tokens: event.raw_input_tokens,
                raw_output_tokens: event.raw_output_tokens,
                cache_read_input_tokens: event.cache_read_input_tokens,
                cache_creation_input_tokens: event.cache_creation_input_tokens,
                model: event.model.clone(),
                external_session_id: event.external_session_id.clone(),
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
        external_session_id: None,
    raw_input_tokens: None,
    raw_output_tokens: None,
    cache_read_input_tokens: None,
    cache_creation_input_tokens: None,
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
        external_session_id: None,
    raw_input_tokens: None,
    raw_output_tokens: None,
    cache_read_input_tokens: None,
    cache_creation_input_tokens: None,
    }
}

pub fn build_system_event(session_id: &str, content: String) -> CliStreamEvent {
    CliStreamEvent {
        event_type: "system".to_string(),
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
        external_session_id: None,
    raw_input_tokens: None,
    raw_output_tokens: None,
    cache_read_input_tokens: None,
    cache_creation_input_tokens: None,
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

impl CliTimeoutConfig {
    const fn from_secs(startup: u64, idle: u64, hard: u64) -> Self {
        Self {
            startup: Duration::from_secs(startup),
            idle: Duration::from_secs(idle),
            hard: Duration::from_secs(hard),
        }
    }
}

impl Default for CliTimeoutConfig {
    fn default() -> Self {
        // 主会话需要允许长时间编码/测试，但仍保留启动与空闲保护。
        Self::from_secs(600, 1_800, 14_400)
    }
}

pub fn timeout_config_for_execution_mode(execution_mode: Option<&str>) -> CliTimeoutConfig {
    match execution_mode {
        // 任务拆分可能要消化较长上下文并生成结构化结果，给更宽松的启动与总时长。
        Some("task_split") => CliTimeoutConfig::from_secs(600, 1_800, 14_400),
        // 计划任务执行经常伴随大规模读写、构建和测试，保留更长的 idle/hard 窗口。
        Some("task_execution") => CliTimeoutConfig::from_secs(600, 3_600, 28_800),
        // SOLO 协调执行会在结构化调度和真实执行间循环切换，按任务执行级别保留最长预算。
        Some("solo_execution") => CliTimeoutConfig::from_secs(600, 3_600, 28_800),
        _ => CliTimeoutConfig::default(),
    }
}

pub fn describe_timeout_config(config: CliTimeoutConfig) -> String {
    format!(
        "startup={}s idle={}s hard={}s",
        config.startup.as_secs(),
        config.idle.as_secs(),
        config.hard.as_secs()
    )
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

pub fn build_cli_failure_report(
    provider: &str,
    session_id: &str,
    command: &str,
    working_directory: Option<&str>,
    failure_reason: &str,
    summary: &str,
    stdout_preview: Option<&str>,
    stderr_preview: Option<&str>,
    stdout_parse_error_count: usize,
    ignored_stderr_warning_count: u32,
) -> String {
    let mut segments = vec![
        format!("{provider} CLI failure"),
        format!("session_id={session_id}"),
        format!("reason={}", preview_text(failure_reason, 240)),
        format!("summary={summary}"),
    ];

    let normalized_command = command.trim();
    if !normalized_command.is_empty() {
        segments.push(format!(
            "command={}",
            preview_text(normalized_command, 320)
        ));
    }

    if let Some(cwd) = working_directory
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        segments.push(format!("cwd={cwd}"));
    }

    if stdout_parse_error_count > 0 {
        segments.push(format!("stdout_parse_errors={stdout_parse_error_count}"));
    }

    if ignored_stderr_warning_count > 0 {
        segments.push(format!(
            "ignored_stderr_warnings={ignored_stderr_warning_count}"
        ));
    }

    if let Some(stdout_preview) = stdout_preview
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        segments.push(format!(
            "stdout_preview={}",
            preview_text(stdout_preview, 240)
        ));
    }

    if let Some(stderr_preview) = stderr_preview
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        segments.push(format!(
            "stderr_preview={}",
            preview_text(stderr_preview, 240)
        ));
    }

    segments.join(" | ")
}

pub fn shell_escape(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub fn render_cli_message(message: &MessageInput) -> String {
    let mut sections = Vec::new();

    if !message.content.trim().is_empty() {
        sections.push(message.content.clone());
    }

    if let Some(attachments) = &message.attachments {
        if !attachments.is_empty() {
            let image_paths: Vec<String> = attachments
                .iter()
                .filter(|a| a.mime_type.starts_with("image/"))
                .filter(|a| !a.path.trim().is_empty())
                .map(|a| a.path.clone())
                .collect();
            if !image_paths.is_empty() {
                sections.push(format!(
                    "Attached image file paths:\n{}",
                    image_paths
                        .iter()
                        .map(|p| format!("- {}", p))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
        }
    }

    let body = if sections.is_empty() {
        "[Empty message]".to_string()
    } else {
        sections.join("\n\n")
    };

    format!("{}:\n{}", message.role, body)
}

pub fn extract_image_paths(messages: &[MessageInput]) -> Vec<String> {
    messages
        .iter()
        .filter(|m| m.role == "user")
        .filter_map(|m| m.attachments.as_ref())
        .flatten()
        .filter(|a| a.mime_type.starts_with("image/"))
        .filter(|a| !a.path.trim().is_empty())
        .map(|a| a.path.clone())
        .collect()
}

pub fn extract_runtime_system_notice(json: &serde_json::Value) -> Option<String> {
    let runtime_payload = find_runtime_notice_payload(json, 0)?;
    render_runtime_notice_markdown(&runtime_payload)
}

pub fn preview_text(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }

    normalized.chars().take(max_chars).collect::<String>() + "..."
}

const MAX_RUNTIME_NOTICE_DEPTH: usize = 6;

fn find_runtime_notice_payload(
    value: &serde_json::Value,
    depth: usize,
) -> Option<serde_json::Value> {
    if depth > MAX_RUNTIME_NOTICE_DEPTH {
        return None;
    }

    if has_runtime_notice_keys(value) {
        return Some(value.clone());
    }

    match value {
        serde_json::Value::String(text) => parse_embedded_json_value(text)
            .and_then(|parsed| find_runtime_notice_payload(&parsed, depth + 1)),
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(found) = find_runtime_notice_payload(item, depth + 1) {
                    return Some(found);
                }
            }
            None
        }
        serde_json::Value::Object(object) => {
            for key in [
                "output",
                "stdout",
                "payload",
                "data",
                "result",
                "response",
                "details",
                "hookSpecificOutput",
                "hook_output",
            ] {
                if let Some(candidate) = object.get(key) {
                    if let Some(found) = find_runtime_notice_payload(candidate, depth + 1) {
                        return Some(found);
                    }
                }
            }

            for candidate in object.values() {
                if let Some(found) = find_runtime_notice_payload(candidate, depth + 1) {
                    return Some(found);
                }
            }

            None
        }
        _ => None,
    }
}

fn parse_embedded_json_value(text: &str) -> Option<serde_json::Value> {
    let trimmed = text.trim();
    if trimmed.is_empty() || (!trimmed.contains('{') && !trimmed.contains('[')) {
        return None;
    }

    parse_json_blob_with_fallback(trimmed).ok()
}

fn has_runtime_notice_keys(value: &serde_json::Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };

    [
        "skills",
        "plugins",
        "mcp_servers",
        "mcpServers",
        "agents",
        "slash_commands",
        "slashCommands",
        "commands",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
}

fn render_runtime_notice_markdown(value: &serde_json::Value) -> Option<String> {
    let skill_names = extract_named_items(value.get("skills"));
    let plugin_names = extract_named_items(value.get("plugins"));
    let mcp_names = extract_mcp_items(value.get("mcp_servers").or_else(|| value.get("mcpServers")));
    let agent_names = extract_named_items(value.get("agents"));
    let command_names = extract_named_items(
        value
            .get("slash_commands")
            .or_else(|| value.get("slashCommands"))
            .or_else(|| value.get("commands")),
    );

    let lines = [
        format_notice_list("Skills", &skill_names),
        format_notice_list("Plugins", &plugin_names),
        format_notice_list("MCP", &mcp_names),
        format_notice_list("Agents", &agent_names),
        format_notice_list("Commands", &command_names),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if lines.is_empty() {
        return None;
    }

    Some(format!("### 已加载运行扩展\n{}", lines.join("\n")))
}

fn extract_named_items(value: Option<&serde_json::Value>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };

    let mut items = Vec::new();
    collect_named_items(value, &mut items);
    dedupe_notice_items(items)
}

fn collect_named_items(value: &serde_json::Value, items: &mut Vec<String>) {
    match value {
        serde_json::Value::String(text) => {
            if let Some(name) = normalize_notice_item(text) {
                items.push(name);
            }
        }
        serde_json::Value::Array(array) => {
            for item in array {
                collect_named_items(item, items);
            }
        }
        serde_json::Value::Object(object) => {
            if let Some(name) = object
                .get("name")
                .or_else(|| object.get("title"))
                .or_else(|| object.get("id"))
                .and_then(|value| value.as_str())
                .and_then(normalize_notice_item)
            {
                items.push(name);
                return;
            }

            for item in object.values() {
                collect_named_items(item, items);
            }
        }
        _ => {}
    }
}

fn extract_mcp_items(value: Option<&serde_json::Value>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };

    let mut items = Vec::new();

    match value {
        serde_json::Value::Array(array) => {
            for item in array {
                match item {
                    serde_json::Value::String(text) => {
                        if let Some(name) = normalize_notice_item(text) {
                            items.push(name);
                        }
                    }
                    serde_json::Value::Object(object) => {
                        let Some(name) = object
                            .get("name")
                            .and_then(|value| value.as_str())
                            .and_then(normalize_notice_item)
                        else {
                            continue;
                        };

                        let status = object.get("status").and_then(|value| value.as_str());
                        if matches!(status, Some("connected") | Some("ready") | None) {
                            items.push(name);
                        } else {
                            items.push(format!("{name}({})", status.unwrap_or_default()));
                        }
                    }
                    _ => {}
                }
            }
        }
        serde_json::Value::Object(object) => {
            for (name, config) in object {
                let Some(name) = normalize_notice_item(name) else {
                    continue;
                };
                let status = config.get("status").and_then(|value| value.as_str());
                if matches!(status, Some("connected") | Some("ready") | None) {
                    items.push(name);
                } else {
                    items.push(format!("{name}({})", status.unwrap_or_default()));
                }
            }
        }
        _ => {}
    }

    dedupe_notice_items(items)
}

fn normalize_notice_item(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let single_line = trimmed.lines().next().unwrap_or(trimmed).trim();
    if single_line.is_empty() {
        return None;
    }

    Some(preview_text(single_line, 48))
}

fn dedupe_notice_items(items: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();

    for item in items {
        if !deduped.contains(&item) {
            deduped.push(item);
        }
    }

    deduped
}

fn format_notice_list(label: &str, items: &[String]) -> Option<String> {
    if items.is_empty() {
        return None;
    }

    let visible_count = items.len().min(5);
    let visible_names = items[..visible_count].join("、");
    let suffix = if items.len() > visible_count {
        format!(" 等 {} 个", items.len())
    } else {
        format!(" ({})", items.len())
    };

    Some(format!("- {label}: {visible_names}{suffix}"))
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
        build_timeout_error_message, describe_timeout_config, detect_cli_timeout,
        extract_runtime_system_notice, timeout_config_for_execution_mode, CliExecutionSnapshot,
        CliTimeoutConfig, CliTimeoutKind,
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

    #[test]
    fn chat_mode_uses_longer_hard_timeout_budget() {
        let config = timeout_config_for_execution_mode(Some("chat"));

        assert_eq!(config.startup, Duration::from_secs(600));
        assert_eq!(config.idle, Duration::from_secs(1_800));
        assert_eq!(config.hard, Duration::from_secs(14_400));
    }

    #[test]
    fn task_execution_mode_uses_largest_timeout_budget() {
        let config = timeout_config_for_execution_mode(Some("task_execution"));

        assert_eq!(config.startup, Duration::from_secs(600));
        assert_eq!(config.idle, Duration::from_secs(3_600));
        assert_eq!(config.hard, Duration::from_secs(28_800));
    }

    #[test]
    fn timeout_description_reports_all_windows() {
        let text = describe_timeout_config(CliTimeoutConfig::from_secs(5, 10, 15));

        assert_eq!(text, "startup=5s idle=10s hard=15s");
    }

    #[test]
    fn extracts_runtime_notice_from_embedded_system_payload() {
        let payload = serde_json::json!({
            "type": "system",
            "subtype": "hook_response",
            "output": "{\"skills\":[{\"name\":\"frontend-design\"}],\"plugins\":[{\"name\":\"context7\"}],\"mcp_servers\":[{\"name\":\"ops-automation\",\"status\":\"connected\"}]}"
        });

        let notice = extract_runtime_system_notice(&payload).expect("runtime notice");

        assert!(notice.contains("### 已加载运行扩展"));
        assert!(notice.contains("Skills: frontend-design"));
        assert!(notice.contains("Plugins: context7"));
        assert!(notice.contains("MCP: ops-automation"));
    }
}
