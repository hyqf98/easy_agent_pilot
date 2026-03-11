use anyhow::Result;
use rusqlite::OptionalExtension;
use std::collections::HashMap;

use super::support::{now_rfc3339, open_db_connection};

/// 获取单个设置值
#[tauri::command]
pub fn get_app_setting(key: String) -> Result<Option<String>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT value FROM app_settings WHERE key = ?1")
        .map_err(|e| e.to_string())?;

    let result = stmt
        .query_row([&key], |row| row.get::<_, String>(0))
        .optional()
        .map_err(|e| e.to_string())?;

    Ok(result)
}

/// 获取所有设置
#[tauri::command]
pub fn get_all_app_settings() -> Result<HashMap<String, String>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT key, value FROM app_settings")
        .map_err(|e| e.to_string())?;

    let settings = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<HashMap<String, String>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(settings)
}

/// 保存单个设置
#[tauri::command]
pub fn save_app_setting(key: String, value: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let updated_at = now_rfc3339();

    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
        [&key, &value, &updated_at],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// 批量保存设置
#[tauri::command]
pub fn save_app_settings(settings: HashMap<String, String>) -> Result<(), String> {
    let mut conn = open_db_connection().map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let updated_at = now_rfc3339();

    for (key, value) in settings {
        tx.execute(
            "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            [&key, &value, &updated_at],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(())
}

/// 删除单个设置
#[tauri::command]
pub fn delete_app_setting(key: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM app_settings WHERE key = ?1", [&key])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 清除所有设置
#[tauri::command]
pub fn clear_app_settings() -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM app_settings", [])
        .map_err(|e| e.to_string())?;

    Ok(())
}
