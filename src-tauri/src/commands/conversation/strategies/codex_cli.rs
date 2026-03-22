use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;
use uuid::Uuid;

use super::cli_common::{
    build_content_event, build_error_event, build_execution_summary, build_system_event,
    build_timeout_error_message, detect_cli_timeout, emit_cli_event, extract_error_from_json_blob,
    extract_result_content_from_json_blob, extract_runtime_system_notice,
    extract_structured_output_from_json_blob, parse_json_blob_with_fallback, preview_text,
    shell_escape, timeout_config_for_execution_mode, CliExecutionMonitor,
};
use crate::commands::cli_support::build_tokio_cli_command;
use crate::commands::conversation::abort::{
    clear_abort_flag, register_session_pid, set_abort_flag, should_abort, unregister_session_pid,
};
use crate::commands::conversation::strategy::{AgentExecutionStrategy, AgentRuntimeKind};
use crate::commands::conversation::types::{CliStreamEvent, ExecutionRequest, MessageInput};

/// Codex CLI 策略
pub struct CodexCliStrategy;

// 简单的日志宏
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            crate::logging::write_log("INFO", "codex-cli", &message);
            println!("[INFO][codex-cli] {}", message)
        }
    };
}

macro_rules! log_error {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            crate::logging::write_log("ERROR", "codex-cli", &message);
            eprintln!("[ERROR][codex-cli] {}", message)
        }
    };
}

macro_rules! log_debug {
    ($($arg:tt)*) => {
        // DEBUG 日志已禁用，如需调试请取消注释
        // println!("[DEBUG][codex-cli] {}", format!($($arg)*))
    };
}

fn is_benign_stderr_warning(line: &str) -> bool {
    let normalized = line.to_lowercase();

    // Codex CLI 偶发会在 stderr 输出一条 rmcp transport warning：
    // "worker quit with fatal: Transport channel closed, when UnexpectedContentType(...)"
    // 目前该告警不会阻塞任务拆分结果落地，不应污染 UI 错误态或计划拆分日志。
    normalized.contains("rmcp::transport::worker")
        && normalized.contains("unexpectedcontenttype")
        && normalized.contains("missing-content-type")
}

struct StdoutReadOutcome {
    emitted_content: bool,
    emitted_error: bool,
    emitted_non_error_event: bool,
}

impl StdoutReadOutcome {
    fn none() -> Self {
        Self {
            emitted_content: false,
            emitted_error: false,
            emitted_non_error_event: false,
        }
    }
}

struct StderrReadOutcome {
    emitted_error: bool,
}

impl StderrReadOutcome {
    fn none() -> Self {
        Self {
            emitted_error: false,
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

fn should_ignore_stderr_line(line: &str) -> bool {
    is_benign_stderr_warning(line) || is_rmcp_transport_closed_warning(line)
}

fn is_rmcp_transport_closed_warning(line: &str) -> bool {
    let normalized = line.to_lowercase();
    normalized.contains("rmcp::transport::worker")
        && normalized.contains("transport channel closed")
}

fn should_treat_process_failure_as_success(
    stdout_outcome: &StdoutReadOutcome,
    stderr_outcome: &StderrReadOutcome,
) -> bool {
    (stdout_outcome.emitted_content || stdout_outcome.emitted_non_error_event)
        && !stdout_outcome.emitted_error
        && !stderr_outcome.emitted_error
}

fn render_cli_message(message: &MessageInput) -> String {
    let mut sections = Vec::new();

    if !message.content.trim().is_empty() {
        sections.push(message.content.clone());
    }

    if let Some(attachments) = &message.attachments {
        if !attachments.is_empty() {
            let attachment_list = attachments
                .iter()
                .map(|attachment| format!("- {} ({})", attachment.name, attachment.path))
                .collect::<Vec<_>>()
                .join("\n");
            sections.push(format!("Attached images:\n{}", attachment_list));
        }
    }

    let body = if sections.is_empty() {
        "[Empty message]".to_string()
    } else {
        sections.join("\n\n")
    };

    format!("{}:\n{}", message.role, body)
}

struct TempSchemaFile {
    path: PathBuf,
}

impl TempSchemaFile {
    async fn create(schema: &str) -> Result<Self> {
        let path =
            std::env::temp_dir().join(format!("codex-output-schema-{}.json", Uuid::new_v4()));
        tokio::fs::write(&path, schema).await?;
        Ok(Self { path })
    }

    fn to_path_string(&self) -> String {
        self.path.to_string_lossy().to_string()
    }
}

impl Drop for TempSchemaFile {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                log_error!(
                    "清理临时 schema 文件失败: {} ({})",
                    self.path.display(),
                    error
                );
            }
        }
    }
}

#[async_trait]
impl AgentExecutionStrategy for CodexCliStrategy {
    fn kind(&self) -> AgentRuntimeKind {
        AgentRuntimeKind::CodexCli
    }

    async fn execute(&self, app: AppHandle, request: ExecutionRequest) -> Result<()> {
        let session_id = request.session_id.clone();
        let event_name = self.kind().event_name(&session_id);

        log_info!("开始执行 Codex CLI, session_id: {}", session_id);

        // 重置中断标志
        set_abort_flag(&session_id, false).await;

        let cli_path = request
            .cli_path
            .clone()
            .unwrap_or_else(|| "codex".to_string());
        let model_id = request.model_id.clone();
        let working_directory = request.working_directory.clone();
        let allowed_tools = request.allowed_tools.clone();
        let cli_output_format = request
            .cli_output_format
            .clone()
            .unwrap_or_else(|| "stream-json".to_string());
        let json_schema = request.json_schema.clone();
        let extra_cli_args = request.extra_cli_args.clone();
        let messages = request.messages.clone();
        let mcp_servers = request.mcp_servers.clone();
        let is_stream_json = cli_output_format == "stream-json";
        let enable_web_search = allowed_tools
            .as_ref()
            .map(|tools| tools.iter().any(|tool| tool == "WebSearch"))
            .unwrap_or(false);

        let schema_text = json_schema
            .as_deref()
            .map(str::trim)
            .filter(|schema| !schema.is_empty());
        let plan_id = request.plan_id.clone();
        let use_exec_mode = cli_output_format != "text" || schema_text.is_some();
        let is_json_output = use_exec_mode
            && (cli_output_format == "json"
                || cli_output_format == "stream-json"
                || schema_text.is_some());

        // 构建命令参数
        let mut global_args = Vec::<String>::new();
        let mut args = Vec::<String>::new();
        if use_exec_mode {
            args.push("exec".to_string());
            if is_json_output {
                args.push("--json".to_string());
            }
        } else {
            // 兼容旧实现
            args.push("ask".to_string());
        }

        // 添加跳过权限循环参数
        args.push("--dangerously-bypass-approvals-and-sandbox".to_string());

        // 添加模型参数
        if let Some(model_id) = &model_id {
            let trimmed = model_id.trim();
            if !trimmed.is_empty() && trimmed != "default" {
                args.push("--model".to_string());
                args.push(trimmed.to_string());
            }
        }

        // `codex exec` 不接受 `--search`，该参数只能挂在顶层 `codex` 命令上。
        // 因此需要在子命令之前注入，避免被解析成 exec 的未知参数。
        if enable_web_search {
            global_args.push("--search".to_string());
        }

        // Codex CLI 0.115.x 不支持 `--mcp-config`，MCP 需要依赖其原生配置文件。
        if let Some(servers) = &mcp_servers {
            if !servers.is_empty() {
                log_info!(
                    "检测到 {} 个 MCP 配置；当前 Codex CLI 版本不注入 --mcp-config，改为依赖本地 Codex 配置文件",
                    servers.len()
                );
            }
        }

        // 添加 JSON schema
        let mut schema_file: Option<TempSchemaFile> = None;
        if let Some(schema) = schema_text {
            if is_json_output {
                let file = TempSchemaFile::create(schema).await?;
                args.push("--output-schema".to_string());
                args.push(file.to_path_string());
                schema_file = Some(file);
            }
        }

        // 添加自定义参数
        if let Some(custom_args) = &extra_cli_args {
            if !custom_args.is_empty() {
                args.extend(custom_args.iter().cloned());
                log_info!("追加自定义 CLI 参数: {:?}", custom_args);
            }
        }

        // 构建输入消息
        let input_text = if use_exec_mode {
            messages
                .iter()
                .map(render_cli_message)
                .collect::<Vec<_>>()
                .join("\n\n")
        } else {
            messages.last().map(render_cli_message).unwrap_or_default()
        };

        let should_pipe_prompt_via_stdin = use_exec_mode;
        if should_pipe_prompt_via_stdin {
            args.push("-".to_string());
        } else {
            args.push(input_text.clone());
        }

        // 构建完整命令（用于日志）
        let full_command = build_full_codex_command(&cli_path, &global_args, &args);
        log_info!("Codex CLI command: {}", full_command);

        // 执行命令
        let mut command_args = global_args.clone();
        command_args.extend(args.clone());
        let mut cmd = build_tokio_cli_command(&cli_path, &command_args);

        // 设置工作目录
        if let Some(working_dir) = &working_directory {
            let trimmed_dir = working_dir.trim();
            if !trimmed_dir.is_empty() {
                cmd.current_dir(trimmed_dir);
                log_info!("设置工作目录: {}", trimmed_dir);
            }
        }

        cmd.stdin(if should_pipe_prompt_via_stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("CLAUDECODE");

        let execution_started_at = Instant::now();
        let monitor = CliExecutionMonitor::new();
        let timeout_config = timeout_config_for_execution_mode(request.execution_mode.as_deref());
        let mut child = cmd.spawn()?;

        let stdin_write_handle = if should_pipe_prompt_via_stdin {
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
        } else {
            None
        };

        // 注册进程 PID，用于后续可能的中断操作
        if let Some(pid) = child.id() {
            register_session_pid(&session_id, pid).await;
        }

        // 获取输出流
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

        // 处理标准输出
        let stdout_handle = tokio::spawn(async move {
            if is_stream_json {
                let reader = tokio::io::BufReader::new(stdout);
                let mut lines = reader.lines();
                let mut outcome = StdoutReadOutcome::none();

                while let Ok(Some(line)) = lines.next_line().await {
                    if should_abort(&session_id_clone).await {
                        break;
                    }

                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<serde_json::Value>(trimmed) {
                        Ok(json_value) => {
                            if let Some(event) =
                                parse_codex_json_output(&session_id_clone, &json_value)
                            {
                                outcome.emitted_non_error_event |=
                                    is_successful_event_type(&event.event_type);
                                outcome.emitted_content |= event.event_type == "content";
                                outcome.emitted_error |= event.event_type == "error";
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
                        Err(error) => {
                            let preview = preview_text(trimmed, 120);
                            log_error!("[stdout] 非 JSON 行已忽略: {} | {}", error, preview);
                        }
                    }
                }

                return outcome;
            }

            // 非流式 JSON 输出处理
            let mut reader = tokio::io::BufReader::new(stdout);
            let mut full_output = String::new();
            if let Err(error) = reader.read_to_string(&mut full_output).await {
                log_error!("[stdout] 读取失败: {}", error);
                return StdoutReadOutcome::none();
            }
            log_info!(
                "[stdout] 已读取完成，长度 {} 字符",
                full_output.chars().count()
            );
            log_info!("[stdout] 输出预览: {}", preview_text(&full_output, 500));

            if should_abort(&session_id_clone).await {
                return StdoutReadOutcome::none();
            }

            let normalized = full_output.trim();
            if normalized.is_empty() {
                log_error!("[stdout] 输出为空");
                return StdoutReadOutcome::none();
            }

            // 尝试解析为 JSON blob
            if let Some(event) = parse_codex_json_blob_output(&session_id_clone, normalized) {
                log_info!(
                    "[stdout] 发送事件: {}, event_type: {}",
                    event_name_clone,
                    event.event_type
                );
                stdout_monitor.note_activity(is_meaningful_event_type(&event.event_type));
                emit_cli_event(
                    &app_clone,
                    &event_name_clone,
                    plan_id_clone.as_ref(),
                    &event,
                );
                return StdoutReadOutcome {
                    emitted_content: event.event_type == "content",
                    emitted_error: event.event_type == "error",
                    emitted_non_error_event: is_successful_event_type(&event.event_type),
                };
            }

            // 无法解析为 JSON，直接发送原始内容
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
            }
        });

        let session_id_clone = session_id.clone();
        let app_clone = app.clone();
        let event_name_clone = event_name.clone();
        let plan_id_clone = plan_id.clone();
        let stderr_monitor = monitor.clone();

        // 处理标准错误
        let stderr_handle = tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut outcome = StderrReadOutcome::none();

            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if should_ignore_stderr_line(trimmed) {
                    stderr_monitor.note_stderr_warning();
                    log_info!("[stderr][ignored-warning] {}", trimmed);
                    continue;
                }

                stderr_monitor.note_activity(false);
                log_error!("[stderr] {}", trimmed);

                if should_abort(&session_id_clone).await {
                    break;
                }

                // 检查是否是真正的错误消息
                let line_lower = trimmed.to_lowercase();
                let is_error = line_lower.contains("error")
                    || line_lower.contains("failed")
                    || line_lower.contains("exception")
                    || line_lower.contains("fatal");

                if is_error {
                    outcome.emitted_error = true;
                    let event = build_error_event(&session_id_clone, trimmed.to_string());
                    emit_cli_event(
                        &app_clone,
                        &event_name_clone,
                        plan_id_clone.as_ref(),
                        &event,
                    );
                }
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
                    build_timeout_error_message("Codex", timeout_kind, &snapshot, now);
                log_error!("{}", error_message);
                let error_event = build_error_event(&session_id, error_message.clone());
                emit_cli_event(&app, &event_name, plan_id.as_ref(), &error_event);
                timeout_error_message = Some(error_message);

                if let Err(error) = child.kill().await {
                    log_error!("终止超时的 Codex CLI 进程失败: {}", error);
                }

                let exit_status = child.wait().await?;
                monitor.note_process_exit(exit_status.code());
                break exit_status;
            }

            sleep(Duration::from_millis(250)).await;
        };
        let elapsed = execution_started_at.elapsed();
        log_info!(
            "CLI 执行完成，退出码: {:?}, 耗时: {:.2}s",
            status.code(),
            elapsed.as_secs_f64()
        );

        // 等待输出处理完成
        let stdout_outcome = match stdout_handle.await {
            Ok(outcome) => outcome,
            Err(error) => {
                log_error!("[stdout] 任务等待失败: {}", error);
                StdoutReadOutcome::none()
            }
        };
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

        let should_treat_failure_as_success =
            should_treat_process_failure_as_success(&stdout_outcome, &stderr_outcome);
        let execution_succeeded = status.success() || should_treat_failure_as_success;

        if timeout_error_message.is_none() && execution_succeeded {
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
            };
            emit_cli_event(&app, &event_name, plan_id.as_ref(), &done_event);
        }

        // 注销进程 PID
        unregister_session_pid(&session_id).await;

        // 清理中断标志
        clear_abort_flag(&session_id).await;

        drop(schema_file);

        if let Some(error_message) = timeout_error_message {
            return Err(anyhow::anyhow!(error_message));
        }

        if !status.success() {
            if should_treat_failure_as_success {
                log_info!(
                    "忽略 CLI 非零/空退出码：已收到有效输出，exit_code={:?}, {}",
                    status.code(),
                    summary
                );
                return Ok(());
            }
            return Err(anyhow::anyhow!(
                "Codex CLI 执行失败，退出码: {:?}, {}",
                status.code(),
                summary
            ));
        }

        Ok(())
    }
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
    }
}

fn build_usage_event(
    session_id: &str,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
) -> CliStreamEvent {
    CliStreamEvent {
        event_type: "usage".to_string(),
        session_id: session_id.to_string(),
        content: None,
        tool_name: None,
        tool_call_id: None,
        tool_input: None,
        tool_result: None,
        error: None,
        input_tokens,
        output_tokens,
        model: None,
    }
}

fn build_command_execution_tool_use_event(
    session_id: &str,
    item: &serde_json::Value,
) -> Option<CliStreamEvent> {
    let tool_call_id = item.get("id").and_then(|value| value.as_str())?;
    let command = item
        .get("command")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let tool_input = serde_json::json!({ "command": command }).to_string();

    Some(CliStreamEvent {
        event_type: "tool_use".to_string(),
        session_id: session_id.to_string(),
        content: None,
        tool_name: Some("Bash".to_string()),
        tool_call_id: Some(tool_call_id.to_string()),
        tool_input: Some(tool_input),
        tool_result: None,
        error: None,
        input_tokens: None,
        output_tokens: None,
        model: None,
    })
}

fn build_command_execution_tool_result_event(
    session_id: &str,
    item: &serde_json::Value,
) -> Option<CliStreamEvent> {
    let tool_call_id = item.get("id").and_then(|value| value.as_str())?;
    let command = item
        .get("command")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let status = item
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let exit_code = item.get("exit_code").and_then(|value| value.as_i64());
    let output = item
        .get("aggregated_output")
        .and_then(|value| value.as_str())
        .unwrap_or_default();

    let mut parts = Vec::new();
    if !command.trim().is_empty() {
        parts.push(format!("command: {}", command));
    }
    if !status.trim().is_empty() {
        parts.push(format!("status: {}", status));
    }
    if let Some(code) = exit_code {
        parts.push(format!("exit_code: {}", code));
    }
    if !output.trim().is_empty() {
        parts.push(output.trim().to_string());
    }

    let tool_result = if parts.is_empty() {
        "command completed".to_string()
    } else {
        parts.join("\n")
    };

    Some(CliStreamEvent {
        event_type: "tool_result".to_string(),
        session_id: session_id.to_string(),
        content: None,
        tool_name: None,
        tool_call_id: Some(tool_call_id.to_string()),
        tool_input: None,
        tool_result: Some(tool_result),
        error: None,
        input_tokens: None,
        output_tokens: None,
        model: None,
    })
}

fn build_full_codex_command(cli_path: &str, global_args: &[String], args: &[String]) -> String {
    let mut cmd_parts = Vec::new();
    cmd_parts.push(shell_escape(cli_path));
    cmd_parts.extend(global_args.iter().map(|arg| shell_escape(arg)));
    cmd_parts.extend(args.iter().map(|arg| shell_escape(arg)));
    cmd_parts.join(" ")
}

fn parse_codex_json_output(session_id: &str, json: &serde_json::Value) -> Option<CliStreamEvent> {
    let event_type = json
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or_default();

    match event_type {
        "system" => extract_runtime_system_notice(json)
            .map(|content| build_system_event(session_id, content)),
        // === Codex CLI 特有事件类型 ===
        "item.started" => {
            let item = json.get("item")?;
            let item_type = item.get("type").and_then(|value| value.as_str())?;

            match item_type {
                "command_execution" => build_command_execution_tool_use_event(session_id, item),
                "reasoning" => Some(build_thinking_start_event(session_id)),
                _ => None,
            }
        }
        "item.completed" => {
            let item = json.get("item")?;
            let item_type = item.get("type").and_then(|value| value.as_str())?;

            match item_type {
                "agent_message" => {
                    let text = extract_item_text(item)?;
                    Some(build_content_event(session_id, text))
                }
                "reasoning" => {
                    let text = extract_item_text(item)?;
                    Some(build_thinking_event(session_id, text))
                }
                "command_execution" => build_command_execution_tool_result_event(session_id, item),
                _ => None,
            }
        }
        "item.delta" => {
            let item_type = json
                .pointer("/item/type")
                .and_then(|value| value.as_str())
                .or_else(|| json.get("item_type").and_then(|value| value.as_str()));

            if matches!(item_type, Some("reasoning")) {
                if let Some(text) = extract_text_value(json.get("delta")) {
                    return Some(build_thinking_event(session_id, text));
                }
            }

            if let Some(text) = extract_text_value(json.get("delta")) {
                return Some(build_content_event(session_id, text));
            }
            None
        }
        "turn.completed" => {
            let usage = json.get("usage");
            let input_tokens = usage
                .and_then(|u| u.get("input_tokens"))
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);
            let output_tokens = usage
                .and_then(|u| u.get("output_tokens"))
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);

            if input_tokens.is_some() || output_tokens.is_some() {
                Some(build_usage_event(session_id, input_tokens, output_tokens))
            } else {
                extract_turn_output(json).map(|content| build_content_event(session_id, content))
            }
        }
        "turn.failed" => {
            let error_text = extract_text_value(json.get("error"))
                .or_else(|| extract_text_value(json.get("message")))
                .unwrap_or_else(|| "Codex CLI turn failed".to_string());
            Some(build_error_event(session_id, error_text))
        }

        // === Claude CLI 兼容事件类型 ===
        "content_block_delta" => {
            let delta = json.get("delta")?;
            let delta_type = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match delta_type {
                "text_delta" => {
                    let text = delta.get("text").and_then(|t| t.as_str())?;
                    Some(build_content_event(session_id, text.to_string()))
                }
                "thinking_delta" => {
                    // 处理思考增量
                    let thinking = delta.get("thinking").and_then(|t| t.as_str())?;
                    Some(build_thinking_event(session_id, thinking.to_string()))
                }
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
                    })
                }
                "thinking" => {
                    // thinking 内容块开始
                    Some(build_thinking_start_event(session_id))
                }
                _ => None,
            }
        }
        "content_block_stop" => {
            // 内容块结束，暂时不处理
            None
        }
        "message_start" => {
            let message = json.get("message")?;

            // 提取 token 使用量
            let usage = message.get("usage");
            let input_tokens = usage
                .and_then(|u| u.get("input_tokens"))
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);
            let output_tokens = usage
                .and_then(|u| u.get("output_tokens"))
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);

            // 提取模型信息
            let model = message
                .get("model")
                .and_then(|m| m.as_str())
                .map(|m| m.to_string());

            if input_tokens.is_some() || output_tokens.is_some() || model.is_some() {
                Some(CliStreamEvent {
                    event_type: "message_start".to_string(),
                    session_id: session_id.to_string(),
                    content: None,
                    tool_name: None,
                    tool_call_id: None,
                    tool_input: None,
                    tool_result: None,
                    error: None,
                    input_tokens,
                    output_tokens,
                    model,
                })
            } else {
                None
            }
        }
        "message_delta" => {
            let usage = json.get("usage");

            let input_tokens = usage
                .and_then(|u| u.get("input_tokens"))
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);
            let output_tokens = usage
                .and_then(|u| u.get("output_tokens"))
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);

            if input_tokens.is_some() || output_tokens.is_some() {
                Some(build_usage_event(session_id, input_tokens, output_tokens))
            } else {
                None
            }
        }
        "message_stop" => Some(CliStreamEvent {
            event_type: "done".to_string(),
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
        }),

        // === 工具相关事件 ===
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
            })
        }

        // === 错误处理 ===
        "error" => {
            let error_msg = json
                .get("error")
                .and_then(|e| e.as_str())
                .or_else(|| json.get("message").and_then(|m| m.as_str()))
                .unwrap_or("Unknown error");

            Some(build_error_event(session_id, error_msg.to_string()))
        }

        // === 消息类型 ===
        "assistant" => {
            let message = json.get("message")?;
            let content_array = message.get("content").and_then(|c| c.as_array())?;

            // 遍历所有 content items，找到第一个有效的并返回
            // 优先级：thinking > text > tool_use
            for content_item in content_array {
                let item_type = content_item
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");

                match item_type {
                    "thinking" => {
                        // 处理 thinking 类型
                        if let Some(thinking_text) =
                            content_item.get("thinking").and_then(|t| t.as_str())
                        {
                            log_debug!("[parse] 找到 thinking 内容，长度: {}", thinking_text.len());
                            return Some(CliStreamEvent {
                                event_type: "thinking".to_string(),
                                session_id: session_id.to_string(),
                                content: Some(thinking_text.to_string()),
                                tool_name: None,
                                tool_call_id: None,
                                tool_input: None,
                                tool_result: None,
                                error: None,
                                input_tokens: None,
                                output_tokens: None,
                                model: None,
                            });
                        }
                    }
                    "text" => {
                        let text = content_item.get("text").and_then(|t| t.as_str())?;
                        return Some(build_content_event(session_id, text.to_string()));
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
                            input_tokens: None,
                            output_tokens: None,
                            model: None,
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

        // === 结果类型 ===
        "result" => extract_structured_payload(json)
            .or_else(|| extract_text_value(json.get("result")))
            .map(|content| build_content_event(session_id, content)),

        // === 默认回退 ===
        _ => {
            if let Some(content) = extract_runtime_system_notice(json) {
                return Some(build_system_event(session_id, content));
            }

            extract_structured_payload(json)
                .or_else(|| extract_turn_output(json))
                .map(|content| build_content_event(session_id, content))
        }
    }
}

fn extract_item_text(item: &serde_json::Value) -> Option<String> {
    extract_text_value(item.get("text"))
        .or_else(|| extract_structured_payload(item))
        .or_else(|| extract_text_value(item.get("content")))
}

fn extract_structured_payload(value: &serde_json::Value) -> Option<String> {
    value
        .get("structured_output")
        .or_else(|| value.get("structuredOutput"))
        .or_else(|| value.get("output_struct"))
        .or_else(|| value.get("outputStruct"))
        .and_then(|v| serde_json::to_string(v).ok())
}

fn extract_turn_output(value: &serde_json::Value) -> Option<String> {
    extract_structured_payload(value)
        .or_else(|| extract_text_value(value.get("output")))
        .or_else(|| extract_text_value(value.get("result")))
        .or_else(|| {
            value
                .pointer("/result/output")
                .and_then(|v| extract_text_value(Some(v)))
        })
}

fn extract_text_value(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?;

    if let Some(text) = value.as_str() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Some(array) = value.as_array() {
        let combined = array
            .iter()
            .filter_map(|part| extract_text_value(Some(part)))
            .collect::<Vec<_>>()
            .join("");
        let trimmed = combined.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
        return None;
    }

    if value.is_object() {
        if let Some(text) = value
            .get("text")
            .and_then(|part| part.as_str())
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
        {
            return Some(text);
        }

        if let Some(content) = value.get("content") {
            if let Some(text) = extract_text_value(Some(content)) {
                return Some(text);
            }
        }

        if let Some(message) = value
            .get("message")
            .and_then(|part| part.as_str())
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
        {
            return Some(message);
        }

        if let Ok(serialized) = serde_json::to_string(value) {
            if !serialized.is_empty() {
                return Some(serialized);
            }
        }
    }

    None
}

/// 解析非流式 JSON blob 输出
fn parse_codex_json_blob_output(session_id: &str, output: &str) -> Option<CliStreamEvent> {
    log_info!(
        "[parse] 开始解析 JSON blob, 长度: {}",
        output.chars().count()
    );

    let parsed = match parse_json_blob_with_fallback(output) {
        Ok(value) => value,
        Err(e) => {
            log_error!("[parse] JSON 解析失败: {:?}", e);
            return None;
        }
    };

    if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
        log_info!("CLI 返回完整内容:\n{}", pretty);
    }

    if let Some(content) = extract_runtime_system_notice(&parsed) {
        return Some(build_system_event(session_id, content));
    }

    // 尝试提取结构化输出
    if let Some(content) = extract_structured_output_from_json_blob(&parsed) {
        log_info!(
            "[parse] 提取到 structured_output, 长度: {}",
            content.chars().count()
        );
        return Some(build_content_event(session_id, content));
    }

    // 尝试提取错误
    if let Some(error) = extract_error_from_json_blob(&parsed) {
        log_info!("[parse] 提取到 error: {}", error);
        return Some(build_error_event(session_id, error));
    }

    // 尝试提取结果内容
    if let Some(content) = extract_result_content_from_json_blob(&parsed) {
        log_info!(
            "[parse] 提取到 result.content, 长度: {}",
            content.chars().count()
        );
        return Some(build_content_event(session_id, content));
    }

    // 返回原始 JSON
    if let Ok(raw_json) = serde_json::to_string(&parsed) {
        log_info!("[parse] 返回原始 JSON, 长度: {}", raw_json.chars().count());
        return Some(build_content_event(session_id, raw_json));
    }

    log_error!("[parse] 无法提取任何内容");
    None
}

#[cfg(test)]
mod tests {
    use super::{
        should_ignore_stderr_line, should_treat_process_failure_as_success, StderrReadOutcome,
        StdoutReadOutcome,
    };

    #[test]
    fn ignores_rmcp_transport_closed_warning() {
        let warning = "rmcp::transport::worker: worker quit with fatal: Transport channel closed";
        assert!(should_ignore_stderr_line(warning));
    }

    #[test]
    fn keeps_real_stderr_failures_as_errors() {
        let failure = "fatal: command exited because authentication failed";
        assert!(!should_ignore_stderr_line(failure));
    }

    #[test]
    fn treats_exit_without_code_as_success_when_output_is_valid() {
        let stdout_outcome = StdoutReadOutcome {
            emitted_content: true,
            emitted_error: false,
            emitted_non_error_event: true,
        };
        let stderr_outcome = StderrReadOutcome {
            emitted_error: false,
        };

        assert!(should_treat_process_failure_as_success(
            &stdout_outcome,
            &stderr_outcome,
        ));
    }

    #[test]
    fn does_not_treat_failure_as_success_when_error_was_emitted() {
        let stdout_outcome = StdoutReadOutcome {
            emitted_content: true,
            emitted_error: true,
            emitted_non_error_event: true,
        };
        let stderr_outcome = StderrReadOutcome {
            emitted_error: false,
        };

        assert!(!should_treat_process_failure_as_success(
            &stdout_outcome,
            &stderr_outcome,
        ));
    }
}
