use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::support::{now_rfc3339, open_db_connection};

const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const SESSION_UPLOADS_DIR: &str = "session-uploads";

/// 工具调用数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String, // JSON string
    pub status: String,
    pub result: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageAttachment {
    pub id: String,
    pub name: String,
    pub path: String,
    pub mime_type: String,
    pub size: usize,
}

/// 消息数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub attachments: Option<Vec<MessageAttachment>>,
    pub status: String,
    pub tokens: Option<i32>,
    pub error_message: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub thinking: Option<String>,
    pub edit_traces: Option<String>,
    pub runtime_notices: Option<String>,
    pub compression_metadata: Option<String>,
    pub created_at: String,
}

/// 创建消息输入
#[derive(Debug, Deserialize)]
pub struct CreateMessageInput {
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub attachments: Option<String>,
    pub status: Option<String>,
    pub tokens: Option<i32>,
    pub error_message: Option<String>,
    pub tool_calls: Option<String>, // JSON string
    pub thinking: Option<String>,
    pub edit_traces: Option<String>,
    pub runtime_notices: Option<String>,
    pub compression_metadata: Option<String>,
}

/// 更新消息输入
#[derive(Debug, Deserialize)]
pub struct UpdateMessageInput {
    pub content: Option<String>,
    pub attachments: Option<String>,
    pub status: Option<String>,
    pub tokens: Option<i32>,
    pub error_message: Option<String>,
    pub tool_calls: Option<String>, // JSON string
    pub thinking: Option<String>,
    pub edit_traces: Option<String>,
    pub runtime_notices: Option<String>,
    pub compression_metadata: Option<String>,
}

fn build_message_updates(input: &UpdateMessageInput) -> Vec<String> {
    let mut updates: Vec<String> = vec![];
    let mut param_index = 1;

    if input.content.is_some() {
        updates.push(format!("content = ?{}", param_index));
        param_index += 1;
    }
    if input.attachments.is_some() {
        updates.push(format!("attachments = ?{}", param_index));
        param_index += 1;
    }
    if input.status.is_some() {
        updates.push(format!("status = ?{}", param_index));
        param_index += 1;
    }
    if input.tokens.is_some() {
        updates.push(format!("tokens = ?{}", param_index));
        param_index += 1;
    }
    if input.error_message.is_some() {
        updates.push(format!("error_message = ?{}", param_index));
        param_index += 1;
    }
    if input.tool_calls.is_some() {
        updates.push(format!("tool_calls = ?{}", param_index));
        param_index += 1;
    }
    if input.thinking.is_some() {
        updates.push(format!("thinking = ?{}", param_index));
        param_index += 1;
    }
    if input.edit_traces.is_some() {
        updates.push(format!("edit_traces = ?{}", param_index));
        param_index += 1;
    }
    if input.runtime_notices.is_some() {
        updates.push(format!("runtime_notices = ?{}", param_index));
        param_index += 1;
    }
    if input.compression_metadata.is_some() {
        updates.push(format!("compression_metadata = ?{}", param_index));
    }

    updates
}

fn bind_message_update_parameters(
    stmt: &mut rusqlite::CachedStatement<'_>,
    input: &UpdateMessageInput,
) -> Result<usize, String> {
    let mut param_count = 1;
    if let Some(ref content) = input.content {
        stmt.raw_bind_parameter(param_count, content)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref attachments) = input.attachments {
        stmt.raw_bind_parameter(param_count, attachments)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref status) = input.status {
        stmt.raw_bind_parameter(param_count, status)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(tokens) = input.tokens {
        stmt.raw_bind_parameter(param_count, tokens)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref error_message) = input.error_message {
        stmt.raw_bind_parameter(param_count, error_message)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref tool_calls) = input.tool_calls {
        stmt.raw_bind_parameter(param_count, tool_calls)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref thinking) = input.thinking {
        stmt.raw_bind_parameter(param_count, thinking)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref edit_traces) = input.edit_traces {
        stmt.raw_bind_parameter(param_count, edit_traces)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref runtime_notices) = input.runtime_notices {
        stmt.raw_bind_parameter(param_count, runtime_notices)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref compression_metadata) = input.compression_metadata {
        stmt.raw_bind_parameter(param_count, compression_metadata)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }

    Ok(param_count)
}

/// 分页消息结果
#[derive(Debug, Serialize)]
pub struct PaginatedMessages {
    pub messages: Vec<Message>,
    pub total: usize,
    pub has_more: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadImageInput {
    pub file_name: Option<String>,
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadSessionImagesResponse {
    pub attachments: Vec<MessageAttachment>,
}

fn session_uploads_root() -> Result<PathBuf, String> {
    super::get_persistence_dir_path()
        .map(|path| path.join("data").join(SESSION_UPLOADS_DIR))
        .map_err(|e| e.to_string())
}

fn session_upload_dir(session_id: &str) -> Result<PathBuf, String> {
    Ok(session_uploads_root()?.join(session_id))
}

fn sanitize_file_name(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|char| match char {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => char,
            _ => '_',
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if sanitized.is_empty() {
        "image".to_string()
    } else {
        sanitized
    }
}

fn extension_from_mime_type(mime_type: &str) -> &'static str {
    match mime_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/svg+xml" => "svg",
        _ => "png",
    }
}

fn parse_tool_calls(tool_calls_json: Option<String>) -> Option<Vec<ToolCall>> {
    let raw_json = tool_calls_json?;
    let value = serde_json::from_str::<serde_json::Value>(&raw_json).ok()?;
    let items = value.as_array()?;
    let mut tool_calls = Vec::with_capacity(items.len());

    for item in items {
        let object = item.as_object()?;
        let arguments = match object.get("arguments") {
            Some(serde_json::Value::String(text)) => text.clone(),
            Some(value) => serde_json::to_string(value).ok()?,
            None => "{}".to_string(),
        };

        tool_calls.push(ToolCall {
            id: object.get("id")?.as_str()?.to_string(),
            name: object.get("name")?.as_str()?.to_string(),
            arguments,
            status: object.get("status")?.as_str()?.to_string(),
            result: object
                .get("result")
                .and_then(|value| value.as_str().map(ToString::to_string)),
            error_message: object
                .get("errorMessage")
                .or_else(|| object.get("error_message"))
                .and_then(|value| value.as_str().map(ToString::to_string)),
        });
    }

    Some(tool_calls)
}

fn parse_attachments(attachments_json: Option<String>) -> Option<Vec<MessageAttachment>> {
    attachments_json.and_then(|json| serde_json::from_str(&json).ok())
}

fn uploads_root_contains(path: &Path) -> Result<bool, String> {
    let root = session_uploads_root()?;
    Ok(path.starts_with(root))
}

fn map_message_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    let attachments_json: Option<String> = row.get(4)?;
    let tool_calls_json: Option<String> = row.get(8)?;

    Ok(Message {
        id: row.get(0)?,
        session_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        attachments: parse_attachments(attachments_json),
        status: row.get(5)?,
        tokens: row.get(6)?,
        error_message: row.get(7)?,
        tool_calls: parse_tool_calls(tool_calls_json),
        thinking: row.get(9)?,
        edit_traces: row.get(10)?,
        runtime_notices: row.get(11)?,
        compression_metadata: row.get(12)?,
        created_at: row.get(13)?,
    })
}

fn remove_attachment_files(attachments: &[MessageAttachment]) -> Result<(), String> {
    for attachment in attachments {
        let path = PathBuf::from(&attachment.path);
        if !uploads_root_contains(&path)? {
            continue;
        }

        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub(crate) fn remove_session_uploads(session_id: &str) -> Result<(), String> {
    let directory = session_upload_dir(session_id)?;
    if directory.exists() {
        fs::remove_dir_all(directory).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn ensure_session_upload_path(session_id: &str, path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(path);
    let session_dir = session_upload_dir(session_id)?;

    if !candidate.starts_with(&session_dir) {
        return Err("非法的图片路径".to_string());
    }

    Ok(candidate)
}

/// 获取指定会话的消息（支持分页）
#[tauri::command]
pub fn list_messages(
    session_id: String,
    limit: Option<usize>,
    before: Option<String>,
) -> Result<PaginatedMessages, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    // 获取总数
    let total: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
            [&session_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // 默认每页 20 条
    let page_limit = limit.unwrap_or(20);

    // 构建查询语句
    let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(before_time) = before {
        (
            r#"
                SELECT id, session_id, role, content, attachments, status, tokens, error_message, tool_calls, thinking, edit_traces, runtime_notices, compression_metadata, created_at
                FROM messages
                WHERE session_id = ?1 AND created_at < ?2
                ORDER BY created_at DESC
                LIMIT ?3
                "#
            .to_string(),
            vec![
                Box::new(session_id.clone()),
                Box::new(before_time),
                Box::new(page_limit as i32),
            ],
        )
    } else {
        (
            r#"
                SELECT id, session_id, role, content, attachments, status, tokens, error_message, tool_calls, thinking, edit_traces, runtime_notices, compression_metadata, created_at
                FROM messages
                WHERE session_id = ?1
                ORDER BY created_at DESC
                LIMIT ?2
                "#
            .to_string(),
            vec![Box::new(session_id.clone()), Box::new(page_limit as i32)],
        )
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let messages = stmt
        .query_map(params_refs.as_slice(), map_message_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 反转顺序，使消息按时间正序显示（旧消息在前）
    let mut messages = messages;
    messages.reverse();

    let has_more = messages.len() < total;

    Ok(PaginatedMessages {
        messages,
        total,
        has_more,
    })
}

/// 创建新消息
#[tauri::command]
pub fn create_message(input: CreateMessageInput) -> Result<Message, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let status = input.status.unwrap_or_else(|| "completed".to_string());
    let attachments = parse_attachments(input.attachments.clone());
    let tool_calls = parse_tool_calls(input.tool_calls.clone());

    conn.execute(
        "INSERT INTO messages (id, session_id, role, content, attachments, status, tokens, error_message, tool_calls, thinking, edit_traces, runtime_notices, compression_metadata, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        rusqlite::params![
            &id,
            &input.session_id,
            &input.role,
            &input.content,
            &input.attachments,
            &status,
            &input.tokens,
            &input.error_message,
            &input.tool_calls,
            &input.thinking,
            &input.edit_traces,
            &input.runtime_notices,
            &input.compression_metadata,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    // 更新会话的 updated_at 时间
    conn.execute(
        "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &input.session_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(Message {
        id,
        session_id: input.session_id,
        role: input.role,
        content: input.content,
        attachments,
        status,
        tokens: input.tokens,
        error_message: input.error_message,
        tool_calls,
        thinking: input.thinking,
        edit_traces: input.edit_traces,
        runtime_notices: input.runtime_notices,
        compression_metadata: input.compression_metadata,
        created_at: now,
    })
}

/// 更新消息
#[tauri::command]
pub fn update_message(id: String, input: UpdateMessageInput) -> Result<Message, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let updates = build_message_updates(&input);

    if updates.is_empty() {
        // 没有更新内容，直接返回当前消息
        return get_message_by_id(&conn, &id);
    }

    let sql = format!(
        "UPDATE messages SET {} WHERE id = ?{}",
        updates.join(", "),
        updates.len() + 1
    );

    let mut stmt = conn.prepare_cached(&sql).map_err(|e| e.to_string())?;

    let param_count = bind_message_update_parameters(&mut stmt, &input)?;

    stmt.raw_bind_parameter(param_count, &id)
        .map_err(|e| e.to_string())?;

    stmt.raw_execute().map_err(|e| e.to_string())?;

    // 获取更新后的消息
    let message = get_message_by_id(&conn, &id)?;

    Ok(message)
}

#[tauri::command]
pub fn update_message_fields(id: String, input: UpdateMessageInput) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let updates = build_message_updates(&input);

    if updates.is_empty() {
        return Ok(());
    }

    let sql = format!(
        "UPDATE messages SET {} WHERE id = ?{}",
        updates.join(", "),
        updates.len() + 1
    );

    let mut stmt = conn.prepare_cached(&sql).map_err(|e| e.to_string())?;
    let param_count = bind_message_update_parameters(&mut stmt, &input)?;
    stmt.raw_bind_parameter(param_count, &id)
        .map_err(|e| e.to_string())?;
    stmt.raw_execute().map_err(|e| e.to_string())?;
    Ok(())
}

/// 获取单个消息
fn get_message_by_id(conn: &Connection, id: &str) -> Result<Message, String> {
    let message = conn
        .query_row(
            "SELECT id, session_id, role, content, attachments, status, tokens, error_message, tool_calls, thinking, edit_traces, runtime_notices, compression_metadata, created_at FROM messages WHERE id = ?1",
            [id],
            map_message_row,
        )
        .map_err(|e| e.to_string())?;

    Ok(message)
}

/// 删除消息
#[tauri::command]
pub fn delete_message(id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let attachments_json: Option<String> = conn
        .query_row(
            "SELECT attachments FROM messages WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )
        .ok();

    if let Some(attachments) = parse_attachments(attachments_json) {
        remove_attachment_files(&attachments)?;
    }

    conn.execute("DELETE FROM messages WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 删除会话的所有消息
#[tauri::command]
pub fn clear_session_messages(session_id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM messages WHERE session_id = ?1", [&session_id])
        .map_err(|e| e.to_string())?;
    remove_session_uploads(&session_id)?;

    Ok(())
}

#[tauri::command]
pub fn upload_session_images(
    session_id: String,
    files: Vec<UploadImageInput>,
) -> Result<UploadSessionImagesResponse, String> {
    if files.is_empty() {
        return Ok(UploadSessionImagesResponse {
            attachments: Vec::new(),
        });
    }

    let session_dir = session_upload_dir(&session_id)?;
    fs::create_dir_all(&session_dir).map_err(|e| e.to_string())?;

    let mut attachments = Vec::with_capacity(files.len());

    for file in files {
        if !file.mime_type.starts_with("image/") {
            return Err(format!("不支持的文件类型: {}", file.mime_type));
        }

        if file.bytes.is_empty() {
            return Err("图片内容为空".to_string());
        }

        if file.bytes.len() > MAX_IMAGE_BYTES {
            return Err(format!(
                "图片超过大小限制: {}",
                file.file_name.unwrap_or_default()
            ));
        }

        let attachment_id = Uuid::new_v4().to_string();
        let original_name = file
            .file_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("image.{}", extension_from_mime_type(&file.mime_type)));
        let safe_name = sanitize_file_name(&original_name);
        let file_path = session_dir.join(format!("{}-{}", attachment_id, safe_name));

        fs::write(&file_path, &file.bytes).map_err(|e| e.to_string())?;

        attachments.push(MessageAttachment {
            id: attachment_id,
            name: original_name,
            path: file_path.to_string_lossy().to_string(),
            mime_type: file.mime_type,
            size: file.bytes.len(),
        });
    }

    Ok(UploadSessionImagesResponse { attachments })
}

#[tauri::command]
pub fn delete_uploaded_image(session_id: String, path: String) -> Result<(), String> {
    let file_path = ensure_session_upload_path(&session_id, &path)?;
    if file_path.exists() {
        fs::remove_file(&file_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}
