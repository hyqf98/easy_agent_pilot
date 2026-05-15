use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::support::open_db_connection;

/// 项目访问记录
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectAccessLog {
    pub project_id: String,
    pub last_accessed_at: i64,
    pub access_count: i64,
}

/// 记录项目访问
#[tauri::command]
pub fn record_project_access(project_id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO project_access_log (project_id, last_accessed_at, access_count) VALUES (?1, strftime('%s', 'now'), 1) ON CONFLICT(project_id) DO UPDATE SET last_accessed_at = strftime('%s', 'now'), access_count = access_count + 1",
        [&project_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取最近访问的项目
#[tauri::command]
pub fn get_recent_projects(limit: i32) -> Result<Vec<String>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT project_id FROM project_access_log ORDER BY last_accessed_at DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let project_ids = stmt
        .query_map([limit], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(project_ids)
}

/// 删除项目访问记录
#[tauri::command]
pub fn delete_project_access_log(project_id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM project_access_log WHERE project_id = ?1",
        [&project_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
