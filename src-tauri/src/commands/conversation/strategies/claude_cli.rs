use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use std::{fs, path::Path};

use anyhow::Result;
use async_trait::async_trait;
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;
use uuid::Uuid;

use super::abnormal_completion::{
    classify_cli_completion, is_shared_benign_stderr_warning, CliTextFragment, CliTextSource,
};
use super::cli_common::{
    build_cli_failure_report, build_content_event, build_error_event, build_execution_summary,
    build_system_event, build_timeout_error_message, classify_cli_completion_disposition,
    describe_timeout_config, detect_cli_timeout, emit_cli_event, extract_error_from_json_blob,
    extract_result_content_from_json_blob, extract_runtime_system_notice,
    extract_structured_output_from_json_blob, parse_json_blob_with_fallback, preview_text,
    read_cli_timeout_minutes, render_cli_message, shell_escape, timeout_config_for_execution_mode,
    CliExecutionMonitor, NonImageAttachmentPromptMode,
};
use crate::commands::cli_support::{build_cli_launch_error_message, build_tokio_cli_command};
use crate::commands::conversation::abort::{
    clear_abort_flag, register_session_pid, set_abort_flag, should_abort, unregister_session_pid,
};
use crate::commands::conversation::strategy::{AgentExecutionStrategy, AgentRuntimeKind};
use crate::commands::conversation::types::{CliStreamEvent, ExecutionRequest, McpServerConfig};
use crate::commands::mcp_shared::parse_args_string;

/// Claude CLI 策略
pub struct ClaudeCliStrategy;

// 简单的日志宏
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            crate::logging::write_log("INFO", "claude-cli", &message);
        }
    };
}

macro_rules! log_error {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            crate::logging::write_log("ERROR", "claude-cli", &message);
            eprintln!("[ERROR][claude-cli] {}", message)
        }
    };
}

macro_rules! log_debug {
    ($($arg:tt)*) => {
        // DEBUG 日志已禁用，如需调试请取消注释
        // println!("[DEBUG][claude-cli] {}", format!($($arg)*))
    };
}

struct StdoutReadOutcome {
    emitted_content: bool,
    emitted_error: bool,
    emitted_non_error_event: bool,
    fragments: Vec<CliTextFragment>,
    preview: Option<String>,
    parse_error_count: usize,
}

impl StdoutReadOutcome {
    fn none() -> Self {
        Self {
            emitted_content: false,
            emitted_error: false,
            emitted_non_error_event: false,
            fragments: Vec::new(),
            preview: None,
            parse_error_count: 0,
        }
    }
}

struct StderrReadOutcome {
    emitted_error: bool,
    fragments: Vec<CliTextFragment>,
    preview: Option<String>,
    ignored_warning_count: u32,
}

impl StderrReadOutcome {
    fn none() -> Self {
        Self {
            emitted_error: false,
            fragments: Vec::new(),
            preview: None,
            ignored_warning_count: 0,
        }
    }
}

fn is_successful_event_type(event_type: &str) -> bool {
    !matches!(event_type, "error" | "usage" | "message_start")
}

fn is_meaningful_event_type(event_type: &str) -> bool {
    matches!(
        event_type,
        "content"
            | "thinking"
            | "thinking_start"
            | "tool_use"
            | "tool_input_delta"
            | "tool_result"
            | "file_edit"
            | "system"
    )
}

fn is_rmcp_transport_closed_warning(line: &str) -> bool {
    let normalized = line.to_lowercase();
    normalized.contains("rmcp::transport::worker")
        && normalized.contains("transport channel closed")
}

fn should_ignore_stderr_line(line: &str) -> bool {
    is_rmcp_transport_closed_warning(line) || is_shared_benign_stderr_warning(line)
}

fn should_treat_process_failure_as_success(
    stdout_outcome: &StdoutReadOutcome,
    stderr_outcome: &StderrReadOutcome,
) -> bool {
    (stdout_outcome.emitted_content || stdout_outcome.emitted_non_error_event)
        && !stdout_outcome.emitted_error
        && !stderr_outcome.emitted_error
}

fn collect_event_fragments(event: &CliStreamEvent) -> Vec<CliTextFragment> {
    let mut fragments = Vec::new();

    if let Some(fragment) = CliTextFragment::new(
        CliTextSource::Content,
        event.content.clone().unwrap_or_default(),
    ) {
        fragments.push(fragment);
    }
    if let Some(fragment) = CliTextFragment::new(
        CliTextSource::Error,
        event.error.clone().unwrap_or_default(),
    ) {
        fragments.push(fragment);
    }
    if let Some(fragment) = CliTextFragment::new(
        CliTextSource::ToolResult,
        event.tool_result.clone().unwrap_or_default(),
    ) {
        fragments.push(fragment);
    }
    if event.event_type == "system" {
        if let Some(fragment) = CliTextFragment::new(
            CliTextSource::System,
            event.content.clone().unwrap_or_default(),
        ) {
            fragments.push(fragment);
        }
    }

    fragments
}

fn build_mcp_config_json(servers: &[McpServerConfig]) -> String {
    let mut mcp_servers = serde_json::Map::new();

    for server in servers {
        let server_name = &server.name;
        let mut server_config: Option<serde_json::Value> = None;

        match server.transport_type.as_str() {
            "stdio" => {
                let mut args_list = Vec::new();
                if let Some(args_str) = &server.args {
                    args_list.extend(parse_args_string(Some(args_str)));
                }

                let mut env_map = serde_json::Map::new();
                if let Some(env_str) = &server.env {
                    if let Ok(env_obj) = serde_json::from_str::<serde_json::Value>(env_str) {
                        if let Some(obj) = env_obj.as_object() {
                            for (key, value) in obj {
                                env_map.insert(key.clone(), value.clone());
                            }
                        }
                    }
                }

                server_config = Some(serde_json::json!({
                    "type": "stdio",
                    "command": server.command.clone().unwrap_or_default(),
                    "args": args_list,
                    "env": env_map
                }));
            }
            "http" | "sse" => {
                let mut headers_map = serde_json::Map::new();
                if let Some(headers_str) = &server.headers {
                    if let Ok(headers_obj) = serde_json::from_str::<serde_json::Value>(headers_str)
                    {
                        if let Some(obj) = headers_obj.as_object() {
                            for (key, value) in obj {
                                headers_map.insert(key.clone(), value.clone());
                            }
                        }
                    }
                }

                server_config = Some(serde_json::json!({
                    "type": server.transport_type,
                    "url": server.url.clone().unwrap_or_default(),
                    "headers": headers_map
                }));
            }
            _ => {}
        }

        if let Some(server_config) = server_config {
            mcp_servers.insert(server_name.clone(), server_config);
        }
    }

    let mcp_config = serde_json::json!({
        "mcpServers": mcp_servers
    });

    serde_json::to_string(&mcp_config).unwrap_or_else(|_| "{}".to_string())
}

fn build_thinking_event(session_id: &str, content: String) -> CliStreamEvent {
    CliStreamEvent {
        event_type: "thinking".to_string(),
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

fn build_thinking_start_event(session_id: &str) -> CliStreamEvent {
    CliStreamEvent {
        event_type: "thinking_start".to_string(),
        session_id: session_id.to_string(),
        content: None,
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

fn build_tool_input_delta_event(
    session_id: &str,
    tool_call_id: Option<String>,
    partial_json: String,
) -> CliStreamEvent {
    CliStreamEvent {
        event_type: "tool_input_delta".to_string(),
        session_id: session_id.to_string(),
        content: None,
        tool_name: None,
        tool_call_id,
        tool_input: Some(partial_json),
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

fn extract_textish_value(value: Option<&serde_json::Value>) -> Option<String> {
    fn collect(value: &serde_json::Value, parts: &mut Vec<String>) {
        match value {
            serde_json::Value::Null => {}
            serde_json::Value::String(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    collect(item, parts);
                }
            }
            serde_json::Value::Object(map) => {
                for key in [
                    "thinking", "summary", "text", "content", "message", "value", "title",
                ] {
                    if let Some(nested) = map.get(key) {
                        collect(nested, parts);
                    }
                }
            }
            serde_json::Value::Bool(flag) => parts.push(flag.to_string()),
            serde_json::Value::Number(number) => parts.push(number.to_string()),
        }
    }

    let value = value?;
    let mut parts = Vec::new();
    collect(value, &mut parts);

    if parts.is_empty() {
        return None;
    }

    Some(parts.join("\n"))
}

fn extract_external_session_id(json: &serde_json::Value) -> Option<String> {
    [
        json.get("session_id"),
        json.get("sessionId"),
        json.pointer("/session/id"),
        json.pointer("/message/session_id"),
        json.pointer("/message/sessionId"),
        json.pointer("/result/session_id"),
        json.pointer("/result/sessionId"),
        json.pointer("/payload/id"),
        json.pointer("/session_meta/payload/id"),
    ]
    .into_iter()
    .flatten()
    .find_map(|value| value.as_str())
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
}

fn attach_external_session_id(
    mut event: CliStreamEvent,
    json: &serde_json::Value,
) -> CliStreamEvent {
    if event.external_session_id.is_none() {
        event.external_session_id = extract_external_session_id(json);
    }

    event
}

fn build_claude_project_slug(working_directory: &str) -> String {
    working_directory.trim().replace(['/', '\\', ':'], "-")
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ClaudeToolUseUsage {
    pub raw_input_tokens: Option<u32>,
    pub raw_output_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
    pub model: Option<String>,
}

fn find_claude_session_transcript(
    working_directory: Option<&str>,
    external_session_id: &str,
) -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let projects_dir = home_dir.join(".claude").join("projects");
    if !projects_dir.is_dir() {
        return None;
    }

    if let Some(working_directory) = working_directory
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let preferred = projects_dir
            .join(build_claude_project_slug(working_directory))
            .join(format!("{external_session_id}.jsonl"));
        if preferred.is_file() {
            return Some(preferred);
        }
    }

    let entries = fs::read_dir(&projects_dir).ok()?;
    for entry in entries.flatten() {
        let candidate = entry.path().join(format!("{external_session_id}.jsonl"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

fn lookup_tool_use_usage_from_transcript(
    transcript_path: &Path,
    external_session_id: &str,
    tool_call_id: Option<&str>,
    tool_name: Option<&str>,
) -> Option<ClaudeToolUseUsage> {
    let file_contents = fs::read_to_string(transcript_path).ok()?;

    for line in file_contents.lines().rev() {
        let json = serde_json::from_str::<serde_json::Value>(line).ok()?;
        if json.get("type").and_then(|value| value.as_str()) != Some("assistant") {
            continue;
        }

        let transcript_session_id = json
            .get("sessionId")
            .or_else(|| json.get("session_id"))
            .and_then(|value| value.as_str());
        if transcript_session_id != Some(external_session_id) {
            continue;
        }

        let message = json.get("message")?;
        let content_items = message.get("content").and_then(|value| value.as_array())?;
        let matches_tool_use = content_items.iter().any(|item| {
            if item.get("type").and_then(|value| value.as_str()) != Some("tool_use") {
                return false;
            }

            let item_tool_call_id = item.get("id").and_then(|value| value.as_str());
            let item_tool_name = item.get("name").and_then(|value| value.as_str());
            if let Some(expected_tool_call_id) = tool_call_id {
                if item_tool_call_id != Some(expected_tool_call_id) {
                    return false;
                }
            }
            if let Some(expected_tool_name) = tool_name {
                if item_tool_name != Some(expected_tool_name) {
                    return false;
                }
            }
            true
        });

        if !matches_tool_use {
            continue;
        }

        let usage = extract_usage_counts(message.get("usage"));
        if usage.raw_input_tokens.is_none()
            && usage.raw_output_tokens.is_none()
            && usage.cache_read_input_tokens.is_none()
            && usage.cache_creation_input_tokens.is_none()
        {
            continue;
        }

        let model = message
            .get("model")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());
        return Some(ClaudeToolUseUsage {
            raw_input_tokens: usage.raw_input_tokens,
            raw_output_tokens: usage.raw_output_tokens,
            cache_read_input_tokens: usage.cache_read_input_tokens,
            cache_creation_input_tokens: usage.cache_creation_input_tokens,
            model,
        });
    }

    None
}

pub(crate) fn lookup_claude_tool_use_usage(
    working_directory: Option<&str>,
    external_session_id: &str,
    tool_call_id: Option<&str>,
    tool_name: Option<&str>,
) -> Option<ClaudeToolUseUsage> {
    let transcript_path = find_claude_session_transcript(working_directory, external_session_id)?;
    lookup_tool_use_usage_from_transcript(
        &transcript_path,
        external_session_id,
        tool_call_id,
        tool_name,
    )
}

fn hydrate_tool_use_usage_from_transcript(
    mut event: CliStreamEvent,
    working_directory: Option<&str>,
) -> CliStreamEvent {
    if event.event_type != "tool_use" {
        return event;
    }

    let should_hydrate = event.raw_output_tokens.unwrap_or(0) == 0
        || event.raw_input_tokens.is_none()
        || event.cache_read_input_tokens.is_none()
        || event.external_session_id.is_none();
    if !should_hydrate {
        return event;
    }

    let Some(external_session_id) = event.external_session_id.clone() else {
        return event;
    };
    let Some(usage) = lookup_claude_tool_use_usage(
        working_directory,
        &external_session_id,
        event.tool_call_id.as_deref(),
        event.tool_name.as_deref(),
    ) else {
        return event;
    };

    event.raw_input_tokens = usage.raw_input_tokens;
    event.raw_output_tokens = usage.raw_output_tokens;
    event.cache_read_input_tokens = usage.cache_read_input_tokens;
    event.cache_creation_input_tokens = usage.cache_creation_input_tokens;
    if event.model.is_none() {
        event.model = usage.model;
    }
    event
}

struct TempMcpConfigFile {
    path: PathBuf,
}

impl TempMcpConfigFile {
    async fn create(contents: &str) -> Result<Self> {
        let path = std::env::temp_dir().join(format!("claude-mcp-config-{}.json", Uuid::new_v4()));
        tokio::fs::write(&path, contents).await?;
        Ok(Self { path })
    }

    fn to_path_string(&self) -> String {
        self.path.to_string_lossy().to_string()
    }
}

impl Drop for TempMcpConfigFile {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                log_error!(
                    "清理临时 MCP 配置文件失败: {} ({})",
                    self.path.display(),
                    error
                );
            }
        }
    }
}

#[async_trait]
impl AgentExecutionStrategy for ClaudeCliStrategy {
    fn kind(&self) -> AgentRuntimeKind {
        AgentRuntimeKind::ClaudeCli
    }

    async fn execute(&self, app: AppHandle, request: ExecutionRequest) -> Result<()> {
        let session_id = request.session_id.clone();
        let event_name = self.kind().event_name(&session_id);

        // 重置中断标志
        set_abort_flag(&session_id, false).await;

        // 转换请求格式
        let cli_path = request
            .cli_path
            .clone()
            .unwrap_or_else(|| "claude".to_string());
        let model_id = request.model_id.clone();
        let working_directory = request.working_directory.clone();

        // 调试日志：检查收到的消息
        log_info!("收到的消息数量: {}", request.messages.len());
        for (i, msg) in request.messages.iter().enumerate() {
            log_info!(
                "消息[{}]: role={}, content_len={}",
                i,
                msg.role,
                msg.content.len()
            );
        }
        let allowed_tools = request.allowed_tools.clone();
        let cli_output_format = request
            .cli_output_format
            .clone()
            .unwrap_or_else(|| "stream-json".to_string());
        let json_schema = request.json_schema.clone();
        let extra_cli_args = request.extra_cli_args.clone();
        let mcp_servers = request.mcp_servers.clone();
        let messages = request.messages.clone();
        let is_stream_json = cli_output_format == "stream-json";
        let schema_text = json_schema
            .as_deref()
            .map(str::trim)
            .filter(|schema| !schema.is_empty());
        let plan_id = request.plan_id.clone();
        let resume_session_id = request
            .resume_session_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        // Claude Code 支持 `echo "..." | claude -p`，统一走 stdin 可避免 Windows 206 长命令问题。
        let mut args = vec![
            "-p".to_string(),
            "--output-format".to_string(),
            cli_output_format.clone(),
        ];
        args.push("--dangerously-skip-permissions".to_string());

        // 非流式 JSON 输出时禁用 verbose，避免 stdout 里出现大段事件数组影响结构化提取
        if cli_output_format == "stream-json" {
            args.insert(0, "--verbose".to_string());
        }

        // 添加模型参数
        if let Some(model_id) = &model_id {
            let trimmed = model_id.trim();
            if !trimmed.is_empty() && trimmed != "default" {
                args.push("--model".to_string());
                args.push(trimmed.to_string());
            }
        }

        if let Some(resume_session_id) = &resume_session_id {
            args.push("--resume".to_string());
            args.push(resume_session_id.clone());
        }

        // 添加允许的工具
        if let Some(tools) = &allowed_tools {
            if !tools.is_empty() {
                args.push("--allowedTools".to_string());
                args.push(tools.join(","));
            }
        }

        let mut mcp_config_file: Option<TempMcpConfigFile> = None;
        if let Some(servers) = &mcp_servers {
            if !servers.is_empty() {
                let mcp_config = build_mcp_config_json(servers);
                log_info!("MCP 配置: {}", mcp_config);
                let file = TempMcpConfigFile::create(&mcp_config).await?;
                args.push("--mcp-config".to_string());
                args.push(file.to_path_string());
                mcp_config_file = Some(file);
            }
        }

        if let Some(schema) = schema_text {
            args.push("--json-schema".to_string());
            args.push(schema.to_string());
        }

        if let Some(custom_args) = &extra_cli_args {
            if !custom_args.is_empty() {
                args.extend(custom_args.iter().cloned());
            }
        }

        // 解析工作目录，用于设置命令的工作目录
        let resolved_working_dir: Option<String> = working_directory
            .as_ref()
            .map(|w| w.trim().to_string())
            .filter(|w| !w.is_empty());

        // 构建输入消息
        let input_text = messages
            .iter()
            .map(|message| render_cli_message(message, true, NonImageAttachmentPromptMode::All))
            .collect::<Vec<_>>()
            .join("\n\n");

        let full_command = build_full_claude_command(&cli_path, &args);
        log_info!("Claude CLI command: {}", full_command);
        // 执行命令
        let command_args = args.clone();
        let mut cmd = build_tokio_cli_command(&cli_path, &command_args);

        // 设置工作目录，确保文件读写操作在指定目录下进行
        if let Some(ref work_dir) = resolved_working_dir {
            cmd.current_dir(work_dir);
            log_info!("设置工作目录: {}", work_dir);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_remove("CLAUDECODE");

        let execution_started_at = Instant::now();
        let monitor = CliExecutionMonitor::new();
        let user_timeout = read_cli_timeout_minutes();
        let timeout_config =
            timeout_config_for_execution_mode(request.execution_mode.as_deref(), user_timeout);
        log_info!(
            "Claude CLI timeout config: mode={}, user_override={:?}, {}",
            request.execution_mode.as_deref().unwrap_or("chat"),
            user_timeout,
            describe_timeout_config(timeout_config)
        );
        let mut child = cmd.spawn().map_err(|error| {
            anyhow::anyhow!(build_cli_launch_error_message(
                "Claude",
                &cli_path,
                &error,
                resolved_working_dir.as_deref(),
                command_args.len(),
                input_text.chars().count(),
                true,
            ))
        })?;

        let stdin_write_handle = {
            let stdin_payload = input_text.clone();
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| anyhow::anyhow!("failed to acquire stdin"))?;

            Some(tokio::spawn(async move {
                if let Err(error) = stdin.write_all(stdin_payload.as_bytes()).await {
                    log_error!("[stdin] failed to write prompt: {}", error);
                    return;
                }

                if let Err(error) = stdin.shutdown().await {
                    log_error!("[stdin] failed to close stdin: {}", error);
                }
            }))
        };

        // 注册进程 PID，用于后续可能的中断操作
        if let Some(pid) = child.id() {
            register_session_pid(&session_id, pid).await;
        }

        // 读取输出
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("无法获取标准输出"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("无法获取标准错误"))?;

        let session_id_clone = session_id.clone();
        let app_clone = app.clone();
        let event_name_clone = event_name.clone();
        let plan_id_clone = plan_id.clone();
        let stdout_monitor = monitor.clone();
        let stdout_working_directory = resolved_working_dir.clone();

        // 处理标准输出
        let stdout_handle = tokio::spawn(async move {
            if is_stream_json {
                let reader = tokio::io::BufReader::new(stdout);
                let mut lines = reader.lines();
                let mut outcome = StdoutReadOutcome::none();
                let mut last_assistant_raw_input: Option<u32> = None;

                while let Ok(Some(line)) = lines.next_line().await {
                    if should_abort(&session_id_clone).await {
                        break;
                    }

                    match serde_json::from_str::<serde_json::Value>(&line) {
                        Ok(json_value) => {
                            let event = parse_claude_json_output(
                                &session_id_clone,
                                &json_value,
                                &mut last_assistant_raw_input,
                            )
                            .map(|event| {
                                hydrate_tool_use_usage_from_transcript(
                                    event,
                                    stdout_working_directory.as_deref(),
                                )
                            });
                            if let Some(event) = event {
                                outcome.emitted_non_error_event |=
                                    is_successful_event_type(&event.event_type);
                                outcome.emitted_content |= event.event_type == "content";
                                outcome.emitted_error |= event.event_type == "error";
                                outcome.fragments.extend(collect_event_fragments(&event));
                                if outcome.preview.is_none() {
                                    outcome.preview = event
                                        .content
                                        .clone()
                                        .or_else(|| event.error.clone())
                                        .or_else(|| event.tool_result.clone());
                                }
                                stdout_monitor
                                    .note_activity(is_meaningful_event_type(&event.event_type));
                                emit_cli_event(
                                    &app_clone,
                                    &event_name_clone,
                                    plan_id_clone.as_ref(),
                                    &event,
                                );
                            }
                        }
                        Err(_parse_error) => {
                            outcome.parse_error_count += 1;
                            if outcome.preview.is_none() {
                                outcome.preview = Some(line.clone());
                            }
                            log_debug!("[stdout] JSON 解析失败: {:?}", _parse_error);
                        }
                    }
                }

                return outcome;
            }

            let mut reader = tokio::io::BufReader::new(stdout);
            let mut full_output = String::new();
            if reader.read_to_string(&mut full_output).await.is_err() {
                log_error!("[stdout] 读取失败");
                return StdoutReadOutcome::none();
            }
            log_info!(
                "[stdout] 已读取完成，长度 {} 字符",
                full_output.chars().count()
            );

            if should_abort(&session_id_clone).await {
                return StdoutReadOutcome::none();
            }

            let normalized = full_output.trim();
            if normalized.is_empty() {
                log_error!("[stdout] 输出为空");
                return StdoutReadOutcome::none();
            }

            if let Some(event) = parse_claude_json_blob_output(&session_id_clone, normalized) {
                stdout_monitor.note_activity(is_meaningful_event_type(&event.event_type));
                emit_cli_event(
                    &app_clone,
                    &event_name_clone,
                    plan_id_clone.as_ref(),
                    &event,
                );
                log_info!("[stdout] 事件发送成功");
                return StdoutReadOutcome {
                    emitted_content: event.event_type == "content",
                    emitted_error: event.event_type == "error",
                    emitted_non_error_event: is_successful_event_type(&event.event_type),
                    fragments: collect_event_fragments(&event),
                    preview: event
                        .content
                        .clone()
                        .or_else(|| event.error.clone())
                        .or_else(|| event.tool_result.clone())
                        .or_else(|| Some(normalized.to_string())),
                    parse_error_count: 0,
                };
            }

            log_info!("[stdout] 无法解析为结构化输出，直接发送原始内容");
            let event = build_content_event(&session_id_clone, normalized.to_string());
            stdout_monitor.note_activity(true);
            emit_cli_event(
                &app_clone,
                &event_name_clone,
                plan_id_clone.as_ref(),
                &event,
            );
            StdoutReadOutcome {
                emitted_content: true,
                emitted_error: false,
                emitted_non_error_event: true,
                fragments: collect_event_fragments(&event),
                preview: Some(normalized.to_string()),
                parse_error_count: 0,
            }
        });

        let session_id_clone = session_id.clone();
        let app_clone = app.clone();
        let event_name_clone = event_name.clone();
        let plan_id_clone = plan_id.clone();
        let stderr_monitor = monitor.clone();

        // 处理标准错误
        let stderr_handle = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut outcome = StderrReadOutcome::none();
            let mut error_output = String::new();
            if reader.read_to_string(&mut error_output).await.is_err() {
                return StderrReadOutcome::none();
            }

            if error_output.is_empty() {
                return StderrReadOutcome::none();
            }

            // 检查是否是真正的错误消息
            let error_lines: Vec<&str> = error_output
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        return false;
                    }
                    if should_ignore_stderr_line(trimmed) {
                        stderr_monitor.note_stderr_warning();
                        outcome.ignored_warning_count += 1;
                        return false;
                    }
                    stderr_monitor.note_activity(false);
                    let line_lower = line.to_lowercase();
                    line_lower.contains("error")
                        || line_lower.contains("failed")
                        || line_lower.contains("exception")
                        || line_lower.contains("fatal")
                })
                .collect();

            if !error_lines.is_empty() {
                let error_msg = error_lines.join("\n");
                outcome.preview = Some(error_msg.clone());
                for line in &error_lines {
                    if let Some(fragment) =
                        CliTextFragment::new(CliTextSource::Stderr, (*line).to_string())
                    {
                        outcome.fragments.push(fragment);
                    }
                }
                let event = build_error_event(&session_id_clone, error_msg.clone());
                emit_cli_event(
                    &app_clone,
                    &event_name_clone,
                    plan_id_clone.as_ref(),
                    &event,
                );
                outcome.emitted_error = true;
                log_error!(
                    "[stderr] actionable_lines={}, preview={}",
                    error_lines.len(),
                    preview_text(&error_msg, 240)
                );
            } else if outcome.ignored_warning_count > 0 {
                log_info!(
                    "[stderr] ignored {} benign warning(s)",
                    outcome.ignored_warning_count
                );
            }

            outcome
        });

        let mut timeout_error_message: Option<String> = None;

        let status = loop {
            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    monitor.note_process_exit(exit_status.code());
                    break exit_status;
                }
                Ok(None) => {}
                Err(error) => return Err(error.into()),
            }

            let snapshot = monitor.snapshot();
            let now = Instant::now();
            if let Some(timeout_kind) = detect_cli_timeout(&snapshot, timeout_config, now) {
                let error_message =
                    build_timeout_error_message("Claude", timeout_kind, &snapshot, now);
                log_error!("{}", error_message);
                let error_event = build_error_event(&session_id, error_message.clone());
                emit_cli_event(&app, &event_name, plan_id.as_ref(), &error_event);
                timeout_error_message = Some(error_message);

                if let Err(error) = child.kill().await {
                    log_error!("终止超时的 Claude CLI 进程失败: {}", error);
                }

                let exit_status = child.wait().await?;
                monitor.note_process_exit(exit_status.code());
                break exit_status;
            }

            sleep(Duration::from_millis(250)).await;
        };
        let elapsed = execution_started_at.elapsed();
        log_info!(
            "CLI 进程已退出: exit_code={:?}, elapsed={:.2}s",
            status.code(),
            elapsed.as_secs_f64()
        );

        // 等待输出处理完成
        let stdout_outcome = stdout_handle.await?;
        let stderr_outcome = match stderr_handle.await {
            Ok(outcome) => outcome,
            Err(error) => {
                log_error!("[stderr] 任务等待失败: {}", error);
                StderrReadOutcome::none()
            }
        };
        if let Some(handle) = stdin_write_handle {
            if let Err(error) = handle.await {
                log_error!("[stdin] task join failed: {}", error);
            }
        }

        let finished_at = Instant::now();
        let summary = build_execution_summary(&monitor.snapshot(), finished_at);
        log_info!("CLI 执行摘要: {}", summary);
        let abort_requested = should_abort(&session_id).await;

        // 注销进程 PID
        unregister_session_pid(&session_id).await;

        let should_treat_failure_as_success =
            should_treat_process_failure_as_success(&stdout_outcome, &stderr_outcome);
        let mut completion_fragments = stdout_outcome.fragments.clone();
        completion_fragments.extend(stderr_outcome.fragments.clone());
        let detected_failure = classify_cli_completion(
            "Claude",
            &completion_fragments,
            stdout_outcome.emitted_error || stderr_outcome.emitted_error,
        );
        let should_complete_as_success = should_treat_failure_as_success
            || (detected_failure.is_none() && stdout_outcome.emitted_content);
        let execution_succeeded = status.success() || should_complete_as_success;
        let completion_disposition = classify_cli_completion_disposition(
            timeout_error_message.is_some(),
            abort_requested,
            status.code(),
            should_complete_as_success,
        );
        let done_emitted =
            timeout_error_message.is_none() && detected_failure.is_none() && execution_succeeded;
        log_info!(
            "CLI 终止判定: disposition={}, exit_code={:?}, abort_requested={}, timeout_triggered={}, should_complete_as_success={}, detected_failure={}, done_emitted={}, {}",
            completion_disposition.as_str(),
            status.code(),
            abort_requested,
            timeout_error_message.is_some(),
            should_complete_as_success,
            detected_failure.is_some(),
            done_emitted,
            summary
        );

        if done_emitted {
            let done_event = CliStreamEvent {
                event_type: "done".to_string(),
                session_id: session_id.clone(),
                content: None,
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
            };
            emit_cli_event(&app, &event_name, plan_id.as_ref(), &done_event);
            log_info!(
                "CLI done 事件已发送: disposition={}",
                completion_disposition.as_str()
            );
        }

        // 清理中断标志
        clear_abort_flag(&session_id).await;
        drop(mcp_config_file);

        if let Some(error_message) = timeout_error_message {
            log_error!(
                "{}",
                build_cli_failure_report(
                    "Claude",
                    &session_id,
                    &full_command,
                    resolved_working_dir.as_deref(),
                    &error_message,
                    &summary,
                    stdout_outcome.preview.as_deref(),
                    stderr_outcome.preview.as_deref(),
                    stdout_outcome.parse_error_count,
                    stderr_outcome.ignored_warning_count,
                )
            );
            return Err(anyhow::anyhow!(error_message));
        }

        if let Some(failure) = detected_failure {
            if !(stdout_outcome.emitted_error || stderr_outcome.emitted_error) {
                let error_event = build_error_event(&session_id, failure.message.clone());
                emit_cli_event(&app, &event_name, plan_id.as_ref(), &error_event);
            }
            log_error!(
                "{}",
                build_cli_failure_report(
                    "Claude",
                    &session_id,
                    &full_command,
                    resolved_working_dir.as_deref(),
                    &failure.message,
                    &summary,
                    stdout_outcome.preview.as_deref(),
                    stderr_outcome.preview.as_deref(),
                    stdout_outcome.parse_error_count,
                    stderr_outcome.ignored_warning_count,
                )
            );
            return Err(anyhow::anyhow!(failure.message));
        }

        if !status.success() {
            if should_complete_as_success {
                log_info!(
                    "忽略 CLI 非零/空退出码：已收到有效输出，exit_code={:?}, {}",
                    status.code(),
                    summary
                );
                return Ok(());
            }
            let failure_message = format!("CLI 执行失败，退出码: {:?}, {}", status.code(), summary);
            log_error!(
                "{}",
                build_cli_failure_report(
                    "Claude",
                    &session_id,
                    &full_command,
                    resolved_working_dir.as_deref(),
                    &failure_message,
                    &summary,
                    stdout_outcome.preview.as_deref(),
                    stderr_outcome.preview.as_deref(),
                    stdout_outcome.parse_error_count,
                    stderr_outcome.ignored_warning_count,
                )
            );
            return Err(anyhow::anyhow!(failure_message));
        }

        Ok(())
    }
}

fn build_full_claude_command(cli_path: &str, args: &[String]) -> String {
    let mut cmd_parts = Vec::new();
    cmd_parts.push(shell_escape(cli_path));
    cmd_parts.extend(args.iter().map(|arg| shell_escape(arg)));
    cmd_parts.push("<stdin>".to_string());
    cmd_parts.join(" ")
}

/// 解析 `--output-format json` 的整块输出
fn parse_claude_json_blob_output(session_id: &str, output: &str) -> Option<CliStreamEvent> {
    log_info!(
        "[parse] 开始解析 JSON blob, 长度: {}",
        output.chars().count()
    );

    let parsed = match parse_json_blob_with_fallback(output) {
        Ok(value) => value,
        Err(_e) => {
            log_debug!("[parse] JSON blob 解析失败: {:?}", _e);
            return None;
        }
    };

    if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
        log_info!(
            "[parse] JSON blob 解析成功, preview={}",
            preview_text(&pretty, 320)
        );
    }

    if let Some(content) = extract_runtime_system_notice(&parsed) {
        return Some(attach_external_session_id(
            build_system_event(session_id, content),
            &parsed,
        ));
    }

    if let Some(content) = extract_structured_output_from_json_blob(&parsed) {
        log_info!(
            "[parse] 提取到 structured_output, 长度: {}",
            content.chars().count()
        );
        return Some(attach_external_session_id(
            build_content_event(session_id, content),
            &parsed,
        ));
    }

    if let Some(error) = extract_error_from_json_blob(&parsed) {
        log_info!("[parse] 提取到 error: {}", error);
        return Some(attach_external_session_id(
            build_error_event(session_id, error),
            &parsed,
        ));
    }

    if let Some(content) = extract_result_content_from_json_blob(&parsed) {
        log_info!(
            "[parse] 提取到 result.content, 长度: {}",
            content.chars().count()
        );
        return Some(attach_external_session_id(
            build_content_event(session_id, content),
            &parsed,
        ));
    }

    if let Ok(raw_json) = serde_json::to_string(&parsed) {
        log_info!("[parse] 返回原始 JSON, 长度: {}", raw_json.chars().count());
        return Some(attach_external_session_id(
            build_content_event(session_id, raw_json),
            &parsed,
        ));
    }

    log_info!("[parse] 未提取到标准字段，放弃结构化解析");
    None
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct UsageCounts {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    raw_input_tokens: Option<u32>,
    raw_output_tokens: Option<u32>,
    cache_read_input_tokens: Option<u32>,
    cache_creation_input_tokens: Option<u32>,
}

fn extract_usage_counts(usage: Option<&serde_json::Value>) -> UsageCounts {
    let raw_input_tokens = usage
        .and_then(|u| u.get("input_tokens").or_else(|| u.get("inputTokens")))
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);
    let raw_output_tokens = usage
        .and_then(|u| u.get("output_tokens").or_else(|| u.get("outputTokens")))
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);

    let cache_read = usage
        .and_then(|u| {
            u.get("cache_read_input_tokens")
                .or_else(|| u.get("cacheReadInputTokens"))
        })
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);
    let cache_creation = usage
        .and_then(|u| {
            u.get("cache_creation_input_tokens")
                .or_else(|| u.get("cacheCreationInputTokens"))
        })
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);

    let has_non_zero_usage =
        raw_input_tokens.unwrap_or(0) > 0 || raw_output_tokens.unwrap_or(0) > 0;
    if !has_non_zero_usage {
        return UsageCounts::default();
    }

    let input_tokens = raw_input_tokens
        .map(|raw| {
            raw.saturating_sub(cache_read.unwrap_or(0))
                .saturating_sub(cache_creation.unwrap_or(0))
        })
        .or(Some(0));
    let output_tokens = raw_output_tokens.or(Some(0));

    UsageCounts {
        input_tokens,
        output_tokens,
        raw_input_tokens,
        raw_output_tokens,
        cache_read_input_tokens: cache_read,
        cache_creation_input_tokens: cache_creation,
    }
}

fn extract_result_model_name(json: &serde_json::Value) -> Option<String> {
    if let Some(model) = json.get("model").and_then(|value| value.as_str()) {
        return Some(model.to_string());
    }

    json.get("modelUsage")
        .and_then(|value| value.as_object())
        .and_then(|models| models.keys().next().cloned())
}

/// 解析 `--output-format stream-json` 的每行 JSON 输出
fn parse_claude_json_output(
    session_id: &str,
    json: &serde_json::Value,
    last_assistant_raw_input: &mut Option<u32>,
) -> Option<CliStreamEvent> {
    let event_type = json
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");

    let event = match event_type {
        "system" => {
            let subtype = json.get("subtype").and_then(|s| s.as_str()).unwrap_or("");

            if subtype == "compact_boundary" {
                let trigger = json
                    .pointer("/compactMetadata/trigger")
                    .and_then(|v| v.as_str())
                    .unwrap_or("auto");
                let pre_tokens = json
                    .pointer("/compactMetadata/preTokens")
                    .and_then(|v| v.as_u64());
                let mut content = format!("### CLI Context Compaction\nTrigger: {}", trigger);
                if let Some(tokens) = pre_tokens {
                    content = format!("{}\nPre-compaction tokens: {}", content, tokens);
                }
                Some(build_system_event(session_id, content))
            } else {
                let notice_event = extract_runtime_system_notice(json)
                    .map(|content| build_system_event(session_id, content));

                if notice_event.is_some() {
                    notice_event
                } else {
                    extract_external_session_id(json).map(|sid| CliStreamEvent {
                        event_type: "system".to_string(),
                        session_id: session_id.to_string(),
                        content: None,
                        tool_name: None,
                        tool_call_id: None,
                        tool_input: None,
                        tool_result: None,
                        error: None,
                        input_tokens: None,
                        output_tokens: None,
                        model: None,
                        external_session_id: Some(sid),
                        raw_input_tokens: None,
                        raw_output_tokens: None,
                        cache_read_input_tokens: None,
                        cache_creation_input_tokens: None,
                    })
                }
            }
        }
        "content_block_delta" => {
            let delta = json.get("delta")?;
            let delta_type = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match delta_type {
                "thinking_delta" => extract_textish_value(
                    delta
                        .get("thinking")
                        .or_else(|| delta.get("summary"))
                        .or_else(|| delta.get("text"))
                        .or_else(|| delta.get("content")),
                )
                .map(|thinking| build_thinking_event(session_id, thinking)),
                "text_delta" => extract_textish_value(
                    delta
                        .get("text")
                        .or_else(|| delta.get("content"))
                        .or_else(|| delta.get("message")),
                )
                .map(|text| build_content_event(session_id, text)),
                "input_json_delta" => {
                    let partial_json = delta.get("partial_json").and_then(|j| j.as_str())?;
                    let tool_call_id = json
                        .pointer("/content_block/id")
                        .and_then(|value| value.as_str())
                        .map(|value| value.to_string())
                        .or_else(|| {
                            json.get("index")
                                .and_then(|value| value.as_u64())
                                .map(|value| value.to_string())
                        });
                    Some(build_tool_input_delta_event(
                        session_id,
                        tool_call_id,
                        partial_json.to_string(),
                    ))
                }
                _ => None,
            }
        }
        "content_block_start" => {
            let content_block = json.get("content_block")?;
            let block_type = content_block
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("");

            match block_type {
                "thinking" | "reasoning" => Some(build_thinking_start_event(session_id)),
                "tool_use" => {
                    let tool_name = content_block.get("name").and_then(|n| n.as_str())?;
                    let tool_id = content_block
                        .get("id")
                        .and_then(|value| value.as_str())
                        .map(|value| value.to_string())
                        .or_else(|| {
                            json.get("index")
                                .and_then(|value| value.as_u64())
                                .map(|value| value.to_string())
                        })?;
                    Some(CliStreamEvent {
                        event_type: "tool_use".to_string(),
                        session_id: session_id.to_string(),
                        content: None,
                        tool_name: Some(tool_name.to_string()),
                        tool_call_id: Some(tool_id),
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
                    })
                }
                _ => None,
            }
        }
        "content_block_stop" => {
            // 内容块结束，暂时不处理
            None
        }
        "message_start" => {
            let _ = json.get("message")?;
            None
        }
        "message_delta" => {
            let _ = json.get("usage");
            None
        }
        // message_stop 只表示当前一轮模型消息结束，不代表整个 CLI 执行完成。
        // 在 agentic 工具调用场景中，CLI 会执行工具后继续下一轮对话，产生多个 message_stop。
        // 因此不能将 message_stop 转换为 done 事件，否则会导致前端过早标记消息为"已完成"。
        // 最终的 done 事件由进程退出后的代码统一发出。
        "message_stop" => {
            // 不再将 message_stop 映射为 done，也不消费其 usage。
            None
        }
        "result" => {
            let model = extract_result_model_name(json);
            let external_session_id = extract_external_session_id(json);
            let usage = extract_usage_counts(json.get("usage"));

            let context_window_input = (*last_assistant_raw_input).or(usage.raw_input_tokens);
            let output_tokens = usage.output_tokens;

            if model.is_some()
                || external_session_id.is_some()
                || context_window_input.is_some()
                || output_tokens.is_some()
            {
                Some(CliStreamEvent {
                    event_type: "usage".to_string(),
                    session_id: session_id.to_string(),
                    content: None,
                    tool_name: None,
                    tool_call_id: None,
                    tool_input: None,
                    tool_result: None,
                    error: None,
                    input_tokens: context_window_input,
                    output_tokens,
                    model,
                    external_session_id,
                    raw_input_tokens: context_window_input,
                    raw_output_tokens: usage.raw_output_tokens,
                    cache_read_input_tokens: None,
                    cache_creation_input_tokens: None,
                })
            } else {
                None
            }
        }
        "turn.completed" => {
            let _ = json.get("usage");
            None
        }
        "tool_use" => {
            let tool_name = json.get("name").and_then(|n| n.as_str())?;
            let tool_input = json
                .get("input")
                .and_then(|i| serde_json::to_string(i).ok());
            let tool_id = json
                .get("id")
                .and_then(|i| i.as_str())
                .map(|i| i.to_string());

            Some(CliStreamEvent {
                event_type: "tool_use".to_string(),
                session_id: session_id.to_string(),
                content: None,
                tool_name: Some(tool_name.to_string()),
                tool_call_id: tool_id,
                tool_input,
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
            })
        }
        "tool_result" => {
            let tool_id = json.get("tool_use_id").and_then(|i| i.as_str());
            let result_content = json.get("content").and_then(|c| c.as_str());

            Some(CliStreamEvent {
                event_type: "tool_result".to_string(),
                session_id: session_id.to_string(),
                content: None,
                tool_name: None,
                tool_call_id: tool_id.map(|i| i.to_string()),
                tool_input: None,
                tool_result: result_content.map(|c| c.to_string()),
                error: None,
                input_tokens: None,
                output_tokens: None,
                model: None,
                external_session_id: None,
                raw_input_tokens: None,
                raw_output_tokens: None,
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
            })
        }
        "error" => {
            let error_msg = json
                .get("error")
                .and_then(|e| e.as_str())
                .or_else(|| json.get("message").and_then(|m| m.as_str()))
                .unwrap_or("Unknown error");

            Some(build_error_event(session_id, error_msg.to_string()))
        }
        "assistant" => {
            let message = json.get("message")?;
            let content_array = message.get("content").and_then(|c| c.as_array())?;
            let model = message
                .get("model")
                .and_then(|m| m.as_str())
                .map(|m| m.to_string());
            let usage = extract_usage_counts(message.get("usage"));

            if usage.raw_input_tokens.is_some() {
                *last_assistant_raw_input = usage.raw_input_tokens;
            }

            // Claude CLI 在一次 agentic 执行中会输出多条 assistant 事件，
            // 同一 message/turn 的 usage 可能被重复附带在多条事件里。
            // 对标准 Claude，最终 result.usage 仍然是 canonical 来源；
            // 但部分 Claude 兼容源只会把 usage 挂在 assistant.message 上。
            // 这里仅透传 raw usage，交给前端按 runtime 归一化，避免重复累计。
            let input_tokens = None;
            let output_tokens = None;
            let _assistant_raw_input = usage.raw_input_tokens;
            let _assistant_raw_output = usage.raw_output_tokens;
            let _assistant_cache_read = usage.cache_read_input_tokens;
            let _assistant_cache_create = usage.cache_creation_input_tokens;

            // 遍历所有 content items，找到第一个有效的并返回
            // 优先级：thinking > text > tool_use
            for content_item in content_array {
                let item_type = content_item
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");

                match item_type {
                    "thinking" | "reasoning" => {
                        // 处理 thinking 类型
                        if let Some(thinking_text) = extract_textish_value(
                            content_item
                                .get("thinking")
                                .or_else(|| content_item.get("summary"))
                                .or_else(|| content_item.get("text"))
                                .or_else(|| content_item.get("content"))
                                .or_else(|| content_item.get("message")),
                        ) {
                            log_debug!("[parse] 找到 thinking 内容，长度: {}", thinking_text.len());
                            return Some(CliStreamEvent {
                                input_tokens,
                                output_tokens,
                                model: model.clone(),
                                external_session_id: extract_external_session_id(json),
                                raw_input_tokens: None,
                                raw_output_tokens: None,
                                cache_read_input_tokens: None,
                                cache_creation_input_tokens: None,
                                ..build_thinking_event(session_id, thinking_text)
                            });
                        }
                    }
                    "text" => {
                        let text = extract_textish_value(
                            content_item
                                .get("text")
                                .or_else(|| content_item.get("content"))
                                .or_else(|| content_item.get("message")),
                        )?;
                        return Some(CliStreamEvent {
                            event_type: "content".to_string(),
                            session_id: session_id.to_string(),
                            content: Some(text),
                            tool_name: None,
                            tool_call_id: None,
                            tool_input: None,
                            tool_result: None,
                            error: None,
                            input_tokens,
                            output_tokens,
                            model: model.clone(),
                            external_session_id: extract_external_session_id(json),
                            raw_input_tokens: None,
                            raw_output_tokens: None,
                            cache_read_input_tokens: None,
                            cache_creation_input_tokens: None,
                        });
                    }
                    "tool_use" => {
                        let tool_name = content_item.get("name").and_then(|n| n.as_str())?;
                        let tool_input = content_item
                            .get("input")
                            .and_then(|i| serde_json::to_string(i).ok());
                        let tool_id = content_item.get("id").and_then(|i| i.as_str());

                        return Some(CliStreamEvent {
                            event_type: "tool_use".to_string(),
                            session_id: session_id.to_string(),
                            content: None,
                            tool_name: Some(tool_name.to_string()),
                            tool_call_id: tool_id.map(|i| i.to_string()),
                            tool_input,
                            tool_result: None,
                            error: None,
                            input_tokens,
                            output_tokens,
                            model: model.clone(),
                            external_session_id: extract_external_session_id(json),
                            raw_input_tokens: None,
                            raw_output_tokens: None,
                            cache_read_input_tokens: None,
                            cache_creation_input_tokens: None,
                        });
                    }
                    _ => {
                        log_debug!("[parse] assistant 消息中未处理的内容类型: {}", item_type);
                        // 继续检查下一个 item
                        continue;
                    }
                }
            }

            // 如果没有找到有效的内容，返回 None
            log_debug!("[parse] assistant 消息中没有找到有效的内容");
            None
        }
        "user" => {
            let message = json.get("message")?;
            let content_array = message.get("content").and_then(|c| c.as_array())?;

            if let Some(content_item) = content_array.first() {
                let item_type = content_item
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");

                match item_type {
                    "tool_result" => {
                        let tool_use_id = content_item
                            .get("tool_use_id")
                            .and_then(|i| i.as_str())
                            .map(|s| s.to_string());
                        let result_content = content_item
                            .get("content")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string());

                        Some(CliStreamEvent {
                            event_type: "tool_result".to_string(),
                            session_id: session_id.to_string(),
                            content: None,
                            tool_name: None,
                            tool_call_id: tool_use_id,
                            tool_input: None,
                            tool_result: result_content,
                            error: None,
                            input_tokens: None,
                            output_tokens: None,
                            model: None,
                            external_session_id: None,
                            raw_input_tokens: None,
                            raw_output_tokens: None,
                            cache_read_input_tokens: None,
                            cache_creation_input_tokens: None,
                        })
                    }
                    _ => {
                        log_debug!("[parse] user 消息中未处理的内容类型: {}", item_type);
                        None
                    }
                }
            } else {
                None
            }
        }
        _ => {
            log_debug!("[parse] 未处理的事件类型: {}", event_type);
            None
        }
    };

    event.map(|event| attach_external_session_id(event, json))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        hydrate_tool_use_usage_from_transcript, parse_claude_json_output,
        should_ignore_stderr_line, should_treat_process_failure_as_success, StderrReadOutcome,
        StdoutReadOutcome,
    };

    #[test]
    fn ignores_rmcp_transport_closed_warning() {
        let warning = "rmcp::transport::worker: worker quit with fatal: Transport channel closed";
        assert!(should_ignore_stderr_line(warning));
    }

    #[test]
    fn treats_non_zero_exit_as_success_when_valid_output_arrived() {
        let stdout_outcome = StdoutReadOutcome {
            emitted_content: true,
            emitted_error: false,
            emitted_non_error_event: true,
            fragments: Vec::new(),
            preview: None,
            parse_error_count: 0,
        };
        let stderr_outcome = StderrReadOutcome {
            emitted_error: false,
            fragments: Vec::new(),
            preview: None,
            ignored_warning_count: 0,
        };

        assert!(should_treat_process_failure_as_success(
            &stdout_outcome,
            &stderr_outcome,
        ));
    }

    #[test]
    fn does_not_hide_real_stderr_errors() {
        let stdout_outcome = StdoutReadOutcome {
            emitted_content: true,
            emitted_error: false,
            emitted_non_error_event: true,
            fragments: Vec::new(),
            preview: None,
            parse_error_count: 0,
        };
        let stderr_outcome = StderrReadOutcome {
            emitted_error: true,
            fragments: Vec::new(),
            preview: None,
            ignored_warning_count: 0,
        };

        assert!(!should_treat_process_failure_as_success(
            &stdout_outcome,
            &stderr_outcome,
        ));
    }

    #[test]
    fn forwards_assistant_raw_usage_without_normalized_tokens() {
        let json = serde_json::json!({
            "type": "assistant",
            "message": {
                "type": "message",
                "model": "glm-5.1",
                "usage": {
                    "input_tokens": 10140,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0,
                    "output_tokens": 0
                },
                "role": "assistant",
                "id": "20260427135221d5f4e77d05f14038",
                "content": [
                    {
                        "type": "text",
                        "text": "ok"
                    }
                ]
            },
            "parent_tool_use_id": null,
            "session_id": "f942ab3e-be1a-4684-8bee-e88f444e1a0e",
            "uuid": "5bf1cbc1-af5e-4f1f-b47f-ff8e02337520"
        });

        let mut state: Option<u32> = None;
        let event = parse_claude_json_output("session-1", &json, &mut state)
            .expect("expected assistant event");

        assert_eq!(event.event_type, "content");
        assert_eq!(event.content.as_deref(), Some("ok"));
        assert_eq!(event.input_tokens, None);
        assert_eq!(event.output_tokens, None);
        assert_eq!(event.raw_input_tokens, None);
        assert_eq!(event.raw_output_tokens, None);
        assert_eq!(event.cache_read_input_tokens, None);
        assert_eq!(event.cache_creation_input_tokens, None);
        assert_eq!(event.model.as_deref(), Some("glm-5.1"));
        assert_eq!(
            event.external_session_id.as_deref(),
            Some("f942ab3e-be1a-4684-8bee-e88f444e1a0e")
        );
        assert_eq!(state, Some(10140));
    }

    #[test]
    fn parses_result_usage_from_usage_payload() {
        let json = serde_json::json!({
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "duration_ms": 11403,
            "duration_api_ms": 11343,
            "num_turns": 1,
            "result": "ok",
            "stop_reason": "end_turn",
            "session_id": "f942ab3e-be1a-4684-8bee-e88f444e1a0e",
            "usage": {
                "input_tokens": 65630,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 320,
                "output_tokens": 3
            },
            "modelUsage": {
                "glm-5.1": {
                    "inputTokens": 65630,
                    "outputTokens": 3
                }
            }
        });

        let mut state: Option<u32> = None;
        let event =
            parse_claude_json_output("session-1", &json, &mut state).expect("expected usage event");

        assert_eq!(event.event_type, "usage");
        assert_eq!(event.input_tokens, Some(65630));
        assert_eq!(event.output_tokens, Some(3));
        assert_eq!(event.raw_input_tokens, Some(65630));
        assert_eq!(event.raw_output_tokens, Some(3));
        assert_eq!(event.cache_read_input_tokens, None);
        assert_eq!(event.cache_creation_input_tokens, None);
        assert_eq!(event.model.as_deref(), Some("glm-5.1"));
        assert_eq!(
            event.external_session_id.as_deref(),
            Some("f942ab3e-be1a-4684-8bee-e88f444e1a0e")
        );
    }

    #[test]
    fn extracts_external_session_id_from_camel_case_field() {
        let json = serde_json::json!({
            "type": "assistant",
            "message": {
                "type": "message",
                "model": "glm-5.1",
                "usage": {
                    "input_tokens": 10,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0,
                    "output_tokens": 0
                },
                "role": "assistant",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "call_123",
                        "name": "StructuredOutput",
                        "input": {
                            "type": "form_request"
                        }
                    }
                ]
            },
            "sessionId": "camel-session-id"
        });

        let mut state: Option<u32> = None;
        let event = parse_claude_json_output("session-1", &json, &mut state)
            .expect("expected tool_use event");

        assert_eq!(event.event_type, "tool_use");
        assert_eq!(
            event.external_session_id.as_deref(),
            Some("camel-session-id")
        );
    }

    #[test]
    fn hydrates_tool_use_usage_from_claude_transcript() {
        let temp_root = std::env::temp_dir().join(format!(
            "easy-agent-claude-hydrate-{}",
            uuid::Uuid::new_v4()
        ));
        let project_dir = temp_root
            .join(".claude")
            .join("projects")
            .join("-Users-test-demo");
        fs::create_dir_all(&project_dir).expect("create project dir");
        let transcript_path = project_dir.join("session-123.jsonl");
        fs::write(
            &transcript_path,
            concat!(
                "{\"type\":\"assistant\",\"sessionId\":\"session-123\",\"message\":{\"model\":\"glm-5.1\",\"usage\":{\"input_tokens\":73887,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":50944,\"output_tokens\":916},\"content\":[{\"type\":\"tool_use\",\"id\":\"call_abc\",\"name\":\"StructuredOutput\",\"input\":{\"type\":\"form_request\"}}]}}\n"
            ),
        )
        .expect("write transcript");

        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &temp_root);

        let event = super::CliStreamEvent {
            event_type: "tool_use".to_string(),
            session_id: "session-local".to_string(),
            content: None,
            tool_name: Some("StructuredOutput".to_string()),
            tool_call_id: Some("call_abc".to_string()),
            tool_input: Some("{\"type\":\"form_request\"}".to_string()),
            tool_result: None,
            error: None,
            input_tokens: None,
            output_tokens: None,
            model: None,
            external_session_id: Some("session-123".to_string()),
            raw_input_tokens: Some(17525),
            raw_output_tokens: Some(0),
            cache_read_input_tokens: Some(0),
            cache_creation_input_tokens: Some(0),
        };

        let hydrated = hydrate_tool_use_usage_from_transcript(event, Some("/Users/test/demo"));

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        let _ = fs::remove_dir_all(&temp_root);

        assert_eq!(hydrated.raw_input_tokens, Some(73887));
        assert_eq!(hydrated.raw_output_tokens, Some(916));
        assert_eq!(hydrated.cache_read_input_tokens, Some(50944));
        assert_eq!(hydrated.cache_creation_input_tokens, Some(0));
        assert_eq!(hydrated.model.as_deref(), Some("glm-5.1"));
    }
}
