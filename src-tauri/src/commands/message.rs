use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::support::{now_rfc3339, open_db_connection};

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

/// 消息数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub status: String,
    pub tokens: Option<i32>,
    pub error_message: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub thinking: Option<String>,
    pub compression_metadata: Option<String>,
    pub created_at: String,
}

/// 创建消息输入
#[derive(Debug, Deserialize)]
pub struct CreateMessageInput {
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub status: Option<String>,
    pub tokens: Option<i32>,
    pub error_message: Option<String>,
    pub tool_calls: Option<String>, // JSON string
    pub thinking: Option<String>,
    pub compression_metadata: Option<String>,
}

/// 更新消息输入
#[derive(Debug, Deserialize)]
pub struct UpdateMessageInput {
    pub content: Option<String>,
    pub status: Option<String>,
    pub tokens: Option<i32>,
    pub error_message: Option<String>,
    pub tool_calls: Option<String>, // JSON string
    pub thinking: Option<String>,
    pub compression_metadata: Option<String>,
}

/// 分页消息结果
#[derive(Debug, Serialize)]
pub struct PaginatedMessages {
    pub messages: Vec<Message>,
    pub total: usize,
    pub has_more: bool,
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
            format!(
                r#"
                SELECT id, session_id, role, content, status, tokens, error_message, tool_calls, thinking, compression_metadata, created_at
                FROM messages
                WHERE session_id = ?1 AND created_at < ?2
                ORDER BY created_at DESC
                LIMIT ?3
                "#
            ),
            vec![
                Box::new(session_id.clone()),
                Box::new(before_time),
                Box::new(page_limit as i32),
            ],
        )
    } else {
        (
            format!(
                r#"
                SELECT id, session_id, role, content, status, tokens, error_message, tool_calls, thinking, compression_metadata, created_at
                FROM messages
                WHERE session_id = ?1
                ORDER BY created_at DESC
                LIMIT ?2
                "#
            ),
            vec![Box::new(session_id.clone()), Box::new(page_limit as i32)],
        )
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let messages = stmt
        .query_map(params_refs.as_slice(), |row| {
            // 解析 tool_calls JSON
            let tool_calls_json: Option<String> = row.get(7)?;
            let tool_calls: Option<Vec<ToolCall>> =
                tool_calls_json.and_then(|json| serde_json::from_str(&json).ok());

            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                status: row.get(4)?,
                tokens: row.get(5)?,
                error_message: row.get(6)?,
                tool_calls,
                thinking: row.get(8)?,
                compression_metadata: row.get(9)?,
                created_at: row.get(10)?,
            })
        })
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

    // 解析 tool_calls JSON
    let tool_calls: Option<Vec<ToolCall>> = input
        .tool_calls
        .as_ref()
        .and_then(|json| serde_json::from_str(json).ok());

    conn.execute(
        "INSERT INTO messages (id, session_id, role, content, status, tokens, error_message, tool_calls, thinking, compression_metadata, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            &id,
            &input.session_id,
            &input.role,
            &input.content,
            &status,
            &input.tokens,
            &input.error_message,
            &input.tool_calls,
            &input.thinking,
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
        status,
        tokens: input.tokens,
        error_message: input.error_message,
        tool_calls,
        thinking: input.thinking,
        compression_metadata: input.compression_metadata,
        created_at: now,
    })
}

/// 更新消息
#[tauri::command]
pub fn update_message(id: String, input: UpdateMessageInput) -> Result<Message, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    // 构建动态更新语句
    let mut updates: Vec<String> = vec![];
    let mut param_index = 1;

    if input.content.is_some() {
        updates.push(format!("content = ?{}", param_index));
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
    if input.compression_metadata.is_some() {
        updates.push(format!("compression_metadata = ?{}", param_index));
        param_index += 1;
    }

    if updates.is_empty() {
        // 没有更新内容，直接返回当前消息
        return get_message_by_id(&conn, &id);
    }

    let sql = format!(
        "UPDATE messages SET {} WHERE id = ?{}",
        updates.join(", "),
        param_index
    );

    let mut stmt = conn.prepare_cached(&sql).map_err(|e| e.to_string())?;

    // 绑定参数
    let mut param_count = 1;
    if let Some(ref content) = input.content {
        stmt.raw_bind_parameter(param_count, content)
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
    if let Some(ref compression_metadata) = input.compression_metadata {
        stmt.raw_bind_parameter(param_count, compression_metadata)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }

    stmt.raw_bind_parameter(param_count, &id)
        .map_err(|e| e.to_string())?;

    stmt.raw_execute().map_err(|e| e.to_string())?;

    // 获取更新后的消息
    let message = get_message_by_id(&conn, &id)?;

    Ok(message)
}

/// 获取单个消息
fn get_message_by_id(conn: &Connection, id: &str) -> Result<Message, String> {
    let message = conn
        .query_row(
            "SELECT id, session_id, role, content, status, tokens, error_message, tool_calls, thinking, compression_metadata, created_at FROM messages WHERE id = ?1",
            [id],
            |row| {
                // 解析 tool_calls JSON
                let tool_calls_json: Option<String> = row.get(7)?;
                let tool_calls: Option<Vec<ToolCall>> = tool_calls_json
                    .and_then(|json| serde_json::from_str(&json).ok());

                Ok(Message {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    status: row.get(4)?,
                    tokens: row.get(5)?,
                    error_message: row.get(6)?,
                    tool_calls,
                    thinking: row.get(8)?,
                    compression_metadata: row.get(9)?,
                    created_at: row.get(10)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(message)
}

/// 删除消息
#[tauri::command]
pub fn delete_message(id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

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

    Ok(())
}
