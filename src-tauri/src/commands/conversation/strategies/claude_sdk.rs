use anyhow::Result;
use async_trait::async_trait;
use base64::Engine;
use tauri::{AppHandle, Emitter};

use crate::commands::conversation::abort::{clear_abort_flag, set_abort_flag, should_abort};
use crate::commands::conversation::strategy::{AgentExecutionStrategy, AgentRuntimeKind};
use crate::commands::conversation::types::{ExecutionRequest, MessageInput, SdkStreamEvent};
use crate::commands::plan_split::{record_plan_split_event, SplitStreamRecord};

/// Claude SDK 策略
pub struct ClaudeSdkStrategy;

fn attachment_to_anthropic_content(
    attachment: &crate::commands::message::MessageAttachment,
) -> Result<serde_json::Value> {
    let bytes = std::fs::read(&attachment.path)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);

    Ok(serde_json::json!({
        "type": "image",
        "source": {
            "type": "base64",
            "media_type": attachment.mime_type,
            "data": encoded
        }
    }))
}

fn build_anthropic_message_content(message: &MessageInput) -> Result<serde_json::Value> {
    let mut content = Vec::new();

    if let Some(attachments) = &message.attachments {
        for attachment in attachments {
            if !attachment.mime_type.starts_with("image/") {
                continue;
            }
            content.push(attachment_to_anthropic_content(attachment)?);
        }
    }

    if !message.content.trim().is_empty() {
        content.push(serde_json::json!({
            "type": "text",
            "text": message.content
        }));
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
                external_session_id: event.external_session_id.clone(),
            },
        );
    }
}

// 简单的日志宏
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            crate::logging::write_log("INFO", "claude-sdk", &message);
            println!("[INFO][claude-sdk] {}", message)
        }
    };
}

#[async_trait]
impl AgentExecutionStrategy for ClaudeSdkStrategy {
    fn kind(&self) -> AgentRuntimeKind {
        AgentRuntimeKind::ClaudeSdk
    }

    async fn execute(&self, app: AppHandle, request: ExecutionRequest) -> Result<()> {
        let session_id = request.session_id.clone();
        let event_name = self.kind().event_name(&session_id);
        let plan_id = request.plan_id.clone();

        log_info!("开始执行 Claude SDK, session_id: {}", session_id);

        // 重置中断标志
        set_abort_flag(&session_id, false).await;

        // 构建请求
        let base_url = request
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com".to_string());
        let url = format!("{}/v1/messages", base_url);

        // 构建消息数组
        let mut messages: Vec<serde_json::Value> = Vec::new();
        for message in request
            .messages
            .iter()
            .filter(|message| message.role != "system")
        {
            messages.push(serde_json::json!({
                "role": message.role,
                "content": build_anthropic_message_content(message)?
            }));
        }

        let mut body = serde_json::json!({
            "model": request.model_id,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true
        });

        if let Some(system) = &request.system_prompt {
            body["system"] = serde_json::json!(system);
        }

        // 发送请求
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("x-api-key", request.api_key.clone().unwrap_or_default())
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("API 请求失败: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
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
                    break;
                }

                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
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

                    if let Some(event) = parse_sse_event(&session_id_clone, &event_str) {
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

/// 解析 SSE 事件
fn parse_sse_event(session_id: &str, event_str: &str) -> Option<SdkStreamEvent> {
    for line in event_str.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
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

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                return parse_anthropic_stream_event(session_id, &json);
            }
        }
    }
    None
}

/// 解析 Anthropic 流式事件
fn parse_anthropic_stream_event(
    session_id: &str,
    json: &serde_json::Value,
) -> Option<SdkStreamEvent> {
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

    let event_type = json.get("type")?.as_str()?;

    match event_type {
        "content_block_delta" => {
            let delta = json.get("delta")?;
            if let Some(text) = extract_textish_value(
                delta
                    .get("text")
                    .or_else(|| delta.get("content"))
                    .or_else(|| delta.get("message")),
            ) {
                Some(SdkStreamEvent {
                    event_type: "content".to_string(),
                    session_id: session_id.to_string(),
                    content: Some(text),
                    tool_name: None,
                    tool_call_id: None,
                    tool_input: None,
                    tool_result: None,
                    error: None,
                    input_tokens: None,
                    output_tokens: None,
                    model: None,
                    external_session_id: None,
                })
            } else {
                extract_textish_value(
                    delta
                        .get("thinking")
                        .or_else(|| delta.get("summary"))
                        .or_else(|| delta.get("text"))
                        .or_else(|| delta.get("content")),
                )
                .map(|thinking| SdkStreamEvent {
                    event_type: "thinking".to_string(),
                    session_id: session_id.to_string(),
                    content: Some(thinking),
                    tool_name: None,
                    tool_call_id: None,
                    tool_input: None,
                    tool_result: None,
                    error: None,
                    input_tokens: None,
                    output_tokens: None,
                    model: None,
                    external_session_id: None,
                })
            }
        }
        "content_block_start" => {
            let content_block = json.get("content_block")?;
            let block_type = content_block.get("type")?.as_str()?;

            match block_type {
                "thinking" | "reasoning" => Some(SdkStreamEvent {
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
                }),
                "tool_use" => Some(SdkStreamEvent {
                    event_type: "tool_use".to_string(),
                    session_id: session_id.to_string(),
                    content: None,
                    tool_name: content_block
                        .get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string()),
                    tool_call_id: content_block
                        .get("id")
                        .and_then(|i| i.as_str())
                        .map(|s| s.to_string()),
                    tool_input: None,
                    tool_result: None,
                    error: None,
                    input_tokens: None,
                    output_tokens: None,
                    model: None,
                    external_session_id: None,
                }),
                _ => None,
            }
        }
        "content_block_stop" => None,
        "message_delta" => None,
        "message_stop" => Some(SdkStreamEvent {
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
        }),
        "error" => Some(SdkStreamEvent {
            event_type: "error".to_string(),
            session_id: session_id.to_string(),
            content: None,
            tool_name: None,
            tool_call_id: None,
            tool_input: None,
            tool_result: None,
            error: json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string()),
            input_tokens: None,
            output_tokens: None,
            model: None,
            external_session_id: None,
        }),
        _ => None,
    }
}
