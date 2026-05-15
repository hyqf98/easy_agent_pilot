use anyhow::Result;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use super::support::open_db_connection;

/// 应用状态键值
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateEntry {
    pub key: String,
    pub value: String,
    pub updated_at: i64,
}

/// 获取应用状态值
#[tauri::command]
pub fn get_app_state(key: String) -> Result<Option<String>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT value FROM app_state WHERE key = ?1")
        .map_err(|e| e.to_string())?;

    let result = stmt
        .query_row([&key], |row| row.get::<_, String>(0))
        .optional()
        .map_err(|e| e.to_string())?;

    Ok(result)
}

/// 设置应用状态值
#[tauri::command]
pub fn set_app_state(key: String, value: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO app_state (key, value, updated_at) VALUES (?1, ?2, strftime('%s', 'now'))",
        [&key, &value],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// 批量获取应用状态
#[tauri::command]
pub fn get_app_states(keys: Vec<String>) -> Result<Vec<AppStateEntry>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let placeholders: Vec<String> = keys.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT key, value, updated_at FROM app_state WHERE key IN ({})",
        placeholders.join(",")
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let params: Vec<&dyn rusqlite::ToSql> =
        keys.iter().map(|k| k as &dyn rusqlite::ToSql).collect();

    let entries = stmt
        .query_map(params.as_slice(), |row| {
            Ok(AppStateEntry {
                key: row.get(0)?,
                value: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(entries)
}
