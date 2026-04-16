use anyhow::Result;
use async_trait::async_trait;
use base64::Engine;
use tauri::{AppHandle, Emitter};

use crate::commands::conversation::abort::{clear_abort_flag, set_abort_flag, should_abort};
use crate::commands::conversation::strategy::{AgentExecutionStrategy, AgentRuntimeKind};
use crate::commands::conversation::types::{ExecutionRequest, MessageInput, SdkStreamEvent};
use crate::commands::plan_split::{record_plan_split_event, SplitStreamRecord};

/// Codex SDK 策略
/// 基于 OpenAI 兼容 API 格式实现
pub struct CodexSdkStrategy;

fn attachment_to_openai_content(
    attachment: &crate::commands::message::MessageAttachment,
) -> Result<serde_json::Value> {
    let bytes = std::fs::read(&attachment.path)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let data_url = format!("data:{};base64,{}", attachment.mime_type, encoded);

    Ok(serde_json::json!({
        "type": "image_url",
        "image_url": {
            "url": data_url
        }
    }))
}

fn build_openai_message_content(message: &MessageInput) -> Result<serde_json::Value> {
    let mut content = Vec::new();

    if !message.content.trim().is_empty() {
        content.push(serde_json::json!({
            "type": "text",
            "text": message.content
        }));
    }

    if let Some(attachments) = &message.attachments {
        for attachment in attachments {
            content.push(attachment_to_openai_content(attachment)?);
        }
    }

    if content.is_empty() {
        Ok(serde_json::json!(message.content))
    } else {
        Ok(serde_json::json!(content))
    }
}

fn emit_sdk_event(
    app: &AppHandle,
    event_name: &str,
    plan_id: Option<&String>,
    event: &SdkStreamEvent,
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
                model: event.model.clone(),
            },
        );
    }
}

// 简单的日志宏
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            crate::logging::write_log("INFO", "codex-sdk", &message);
            println!("[INFO][codex-sdk] {}", message)
        }
    };
}

macro_rules! log_error {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            crate::logging::write_log("ERROR", "codex-sdk", &message);
            eprintln!("[ERROR][codex-sdk] {}", message)
        }
    };
}

#[async_trait]
impl AgentExecutionStrategy for CodexSdkStrategy {
    fn kind(&self) -> AgentRuntimeKind {
        AgentRuntimeKind::CodexSdk
    }

    async fn execute(&self, app: AppHandle, request: ExecutionRequest) -> Result<()> {
        let session_id = request.session_id.clone();
        let event_name = self.kind().event_name(&session_id);
        let plan_id = request.plan_id.clone();

        log_info!("开始执行 Codex SDK, session_id: {}", session_id);

        // 重置中断标志
        set_abort_flag(&session_id, false).await;

        // 构建请求 URL
        let base_url = request
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let url = format!("{}/chat/completions", base_url);

        // 构建 OpenAI 兼容格式的消息数组
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // 添加系统提示
        if let Some(system) = &request.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }

        // 添加对话消息
        for m in &request.messages {
            if m.role != "system" {
                messages.push(serde_json::json!({
                    "role": m.role,
                    "content": build_openai_message_content(m)?
                }));
            }
        }

        // 构建请求体
        let mut body = serde_json::json!({
            "model": request.model_id,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true
        });

        // 添加工具定义（如果有的话）
        if let Some(tools) = &request.tools {
            body["tools"] = serde_json::json!(tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema
                        }
                    })
                })
                .collect::<Vec<_>>());
        }

        log_info!("请求 URL: {}", url);
        log_info!("请求模型: {:?}", request.model_id);

        // 发送请求
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", request.api_key.clone().unwrap_or_default()),
            )
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("API 请求失败: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log_error!("API 错误: {}", error_text);
            return Err(anyhow::anyhow!("API 错误: {}", error_text));
        }

        // 处理 SSE 流
        let session_id_clone = session_id.clone();
        let app_clone = app.clone();
        let event_name_clone = event_name.clone();
        let plan_id_clone = plan_id.clone();

        tokio::spawn(async move {
            use futures::StreamExt;

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                if should_abort(&session_id_clone).await {
                    log_info!("检测到中断信号，停止处理");
                    break;
                }

                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        log_error!("读取流失败: {}", e);
                        let event = SdkStreamEvent {
                            event_type: "error".to_string(),
                            session_id: session_id_clone.clone(),
                            content: None,
                            tool_name: None,
                            tool_call_id: None,
                            tool_input: None,
                            tool_result: None,
                            error: Some(e.to_string()),
                            input_tokens: None,
                            output_tokens: None,
                            model: None,
                            external_session_id: None,
                        };
                        emit_sdk_event(
                            &app_clone,
                            &event_name_clone,
                            plan_id_clone.as_ref(),
                            &event,
                        );
                        break;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(chunk.as_ref()));

                // 解析 SSE 事件
                while let Some(pos) = buffer.find("\n\n") {
                    let event_str = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    if let Some(event) = parse_openai_sse_event(&session_id_clone, &event_str) {
                        emit_sdk_event(
                            &app_clone,
                            &event_name_clone,
                            plan_id_clone.as_ref(),
                            &event,
                        );
                    }
                }
            }

            // 发送完成事件
            log_info!("发送完成事件");
            let done_event = SdkStreamEvent {
                event_type: "done".to_string(),
                session_id: session_id_clone.clone(),
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
            emit_sdk_event(
                &app_clone,
                &event_name_clone,
                plan_id_clone.as_ref(),
                &done_event,
            );

            // 清理中断标志
            clear_abort_flag(&session_id_clone).await;
        });

        Ok(())
    }
}

/// 解析 OpenAI SSE 事件
fn parse_openai_sse_event(session_id: &str, event_str: &str) -> Option<SdkStreamEvent> {
    for line in event_str.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            // 检查是否是结束标记
            if data == "[DONE]" {
                return Some(SdkStreamEvent {
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
                    external_session_id: None,
                });
            }

            // 解析 JSON 数据
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                return parse_openai_stream_event(session_id, &json);
            }
        }
    }
    None
}

/// 解析 OpenAI 流式事件
fn parse_openai_stream_event(session_id: &str, json: &serde_json::Value) -> Option<SdkStreamEvent> {
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
                        "reasoning",
                        "thinking",
                        "summary",
                        "text",
                        "content",
                        "message",
                        "value",
                        "title",
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

    let choices = json.get("choices")?.as_array()?;
    let first_choice = choices.first()?;

    let delta = first_choice.get("delta")?;
    let finish_reason = first_choice.get("finish_reason").and_then(|f| f.as_str());

    // 检查是否结束
    if finish_reason == Some("stop") || finish_reason == Some("tool_calls") {
        return Some(SdkStreamEvent {
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
            external_session_id: None,
        });
    }

    // 处理文本内容
    if let Some(content) = extract_textish_value(delta.get("content")) {
        if !content.is_empty() {
            // 尝试获取 token 使用量
            let usage = json.get("usage");
            let input_tokens = usage
                .and_then(|u| u.get("prompt_tokens"))
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);
            let output_tokens = usage
                .and_then(|u| u.get("completion_tokens"))
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);
            let model = json
                .get("model")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());

            return Some(SdkStreamEvent {
                event_type: "content".to_string(),
                session_id: session_id.to_string(),
                content: Some(content),
                tool_name: None,
                tool_call_id: None,
                tool_input: None,
                tool_result: None,
                error: None,
                input_tokens,
                output_tokens,
                model,
                external_session_id: None,
            });
        }
    }

    if let Some(thinking) = extract_textish_value(
        delta.get("reasoning")
            .or_else(|| delta.get("reasoning_content"))
            .or_else(|| delta.get("thinking"))
            .or_else(|| delta.get("summary")),
    ) {
        let usage = json.get("usage");
        let input_tokens = usage
            .and_then(|u| u.get("prompt_tokens"))
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);
        let output_tokens = usage
            .and_then(|u| u.get("completion_tokens"))
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);
        let model = json
            .get("model")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());

        return Some(SdkStreamEvent {
            event_type: "thinking".to_string(),
            session_id: session_id.to_string(),
            content: Some(thinking),
            tool_name: None,
            tool_call_id: None,
            tool_input: None,
            tool_result: None,
            error: None,
            input_tokens,
            output_tokens,
            model,
            external_session_id: None,
        });
    }

    // 处理工具调用
    if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
        if let Some(tool_call) = tool_calls.first() {
            let tool_id = tool_call
                .get("id")
                .and_then(|i| i.as_str())
                .map(|s| s.to_string());
            let function = tool_call.get("function")?;

            let tool_name = function
                .get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());
            let tool_args = function
                .get("arguments")
                .and_then(|a| a.as_str())
                .map(|s| s.to_string());

            // 如果有 ID，表示工具调用开始
            if tool_id.is_some() {
                return Some(SdkStreamEvent {
                    event_type: "tool_use".to_string(),
                    session_id: session_id.to_string(),
                    content: None,
                    tool_name,
                    tool_call_id: tool_id,
                    tool_input: tool_args,
                    tool_result: None,
                    error: None,
                    input_tokens: None,
                    output_tokens: None,
                    model: None,
                    external_session_id: None,
                });
            }
            // 否则是工具参数增量
            else if tool_args.is_some() {
                let index = tool_call
                    .get("index")
                    .and_then(|i| i.as_u64())
                    .map(|i| i.to_string());
                return Some(SdkStreamEvent {
                    event_type: "tool_input_delta".to_string(),
                    session_id: session_id.to_string(),
                    content: None,
                    tool_name,
                    tool_call_id: index,
                    tool_input: tool_args,
                    tool_result: None,
                    error: None,
                    input_tokens: None,
                    output_tokens: None,
                    model: None,
                    external_session_id: None,
                });
            }
        }
    }

    None
}
