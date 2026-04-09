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
    build_timeout_error_message, describe_timeout_config, detect_cli_timeout, emit_cli_event,
    extract_error_from_json_blob, extract_result_content_from_json_blob, extract_runtime_system_notice,
    extract_structured_output_from_json_blob, parse_json_blob_with_fallback, preview_text,
    render_cli_message, shell_escape, timeout_config_for_execution_mode, CliExecutionMonitor,
};
use crate::commands::cli_support::{build_cli_launch_error_message, build_tokio_cli_command};
use crate::commands::conversation::abort::{
    clear_abort_flag, register_session_pid, set_abort_flag, should_abort, unregister_session_pid,
};
use crate::commands::conversation::strategy::{AgentExecutionStrategy, AgentRuntimeKind};
use crate::commands::conversation::types::{
    CliStreamEvent, ExecutionRequest, McpServerConfig, MessageInput,
};
use crate::commands::mcp_shared::parse_args_string;

/// Claude CLI 策略
pub struct ClaudeCliStrategy;

// 简单的日志宏
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            crate::logging::write_log("INFO", "claude-cli", &message);
            println!("[INFO][claude-cli] {}", message)
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

fn is_rmcp_transport_closed_warning(line: &str) -> bool {
    let normalized = line.to_lowercase();
    normalized.contains("rmcp::transport::worker")
        && normalized.contains("transport channel closed")
}

fn should_ignore_stderr_line(line: &str) -> bool {
    is_rmcp_transport_closed_warning(line)
}

fn should_treat_process_failure_as_success(
    stdout_outcome: &StdoutReadOutcome,
    stderr_outcome: &StderrReadOutcome,
) -> bool {
    (stdout_outcome.emitted_content || stdout_outcome.emitted_non_error_event)
        && !stdout_outcome.emitted_error
        && !stderr_outcome.emitted_error
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
    }
}

fn extract_external_session_id(json: &serde_json::Value) -> Option<String> {
    [
        json.get("session_id"),
        json.pointer("/session/id"),
        json.pointer("/message/session_id"),
        json.pointer("/result/session_id"),
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
        let mut args = vec!["-p".to_string(), "--output-format".to_string(), cli_output_format.clone()];
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
            .map(render_cli_message)
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
        let timeout_config = timeout_config_for_execution_mode(request.execution_mode.as_deref());
        log_info!(
            "Claude CLI timeout config: mode={}, {}",
            request.execution_mode.as_deref().unwrap_or("chat"),
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

        // 处理标准输出
        let stdout_handle = tokio::spawn(async move {
            if is_stream_json {
                let reader = tokio::io::BufReader::new(stdout);
                let mut lines = reader.lines();
                let mut outcome = StdoutReadOutcome::none();

                while let Ok(Some(line)) = lines.next_line().await {
                    log_info!("[stdout] 原始行: {}", line);

                    if should_abort(&session_id_clone).await {
                        break;
                    }

                    match serde_json::from_str::<serde_json::Value>(&line) {
                        Ok(json_value) => {
                            let event_type = json_value
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("unknown");
                            log_info!("[stdout] JSON type: {}", event_type);
                            let event = parse_claude_json_output(&session_id_clone, &json_value);
                            if let Some(event) = event {
                                outcome.emitted_non_error_event |=
                                    is_successful_event_type(&event.event_type);
                                outcome.emitted_content |= event.event_type == "content";
                                outcome.emitted_error |= event.event_type == "error";
                                stdout_monitor
                                    .note_activity(is_meaningful_event_type(&event.event_type));
                                log_info!(
                                    "[stdout] 发送事件: type={}, content_len={:?}",
                                    event.event_type,
                                    event.content.as_ref().map(|c| c.len())
                                );
                                emit_cli_event(
                                    &app_clone,
                                    &event_name_clone,
                                    plan_id_clone.as_ref(),
                                    &event,
                                );
                            } else {
                                log_info!("[stdout] 解析返回 None");
                            }
                        }
                        Err(e) => {
                            log_error!("[stdout] JSON 解析失败: {:?}", e);
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
            log_info!("[stdout] 输出预览: {}", preview_text(&full_output, 500));

            if should_abort(&session_id_clone).await {
                return StdoutReadOutcome::none();
            }

            let normalized = full_output.trim();
            if normalized.is_empty() {
                log_error!("[stdout] 输出为空");
                return StdoutReadOutcome::none();
            }

            if let Some(event) = parse_claude_json_blob_output(&session_id_clone, normalized) {
                log_info!(
                    "[stdout] 发送事件: {}, event_type: {}",
                    event_name_clone,
                    event.event_type
                );
                log_info!(
                    "[stdout] 事件内容长度: {:?}",
                    event.content.as_ref().map(|c| c.len())
                );
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
                };
            }

            log_info!("[stdout] 无法解析为结构化输出，直接发送原始内容");
            let event = build_content_event(&session_id_clone, normalized.to_string());
            log_info!(
                "[stdout] 发送原始内容事件: {}, event_type: {}",
                event_name_clone,
                event.event_type
            );
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
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut error_output = String::new();
            if reader.read_to_string(&mut error_output).await.is_err() {
                return StderrReadOutcome::none();
            }

            if error_output.is_empty() {
                return StderrReadOutcome::none();
            }

            log_error!("[stderr] {}", error_output);

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
                let event = build_error_event(&session_id_clone, error_msg);
                emit_cli_event(
                    &app_clone,
                    &event_name_clone,
                    plan_id_clone.as_ref(),
                    &event,
                );
                return StderrReadOutcome {
                    emitted_error: true,
                };
            }

            StderrReadOutcome::none()
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
            "CLI 执行完成，退出码: {:?}, 耗时: {:.2}s",
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

        // 注销进程 PID
        unregister_session_pid(&session_id).await;

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
                external_session_id: None,
            };
            emit_cli_event(&app, &event_name, plan_id.as_ref(), &done_event);
        }

        // 清理中断标志
        clear_abort_flag(&session_id).await;
        drop(mcp_config_file);

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
                "CLI 执行失败，退出码: {:?}, {}",
                status.code(),
                summary
            ));
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
        Err(e) => {
            log_error!("[parse] JSON 解析失败: {:?}", e);
            return None;
        }
    };

    if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
        log_info!("CLI 返回完整内容:\n{}", pretty);
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

    log_error!("[parse] 无法提取任何内容");
    None
}

fn extract_usage_counts(usage: Option<&serde_json::Value>) -> (Option<u32>, Option<u32>) {
    let raw_input_tokens = usage
        .and_then(|u| {
            u.get("input_tokens")
                .or_else(|| u.get("inputTokens"))
                .or_else(|| u.get("cache_read_input_tokens"))
                .or_else(|| u.get("cacheReadInputTokens"))
        })
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);
    let raw_output_tokens = usage
        .and_then(|u| u.get("output_tokens").or_else(|| u.get("outputTokens")))
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);

    let has_non_zero_usage =
        raw_input_tokens.unwrap_or(0) > 0 || raw_output_tokens.unwrap_or(0) > 0;
    if !has_non_zero_usage {
        return (None, None);
    }

    let input_tokens = raw_input_tokens.or(Some(0));
    let output_tokens = raw_output_tokens.or(Some(0));

    (input_tokens, output_tokens)
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
fn parse_claude_json_output(session_id: &str, json: &serde_json::Value) -> Option<CliStreamEvent> {
    let event_type = json
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");

    let event = match event_type {
        "system" => {
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
                })
            }
        }
        "content_block_delta" => {
            let delta = json.get("delta")?;
            let delta_type = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match delta_type {
                "thinking_delta" => {
                    let thinking = delta.get("thinking").and_then(|t| t.as_str())?;
                    Some(build_thinking_event(session_id, thinking.to_string()))
                }
                "text_delta" => {
                    let text = delta.get("text").and_then(|t| t.as_str())?;
                    Some(build_content_event(session_id, text.to_string()))
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
                "thinking" => Some(build_thinking_start_event(session_id)),
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
            let message = json.get("message")?;

            // 提取 token 使用量
            let (input_tokens, output_tokens) = extract_usage_counts(message.get("usage"));

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
                    external_session_id: None,
                })
            } else {
                None
            }
        }
        "message_delta" => {
            let (input_tokens, output_tokens) = extract_usage_counts(json.get("usage"));

            if input_tokens.is_some() || output_tokens.is_some() {
                Some(CliStreamEvent {
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
                    external_session_id: None,
                })
            } else {
                None
            }
        }
        // message_stop 只表示当前一轮模型消息结束，不代表整个 CLI 执行完成。
        // 在 agentic 工具调用场景中，CLI 会执行工具后继续下一轮对话，产生多个 message_stop。
        // 因此不能将 message_stop 转换为 done 事件，否则会导致前端过早标记消息为"已完成"。
        // 最终的 done 事件由进程退出后的代码统一发出。
        "message_stop" => {
            // 不再将 message_stop 映射为 "done"。
            // 最终的 done 事件由进程退出后的代码统一发出，
            // 确保 result 事件中的 session_id 先被处理和持久化，
            // 避免排队消息在 session_id 存储前就开始处理。
            let (input_tokens, output_tokens) = extract_usage_counts(json.get("usage"));
            if input_tokens.is_some() || output_tokens.is_some() {
                Some(CliStreamEvent {
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
                    external_session_id: None,
                })
            } else {
                None
            }
        }
        "result" => {
            let (input_tokens, output_tokens) = extract_usage_counts(json.get("usage"));
            let model = extract_result_model_name(json);

            if input_tokens.is_some() || output_tokens.is_some() || model.is_some() {
                Some(CliStreamEvent {
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
                    model,
                    external_session_id: None,
                })
            } else {
                // result 事件可能不携带 usage/model，但仍包含 session_id，
                // 需要提取以保证 resume 绑定能正确存储。
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
                })
            }
        }
        "turn.completed" => {
            let (input_tokens, output_tokens) = extract_usage_counts(json.get("usage"));

            if input_tokens.is_some() || output_tokens.is_some() {
                Some(CliStreamEvent {
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
                    external_session_id: None,
                })
            } else {
                None
            }
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
            let (input_tokens, output_tokens) = extract_usage_counts(message.get("usage"));
            let model = message
                .get("model")
                .and_then(|m| m.as_str())
                .map(|m| m.to_string());

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
                                input_tokens,
                                output_tokens,
                                model: model.clone(),
                                external_session_id: extract_external_session_id(json),
                                ..build_thinking_event(session_id, thinking_text.to_string())
                            });
                        }
                    }
                    "text" => {
                        let text = content_item.get("text").and_then(|t| t.as_str())?;
                        return Some(CliStreamEvent {
                            event_type: "content".to_string(),
                            session_id: session_id.to_string(),
                            content: Some(text.to_string()),
                            tool_name: None,
                            tool_call_id: None,
                            tool_input: None,
                            tool_result: None,
                            error: None,
                            input_tokens,
                            output_tokens,
                            model: model.clone(),
                            external_session_id: None,
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
                            external_session_id: None,
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
    fn treats_non_zero_exit_as_success_when_valid_output_arrived() {
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
    fn does_not_hide_real_stderr_errors() {
        let stdout_outcome = StdoutReadOutcome {
            emitted_content: true,
            emitted_error: false,
            emitted_non_error_event: true,
        };
        let stderr_outcome = StderrReadOutcome {
            emitted_error: true,
        };

        assert!(!should_treat_process_failure_as_success(
            &stdout_outcome,
            &stderr_outcome,
        ));
    }
}
