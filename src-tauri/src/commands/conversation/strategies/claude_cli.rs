use std::process::Stdio;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio::process::Command as TokioCommand;

use super::cli_common::{
    build_content_event, build_error_event, emit_cli_event, extract_error_from_json_blob,
    extract_result_content_from_json_blob, extract_structured_output_from_json_blob,
    parse_json_blob_with_fallback, preview_text, shell_escape,
};
use crate::commands::conversation::abort::{
    clear_abort_flag, register_session_pid, set_abort_flag, should_abort, unregister_session_pid,
};
use crate::commands::conversation::strategy::AgentExecutionStrategy;
use crate::commands::conversation::types::{CliStreamEvent, ExecutionRequest};

/// Claude CLI 策略
pub struct ClaudeCliStrategy;

// 简单的日志宏
macro_rules! log_info {
    ($($arg:tt)*) => {
        println!("[INFO][claude-cli] {}", format!($($arg)*))
    };
}

macro_rules! log_error {
    ($($arg:tt)*) => {
        eprintln!("[ERROR][claude-cli] {}", format!($($arg)*))
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
}

impl StdoutReadOutcome {
    fn none() -> Self {
        Self {
            emitted_content: false,
            emitted_error: false,
        }
    }
}

#[async_trait]
impl AgentExecutionStrategy for ClaudeCliStrategy {
    fn supports(&self, agent_type: &str, provider: &str) -> bool {
        agent_type == "cli" && provider == "claude"
    }

    async fn execute(&self, app: AppHandle, request: ExecutionRequest) -> Result<()> {
        let session_id = request.session_id.clone();
        let event_name = format!("claude-stream-{}", session_id);

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
            log_info!("消息[{}]: role={}, content_len={}", i, msg.role, msg.content.len());
        }
        let allowed_tools = request.allowed_tools.clone();
        let cli_output_format = request
            .cli_output_format
            .clone()
            .unwrap_or_else(|| "stream-json".to_string());
        let json_schema = request.json_schema.clone();
        let extra_cli_args = request.extra_cli_args.clone();
        let messages = request.messages.clone();
        let is_stream_json = cli_output_format == "stream-json";
        let schema_text = json_schema
            .as_deref()
            .map(str::trim)
            .filter(|schema| !schema.is_empty());
        let plan_id = request.plan_id.clone();

        // 构建命令参数（prompt 通过 `-p <prompt>` 单独传递）
        let mut args = vec!["--output-format".to_string(), cli_output_format.clone()];
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

        // 添加允许的工具
        if let Some(tools) = &allowed_tools {
            if !tools.is_empty() {
                args.push("--allowedTools".to_string());
                args.push(tools.join(","));
            }
        }

        // MCP 配置暂时禁用，不传递给 Claude CLI
        // if let Some(servers) = &mcp_servers {
        //     if !servers.is_empty() {
        //         let mcp_config = build_mcp_config_json(servers);
        //         log_info!("MCP 配置: {}", mcp_config);
        //         args.push("--mcp-config".to_string());
        //         args.push(mcp_config);
        //     }
        // }

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
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let full_command = build_full_claude_command(&cli_path, &input_text, &args);
        log_info!("Claude CLI command: {}", full_command);
        // 执行命令
        let mut cmd = TokioCommand::new(&cli_path);
        cmd.arg("-p").arg(&input_text).args(&args);

        // 设置工作目录，确保文件读写操作在指定目录下进行
        if let Some(ref work_dir) = resolved_working_dir {
            cmd.current_dir(work_dir);
            log_info!("设置工作目录: {}", work_dir);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_remove("CLAUDECODE");

        let execution_started_at = Instant::now();
        let mut child = cmd.spawn()?;

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

        // 处理标准输出
        let stdout_handle = tokio::spawn(async move {
            if is_stream_json {
                let reader = tokio::io::BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    log_info!("[stdout] 原始行: {}", line);

                    if should_abort(&session_id_clone).await {
                        break;
                    }

                    match serde_json::from_str::<serde_json::Value>(&line) {
                        Ok(json_value) => {
                            let event_type = json_value.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
                            log_info!("[stdout] JSON type: {}", event_type);
                            let event = parse_claude_json_output(&session_id_clone, &json_value);
                            if let Some(event) = event {
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

                return StdoutReadOutcome::none();
            }

            let mut reader = tokio::io::BufReader::new(stdout);
            let mut full_output = String::new();
            if let Err(_) = reader.read_to_string(&mut full_output).await {
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
                emit_cli_event(
                    &app_clone,
                    &event_name_clone,
                    plan_id_clone.as_ref(),
                    &event,
                );
                log_info!("[stdout] 事件发送成功");
                return StdoutReadOutcome {
                    emitted_content: true,
                    emitted_error: false,
                };
            }

            log_info!("[stdout] 无法解析为结构化输出，直接发送原始内容");
            let event = build_content_event(&session_id_clone, normalized.to_string());
            log_info!(
                "[stdout] 发送原始内容事件: {}, event_type: {}",
                event_name_clone,
                event.event_type
            );
            emit_cli_event(
                &app_clone,
                &event_name_clone,
                plan_id_clone.as_ref(),
                &event,
            );
            StdoutReadOutcome {
                emitted_content: true,
                emitted_error: false,
            }
        });

        let session_id_clone = session_id.clone();
        let app_clone = app.clone();
        let event_name_clone = event_name.clone();
        let plan_id_clone = plan_id.clone();

        // 处理标准错误
        let stderr_handle = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut error_output = String::new();
            if let Err(_) = reader.read_to_string(&mut error_output).await {
                return;
            }

            if error_output.is_empty() {
                return;
            }

            log_error!("[stderr] {}", error_output);

            // 检查是否是真正的错误消息
            let error_lines: Vec<&str> = error_output
                .lines()
                .filter(|line| {
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
            }
        });

        // 等待进程完成
        let status = child.wait().await?;
        let elapsed = execution_started_at.elapsed();
        log_info!(
            "CLI 执行完成，退出码: {:?}, 耗时: {:.2}s",
            status.code(),
            elapsed.as_secs_f64()
        );

        // 等待输出处理完成
        let stdout_outcome = stdout_handle.await?;
        stderr_handle.await?;

        // 注销进程 PID
        unregister_session_pid(&session_id).await;

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

        // 清理中断标志
        clear_abort_flag(&session_id).await;

        if !status.success() {
            // 对于非流式 JSON 输出，如果已经输出了内容且没有错误，则视为成功
            if !is_stream_json && stdout_outcome.emitted_content && !stdout_outcome.emitted_error {
                return Ok(());
            }
            return Err(anyhow::anyhow!("CLI 执行失败，退出码: {:?}", status.code()));
        }

        Ok(())
    }
}

fn build_full_claude_command(cli_path: &str, input_text: &str, args: &[String]) -> String {
    let mut cmd_parts = Vec::new();
    cmd_parts.push(shell_escape(cli_path));
    cmd_parts.push("-p".to_string());
    cmd_parts.push(shell_escape(input_text));
    cmd_parts.extend(args.iter().map(|arg| shell_escape(arg)));
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

    if let Some(content) = extract_structured_output_from_json_blob(&parsed) {
        log_info!(
            "[parse] 提取到 structured_output, 长度: {}",
            content.chars().count()
        );
        return Some(build_content_event(session_id, content));
    }

    if let Some(error) = extract_error_from_json_blob(&parsed) {
        log_info!("[parse] 提取到 error: {}", error);
        return Some(build_error_event(session_id, error));
    }

    if let Some(content) = extract_result_content_from_json_blob(&parsed) {
        log_info!(
            "[parse] 提取到 result.content, 长度: {}",
            content.chars().count()
        );
        return Some(build_content_event(session_id, content));
    }

    if let Ok(raw_json) = serde_json::to_string(&parsed) {
        log_info!("[parse] 返回原始 JSON, 长度: {}", raw_json.chars().count());
        return Some(build_content_event(session_id, raw_json));
    }

    log_error!("[parse] 无法提取任何内容");
    None
}

/// 解析 `--output-format stream-json` 的每行 JSON 输出
fn parse_claude_json_output(session_id: &str, json: &serde_json::Value) -> Option<CliStreamEvent> {
    let event_type = json
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");

    match event_type {
        "content_block_delta" => {
            let delta = json.get("delta")?;
            let delta_type = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match delta_type {
                "thinking_delta" => {
                    // 处理思考内容
                    let thinking = delta.get("thinking").and_then(|t| t.as_str())?;
                    Some(CliStreamEvent {
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
                    })
                }
                "text_delta" => {
                    let text = delta.get("text").and_then(|t| t.as_str())?;
                    Some(build_content_event(session_id, text.to_string()))
                }
                "input_json_delta" => {
                    let partial_json = delta.get("partial_json").and_then(|j| j.as_str())?;
                    Some(build_content_event(session_id, partial_json.to_string()))
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
                "thinking" => {
                    // thinking 内容块开始
                    Some(CliStreamEvent {
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
                    })
                }
                "tool_use" => {
                    let tool_name = content_block.get("name").and_then(|n| n.as_str())?;
                    let tool_id = json.get("index").and_then(|i| i.as_u64())?;
                    Some(CliStreamEvent {
                        event_type: "tool_use".to_string(),
                        session_id: session_id.to_string(),
                        content: None,
                        tool_name: Some(tool_name.to_string()),
                        tool_call_id: Some(tool_id.to_string()),
                        tool_input: None,
                        tool_result: None,
                        error: None,
                        input_tokens: None,
                        output_tokens: None,
                        model: None,
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
                })
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
                        if let Some(thinking_text) = content_item.get("thinking").and_then(|t| t.as_str()) {
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
        _ => {
            log_debug!("[parse] 未处理的事件类型: {}", event_type);
            None
        }
    }
}
