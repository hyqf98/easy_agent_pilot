use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

use crate::commands::cli_support::{
    configure_windows_std_command, get_cli_version, normalize_cli_identifier,
};

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

pub struct DetectionResult {
    pub tools: Vec<CliTool>,
    pub total_found: usize,
}

pub struct CliPathEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 获取数据库连�?fn get_db_connection() -> Result<Connection, String> {
    let persistence_dir = crate::commands::get_persistence_dir_path().map_err(|e| e.to_string())?;
    let db_path = persistence_dir.join("data").join("easy-agent.db");
    Connection::open(&db_path).map_err(|e| e.to_string())
}

fn canonicalize_existing_path(path: &Path) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }

    std::fs::canonicalize(path)
        .ok()
        .or_else(|| Some(path.to_path_buf()))
}

fn append_candidate_path(
    path: PathBuf,
    seen: &mut HashSet<PathBuf>,
    candidates: &mut Vec<PathBuf>,
) {
    let Some(canonical_path) = canonicalize_existing_path(&path) else {
        return;
    };

    if seen.insert(canonical_path.clone()) {
        candidates.push(canonical_path);
    }
}

fn shell_lookup_paths(cli_name: &str) -> Vec<PathBuf> {
    let output = if cfg!(target_os = "windows") {
        let mut command = Command::new("where.exe");
        #[cfg(target_os = "windows")]
        configure_windows_std_command(&mut command);
        command.arg(cli_name).output()
    } else {
        Command::new("which").arg("-a").arg(cli_name).output()
    };

    let Ok(output) = output else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    output
        .stdout
        .split(|byte| *byte == b'\n' || *byte == b'\r')
        .filter_map(|line| {
            let text = String::from_utf8_lossy(line)
                .trim()
                .trim_matches('"')
                .to_string();
            if text.is_empty() {
                None
            } else {
                canonicalize_existing_path(Path::new(&text))
            }
        })
        .collect()
}

fn npm_global_prefix() -> Option<PathBuf> {
    let mut command = Command::new("npm");
    #[cfg(target_os = "windows")]
    configure_windows_std_command(&mut command);
    let output = command.args(["config", "get", "prefix"]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if prefix.is_empty() || prefix.eq_ignore_ascii_case("undefined") {
        return None;
    }

    let path = PathBuf::from(prefix);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn cli_file_names(cli_name: &str) -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        return vec![
            cli_name.to_string(),
            format!("{cli_name}.cmd"),
            format!("{cli_name}.exe"),
            format!("{cli_name}.bat"),
            format!("{cli_name}.com"),
        ];
    }

    #[cfg(not(target_os = "windows"))]
    {
        vec![cli_name.to_string()]
    }
}

fn append_cli_paths_from_dir(
    dir: &Path,
    cli_name: &str,
    seen: &mut HashSet<PathBuf>,
    candidates: &mut Vec<PathBuf>,
) {
    if !dir.exists() || !dir.is_dir() {
        return;
    }

    for file_name in cli_file_names(cli_name) {
        append_candidate_path(dir.join(file_name), seen, candidates);
    }
}

fn npm_fallback_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(prefix) = npm_global_prefix() {
        #[cfg(target_os = "windows")]
        {
            dirs.push(prefix.clone());
            dirs.push(prefix.join("node_modules").join(".bin"));
        }

        #[cfg(not(target_os = "windows"))]
        {
            dirs.push(prefix.join("bin"));
            dirs.push(prefix.join("lib").join("node_modules").join(".bin"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(app_data) = std::env::var("APPDATA") {
            dirs.push(PathBuf::from(app_data).join("npm"));
        }
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local_app_data).join("npm"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/usr/local/bin"));
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
    }

    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/local/bin"));
        dirs.push(PathBuf::from("/usr/bin"));
        dirs.push(PathBuf::from("/snap/bin"));
    }

    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local").join("bin"));
        dirs.push(home.join(".npm-global").join("bin"));
    }

    dirs
}

fn collect_cli_candidates(cli_name: &str) -> Vec<PathBuf> {
    let raw_path = Path::new(cli_name);
    if raw_path.is_absolute() || raw_path.components().count() > 1 {
        return canonicalize_existing_path(raw_path).into_iter().collect();
    }

    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    for path in shell_lookup_paths(cli_name) {
        append_candidate_path(path, &mut seen, &mut candidates);
    }

    for dir in npm_fallback_dirs() {
        append_cli_paths_from_dir(&dir, cli_name, &mut seen, &mut candidates);
    }

    candidates
}

fn detect_cli(cli_name: &str) -> CliTool {
    let mut first_invalid_match: Option<PathBuf> = None;

    for cli_path in collect_cli_candidates(cli_name) {
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

    let status = if version.is_some() {
        CliStatus::Available
    } else {
        CliStatus::Error
    };

    let name = normalize_cli_identifier(&path)
        .or_else(|| {
            cli_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    Ok(CliTool {
        name,
        path,
        version,
        status,
    })
}


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

#[tauri::command]
pub fn add_cli_path(name: String, path: String) -> Result<CliPathEntry, String> {
    let conn = get_db_connection()?;

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

#[tauri::command]
pub fn update_cli_path(id: String, name: String, path: String) -> Result<CliPathEntry, String> {
    let conn = get_db_connection()?;

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
        created_at: String::new(), // 不需要返回原始创建时�?        updated_at: now,
    })
}

#[tauri::command]
pub fn delete_cli_path(id: String) -> Result<(), String> {
    let conn = get_db_connection()?;

    conn.execute("DELETE FROM cli_paths WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}


/// 迁移状��键�?const MIGRATION_STATUS_KEY: &str = "cli_paths_migrated";

/// 迁移结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    /// 是否成功
    pub success: bool,
    /// 迁移�?CLI 路径数量
    pub migrated_count: usize,
    /// 跳过的数量（已存在相同路径的智能体）
    pub skipped_count: usize,
    pub migrated_agent_ids: Vec<String>,
    pub error_message: Option<String>,
}

#[tauri::command]
pub fn check_cli_paths_migration_needed() -> Result<bool, String> {
    let conn = get_db_connection()?;

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

#[tauri::command]
pub fn migrate_cli_paths_to_agents() -> Result<MigrationResult, String> {
    let conn = get_db_connection()?;

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

        let provider = infer_provider_from_path(&cli_path.path);

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

    let path_lower = path.to_lowercase();

    if path_lower.contains("claude") {
        Some("claude".to_string())
    } else if path_lower.contains("codex") {
        Some("codex".to_string())
    } else {
        // 默认�?claude
        Some("claude".to_string())
    }
}
