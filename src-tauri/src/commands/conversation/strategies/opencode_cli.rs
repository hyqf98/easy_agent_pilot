use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;

use super::abnormal_completion::{
    classify_cli_completion, is_shared_benign_stderr_warning, CliTextFragment, CliTextSource,
};
use super::cli_common::{
    build_cli_failure_report, build_content_event, build_error_event, build_execution_summary,
    build_system_event, build_timeout_error_message, describe_timeout_config, detect_cli_timeout,
    emit_cli_event, extract_file_paths, extract_image_paths, preview_text, render_cli_message,
    shell_escape, timeout_config_for_execution_mode, CliExecutionMonitor,
    NonImageAttachmentPromptMode,
};
use crate::commands::cli_support::{build_cli_launch_error_message, build_tokio_cli_command};
use crate::commands::conversation::abort::{
    clear_abort_flag, register_session_pid, set_abort_flag, should_abort, unregister_session_pid,
};
use crate::commands::conversation::strategy::{AgentExecutionStrategy, AgentRuntimeKind};
use crate::commands::conversation::types::{CliStreamEvent, ExecutionRequest, McpServerConfig};
use crate::commands::mcp_shared::parse_args_string;

pub struct OpenCodeCliStrategy;

macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            crate::logging::write_log("INFO", "opencode-cli", &message);
        }
    };
}

macro_rules! log_error {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            crate::logging::write_log("ERROR", "opencode-cli", &message);
            eprintln!("[ERROR][opencode-cli] {}", message)
        }
    };
}

macro_rules! log_debug {
    ($($arg:tt)*) => {
        let _ = format!($($arg)*);
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
            | "reasoning"
            | "reasoning_start"
    )
}

fn should_treat_process_failure_as_success(
    stdout_outcome: &StdoutReadOutcome,
    stderr_outcome: &StderrReadOutcome,
) -> bool {
    (stdout_outcome.emitted_content || stdout_outcome.emitted_non_error_event)
        && !stdout_outcome.emitted_error
        && !stderr_outcome.emitted_error
}

fn should_ignore_stderr_line(line: &str) -> bool {
    is_shared_benign_stderr_warning(line)
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

fn build_opencode_mcp_config_env(servers: &[McpServerConfig]) -> String {
    let mut mcp_map = serde_json::Map::new();

    for server in servers {
        let server_name = &server.name;

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

                mcp_map.insert(
                    server_name.clone(),
                    serde_json::json!({
                        "type": "local",
                        "command": if args_list.is_empty() {
                            vec![server.command.clone().unwrap_or_default()]
                        } else {
                            let mut cmd = vec![server.command.clone().unwrap_or_default()];
                            cmd.extend(args_list);
                            cmd
                        },
                        "enabled": true,
                        "environment": env_map
                    }),
                );
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

                mcp_map.insert(
                    server_name.clone(),
                    serde_json::json!({
                        "type": "remote",
                        "url": server.url.clone().unwrap_or_default(),
                        "enabled": true,
                        "headers": headers_map
                    }),
                );
            }
            _ => {}
        }
    }

    let full_config = serde_json::json!({
        "mcp": mcp_map
    });

    serde_json::to_string(&full_config).unwrap_or_else(|_| "{}".to_string())
}

fn extract_external_session_id(json: &serde_json::Value) -> Option<String> {
    [
        json.get("session_id"),
        json.get("sessionID"),
        json.pointer("/session/id"),
        json.pointer("/payload/id"),
        json.pointer("/properties/id"),
        json.pointer("/properties/sessionID"),
        json.pointer("/result/session_id"),
        json.pointer("/part/sessionID"),
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

#[async_trait]
impl AgentExecutionStrategy for OpenCodeCliStrategy {
    fn kind(&self) -> AgentRuntimeKind {
        AgentRuntimeKind::OpenCodeCli
    }

    async fn execute(&self, app: AppHandle, request: ExecutionRequest) -> Result<()> {
        let session_id = request.session_id.clone();
        let event_name = self.kind().event_name(&session_id);

        set_abort_flag(&session_id, false).await;

        let cli_path = request
            .cli_path
            .clone()
            .unwrap_or_else(|| "opencode".to_string());
        let model_id = request.model_id.clone();
        let working_directory = request.working_directory.clone();
        let mcp_servers = request.mcp_servers.clone();
        let messages = request.messages.clone();
        let extra_cli_args = request.extra_cli_args.clone();
        let plan_id = request.plan_id.clone();
        let resume_session_id = request
            .resume_session_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        let input_text = messages
            .iter()
            .map(|message| {
                render_cli_message(message, false, NonImageAttachmentPromptMode::MissingOnly)
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let file_paths = extract_file_paths(&messages);
        let image_paths = extract_image_paths(&messages).map_err(anyhow::Error::msg)?;

        let mut args = vec!["run".to_string()];
        args.push("--format".to_string());
        args.push("json".to_string());
        args.push("--thinking".to_string());

        if let Some(model_id) = &model_id {
            let trimmed = model_id.trim();
            if !trimmed.is_empty() && trimmed != "default" {
                args.push("--model".to_string());
                args.push(trimmed.to_string());
            }
        }

        if let Some(resume_id) = &resume_session_id {
            args.push("--session".to_string());
            args.push(resume_id.clone());
        }

        if let Some(custom_args) = &extra_cli_args {
            if !custom_args.is_empty() {
                args.extend(custom_args.iter().cloned());
            }
        }

        if !file_paths.is_empty() {
            for path in &file_paths {
                args.push("-f".to_string());
                args.push(path.clone());
            }
            log_info!("追加 OpenCode CLI 文件参数: -f x{}", file_paths.len());
        }

        if !image_paths.is_empty() {
            for path in &image_paths {
                args.push("-f".to_string());
                args.push(path.clone());
            }
            log_info!("追加 OpenCode CLI 图片参数: -f x{}", image_paths.len());
        }

        let resolved_working_dir: Option<String> = working_directory
            .as_ref()
            .map(|w| w.trim().to_string())
            .filter(|w| !w.is_empty());

        let command_args = args.clone();
        let mut cmd = build_tokio_cli_command(&cli_path, &command_args);

        if let Some(ref work_dir) = resolved_working_dir {
            cmd.current_dir(work_dir);
            log_info!("设置工作目录: {}", work_dir);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(servers) = &mcp_servers {
            if !servers.is_empty() {
                let mcp_config_env = build_opencode_mcp_config_env(servers);
                log_info!(
                    "OpenCode MCP 配置 (通过 OPENCODE_CONFIG_CONTENT): {}",
                    mcp_config_env
                );
                cmd.env("OPENCODE_CONFIG_CONTENT", mcp_config_env);
            }
        }

        let full_command = build_full_opencode_command(&cli_path, &args, Some(input_text.len()));
        log_info!("OpenCode CLI command: {}", full_command);

        let execution_started_at = Instant::now();
        let monitor = CliExecutionMonitor::new();
        let timeout_config = timeout_config_for_execution_mode(request.execution_mode.as_deref());
        log_info!(
            "OpenCode CLI timeout config: mode={}, {}",
            request.execution_mode.as_deref().unwrap_or("chat"),
            describe_timeout_config(timeout_config)
        );

        let mut child = cmd.spawn().map_err(|error| {
            anyhow::anyhow!(build_cli_launch_error_message(
                "OpenCode",
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

        if let Some(pid) = child.id() {
            register_session_pid(&session_id, pid).await;
        }

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
        let requested_model_clone = model_id.clone();
        let stdout_monitor = monitor.clone();

        let stdout_handle = tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut outcome = StdoutReadOutcome::none();

            while let Ok(Some(line)) = lines.next_line().await {
                if should_abort(&session_id_clone).await {
                    break;
                }

                match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(json_value) => {
                        if let Some(mut event) = parse_opencode_json_output(
                            &session_id_clone,
                            &json_value,
                            requested_model_clone.as_deref(),
                        ) {
                            apply_requested_model_fallback(
                                &mut event,
                                requested_model_clone.as_deref(),
                            );
                            let mut events = vec![event];
                            events.extend(build_opencode_followup_events(
                                &session_id_clone,
                                &json_value,
                                events.first().expect("primary event exists"),
                            ));

                            for event in events {
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
                    }
                    Err(e) => {
                        outcome.parse_error_count += 1;
                        if outcome.preview.is_none() {
                            outcome.preview = Some(line.clone());
                        }
                        log_debug!("[stdout] JSON 解析失败: {:?}", e);
                    }
                }
            }

            outcome
        });

        let session_id_clone = session_id.clone();
        let app_clone = app.clone();
        let event_name_clone = event_name.clone();
        let plan_id_clone = plan_id.clone();
        let stderr_monitor = monitor.clone();

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
                    build_timeout_error_message("OpenCode", timeout_kind, &snapshot, now);
                log_error!("{}", error_message);
                let error_event = build_error_event(&session_id, error_message.clone());
                emit_cli_event(&app, &event_name, plan_id.as_ref(), &error_event);
                timeout_error_message = Some(error_message);

                if let Err(error) = child.kill().await {
                    log_error!("终止超时的 OpenCode CLI 进程失败: {}", error);
                }

                let exit_status = child.wait().await?;
                monitor.note_process_exit(exit_status.code());
                break exit_status;
            }

            sleep(Duration::from_millis(250)).await;
        };
        let elapsed = execution_started_at.elapsed();
        log_info!(
            "OpenCode CLI 执行完成，退出码: {:?}, 耗时: {:.2}s",
            status.code(),
            elapsed.as_secs_f64()
        );

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
        log_info!("OpenCode CLI 执行摘要: {}", summary);

        unregister_session_pid(&session_id).await;

        let should_treat_failure_as_success =
            should_treat_process_failure_as_success(&stdout_outcome, &stderr_outcome);
        let mut completion_fragments = stdout_outcome.fragments.clone();
        completion_fragments.extend(stderr_outcome.fragments.clone());
        let detected_failure = classify_cli_completion(
            "OpenCode",
            &completion_fragments,
            stdout_outcome.emitted_error || stderr_outcome.emitted_error,
        );
        let should_complete_as_success = should_treat_failure_as_success
            || (detected_failure.is_none() && stdout_outcome.emitted_content);
        let execution_succeeded = status.success() || should_complete_as_success;

        if timeout_error_message.is_none() && detected_failure.is_none() && execution_succeeded {
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
        }

        clear_abort_flag(&session_id).await;

        if let Some(error_message) = timeout_error_message {
            log_error!(
                "{}",
                build_cli_failure_report(
                    "OpenCode",
                    &session_id,
                    &full_command,
                    working_directory.as_deref(),
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
                    "OpenCode",
                    &session_id,
                    &full_command,
                    working_directory.as_deref(),
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
                    "忽略 OpenCode CLI 非零/空退出码：已收到有效输出，exit_code={:?}, {}",
                    status.code(),
                    summary
                );
                return Ok(());
            }
            let failure_message = format!(
                "OpenCode CLI 执行失败，退出码: {:?}, {}",
                status.code(),
                summary
            );
            log_error!(
                "{}",
                build_cli_failure_report(
                    "OpenCode",
                    &session_id,
                    &full_command,
                    working_directory.as_deref(),
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

fn build_full_opencode_command(
    cli_path: &str,
    args: &[String],
    stdin_payload_len: Option<usize>,
) -> String {
    let mut cmd_parts = Vec::new();
    cmd_parts.push(shell_escape(cli_path));
    cmd_parts.extend(args.iter().map(|arg| shell_escape(arg)));
    if let Some(stdin_len) = stdin_payload_len {
        cmd_parts.push(format!("<stdin, {} chars>", stdin_len));
    }
    cmd_parts.join(" ")
}

/// 解析 OpenCode pubsub 消息更新事件中的 payload，提取 Parts 数组中的 tool_call、reasoning、text 等。
fn stringify_json_value(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?;

    match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        _ => serde_json::to_string(value).ok(),
    }
}

fn extract_textish_value(value: Option<&serde_json::Value>) -> Option<String> {
    fn collect_textish_parts(value: &serde_json::Value, parts: &mut Vec<String>) {
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
                    collect_textish_parts(item, parts);
                }
            }
            serde_json::Value::Object(map) => {
                for key in [
                    "thinking", "summary", "text", "content", "message", "value", "title",
                ] {
                    if let Some(nested) = map.get(key) {
                        collect_textish_parts(nested, parts);
                    }
                }
            }
            serde_json::Value::Bool(flag) => parts.push(flag.to_string()),
            serde_json::Value::Number(number) => parts.push(number.to_string()),
        }
    }

    let value = value?;
    let mut parts = Vec::new();
    collect_textish_parts(value, &mut parts);
    if parts.is_empty() {
        return None;
    }

    Some(parts.join("\n"))
}

fn extract_model_fragment(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?;

    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_json::Value::Object(map) => [
            map.get("id"),
            map.get("name"),
            map.get("model"),
            map.get("modelID"),
            map.get("modelId"),
        ]
        .into_iter()
        .flatten()
        .find_map(|nested| nested.as_str())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned),
        _ => None,
    }
}

fn extract_provider_fragment(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?;

    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_json::Value::Object(map) => [map.get("id"), map.get("name"), map.get("provider")]
            .into_iter()
            .flatten()
            .find_map(|nested| nested.as_str())
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned),
        _ => None,
    }
}

fn build_thinking_cli_event(session_id: &str, content: String) -> CliStreamEvent {
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

fn build_tool_use_cli_event(
    session_id: &str,
    tool_name: String,
    tool_call_id: Option<String>,
    tool_input: Option<String>,
) -> CliStreamEvent {
    CliStreamEvent {
        event_type: "tool_use".to_string(),
        session_id: session_id.to_string(),
        content: None,
        tool_name: Some(tool_name),
        tool_call_id,
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
    }
}

fn build_tool_result_cli_event(
    session_id: &str,
    tool_call_id: Option<String>,
    tool_result: Option<String>,
) -> CliStreamEvent {
    CliStreamEvent {
        event_type: "tool_result".to_string(),
        session_id: session_id.to_string(),
        content: None,
        tool_name: None,
        tool_call_id,
        tool_input: None,
        tool_result,
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

fn extract_opencode_part_tool_name(part: &serde_json::Value) -> Option<String> {
    part.get("tool")
        .or_else(|| part.get("name"))
        .or_else(|| part.pointer("/data/name"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn extract_opencode_part_tool_call_id(part: &serde_json::Value) -> Option<String> {
    part.get("callID")
        .or_else(|| part.get("callId"))
        .or_else(|| part.get("id"))
        .or_else(|| part.pointer("/data/id"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn extract_opencode_part_tool_input(part: &serde_json::Value) -> Option<String> {
    stringify_json_value(
        part.pointer("/state/input")
            .or_else(|| part.get("input"))
            .or_else(|| part.pointer("/data/input"))
            .or_else(|| part.pointer("/properties/input")),
    )
}

fn extract_opencode_part_tool_output(part: &serde_json::Value) -> Option<String> {
    stringify_json_value(
        part.pointer("/state/output")
            .or_else(|| part.get("output"))
            .or_else(|| part.get("content"))
            .or_else(|| part.pointer("/data/output"))
            .or_else(|| part.pointer("/data/content")),
    )
}

fn apply_requested_model_fallback(event: &mut CliStreamEvent, requested_model: Option<&str>) {
    if event.model.is_some() || event.event_type == "error" {
        return;
    }

    let Some(model) = requested_model
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };

    event.model = Some(model.to_string());
}

fn extract_opencode_model_name(
    json: &serde_json::Value,
    requested_model: Option<&str>,
) -> Option<String> {
    let model = [
        json.get("model"),
        json.get("modelName"),
        json.pointer("/part/model"),
        json.pointer("/part/modelName"),
        json.pointer("/part/data/model"),
        json.pointer("/part/data/modelName"),
        json.pointer("/message/model"),
        json.pointer("/message/modelName"),
        json.pointer("/payload/model"),
        json.pointer("/payload/modelName"),
        json.pointer("/payload/message/model"),
        json.pointer("/payload/message/modelName"),
        json.pointer("/payload/part/model"),
        json.pointer("/payload/part/modelName"),
        json.pointer("/data/model"),
        json.pointer("/data/modelName"),
        json.pointer("/metadata/model"),
        json.pointer("/properties/model"),
        json.pointer("/properties/modelName"),
    ]
    .into_iter()
    .find_map(extract_model_fragment);

    let provider = [
        json.get("provider"),
        json.get("providerID"),
        json.get("providerName"),
        json.pointer("/message/provider"),
        json.pointer("/message/providerID"),
        json.pointer("/message/providerName"),
        json.pointer("/payload/provider"),
        json.pointer("/payload/providerID"),
        json.pointer("/payload/providerName"),
        json.pointer("/payload/message/provider"),
        json.pointer("/payload/message/providerID"),
        json.pointer("/payload/message/providerName"),
        json.pointer("/data/provider"),
        json.pointer("/data/providerID"),
        json.pointer("/data/providerName"),
        json.pointer("/properties/provider"),
        json.pointer("/properties/providerID"),
        json.pointer("/properties/providerName"),
    ]
    .into_iter()
    .find_map(extract_provider_fragment);

    model
        .map(|value| {
            if value.contains('/') || provider.as_deref().is_none() {
                value
            } else {
                format!("{}/{}", provider.as_deref().unwrap_or_default(), value)
            }
        })
        .or_else(|| {
            requested_model
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string())
        })
}

fn build_opencode_followup_events(
    session_id: &str,
    json: &serde_json::Value,
    primary_event: &CliStreamEvent,
) -> Vec<CliStreamEvent> {
    if primary_event.event_type != "tool_use" {
        return Vec::new();
    }

    let part = json
        .get("part")
        .or_else(|| json.pointer("/payload/part"))
        .unwrap_or(json);
    let tool_status = part
        .pointer("/state/status")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if tool_status != "completed" {
        return Vec::new();
    }

    let tool_result = extract_opencode_part_tool_output(part);
    if tool_result.is_none() {
        return Vec::new();
    }

    vec![build_tool_result_cli_event(
        session_id,
        primary_event.tool_call_id.clone(),
        tool_result,
    )]
}

fn parse_opencode_part_event(session_id: &str, part: &serde_json::Value) -> Option<CliStreamEvent> {
    let part_type = part
        .get("type")
        .or_else(|| part.pointer("/part/type"))
        .or_else(|| part.pointer("/data/type"))
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let data = part.get("data").unwrap_or(part);

    match part_type {
        "text" | "output_text" | "input_text" | "message" => extract_textish_value(
            data.get("text")
                .or_else(|| part.get("text"))
                .or_else(|| data.get("content"))
                .or_else(|| part.get("content"))
                .or_else(|| data.get("message"))
                .or_else(|| part.get("message")),
        )
        .map(|text| build_content_event(session_id, text)),
        "reasoning" | "thinking" | "reasoning_content" => extract_textish_value(
            data.get("thinking")
                .or_else(|| part.get("thinking"))
                .or_else(|| data.get("summary"))
                .or_else(|| part.get("summary"))
                .or_else(|| data.get("text"))
                .or_else(|| part.get("text"))
                .or_else(|| data.get("content"))
                .or_else(|| part.get("content"))
                .or_else(|| data.get("message"))
                .or_else(|| part.get("message")),
        )
        .map(|text| build_thinking_cli_event(session_id, text)),
        "tool_call" | "tool_use" | "tool" | "function_call" => {
            let tool_name = data
                .get("name")
                .or_else(|| part.get("name"))
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
                .or_else(|| extract_opencode_part_tool_name(part))?;
            let tool_call_id = data
                .get("id")
                .or_else(|| data.get("call_id"))
                .or_else(|| part.get("id"))
                .or_else(|| part.get("call_id"))
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
                .or_else(|| extract_opencode_part_tool_call_id(part));
            let tool_input = stringify_json_value(
                data.get("input")
                    .or_else(|| data.get("arguments"))
                    .or_else(|| part.get("input"))
                    .or_else(|| part.get("arguments")),
            )
            .or_else(|| extract_opencode_part_tool_input(part));

            Some(build_tool_use_cli_event(
                session_id,
                tool_name,
                tool_call_id,
                tool_input,
            ))
        }
        "tool_result" | "function_call_output" => {
            let tool_call_id = data
                .get("tool_call_id")
                .or_else(|| data.get("tool_use_id"))
                .or_else(|| data.get("call_id"))
                .or_else(|| part.get("tool_call_id"))
                .or_else(|| part.get("tool_use_id"))
                .or_else(|| part.get("call_id"))
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
                .or_else(|| extract_opencode_part_tool_call_id(part));
            let tool_result = stringify_json_value(
                data.get("content")
                    .or_else(|| data.get("output"))
                    .or_else(|| data.get("result"))
                    .or_else(|| part.get("content"))
                    .or_else(|| part.get("output"))
                    .or_else(|| part.get("result")),
            )
            .or_else(|| extract_opencode_part_tool_output(part));

            Some(build_tool_result_cli_event(
                session_id,
                tool_call_id,
                tool_result,
            ))
        }
        _ => None,
    }
}

fn parse_opencode_parts_event(
    session_id: &str,
    parts: &[serde_json::Value],
) -> Option<CliStreamEvent> {
    let mut content_text = String::new();
    let mut thinking_text = String::new();

    for part in parts {
        let part_type = part
            .get("type")
            .or_else(|| part.pointer("/part/type"))
            .or_else(|| part.pointer("/data/type"))
            .and_then(|value| value.as_str())
            .unwrap_or("");

        match part_type {
            "text" | "output_text" | "input_text" | "message" => {
                if let Some(text) = extract_textish_value(
                    part.get("data")
                        .and_then(|value| value.get("text"))
                        .or_else(|| part.get("text"))
                        .or_else(|| part.pointer("/data/content"))
                        .or_else(|| part.get("content"))
                        .or_else(|| part.pointer("/data/message"))
                        .or_else(|| part.get("message")),
                ) {
                    content_text.push_str(&text);
                }
            }
            "reasoning" | "thinking" | "reasoning_content" => {
                if let Some(text) = extract_textish_value(
                    part.get("data")
                        .and_then(|value| value.get("thinking"))
                        .or_else(|| part.get("thinking"))
                        .or_else(|| part.pointer("/data/summary"))
                        .or_else(|| part.get("summary"))
                        .or_else(|| part.pointer("/data/text"))
                        .or_else(|| part.get("text"))
                        .or_else(|| part.pointer("/data/content"))
                        .or_else(|| part.get("content"))
                        .or_else(|| part.pointer("/data/message"))
                        .or_else(|| part.get("message")),
                ) {
                    thinking_text.push_str(&text);
                }
            }
            "tool_call"
            | "tool_use"
            | "tool"
            | "function_call"
            | "tool_result"
            | "function_call_output" => {
                if let Some(event) = parse_opencode_part_event(session_id, part) {
                    return Some(event);
                }
            }
            _ => {}
        }
    }

    if !thinking_text.is_empty() {
        return Some(build_thinking_cli_event(session_id, thinking_text));
    }

    if !content_text.is_empty() {
        return Some(build_content_event(session_id, content_text));
    }

    None
}

fn parse_opencode_message_payload(
    session_id: &str,
    json: &serde_json::Value,
) -> Option<CliStreamEvent> {
    let payload = json.get("payload").cloned().unwrap_or_else(|| json.clone());

    let role = payload
        .get("Role")
        .or_else(|| payload.get("role"))
        .and_then(|r| r.as_str())
        .unwrap_or("");

    if role != "assistant" {
        return None;
    }

    let parts = payload
        .get("Parts")
        .or_else(|| payload.get("parts"))
        .and_then(|p| p.as_array());

    if let Some(parts_array) = parts {
        return parse_opencode_parts_event(session_id, parts_array);
    }

    None
}

fn parse_opencode_response_item(
    session_id: &str,
    json: &serde_json::Value,
) -> Option<CliStreamEvent> {
    let payload = json.get("payload").unwrap_or(json);
    let payload_type = payload
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    match payload_type {
        "message" => {
            if let Some(content) = payload.get("content").and_then(|value| value.as_array()) {
                return parse_opencode_parts_event(session_id, content);
            }

            extract_textish_value(
                payload
                    .get("content")
                    .or_else(|| payload.get("message"))
                    .or_else(|| payload.get("text")),
            )
            .map(|text| build_content_event(session_id, text))
        }
        "function_call" => parse_opencode_part_event(
            session_id,
            &serde_json::json!({
                "type": "function_call",
                "name": payload.get("name").cloned().unwrap_or(serde_json::Value::Null),
                "call_id": payload.get("call_id").cloned().unwrap_or(serde_json::Value::Null),
                "arguments": payload.get("arguments").cloned().unwrap_or(serde_json::Value::Null)
            }),
        ),
        "function_call_output" => parse_opencode_part_event(
            session_id,
            &serde_json::json!({
                "type": "function_call_output",
                "call_id": payload.get("call_id").cloned().unwrap_or(serde_json::Value::Null),
                "output": payload.get("output").cloned().unwrap_or(serde_json::Value::Null)
            }),
        ),
        "reasoning" => extract_textish_value(
            payload
                .get("summary")
                .or_else(|| payload.get("text"))
                .or_else(|| payload.get("content"))
                .or_else(|| payload.get("message")),
        )
        .map(|text| build_thinking_cli_event(session_id, text)),
        _ => None,
    }
}

fn parse_opencode_event_msg(session_id: &str, json: &serde_json::Value) -> Option<CliStreamEvent> {
    let payload = json.get("payload").unwrap_or(json);
    let payload_type = payload
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    match payload_type {
        "agent_message" => extract_textish_value(
            payload
                .get("message")
                .or_else(|| payload.get("content"))
                .or_else(|| payload.get("text")),
        )
        .map(|text| build_content_event(session_id, text)),
        "agent_reasoning" => extract_textish_value(
            payload
                .get("text")
                .or_else(|| payload.get("content"))
                .or_else(|| payload.get("summary"))
                .or_else(|| payload.get("message")),
        )
        .map(|text| build_thinking_cli_event(session_id, text)),
        "token_count" => {
            let counts = extract_usage_counts(Some(payload));
            Some(CliStreamEvent {
                event_type: "usage".to_string(),
                session_id: session_id.to_string(),
                content: None,
                tool_name: None,
                tool_call_id: None,
                tool_input: None,
                tool_result: None,
                error: None,
                input_tokens: counts.input_tokens,
                output_tokens: counts.output_tokens,
                model: None,
                external_session_id: None,
                raw_input_tokens: counts.raw_input_tokens,
                raw_output_tokens: counts.raw_output_tokens,
                cache_read_input_tokens: counts.cache_read_input_tokens,
                cache_creation_input_tokens: counts.cache_creation_input_tokens,
            })
        }
        "task_started" => stringify_json_value(
            payload
                .get("message")
                .or_else(|| payload.get("content"))
                .or_else(|| payload.get("text")),
        )
        .map(|text| build_system_event(session_id, text))
        .or_else(|| Some(build_system_event(session_id, "任务已开始".to_string()))),
        "compaction" => {
            let auto = payload
                .get("auto")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let trigger = if auto { "auto" } else { "manual" };
            Some(build_system_event(
                session_id,
                format!("### CLI Context Compaction\nTrigger: {}", trigger),
            ))
        }
        _ => None,
    }
}

/// 解析 `opencode run --format json` 的每行 JSON 输出。
///
/// OpenCode 的 JSON 事件格式与 Claude CLI 类似但字段名不同。
/// 初期实现基于实际运行样本迭代，支持基本事件类型。
fn parse_opencode_json_output(
    session_id: &str,
    json: &serde_json::Value,
    requested_model: Option<&str>,
) -> Option<CliStreamEvent> {
    let event_type = json
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");

    let event = match event_type {
        // 文本内容增量（流式）/ OpenCode text 事件
        "content_block_delta" | "text_delta" | "delta" | "text" => {
            let text = extract_textish_value(
                json.get("text")
                    .or_else(|| json.pointer("/part/text"))
                    .or_else(|| json.pointer("/delta/text"))
                    .or_else(|| json.pointer("/properties/text"))
                    .or_else(|| json.pointer("/data/text"))
                    .or_else(|| json.get("content"))
                    .or_else(|| json.pointer("/delta/content")),
            )?;
            Some(build_content_event(session_id, text.to_string()))
        }
        // thinking / reasoning 内容
        "thinking_delta" | "thinking" | "reasoning" | "reasoning_delta" => {
            let text = extract_textish_value(
                json.get("thinking")
                    .or_else(|| json.pointer("/part/thinking"))
                    .or_else(|| json.pointer("/delta/thinking"))
                    .or_else(|| json.pointer("/data/thinking"))
                    .or_else(|| json.pointer("/part/summary"))
                    .or_else(|| json.pointer("/part/text"))
                    .or_else(|| json.pointer("/data/summary"))
                    .or_else(|| json.pointer("/data/text"))
                    .or_else(|| json.get("content"))
                    .or_else(|| json.get("text"))
                    .or_else(|| json.pointer("/payload/content"))
                    .or_else(|| json.pointer("/payload/text")),
            )?;
            Some(CliStreamEvent {
                event_type: "thinking".to_string(),
                session_id: session_id.to_string(),
                content: Some(text.to_string()),
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
            })
        }
        // thinking_start 显式事件（OpenCode 可能单独发送）
        "thinking_start" | "reasoning_start" => Some(CliStreamEvent {
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
        }),
        // 工具调用开始
        "tool_use" | "content_block_start" | "tool_call" => {
            let part = json
                .get("part")
                .or_else(|| json.pointer("/payload/part"))
                .unwrap_or(json);
            let tool_name = json
                .get("name")
                .or_else(|| json.pointer("/content_block/name"))
                .or_else(|| json.pointer("/properties/name"))
                .or_else(|| json.pointer("/data/name"))
                .and_then(|n| n.as_str())
                .map(|value| value.to_string())
                .or_else(|| extract_opencode_part_tool_name(part))?;
            let tool_id = json
                .get("id")
                .or_else(|| json.pointer("/content_block/id"))
                .or_else(|| json.pointer("/data/id"))
                .and_then(|i| i.as_str())
                .map(|i| i.to_string())
                .or_else(|| extract_opencode_part_tool_call_id(part));
            let tool_input = json
                .get("input")
                .or_else(|| json.pointer("/data/input"))
                .and_then(|i| serde_json::to_string(i).ok())
                .or_else(|| extract_opencode_part_tool_input(part));
            Some(CliStreamEvent {
                event_type: "tool_use".to_string(),
                session_id: session_id.to_string(),
                content: None,
                tool_name: Some(tool_name),
                tool_call_id: tool_id,
                tool_input,
                tool_result: None,
                error: None,
                input_tokens: None,
                output_tokens: None,
                model: extract_opencode_model_name(json, requested_model),
                external_session_id: None,
                raw_input_tokens: None,
                raw_output_tokens: None,
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
            })
        }
        // 工具输入增量
        "tool_input_delta" | "input_json_delta" => {
            let partial_json = json
                .get("partial_json")
                .or_else(|| json.pointer("/delta/partial_json"))
                .and_then(|j| j.as_str())
                .unwrap_or("");
            let tool_call_id = json
                .get("id")
                .and_then(|i| i.as_str())
                .map(|i| i.to_string());
            Some(CliStreamEvent {
                event_type: "tool_input_delta".to_string(),
                session_id: session_id.to_string(),
                content: None,
                tool_name: None,
                tool_call_id,
                tool_input: Some(partial_json.to_string()),
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
        // 工具结果
        "tool_result" => {
            let tool_id = json
                .get("tool_use_id")
                .or_else(|| json.get("tool_call_id"))
                .or_else(|| json.pointer("/data/tool_call_id"))
                .and_then(|i| i.as_str())
                .map(|i| i.to_string());
            let result_content = json
                .get("content")
                .or_else(|| json.pointer("/data/content"))
                .and_then(|c| {
                    if let Some(s) = c.as_str() {
                        Some(s.to_string())
                    } else if c.is_object() || c.is_array() {
                        serde_json::to_string(c).ok()
                    } else {
                        None
                    }
                });
            Some(CliStreamEvent {
                event_type: "tool_result".to_string(),
                session_id: session_id.to_string(),
                content: None,
                tool_name: None,
                tool_call_id: tool_id,
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
        // 完整 assistant 消息（非流式）/ OpenCode 消息快照
        "assistant" | "message" => {
            let content: Option<&str> = json
                .get("content")
                .and_then(|c| c.as_str())
                .or_else(|| json.pointer("/message/content").and_then(|c| c.as_str()));
            if let Some(text) = content {
                let model = json
                    .get("model")
                    .and_then(|m| m.as_str())
                    .map(|m| m.to_string());
                Some(CliStreamEvent {
                    event_type: "content".to_string(),
                    session_id: session_id.to_string(),
                    content: Some(text.to_string()),
                    tool_name: None,
                    tool_call_id: None,
                    tool_input: None,
                    tool_result: None,
                    error: None,
                    input_tokens: None,
                    output_tokens: None,
                    model,
                    external_session_id: None,
                    raw_input_tokens: None,
                    raw_output_tokens: None,
                    cache_read_input_tokens: None,
                    cache_creation_input_tokens: None,
                })
            } else {
                None
            }
        }
        // 错误
        "error" => {
            let error_msg = json
                .get("error")
                .and_then(|e| {
                    e.as_str()
                        .map(|value| value.to_string())
                        .or_else(|| {
                            stringify_json_value(e.get("data").and_then(|data| data.get("message")))
                        })
                        .or_else(|| stringify_json_value(e.get("message")))
                        .or_else(|| stringify_json_value(e.get("name")))
                })
                .or_else(|| {
                    json.get("message")
                        .and_then(|m| m.as_str())
                        .map(|value| value.to_string())
                })
                .or_else(|| {
                    json.pointer("/properties/error")
                        .and_then(|e| e.as_str())
                        .map(|value| value.to_string())
                })
                .unwrap_or_else(|| "Unknown error".to_string());
            Some(build_error_event(session_id, error_msg))
        }
        // 用量统计
        "usage" | "message_start" | "message_delta" | "message_stop" | "turn.failed"
        | "session_meta" => {
            let model = extract_opencode_model_name(json, requested_model);

            if model.is_some() {
                Some(CliStreamEvent {
                    event_type: "usage".to_string(),
                    session_id: session_id.to_string(),
                    content: None,
                    tool_name: None,
                    tool_call_id: None,
                    tool_input: None,
                    tool_result: None,
                    error: None,
                    input_tokens: None,
                    output_tokens: None,
                    model,
                    external_session_id: None,
                    raw_input_tokens: None,
                    raw_output_tokens: None,
                    cache_read_input_tokens: None,
                    cache_creation_input_tokens: None,
                })
            } else {
                extract_external_session_id(json).map(|sid| CliStreamEvent {
                    event_type: "usage".to_string(),
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
        // 系统通知 / step 开始
        "system" | "step_start" => {
            let content = json
                .get("content")
                .and_then(|c| c.as_str())
                .or_else(|| json.get("message").and_then(|m| m.as_str()));
            if let Some(text) = content {
                Some(build_system_event(session_id, text.to_string()))
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
        "response_item" => parse_opencode_response_item(session_id, json),
        "event_msg" => parse_opencode_event_msg(session_id, json),
        "message.part.created"
        | "message.part.updated"
        | "message.part.delta"
        | "message.part.completed" => json
            .get("part")
            .or_else(|| json.pointer("/payload/part"))
            .or_else(|| json.pointer("/properties/part"))
            .or_else(|| json.pointer("/data/part"))
            .and_then(|part| parse_opencode_part_event(session_id, part)),
        // OpenCode 长任务期间会频繁推送整条 assistant message 的快照。
        // 前端真实增量由 message.part.* 事件消费；这里如果继续透传 created/updated，
        // 会把整段内容和工具快照重复追加，造成重复回复和工具数量暴涨。
        "message.created" | "message.updated" => None,
        "message.completed" => parse_opencode_message_payload(session_id, json),
        // OpenCode pubsub 消息更新事件 {"type":"updated","payload":{"Role":"assistant","Parts":[...]}}
        "created" | "updated" => None,
        // item 事件（OpenCode 特有）
        "item.started" | "item.completed" | "item.delta" => {
            let item_type = json
                .get("item_type")
                .or_else(|| json.get("type"))
                .or_else(|| json.pointer("/properties/type"))
                .or_else(|| json.pointer("/data/type"))
                .and_then(|t| t.as_str())
                .unwrap_or("");

            match item_type {
                "tool_use" | "tool" | "tool_call" => {
                    let tool_name = json
                        .get("name")
                        .or_else(|| json.pointer("/properties/name"))
                        .or_else(|| json.pointer("/data/name"))
                        .and_then(|n| n.as_str());
                    let tool_input = json
                        .get("input")
                        .or_else(|| json.pointer("/properties/input"))
                        .or_else(|| json.pointer("/data/input"))
                        .and_then(|i| serde_json::to_string(i).ok());
                    if let Some(name) = tool_name {
                        Some(CliStreamEvent {
                            event_type: "tool_use".to_string(),
                            session_id: session_id.to_string(),
                            content: None,
                            tool_name: Some(name.to_string()),
                            tool_call_id: json
                                .get("id")
                                .or_else(|| json.pointer("/data/id"))
                                .and_then(|i| i.as_str())
                                .map(|i| i.to_string()),
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
                    } else {
                        None
                    }
                }
                "tool_result" => {
                    let tool_id = json
                        .get("tool_use_id")
                        .or_else(|| json.get("tool_call_id"))
                        .or_else(|| json.pointer("/data/tool_call_id"))
                        .and_then(|i| i.as_str())
                        .map(|i| i.to_string());
                    let result = json
                        .get("content")
                        .or_else(|| json.pointer("/data/content"))
                        .and_then(|c| {
                            if let Some(s) = c.as_str() {
                                Some(s.to_string())
                            } else if c.is_object() || c.is_array() {
                                serde_json::to_string(c).ok()
                            } else {
                                None
                            }
                        })
                        .or_else(|| {
                            json.get("content")
                                .or_else(|| json.pointer("/data/content"))
                                .and_then(|c| c.as_str())
                                .map(|s| s.to_string())
                        });
                    Some(CliStreamEvent {
                        event_type: "tool_result".to_string(),
                        session_id: session_id.to_string(),
                        content: None,
                        tool_name: None,
                        tool_call_id: tool_id,
                        tool_input: None,
                        tool_result: result,
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
                "reasoning" | "thinking" | "reasoning_content" => {
                    let text = extract_textish_value(
                        json.get("thinking")
                            .or_else(|| json.pointer("/data/thinking"))
                            .or_else(|| json.get("content"))
                            .or_else(|| json.pointer("/data/content"))
                            .or_else(|| json.get("text")),
                    );
                    text.map(|t| CliStreamEvent {
                        event_type: "thinking".to_string(),
                        session_id: session_id.to_string(),
                        content: Some(t),
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
                    })
                }
                "text" | "message" => {
                    let text = json
                        .get("text")
                        .or_else(|| json.get("content"))
                        .or_else(|| json.pointer("/data/text"))
                        .and_then(|t| t.as_str());
                    text.map(|t| build_content_event(session_id, t.to_string()))
                }
                _ => {
                    // 尝试从 delta/data 中提取文本
                    let text = extract_textish_value(
                        json.get("text")
                            .or_else(|| json.pointer("/properties/text"))
                            .or_else(|| json.pointer("/delta/text"))
                            .or_else(|| json.pointer("/data/text")),
                    );
                    if let Some(t) = text {
                        Some(build_content_event(session_id, t))
                    } else if let Some(reasoning) = extract_textish_value(
                        json.get("thinking")
                            .or_else(|| json.pointer("/data/thinking"))
                            .or_else(|| json.get("content"))
                            .or_else(|| json.pointer("/data/content")),
                    ) {
                        Some(CliStreamEvent {
                            event_type: "thinking".to_string(),
                            session_id: session_id.to_string(),
                            content: Some(reasoning),
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
                        })
                    } else {
                        None
                    }
                }
            }
        }
        // turn / session / step 元信息
        "turn.completed" | "result" | "step_finish" => {
            let is_step_finish = event_type == "step_finish";
            let usage = if is_step_finish {
                json.pointer("/part/tokens")
            } else {
                json.get("usage")
            };
            let counts = extract_usage_counts(usage);

            let model = extract_opencode_model_name(json, requested_model);
            let external_session_id = extract_external_session_id(json);

            let has_any_data = counts.input_tokens.is_some()
                || counts.output_tokens.is_some()
                || model.is_some()
                || external_session_id.is_some();

            if has_any_data {
                Some(CliStreamEvent {
                    event_type: "usage".to_string(),
                    session_id: session_id.to_string(),
                    content: None,
                    tool_name: None,
                    tool_call_id: None,
                    tool_input: None,
                    tool_result: None,
                    error: None,
                    input_tokens: counts.input_tokens,
                    output_tokens: counts.output_tokens,
                    model,
                    external_session_id,
                    raw_input_tokens: counts.raw_input_tokens,
                    raw_output_tokens: counts.raw_output_tokens,
                    cache_read_input_tokens: counts.cache_read_input_tokens,
                    cache_creation_input_tokens: counts.cache_creation_input_tokens,
                })
            } else {
                None
            }
        }
        // OpenCode compaction 事件（上下文压缩信号）
        "compaction" => {
            let payload = json
                .get("data")
                .or_else(|| json.get("part"))
                .or_else(|| json.get("payload"))
                .unwrap_or(json);
            let auto = payload
                .get("auto")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let trigger = if auto { "auto" } else { "manual" };
            Some(build_system_event(
                session_id,
                format!("### CLI Context Compaction\nTrigger: {}", trigger),
            ))
        }
        // 未知事件：尝试提取文本内容，否则跳过
        _ => {
            // 处理 OpenCode 最终输出格式 {"response": "..."}
            if let Some(response) = json.get("response").and_then(|r| r.as_str()) {
                if !response.is_empty() {
                    return Some(build_content_event(session_id, response.to_string()));
                }
            }

            // 处理 OpenCode 消息格式 {"role": "assistant", "content": "..."}
            let role = json.get("role").and_then(|r| r.as_str()).unwrap_or("");
            if role == "assistant" {
                if let Some(content) = json.get("content").and_then(|c| c.as_str()) {
                    if !content.is_empty() {
                        return Some(build_content_event(session_id, content.to_string()));
                    }
                }
                // 检查 Parts 数组格式（OpenCode message 格式）
                if let Some(parts) = json.get("parts").and_then(|p| p.as_array()) {
                    let mut content_parts = Vec::new();
                    for part in parts {
                        let part_type = part.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        match part_type {
                            "text" => {
                                if let Some(text) = part
                                    .get("data")
                                    .and_then(|d| d.get("text"))
                                    .or_else(|| part.get("text"))
                                    .and_then(|t| t.as_str())
                                {
                                    content_parts.push(text.to_string());
                                }
                            }
                            "reasoning" => {
                                if let Some(thinking) = part
                                    .get("data")
                                    .and_then(|d| d.get("thinking"))
                                    .or_else(|| part.get("thinking"))
                                    .and_then(|t| t.as_str())
                                {
                                    if !thinking.is_empty() {
                                        return Some(CliStreamEvent {
                                            event_type: "thinking".to_string(),
                                            session_id: session_id.to_string(),
                                            content: Some(thinking.to_string()),
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
                                        });
                                    }
                                }
                            }
                            "tool_call" => {
                                let data =
                                    part.get("data").cloned().unwrap_or_else(|| part.clone());
                                let name = data.get("name").and_then(|n| n.as_str());
                                if let Some(tool_name) = name {
                                    let input = data
                                        .get("input")
                                        .and_then(|i| serde_json::to_string(i).ok());
                                    return Some(CliStreamEvent {
                                        event_type: "tool_use".to_string(),
                                        session_id: session_id.to_string(),
                                        content: None,
                                        tool_name: Some(tool_name.to_string()),
                                        tool_call_id: data
                                            .get("id")
                                            .and_then(|i| i.as_str())
                                            .map(|i| i.to_string()),
                                        tool_input: input,
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
                                    });
                                }
                            }
                            "tool_result" => {
                                let data =
                                    part.get("data").cloned().unwrap_or_else(|| part.clone());
                                let tool_id = data
                                    .get("tool_call_id")
                                    .and_then(|i| i.as_str())
                                    .map(|i| i.to_string());
                                let result = data
                                    .get("content")
                                    .and_then(|c| c.as_str())
                                    .map(|s| s.to_string());
                                return Some(CliStreamEvent {
                                    event_type: "tool_result".to_string(),
                                    session_id: session_id.to_string(),
                                    content: None,
                                    tool_name: None,
                                    tool_call_id: tool_id,
                                    tool_input: None,
                                    tool_result: result,
                                    error: None,
                                    input_tokens: None,
                                    output_tokens: None,
                                    model: None,
                                    external_session_id: None,
                                    raw_input_tokens: None,
                                    raw_output_tokens: None,
                                    cache_read_input_tokens: None,
                                    cache_creation_input_tokens: None,
                                });
                            }
                            _ => {}
                        }
                    }
                    if !content_parts.is_empty() {
                        return Some(build_content_event(session_id, content_parts.join("\n")));
                    }
                }
            }

            let text = json.get("text").and_then(|t| t.as_str());
            text.map(|t| build_content_event(session_id, t.to_string()))
        }
    };

    event.map(|event| attach_external_session_id(event, json))
}

struct OpenCodeUsageCounts {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    raw_input_tokens: Option<u32>,
    raw_output_tokens: Option<u32>,
    cache_read_input_tokens: Option<u32>,
    cache_creation_input_tokens: Option<u32>,
}

impl Default for OpenCodeUsageCounts {
    fn default() -> Self {
        Self {
            input_tokens: None,
            output_tokens: None,
            raw_input_tokens: None,
            raw_output_tokens: None,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
        }
    }
}

fn extract_usage_counts(usage: Option<&serde_json::Value>) -> OpenCodeUsageCounts {
    let raw_input_tokens = usage
        .and_then(|u| {
            u.get("input_tokens")
                .or_else(|| u.get("inputTokens"))
                .or_else(|| u.get("input"))
        })
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);
    let raw_output_tokens = usage
        .and_then(|u| {
            u.get("output_tokens")
                .or_else(|| u.get("outputTokens"))
                .or_else(|| u.get("output"))
        })
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);

    let cache_read = usage
        .and_then(|u| {
            u.get("cache_read_input_tokens")
                .or_else(|| u.get("cacheReadInputTokens"))
                .or_else(|| u.get("cached_input_tokens"))
                .or_else(|| u.get("cachedInputTokens"))
                .or_else(|| u.pointer("/cache/read"))
                .or_else(|| u.pointer("/cacheRead"))
        })
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);
    let cache_creation = usage
        .and_then(|u| {
            u.get("cache_creation_input_tokens")
                .or_else(|| u.get("cacheCreationInputTokens"))
                .or_else(|| u.pointer("/cache/write"))
                .or_else(|| u.pointer("/cacheWrite"))
        })
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);

    let has_non_zero_usage =
        raw_input_tokens.unwrap_or(0) > 0 || raw_output_tokens.unwrap_or(0) > 0;
    if !has_non_zero_usage {
        return OpenCodeUsageCounts::default();
    }

    OpenCodeUsageCounts {
        input_tokens: raw_input_tokens,
        output_tokens: raw_output_tokens,
        raw_input_tokens,
        raw_output_tokens,
        cache_read_input_tokens: cache_read,
        cache_creation_input_tokens: cache_creation,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_opencode_json_output;

    #[test]
    fn parses_step_finish_usage_from_part_tokens() {
        let json = serde_json::json!({
            "type": "step_finish",
            "timestamp": 1777269151308_u64,
            "sessionID": "ses_232812fe3ffeEiKsp35xj8vNn2",
            "part": {
                "id": "prt_dcd7f1613001GHuOCAqD1gdr3V",
                "reason": "stop",
                "snapshot": "75222364c0977dfe3470d763b51eec056614e5df",
                "messageID": "msg_dcd7ed115001Iz687Pb0fRjShs",
                "sessionID": "ses_232812fe3ffeEiKsp35xj8vNn2",
                "type": "step-finish",
                "tokens": {
                    "total": 50913,
                    "input": 33054,
                    "output": 3,
                    "reasoning": 0,
                    "cache": {
                        "write": 0,
                        "read": 17856
                    }
                },
                "cost": 0
            }
        });

        let event = parse_opencode_json_output("session-1", &json, Some("modelhub/glm-5.1"))
            .expect("expected usage event");

        assert_eq!(event.event_type, "usage");
        assert_eq!(event.input_tokens, Some(33054));
        assert_eq!(event.output_tokens, Some(3));
        assert_eq!(event.raw_input_tokens, Some(33054));
        assert_eq!(event.raw_output_tokens, Some(3));
        assert_eq!(event.cache_read_input_tokens, Some(17856));
        assert_eq!(
            event.external_session_id.as_deref(),
            Some("ses_232812fe3ffeEiKsp35xj8vNn2")
        );
    }
}
