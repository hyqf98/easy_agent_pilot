use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::message::remove_session_uploads;
use super::support::{now_rfc3339, open_db_connection, open_db_connection_with_foreign_keys};

/// 会话数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub agent_type: String,
    pub status: String,
    pub pinned: bool,
    pub last_message: Option<String>,
    pub error_message: Option<String>,
    pub message_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建会话输入
#[derive(Debug, Deserialize)]
pub struct CreateSessionInput {
    pub project_id: String,
    pub name: Option<String>,
    pub agent_type: String,
    pub status: Option<String>,
}

/// 更新会话输入
#[derive(Debug, Deserialize)]
pub struct UpdateSessionInput {
    pub name: Option<String>,
    pub status: Option<String>,
    pub pinned: Option<bool>,
    pub last_message: Option<String>,
    pub error_message: Option<String>,
    pub agent_type: Option<String>,
}

/// 生成默认会话名称（带时间戳）
fn generate_default_session_name() -> String {
    let now = chrono::Local::now();
    format!("新会话 {}", now.format("%m-%d %H:%M"))
}

fn delete_session_related_runtime_data(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
) -> Result<(), String> {
    tx.execute(
        "DELETE FROM window_session_locks WHERE session_id = ?1",
        [session_id],
    )
    .map_err(|e| e.to_string())?;

    tx.execute(
        "UPDATE tasks SET session_id = NULL, updated_at = ?1 WHERE session_id = ?2",
        rusqlite::params![now_rfc3339(), session_id],
    )
    .map_err(|e| e.to_string())?;

    tx.execute(
        "DELETE FROM agent_cli_usage_records WHERE session_id = ?1",
        [session_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取指定项目的所有会话
#[tauri::command]
pub fn list_sessions(project_id: String) -> Result<Vec<Session>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT s.id, s.project_id, s.name, s.agent_type, s.status,
                   COALESCE(s.pinned, 0) as pinned,
                   m.last_message, s.error_message, COALESCE(m.message_count, 0) as message_count,
                   s.created_at, s.updated_at
            FROM sessions s
            LEFT JOIN (
                SELECT session_id,
                       MAX(content) as last_message,
                       COUNT(*) as message_count
                FROM messages
                GROUP BY session_id
            ) m ON s.id = m.session_id
            WHERE s.project_id = ?1
            ORDER BY s.pinned DESC, s.updated_at DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let sessions = stmt
        .query_map([&project_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                agent_type: row.get(3)?,
                status: row.get(4)?,
                pinned: row.get::<_, i32>(5)? != 0,
                last_message: row.get(6)?,
                error_message: row.get(7)?,
                message_count: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(sessions)
}

/// 创建新会话
#[tauri::command]
pub fn create_session(input: CreateSessionInput) -> Result<Session, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let name = input.name.unwrap_or_else(generate_default_session_name);
    let status = input.status.unwrap_or_else(|| "idle".to_string());

    conn.execute(
        "INSERT INTO sessions (id, project_id, name, agent_type, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            &id,
            &input.project_id,
            &name,
            &input.agent_type,
            &status,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    // 更新项目的 updated_at 时间
    conn.execute(
        "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &input.project_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(Session {
        id,
        project_id: input.project_id,
        name,
        agent_type: input.agent_type,
        status,
        pinned: false,
        last_message: None,
        error_message: None,
        message_count: 0,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 更新会话
#[tauri::command]
pub fn update_session(id: String, input: UpdateSessionInput) -> Result<Session, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let now = now_rfc3339();

    // 构建动态更新语句
    let mut updates: Vec<String> = vec!["updated_at = ?1".to_string()];
    let mut param_index = 2;

    if input.name.is_some() {
        updates.push(format!("name = ?{}", param_index));
        param_index += 1;
    }
    if input.status.is_some() {
        updates.push(format!("status = ?{}", param_index));
        param_index += 1;
    }
    if input.pinned.is_some() {
        updates.push(format!("pinned = ?{}", param_index));
        param_index += 1;
    }
    if input.last_message.is_some() {
        updates.push(format!("last_message = ?{}", param_index));
        param_index += 1;
    }
    if input.error_message.is_some() {
        updates.push(format!("error_message = ?{}", param_index));
        param_index += 1;
    }
    if input.agent_type.is_some() {
        updates.push(format!("agent_type = ?{}", param_index));
        param_index += 1;
    }

    let sql = format!(
        "UPDATE sessions SET {} WHERE id = ?{}",
        updates.join(", "),
        param_index
    );

    let mut stmt = conn.prepare_cached(&sql).map_err(|e| e.to_string())?;

    // 绑定参数
    let mut param_count = 1;
    stmt.raw_bind_parameter(param_count, &now)
        .map_err(|e| e.to_string())?;
    param_count += 1;

    if let Some(ref name) = input.name {
        stmt.raw_bind_parameter(param_count, name)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref status) = input.status {
        stmt.raw_bind_parameter(param_count, status)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(pinned) = input.pinned {
        stmt.raw_bind_parameter(param_count, if pinned { 1 } else { 0 })
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref last_message) = input.last_message {
        stmt.raw_bind_parameter(param_count, last_message)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref error_message) = input.error_message {
        stmt.raw_bind_parameter(param_count, error_message)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let Some(ref agent_type) = input.agent_type {
        stmt.raw_bind_parameter(param_count, agent_type)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }

    stmt.raw_bind_parameter(param_count, &id)
        .map_err(|e| e.to_string())?;

    stmt.raw_execute().map_err(|e| e.to_string())?;

    // 获取更新后的会话
    let session = get_session_by_id(&conn, &id)?;

    Ok(session)
}

/// 获取单个会话
fn get_session_by_id(conn: &Connection, id: &str) -> Result<Session, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT s.id, s.project_id, s.name, s.agent_type, s.status,
                   COALESCE(s.pinned, 0) as pinned,
                   m.last_message, s.error_message, COALESCE(m.message_count, 0) as message_count,
                   s.created_at, s.updated_at
            FROM sessions s
            LEFT JOIN (
                SELECT session_id,
                       MAX(content) as last_message,
                       COUNT(*) as message_count
                FROM messages
                GROUP BY session_id
            ) m ON s.id = m.session_id
            WHERE s.id = ?1
            "#,
        )
        .map_err(|e| e.to_string())?;

    let session = stmt
        .query_row([id], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                agent_type: row.get(3)?,
                status: row.get(4)?,
                pinned: row.get::<_, i32>(5)? != 0,
                last_message: row.get(6)?,
                error_message: row.get(7)?,
                message_count: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;

    Ok(session)
}

/// 删除会话
#[tauri::command]
pub fn delete_session(id: String) -> Result<(), String> {
    let mut conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    delete_session_related_runtime_data(&tx, &id)?;

    tx.execute("DELETE FROM sessions WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    remove_session_uploads(&id)?;

    Ok(())
}

/// 切换会话固定状态
#[tauri::command]
pub fn toggle_session_pin(id: String) -> Result<Session, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let now = now_rfc3339();

    // 先获取当前固定状态
    let current_pinned: bool = conn
        .query_row(
            "SELECT COALESCE(pinned, 0) FROM sessions WHERE id = ?1",
            [&id],
            |row| Ok(row.get::<_, i32>(0)? != 0),
        )
        .map_err(|e| e.to_string())?;

    // 切换状态
    conn.execute(
        "UPDATE sessions SET pinned = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![if current_pinned { 0 } else { 1 }, &now, &id],
    )
    .map_err(|e| e.to_string())?;

    let session = get_session_by_id(&conn, &id)?;

    Ok(session)
}
