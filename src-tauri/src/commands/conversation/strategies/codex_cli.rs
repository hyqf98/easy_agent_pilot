use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

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
    build_system_event, build_timeout_error_message, describe_timeout_config, detect_cli_timeout,
    emit_cli_event, extract_error_from_json_blob, extract_image_paths,
    extract_result_content_from_json_blob, extract_runtime_system_notice,
    extract_structured_output_from_json_blob, parse_json_blob_with_fallback, preview_text,
    render_cli_message, shell_escape, timeout_config_for_execution_mode, CliExecutionMonitor,
    NonImageAttachmentPromptMode,
};
use crate::commands::cli_support::{build_cli_launch_error_message, build_tokio_cli_command};
use crate::commands::conversation::abort::{
    clear_abort_flag, register_session_pid, set_abort_flag, should_abort, unregister_session_pid,
};
use crate::commands::conversation::strategy::{AgentExecutionStrategy, AgentRuntimeKind};
use crate::commands::conversation::types::{CliStreamEvent, ExecutionRequest};

/// Codex CLI 策略
pub struct CodexCliStrategy;

const CODEX_RUNTIME_HOME_DIR: &str = "codex-home";
const CODEX_RUNTIME_SYNC_FILES: &[&str] = &[
    "config.toml",
    "auth.json",
    "version.json",
    ".personality_migration",
    "installation_id",
];
const CODEX_RUNTIME_SYNC_DIRS: &[&str] = &["skills", "plugins", "rules", "sessions", "memories"];

// 简单的日志宏
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            crate::logging::write_log("INFO", "codex-cli", &message);
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
    (normalized.contains("rmcp::transport::worker")
        && normalized.contains("unexpectedcontenttype")
        && normalized.contains("missing-content-type"))
        || (normalized.contains("rmcp::transport::async_rw")
            && normalized.contains("serde error expected")
            && normalized.contains("line 1 column"))
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

fn should_ignore_stderr_line(line: &str) -> bool {
    is_benign_stderr_warning(line) || is_shared_benign_stderr_warning(line)
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

fn copy_directory_recursive(source: &Path, target: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }

    fs::create_dir_all(target)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if source_path.is_dir() {
            copy_directory_recursive(&source_path, &target_path)?;
            continue;
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source_path, &target_path)?;
    }

    Ok(())
}

fn copy_file_if_exists(source_root: &Path, target_root: &Path, relative_path: &str) -> Result<()> {
    let source_path = source_root.join(relative_path);
    if !source_path.exists() {
        return Ok(());
    }

    let target_path = target_root.join(relative_path);
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source_path, target_path)?;
    Ok(())
}

fn sync_codex_runtime_home(source_root: &Path, target_root: &Path) -> Result<()> {
    for relative_path in CODEX_RUNTIME_SYNC_FILES {
        copy_file_if_exists(source_root, target_root, relative_path)?;
    }

    for relative_path in CODEX_RUNTIME_SYNC_DIRS {
        let source_path = source_root.join(relative_path);
        if !source_path.exists() {
            continue;
        }

        copy_directory_recursive(&source_path, &target_root.join(relative_path))?;
    }

    Ok(())
}

fn discover_codex_session_id(codex_home: &Path, cutoff: std::time::Instant) -> Option<String> {
    let sessions_dir = codex_home.join("sessions");
    if !sessions_dir.exists() {
        return None;
    }

    let cutoff_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .saturating_sub(cutoff.elapsed());
    let mut best_epoch = std::time::Duration::ZERO;
    let mut best_id: Option<String> = None;

    let Ok(entries) = std::fs::read_dir(&sessions_dir) else {
        return None;
    };
    for year_entry in entries.flatten() {
        if !year_entry.file_type().ok()?.is_dir() {
            continue;
        }
        let Ok(month_entries) = std::fs::read_dir(year_entry.path()) else {
            continue;
        };
        for month_entry in month_entries.flatten() {
            if !month_entry.file_type().ok()?.is_dir() {
                continue;
            }
            let Ok(day_entries) = std::fs::read_dir(month_entry.path()) else {
                continue;
            };
            for day_entry in day_entries.flatten() {
                let path = day_entry.path();
                let fname = path.file_name()?.to_string_lossy();
                if !fname.starts_with("rollout-") || !fname.ends_with(".jsonl") {
                    continue;
                }
                let Ok(metadata) = path.metadata() else {
                    continue;
                };
                let modified_epoch = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())?;
                if modified_epoch < cutoff_epoch {
                    continue;
                }
                if let Some(sid) = extract_codex_session_id_from_file(&path) {
                    if modified_epoch > best_epoch {
                        best_epoch = modified_epoch;
                        best_id = Some(sid);
                    }
                }
            }
        }
    }

    best_id
}

fn extract_codex_session_id_from_file(path: &Path) -> Option<String> {
    let Ok(file) = std::fs::File::open(path) else {
        return None;
    };
    let reader = std::io::BufReader::new(file);
    for line in std::io::BufRead::lines(reader).take(10) {
        let Ok(line) = line else {
            continue;
        };
        let Ok(obj) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if obj.get("type").and_then(|v| v.as_str()) == Some("session_meta") {
            if let Some(id) = obj
                .pointer("/payload/id")
                .and_then(|v| v.as_str())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
            {
                return Some(id);
            }
        }
    }
    None
}

fn prepare_codex_runtime_home() -> Result<PathBuf> {
    let runtime_home = crate::commands::get_persistence_dir_path()?
        .join("cache")
        .join(CODEX_RUNTIME_HOME_DIR);
    fs::create_dir_all(&runtime_home)?;

    if let Some(user_home) = dirs::home_dir() {
        let source_root = user_home.join(".codex");
        if source_root.exists() {
            sync_codex_runtime_home(&source_root, &runtime_home)?;
        }
    }

    Ok(runtime_home)
}

fn load_codex_runtime_auth_env(codex_home: &Path) -> HashMap<String, String> {
    let auth_path = codex_home.join("auth.json");
    let Ok(content) = fs::read_to_string(&auth_path) else {
        return HashMap::new();
    };

    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return HashMap::new();
    };

    let Some(object) = value.as_object() else {
        return HashMap::new();
    };

    object
        .iter()
        .filter_map(|(key, value)| {
            value
                .as_str()
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| (key.clone(), item.to_string()))
        })
        .collect()
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
        let resume_session_id = request
            .resume_session_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
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
        let use_exec_mode = true;
        let is_json_output = use_exec_mode
            && (cli_output_format == "json"
                || cli_output_format == "stream-json"
                || schema_text.is_some());

        // 构建命令参数
        let mut global_args = Vec::<String>::new();
        let mut args = Vec::<String>::new();
        args.push("exec".to_string());
        if resume_session_id.is_some() {
            args.push("resume".to_string());
        }
        if is_json_output {
            args.push("--json".to_string());
        }

        // 添加跳过权限循环参数
        if resume_session_id.is_none() {
            args.push("--dangerously-bypass-approvals-and-sandbox".to_string());
        }

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

        // 添加图片参数
        let image_paths = extract_image_paths(&messages).map_err(anyhow::Error::msg)?;
        if !image_paths.is_empty() {
            for path in &image_paths {
                args.push("-i".to_string());
                args.push(path.clone());
            }
            log_info!("追加 Codex CLI 图片参数: -i x{}", image_paths.len());
        }

        // 构建输入消息
        let input_text = messages
            .iter()
            .map(|message| render_cli_message(message, false, NonImageAttachmentPromptMode::All))
            .collect::<Vec<_>>()
            .join("\n\n");

        if let Some(resume_session_id) = &resume_session_id {
            args.push(resume_session_id.clone());
        }
        args.push("-".to_string());

        // 构建完整命令（用于日志）
        let full_command = build_full_codex_command(&cli_path, &global_args, &args);
        log_info!("Codex CLI command: {}", full_command);

        // 执行命令
        let mut command_args = global_args.clone();
        command_args.extend(args.clone());
        let mut cmd = build_tokio_cli_command(&cli_path, &command_args);
        let codex_runtime_home = prepare_codex_runtime_home()?;

        // 设置工作目录
        if let Some(working_dir) = &working_directory {
            let trimmed_dir = working_dir.trim();
            if !trimmed_dir.is_empty() {
                cmd.current_dir(trimmed_dir);
                log_info!("设置工作目录: {}", trimmed_dir);
            }
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("CODEX_HOME", &codex_runtime_home)
            .env_remove("CLAUDECODE");

        for (env_key, env_value) in load_codex_runtime_auth_env(&codex_runtime_home) {
            cmd.env(&env_key, &env_value);
            log_info!("注入 Codex 认证环境变量: {}", env_key);
        }

        log_info!("设置 CODEX_HOME: {}", codex_runtime_home.to_string_lossy());

        let execution_started_at = Instant::now();
        let monitor = CliExecutionMonitor::new();
        let timeout_config = timeout_config_for_execution_mode(request.execution_mode.as_deref());
        log_info!(
            "Codex CLI timeout config: mode={}, {}",
            request.execution_mode.as_deref().unwrap_or("chat"),
            describe_timeout_config(timeout_config)
        );
        let mut child = cmd.spawn().map_err(|error| {
            anyhow::anyhow!(build_cli_launch_error_message(
                "Codex",
                &cli_path,
                &error,
                working_directory.as_deref(),
                command_args.len(),
                input_text.chars().count(),
                true,
            ))
        })?;

        let stdin_payload = input_text.clone();
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to acquire stdin"))?;

        let stdin_write_handle = Some(tokio::spawn(async move {
            if let Err(error) = stdin.write_all(stdin_payload.as_bytes()).await {
                log_error!("[stdin] failed to write prompt: {}", error);
                return;
            }

            if let Err(error) = stdin.shutdown().await {
                log_error!("[stdin] failed to close stdin: {}", error);
            }
        }));

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
                                outcome.preview = Some(trimmed.to_string());
                            }
                            log_debug!("[stdout] 非 JSON 行已忽略: {}", _parse_error);
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
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut outcome = StderrReadOutcome::none();
            let mut actionable_lines: Vec<String> = Vec::new();

            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if should_ignore_stderr_line(trimmed) {
                    stderr_monitor.note_stderr_warning();
                    outcome.ignored_warning_count += 1;
                    continue;
                }

                stderr_monitor.note_activity(false);

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
                    if outcome.preview.is_none() {
                        outcome.preview = Some(trimmed.to_string());
                    }
                    actionable_lines.push(trimmed.to_string());
                    if let Some(fragment) =
                        CliTextFragment::new(CliTextSource::Stderr, trimmed.to_string())
                    {
                        outcome.fragments.push(fragment);
                    }
                } else if outcome.preview.is_none() {
                    outcome.preview = Some(trimmed.to_string());
                }
            }

            if !actionable_lines.is_empty() {
                let error_msg = actionable_lines.join("\n");
                let event = build_error_event(&session_id_clone, error_msg.clone());
                emit_cli_event(
                    &app_clone,
                    &event_name_clone,
                    plan_id_clone.as_ref(),
                    &event,
                );
                log_error!(
                    "[stderr] actionable_lines={}, preview={}",
                    actionable_lines.len(),
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
        let mut completion_fragments = stdout_outcome.fragments.clone();
        completion_fragments.extend(stderr_outcome.fragments.clone());
        let detected_failure = classify_cli_completion(
            "Codex",
            &completion_fragments,
            stdout_outcome.emitted_error || stderr_outcome.emitted_error,
        );
        let should_complete_as_success = should_treat_failure_as_success
            || (detected_failure.is_none() && stdout_outcome.emitted_content);
        let execution_succeeded = status.success() || should_complete_as_success;

        if timeout_error_message.is_none() && detected_failure.is_none() && execution_succeeded {
            let discovered_session_id = if resume_session_id.is_none() {
                discover_codex_session_id(&codex_runtime_home, execution_started_at)
            } else {
                resume_session_id.clone()
            };
            if let Some(ref sid) = discovered_session_id {
                log_info!("Codex session ID discovered: {}", sid);
            }
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
                external_session_id: discovered_session_id,
                raw_input_tokens: None,
                raw_output_tokens: None,
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
            };
            emit_cli_event(&app, &event_name, plan_id.as_ref(), &done_event);
        }

        // 注销进程 PID
        unregister_session_pid(&session_id).await;

        // 清理中断标志
        clear_abort_flag(&session_id).await;

        drop(schema_file);

        if let Some(error_message) = timeout_error_message {
            log_error!(
                "{}",
                build_cli_failure_report(
                    "Codex",
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
                    "Codex",
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
                    "忽略 CLI 非零/空退出码：已收到有效输出，exit_code={:?}, {}",
                    status.code(),
                    summary
                );
                return Ok(());
            }
            let failure_message = format!(
                "Codex CLI 执行失败，退出码: {:?}, {}",
                status.code(),
                summary
            );
            log_error!(
                "{}",
                build_cli_failure_report(
                    "Codex",
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

fn build_usage_event(
    session_id: &str,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    raw_input_tokens: Option<u32>,
    raw_output_tokens: Option<u32>,
    cache_read_input_tokens: Option<u32>,
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
        raw_input_tokens,
        raw_output_tokens,
        cache_read_input_tokens,
        cache_creation_input_tokens: None,
        model: None,
        external_session_id: None,
    }
}

fn extract_external_session_id(json: &serde_json::Value) -> Option<String> {
    [
        json.get("session_id"),
        json.pointer("/session/id"),
        json.pointer("/payload/id"),
        json.pointer("/session_meta/payload/id"),
        json.pointer("/item/session_id"),
        json.pointer("/message/session_id"),
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
        external_session_id: None,
        raw_input_tokens: None,
        raw_output_tokens: None,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
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
        external_session_id: None,
        raw_input_tokens: None,
        raw_output_tokens: None,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
    })
}

fn build_todo_list_tool_use_event(
    session_id: &str,
    item: &serde_json::Value,
) -> Option<CliStreamEvent> {
    let tool_call_id = item.get("id").and_then(|value| value.as_str())?;
    let items = item.get("items")?.as_array()?;
    let normalized_items = items
        .iter()
        .filter_map(|entry| {
            let text = entry.get("text").and_then(|value| value.as_str())?;
            let completed = entry
                .get("completed")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);

            Some(serde_json::json!({
                "text": text,
                "completed": completed,
                "status": if completed { "completed" } else { "pending" }
            }))
        })
        .collect::<Vec<_>>();

    let tool_input = serde_json::json!({ "items": normalized_items }).to_string();

    Some(CliStreamEvent {
        event_type: "tool_use".to_string(),
        session_id: session_id.to_string(),
        content: None,
        tool_name: Some("todo_list".to_string()),
        tool_call_id: Some(tool_call_id.to_string()),
        tool_input: Some(tool_input),
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

fn build_full_codex_command(cli_path: &str, global_args: &[String], args: &[String]) -> String {
    let mut cmd_parts = Vec::new();
    cmd_parts.push(shell_escape(cli_path));
    cmd_parts.extend(global_args.iter().map(|arg| shell_escape(arg)));
    cmd_parts.extend(args.iter().map(|arg| shell_escape(arg)));
    cmd_parts.join(" ")
}

fn parse_event_msg_usage_event(
    session_id: &str,
    json: &serde_json::Value,
) -> Option<CliStreamEvent> {
    let payload = json.get("payload")?;
    if payload.get("type").and_then(|value| value.as_str()) != Some("token_count") {
        return None;
    }

    let info = payload.get("info")?;
    let usage = info
        .get("last_token_usage")
        .or_else(|| info.get("total_token_usage"))?;

    let raw_input_tokens = usage
        .get("input_tokens")
        .and_then(|value| value.as_u64())
        .map(|value| value as u32);
    let raw_output_tokens = usage
        .get("output_tokens")
        .and_then(|value| value.as_u64())
        .map(|value| value as u32);
    let cached_input_tokens = usage
        .get("cached_input_tokens")
        .and_then(|value| value.as_u64())
        .map(|value| value as u32);
    let input_tokens =
        raw_input_tokens.map(|raw| raw.saturating_sub(cached_input_tokens.unwrap_or(0)));

    if input_tokens.is_none() && raw_output_tokens.is_none() {
        return None;
    }

    Some(build_usage_event(
        session_id,
        input_tokens,
        raw_output_tokens,
        raw_input_tokens,
        raw_output_tokens,
        cached_input_tokens,
    ))
}

fn parse_response_item_event(session_id: &str, json: &serde_json::Value) -> Option<CliStreamEvent> {
    let payload = json.get("payload")?;
    let item_type = payload.get("type").and_then(|value| value.as_str())?;

    match item_type {
        "reasoning" => Some(build_thinking_start_event(session_id)),
        "message" => {
            if payload.get("role").and_then(|value| value.as_str()) != Some("assistant") {
                return None;
            }

            let text = extract_response_item_message_text(payload)?;
            let phase = payload
                .get("phase")
                .and_then(|value| value.as_str())
                .unwrap_or_default();

            if phase == "commentary" {
                return Some(build_thinking_event(session_id, text));
            }

            Some(build_content_event(session_id, text))
        }
        "function_call" => {
            let tool_name = payload.get("name").and_then(|value| value.as_str())?;
            let tool_call_id = payload
                .get("call_id")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            let tool_input = payload
                .get("arguments")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
                .or_else(|| payload.get("arguments").map(|value| value.to_string()));

            Some(CliStreamEvent {
                event_type: "tool_use".to_string(),
                session_id: session_id.to_string(),
                content: None,
                tool_name: Some(tool_name.to_string()),
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
            })
        }
        "function_call_output" => {
            let tool_call_id = payload
                .get("call_id")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())?;
            let tool_result = payload
                .get("output")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
                .or_else(|| payload.get("output").map(|value| value.to_string()));

            Some(CliStreamEvent {
                event_type: "tool_result".to_string(),
                session_id: session_id.to_string(),
                content: None,
                tool_name: None,
                tool_call_id: Some(tool_call_id),
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
            })
        }
        _ => None,
    }
}

fn parse_codex_json_output(session_id: &str, json: &serde_json::Value) -> Option<CliStreamEvent> {
    let event_type = json
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or_default();

    let event = match event_type {
        "session_meta" => {
            extract_external_session_id(json).map(|external_session_id| CliStreamEvent {
                event_type: "message_start".to_string(),
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
                external_session_id: Some(external_session_id),
                raw_input_tokens: None,
                raw_output_tokens: None,
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
            })
        }
        "event_msg" => parse_event_msg_usage_event(session_id, json),
        "response_item" => parse_response_item_event(session_id, json),
        "system" => extract_runtime_system_notice(json)
            .map(|content| build_system_event(session_id, content)),
        // === Codex CLI 特有事件类型 ===
        "item.started" => {
            let item = json.get("item")?;
            let item_type = item.get("type").and_then(|value| value.as_str())?;

            match item_type {
                "command_execution" => build_command_execution_tool_use_event(session_id, item),
                "todo_list" => build_todo_list_tool_use_event(session_id, item),
                "reasoning" | "thinking" => Some(build_thinking_start_event(session_id)),
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
                "todo_list" => build_todo_list_tool_use_event(session_id, item),
                "reasoning" | "thinking" => {
                    let text = extract_item_text(item)?;
                    Some(build_thinking_event(session_id, text))
                }
                "command_execution" => build_command_execution_tool_result_event(session_id, item),
                _ => None,
            }
        }
        "item.updated" => {
            let item = json.get("item")?;
            let item_type = item.get("type").and_then(|value| value.as_str())?;

            match item_type {
                "todo_list" => build_todo_list_tool_use_event(session_id, item),
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
            let raw_input_tokens = usage
                .and_then(|u| u.get("input_tokens"))
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);
            let raw_output_tokens = usage
                .and_then(|u| u.get("output_tokens"))
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);
            let cached_input = usage
                .and_then(|u| u.get("cached_input_tokens"))
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);

            let input_tokens =
                raw_input_tokens.map(|raw| raw.saturating_sub(cached_input.unwrap_or(0)));

            if input_tokens.is_some() || raw_output_tokens.is_some() {
                Some(build_usage_event(
                    session_id,
                    input_tokens,
                    raw_output_tokens,
                    raw_input_tokens,
                    raw_output_tokens,
                    cached_input,
                ))
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
                "text_delta" => extract_text_value(
                    delta
                        .get("text")
                        .or_else(|| delta.get("content"))
                        .or_else(|| delta.get("message")),
                )
                .map(|text| build_content_event(session_id, text)),
                "thinking_delta" => extract_text_value(
                    delta
                        .get("thinking")
                        .or_else(|| delta.get("summary"))
                        .or_else(|| delta.get("text"))
                        .or_else(|| delta.get("content")),
                )
                .map(|thinking| build_thinking_event(session_id, thinking)),
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
                        external_session_id: None,
                        raw_input_tokens: None,
                        raw_output_tokens: None,
                        cache_read_input_tokens: None,
                        cache_creation_input_tokens: None,
                    })
                }
                "thinking" | "reasoning" => {
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
            let _ = json.get("usage");
            None
        }

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
                    "thinking" | "reasoning" => {
                        // 处理 thinking 类型
                        if let Some(thinking_text) = extract_text_value(
                            content_item
                                .get("thinking")
                                .or_else(|| content_item.get("summary"))
                                .or_else(|| content_item.get("text"))
                                .or_else(|| content_item.get("content"))
                                .or_else(|| content_item.get("message")),
                        ) {
                            log_debug!("[parse] 找到 thinking 内容，长度: {}", thinking_text.len());
                            return Some(CliStreamEvent {
                                event_type: "thinking".to_string(),
                                session_id: session_id.to_string(),
                                content: Some(thinking_text),
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
                    "text" => {
                        let text = extract_text_value(
                            content_item
                                .get("text")
                                .or_else(|| content_item.get("content"))
                                .or_else(|| content_item.get("message")),
                        )?;
                        return Some(build_content_event(session_id, text));
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
                            external_session_id: None,
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

        // === 结果类型 ===
        "result" => extract_structured_payload(json)
            .or_else(|| extract_text_value(json.get("result")))
            .map(|content| build_content_event(session_id, content)),

        // === Codex 上下文截断事件 ===
        "turn_context" => {
            let truncation_mode = json
                .pointer("/truncation_policy/mode")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let summary_mode = json.get("summary").and_then(|v| v.as_str()).unwrap_or("");
            if truncation_mode == "tokens" && summary_mode == "auto" {
                let limit = json
                    .pointer("/truncation_policy/limit")
                    .and_then(|v| v.as_u64());
                let mut content = "### CLI Context Compaction\nTrigger: auto".to_string();
                if let Some(l) = limit {
                    content = format!("{}\nTruncation limit: {} tokens", content, l);
                }
                Some(build_system_event(session_id, content))
            } else {
                None
            }
        }

        // === 默认回退 ===
        _ => {
            if let Some(content) = extract_runtime_system_notice(json) {
                return Some(build_system_event(session_id, content));
            }

            extract_structured_payload(json)
                .or_else(|| extract_turn_output(json))
                .map(|content| build_content_event(session_id, content))
        }
    };

    event.map(|event| attach_external_session_id(event, json))
}

fn extract_item_text(item: &serde_json::Value) -> Option<String> {
    extract_text_value(item.get("thinking"))
        .or_else(|| extract_text_value(item.get("summary")))
        .or_else(|| extract_text_value(item.get("text")))
        .or_else(|| extract_structured_payload(item))
        .or_else(|| extract_text_value(item.get("content")))
        .or_else(|| extract_text_value(item.get("message")))
}

fn extract_response_item_message_text(payload: &serde_json::Value) -> Option<String> {
    let content = payload.get("content")?.as_array()?;
    let parts = content
        .iter()
        .filter_map(|entry| {
            let entry_type = entry
                .get("type")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            match entry_type {
                "output_text" => entry
                    .get("text")
                    .and_then(|value| value.as_str())
                    .map(|value| value.trim().to_string()),
                _ => None,
            }
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if parts.is_empty() {
        return None;
    }

    Some(parts.join("\n\n"))
}

fn extract_structured_payload(value: &serde_json::Value) -> Option<String> {
    let raw = value
        .get("structured_output")
        .or_else(|| value.get("structuredOutput"))
        .or_else(|| value.get("output_struct"))
        .or_else(|| value.get("outputStruct"))?;

    // structured_output 可能是字符串（已序列化的 JSON）或对象
    match raw {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return None;
            }
            // 如果是有效的 JSON 字符串，直接返回（避免二次序列化导致转义）
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                    return Some(trimmed.to_string());
                }
            }
            // 非 JSON 字符串，按原样返回
            Some(trimmed.to_string())
        }
        other => serde_json::to_string(other).ok(),
    }
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
        for key in [
            "thinking", "summary", "text", "content", "message", "value", "title",
        ] {
            if let Some(text) = extract_text_value(value.get(key)) {
                return Some(text);
            }
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

    // 尝试提取结构化输出
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

    // 尝试提取错误
    if let Some(error) = extract_error_from_json_blob(&parsed) {
        log_info!("[parse] 提取到 error: {}", error);
        return Some(attach_external_session_id(
            build_error_event(session_id, error),
            &parsed,
        ));
    }

    // 尝试提取结果内容
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

    // 返回原始 JSON
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

#[cfg(test)]
mod tests {
    use super::{
        load_codex_runtime_auth_env, should_ignore_stderr_line,
        should_treat_process_failure_as_success, StderrReadOutcome, StdoutReadOutcome,
    };
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn ignores_rmcp_transport_closed_warning() {
        let warning = "rmcp::transport::worker: worker quit with fatal: Transport channel closed";
        assert!(should_ignore_stderr_line(warning));
    }

    #[test]
    fn ignores_rmcp_async_rw_serde_warning() {
        let warning =
            "ERROR rmcp::transport::async_rw: Error reading from stream: serde error expected `,` or `]` at line 1 column 6";
        assert!(should_ignore_stderr_line(warning));
    }

    #[test]
    fn ignores_rmcp_process_group_termination_eperm_warning() {
        let warning = "codex_rmcp_client::rmcp_client: Failed to terminate MCP process group 37060: Operation not permitted (os error 1)";
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
    fn does_not_treat_failure_as_success_when_error_was_emitted() {
        let stdout_outcome = StdoutReadOutcome {
            emitted_content: true,
            emitted_error: true,
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

        assert!(!should_treat_process_failure_as_success(
            &stdout_outcome,
            &stderr_outcome,
        ));
    }

    #[test]
    fn loads_string_env_values_from_codex_auth_json() {
        let temp_dir = std::env::temp_dir().join(format!("codex-auth-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let auth_path = temp_dir.join("auth.json");
        fs::write(
            &auth_path,
            r#"{
  "OPENAI_API_KEY": "sk-test",
  "EMPTY_VALUE": "   ",
  "NUMBER_VALUE": 1,
  "CUSTOM_TOKEN": "token-value"
}"#,
        )
        .expect("write auth json");

        let env_map = load_codex_runtime_auth_env(&temp_dir);

        assert_eq!(env_map.get("OPENAI_API_KEY").map(String::as_str), Some("sk-test"));
        assert_eq!(env_map.get("CUSTOM_TOKEN").map(String::as_str), Some("token-value"));
        assert!(!env_map.contains_key("EMPTY_VALUE"));
        assert!(!env_map.contains_key("NUMBER_VALUE"));

        let _ = fs::remove_file(auth_path);
        let _ = fs::remove_dir(temp_dir);
    }
}
