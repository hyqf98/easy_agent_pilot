use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::commands::cli_support::{find_cli_executables, get_cli_version};

/// CLI 工具信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliTool {
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub status: CliStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CliStatus {
    Available,
    NotFound,
    Error,
}

/// 检测结果
#[derive(Debug, Serialize, Deserialize)]
pub struct DetectionResult {
    pub tools: Vec<CliTool>,
    pub total_found: usize,
}

/// CLI 路径配置（手动添加的）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliPathEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 获取数据库连接
fn get_db_connection() -> Result<Connection, String> {
    let persistence_dir = crate::commands::get_persistence_dir_path().map_err(|e| e.to_string())?;
    let db_path = persistence_dir.join("data").join("easy-agent.db");
    Connection::open(&db_path).map_err(|e| e.to_string())
}

/// 获取不同操作系统的扫描路径
fn get_scan_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let home = dirs::home_dir();

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/usr/local/bin"));
        paths.push(PathBuf::from("/opt/homebrew/bin"));
        if let Some(h) = &home {
            paths.push(h.join(".local/bin"));
            paths.push(h.join("Applications"));
        }
        paths.push(PathBuf::from("/Applications"));
    }

    #[cfg(target_os = "linux")]
    {
        paths.push(PathBuf::from("/usr/local/bin"));
        paths.push(PathBuf::from("/usr/bin"));
        if let Some(h) = &home {
            paths.push(h.join(".local/bin"));
            paths.push(h.join(".npm-global/bin"));
        }
        paths.push(PathBuf::from("/snap/bin"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(local_app_data) = std::env::var("LOCALAPPDATA").ok() {
            paths.push(PathBuf::from(&local_app_data).join("npm"));
            paths.push(PathBuf::from(&local_app_data).join("Programs"));
        }
        if let Some(app_data) = std::env::var("APPDATA").ok() {
            paths.push(PathBuf::from(&app_data).join("npm"));
        }
        if let Some(h) = &home {
            paths.push(h.join(".local/bin"));
        }
    }

    paths
}

/// 检测单个 CLI 工具
fn detect_cli(cli_name: &str) -> CliTool {
    let scan_paths = get_scan_paths();
    let mut first_invalid_match: Option<PathBuf> = None;

    for cli_path in find_cli_executables(cli_name, &scan_paths) {
        if let Some(version) = get_cli_version(&cli_path) {
            return CliTool {
                name: cli_name.to_string(),
                path: cli_path.to_string_lossy().to_string(),
                version: Some(version),
                status: CliStatus::Available,
            };
        }

        if first_invalid_match.is_none() {
            first_invalid_match = Some(cli_path);
        }
    }

    if let Some(cli_path) = first_invalid_match {
        return CliTool {
            name: cli_name.to_string(),
            path: cli_path.to_string_lossy().to_string(),
            version: None,
            status: CliStatus::Error,
        };
    }

    CliTool {
        name: cli_name.to_string(),
        path: String::new(),
        version: None,
        status: CliStatus::NotFound,
    }
}

/// 检测所有 CLI 工具 (Tauri 命令)
#[tauri::command]
pub fn detect_cli_tools() -> Result<DetectionResult, String> {
    let cli_names = vec!["claude", "codex"];
    let mut tools = Vec::new();

    for cli_name in cli_names {
        let tool = detect_cli(cli_name);
        tools.push(tool);
    }

    let total_found = tools
        .iter()
        .filter(|t| t.status == CliStatus::Available)
        .count();

    Ok(DetectionResult { tools, total_found })
}

/// 手动验证 CLI 路径 (Tauri 命令)
#[tauri::command]
pub fn verify_cli_path(path: String) -> Result<CliTool, String> {
    let cli_path = PathBuf::from(&path);

    if !cli_path.exists() {
        return Ok(CliTool {
            name: "custom".to_string(),
            path,
            version: None,
            status: CliStatus::NotFound,
        });
    }

    // 尝试获取版本号来验证可执行性
    let version = get_cli_version(&cli_path);
    let status = if version.is_some() {
        CliStatus::Available
    } else {
        CliStatus::Error
    };

    // 从路径推断 CLI 名称
    let name = cli_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(CliTool {
        name,
        path,
        version,
        status,
    })
}

// ============== CLI 路径配置 CRUD ==============

/// 获取所有 CLI 路径配置 (Tauri 命令)
#[tauri::command]
pub fn list_cli_paths() -> Result<Vec<CliPathEntry>, String> {
    let conn = get_db_connection()?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, path, version, created_at, updated_at FROM cli_paths ORDER BY created_at DESC"
        )
        .map_err(|e| e.to_string())?;

    let paths = stmt
        .query_map([], |row| {
            Ok(CliPathEntry {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                version: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(paths)
}

/// 添加 CLI 路径配置 (Tauri 命令)
#[tauri::command]
pub fn add_cli_path(name: String, path: String) -> Result<CliPathEntry, String> {
    let conn = get_db_connection()?;

    // 验证路径并获取版本
    let cli_path = PathBuf::from(&path);
    let version = get_cli_version(&cli_path);

    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO cli_paths (id, name, path, version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, name, path, version, now, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(CliPathEntry {
        id,
        name,
        path,
        version,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 更新 CLI 路径配置 (Tauri 命令)
#[tauri::command]
pub fn update_cli_path(id: String, name: String, path: String) -> Result<CliPathEntry, String> {
    let conn = get_db_connection()?;

    // 验证路径并获取版本
    let cli_path = PathBuf::from(&path);
    let version = get_cli_version(&cli_path);

    let now = Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE cli_paths SET name = ?1, path = ?2, version = ?3, updated_at = ?4 WHERE id = ?5",
        params![name, path, version, now, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(CliPathEntry {
        id,
        name,
        path,
        version,
        created_at: String::new(), // 不需要返回原始创建时间
        updated_at: now,
    })
}

/// 删除 CLI 路径配置 (Tauri 命令)
#[tauri::command]
pub fn delete_cli_path(id: String) -> Result<(), String> {
    let conn = get_db_connection()?;

    conn.execute("DELETE FROM cli_paths WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============== CLI 路径迁移到智能体配置 ==============

/// 迁移状态键名
const MIGRATION_STATUS_KEY: &str = "cli_paths_migrated";

/// 迁移结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    /// 是否成功
    pub success: bool,
    /// 迁移的 CLI 路径数量
    pub migrated_count: usize,
    /// 跳过的数量（已存在相同路径的智能体）
    pub skipped_count: usize,
    /// 迁移的智能体 ID 列表
    pub migrated_agent_ids: Vec<String>,
    /// 错误消息
    pub error_message: Option<String>,
}

/// 检查是否需要迁移（是否存在旧的 CLI 路径配置且未迁移过）
#[tauri::command]
pub fn check_cli_paths_migration_needed() -> Result<bool, String> {
    let conn = get_db_connection()?;

    // 检查是否已经迁移过
    let mut stmt = conn
        .prepare("SELECT value FROM app_settings WHERE key = ?1")
        .map_err(|e| e.to_string())?;

    let migrated = stmt
        .query_row([MIGRATION_STATUS_KEY], |row| row.get::<_, String>(0))
        .optional()
        .map_err(|e| e.to_string())?;

    if migrated.is_some() {
        return Ok(false);
    }

    // 检查是否有 CLI 路径配置
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM cli_paths", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    Ok(count > 0)
}

/// 获取待迁移的 CLI 路径数量
#[tauri::command]
pub fn get_pending_migration_count() -> Result<usize, String> {
    let conn = get_db_connection()?;

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM cli_paths", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    Ok(count as usize)
}

/// 执行 CLI 路径迁移到智能体配置
#[tauri::command]
pub fn migrate_cli_paths_to_agents() -> Result<MigrationResult, String> {
    let conn = get_db_connection()?;

    // 检查是否已经迁移过
    let mut stmt = conn
        .prepare("SELECT value FROM app_settings WHERE key = ?1")
        .map_err(|e| e.to_string())?;

    let migrated = stmt
        .query_row([MIGRATION_STATUS_KEY], |row| row.get::<_, String>(0))
        .optional()
        .map_err(|e| e.to_string())?;

    if migrated.is_some() {
        return Ok(MigrationResult {
            success: true,
            migrated_count: 0,
            skipped_count: 0,
            migrated_agent_ids: Vec::new(),
            error_message: Some("已经完成迁移".to_string()),
        });
    }

    // 获取所有 CLI 路径配置
    let mut stmt = conn
        .prepare("SELECT id, name, path, version, created_at, updated_at FROM cli_paths")
        .map_err(|e| e.to_string())?;

    let cli_paths = stmt
        .query_map([], |row| {
            Ok(CliPathEntry {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                version: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut migrated_count = 0;
    let mut skipped_count = 0;
    let mut migrated_agent_ids = Vec::new();
    let now = Utc::now().to_rfc3339();

    for cli_path in cli_paths {
        // 检查是否已存在相同路径的智能体
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM agents WHERE cli_path = ?1",
                [&cli_path.path],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        if existing.is_some() {
            skipped_count += 1;
            continue;
        }

        // 从路径推断提供商
        let provider = infer_provider_from_path(&cli_path.path);

        // 创建新的智能体配置
        let agent_id = Uuid::new_v4().to_string();
        let agent_name = cli_path.name.clone();

        conn.execute(
            "INSERT INTO agents (id, name, type, provider, cli_path, status, created_at, updated_at)
             VALUES (?1, ?2, 'cli', ?3, ?4, 'offline', ?5, ?6)",
            params![&agent_id, &agent_name, &provider, &cli_path.path, &now, &now],
        )
        .map_err(|e| e.to_string())?;

        migrated_count += 1;
        migrated_agent_ids.push(agent_id);
    }

    // 标记迁移完成
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
        params![MIGRATION_STATUS_KEY, "true", &now],
    )
    .map_err(|e| e.to_string())?;

    Ok(MigrationResult {
        success: true,
        migrated_count,
        skipped_count,
        migrated_agent_ids,
        error_message: None,
    })
}

/// 从 CLI 路径推断提供商
fn infer_provider_from_path(path: &str) -> Option<String> {
    let path_lower = path.to_lowercase();

    if path_lower.contains("claude") {
        Some("claude".to_string())
    } else if path_lower.contains("codex") {
        Some("codex".to_string())
    } else {
        // 默认为 claude
        Some("claude".to_string())
    }
}
