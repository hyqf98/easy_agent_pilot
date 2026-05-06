use anyhow::Result;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use std::process::Command;

use super::message::remove_session_uploads;
use super::support::{
    now_rfc3339, open_db_connection, open_db_connection_with_foreign_keys,
    repair_memory_search_indexes,
};

/// 文件操作结果
#[derive(Debug, Serialize)]
pub struct FileOperationResult {
    pub success: bool,
    pub message: Option<String>,
    pub new_path: Option<String>,
}

/// 重命名文件输入
#[derive(Debug, Deserialize)]
pub struct RenameFileInput {
    pub old_path: String,
    pub new_name: String,
}

/// 移动文件输入
#[derive(Debug, Deserialize)]
pub struct MoveFileInput {
    pub source_path: String,
    pub target_path: String,
}

/// 批量删除输入
#[derive(Debug, Deserialize)]
pub struct BatchDeleteInput {
    pub paths: Vec<String>,
}

/// 新建文件/目录类型
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CreateEntryType {
    File,
    Directory,
}

/// 新建文件/目录输入
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEntryInput {
    pub parent_path: String,
    pub name: String,
    pub entry_type: CreateEntryType,
}

/// 项目数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub session_count: i32,
    #[serde(default, rename = "memoryLibraryIds")]
    pub memory_library_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
struct ProjectRecord {
    id: String,
    name: String,
    path: String,
    description: Option<String>,
    session_count: i32,
    created_at: String,
    updated_at: String,
}

/// 文件树节点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileNodeType {
    File,
    Directory,
}

/// 文件树节点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub node_type: FileNodeType,
    pub children: Option<Vec<FileTreeNode>>,
    /// 文件扩展名（仅文件类型有）
    pub extension: Option<String>,
}

#[derive(Debug, Clone)]
struct ProjectFileCacheEntry {
    files: Vec<FlatFileInfo>,
    cached_at: Instant,
}

#[derive(Debug, Clone)]
struct GlobalFileIndexEntry {
    name: String,
    path: String,
    display_path: String,
    node_type: FileNodeType,
    extension: Option<String>,
}

#[derive(Debug, Default)]
struct GlobalFileSearchCache {
    entries: Vec<GlobalFileIndexEntry>,
    pending_dirs: VecDeque<PathBuf>,
    visited_dirs: HashSet<String>,
    initialized: bool,
    completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileMentionScope {
    Project,
    Global,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFileMentionsInput {
    pub query: String,
    pub scope: FileMentionScope,
    pub project_path: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMentionSearchResult {
    pub name: String,
    pub path: String,
    pub insert_path: String,
    pub display_path: String,
    pub node_type: FileNodeType,
    pub extension: Option<String>,
    pub scope: FileMentionScope,
}

const PROJECT_FILE_CACHE_TTL: Duration = Duration::from_secs(12);
const DEFAULT_FILE_MENTION_LIMIT: usize = 80;
const MAX_FILE_MENTION_LIMIT: usize = 200;

static PROJECT_FILE_CACHE: Lazy<Mutex<HashMap<String, ProjectFileCacheEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static GLOBAL_FILE_CACHE: Lazy<Mutex<GlobalFileSearchCache>> =
    Lazy::new(|| Mutex::new(GlobalFileSearchCache::default()));

fn normalize_path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn invalidate_project_file_cache_for_path(path: &Path) {
    let normalized = normalize_path_key(path);
    let normalized_prefix = format!("{}/", normalized.trim_end_matches('/'));
    let mut cache = PROJECT_FILE_CACHE.lock();
    cache.retain(|key, _| {
        let normalized_key = key.replace('\\', "/");
        normalized_key != normalized && !normalized_key.starts_with(&normalized_prefix)
    });
}

fn invalidate_project_file_cache_for_paths(paths: &[&Path]) {
    for path in paths {
        invalidate_project_file_cache_for_path(path);
    }
}

/// 创建项目输入
#[derive(Debug, Deserialize)]
pub struct CreateProjectInput {
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    #[serde(default, rename = "memoryLibraryIds")]
    pub memory_library_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRuntimeCleanupResult {
    pub project_id: String,
    pub cleared_sessions: usize,
    pub cleared_messages: usize,
    pub cleared_plans: usize,
    pub cleared_tasks: usize,
    pub cleared_plan_split_logs: usize,
    pub cleared_plan_split_sessions: usize,
    pub cleared_execution_results: usize,
    pub cleared_execution_logs: usize,
}

fn normalize_memory_library_ids(library_ids: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();

    for library_id in library_ids {
        let trimmed = library_id.trim();
        if trimmed.is_empty() || normalized.iter().any(|existing| existing == trimmed) {
            continue;
        }
        normalized.push(trimmed.to_string());
    }

    normalized
}

fn list_project_memory_library_ids(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT library_id
            FROM project_memory_libraries
            WHERE project_id = ?1
            ORDER BY created_at ASC, library_id ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([project_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rows)
}

fn replace_project_memory_libraries(
    conn: &rusqlite::Connection,
    project_id: &str,
    library_ids: &[String],
    now: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM project_memory_libraries WHERE project_id = ?1",
        [project_id],
    )
    .map_err(|e| e.to_string())?;

    let normalized_ids = normalize_memory_library_ids(library_ids);

    for library_id in normalized_ids {
        conn.execute(
            r#"
            INSERT INTO project_memory_libraries (project_id, library_id, created_at)
            VALUES (?1, ?2, ?3)
            "#,
            params![project_id, library_id, now],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn get_project_session_count(conn: &rusqlite::Connection, project_id: &str) -> Result<i32, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE project_id = ?1",
        [project_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

fn get_project_created_at(conn: &rusqlite::Connection, project_id: &str) -> Result<String, String> {
    conn.query_row(
        "SELECT created_at FROM projects WHERE id = ?1",
        [project_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

fn hidden_mini_panel_project_id(conn: &rusqlite::Connection) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [super::mini_panel::MINI_PANEL_PROJECT_ID_KEY],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// 获取所有项目
#[tauri::command]
pub fn list_projects() -> Result<Vec<Project>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let hidden_project_id = hidden_mini_panel_project_id(&conn)?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT p.id, p.name, p.path, p.description, p.created_at, p.updated_at,
                   COALESCE(s.session_count, 0) as session_count
            FROM projects p
            LEFT JOIN (
                SELECT project_id, COUNT(*) as session_count
                FROM sessions
                GROUP BY project_id
            ) s ON p.id = s.project_id
            ORDER BY p.updated_at DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let project_rows = stmt
        .query_map([], |row| {
            Ok(ProjectRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                description: row.get(3)?,
                session_count: row.get(6)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    drop(stmt);

    let mut projects = Vec::with_capacity(project_rows.len());
    for project_row in project_rows {
        if hidden_project_id.as_deref() == Some(project_row.id.as_str()) {
            continue;
        }

        projects.push(Project {
            id: project_row.id.clone(),
            name: project_row.name,
            path: project_row.path,
            description: project_row.description,
            session_count: project_row.session_count,
            memory_library_ids: list_project_memory_library_ids(&conn, &project_row.id)?,
            created_at: project_row.created_at,
            updated_at: project_row.updated_at,
        });
    }

    Ok(projects)
}

/// 创建新项目
#[tauri::command]
pub fn create_project(input: CreateProjectInput) -> Result<Project, String> {
    let mut conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;

    // 解析并创建项目目录
    let resolved_path = resolve_path(&input.path)?;

    // 如果目录不存在，则创建
    if !resolved_path.exists() {
        fs::create_dir_all(&resolved_path).map_err(|e| format!("创建项目目录失败: {}", e))?;
    } else if !resolved_path.is_dir() {
        return Err("路径已存在但不是目录".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let memory_library_ids = normalize_memory_library_ids(&input.memory_library_ids);

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO projects (id, name, path, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            &id,
            &input.name,
            &input.path,
            &input.description,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    replace_project_memory_libraries(&tx, &id, &memory_library_ids, &now)?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(Project {
        id,
        name: input.name,
        path: input.path,
        description: input.description,
        session_count: 0,
        memory_library_ids,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 更新项目
#[tauri::command]
pub fn update_project(id: String, input: CreateProjectInput) -> Result<Project, String> {
    let mut conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;

    let now = now_rfc3339();
    let memory_library_ids = normalize_memory_library_ids(&input.memory_library_ids);

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "UPDATE projects SET name = ?1, path = ?2, description = ?3, updated_at = ?4 WHERE id = ?5",
        rusqlite::params![&input.name, &input.path, &input.description, &now, &id],
    )
    .map_err(|e| e.to_string())?;

    replace_project_memory_libraries(&tx, &id, &memory_library_ids, &now)?;
    let created_at = get_project_created_at(&tx, &id)?;
    let session_count = get_project_session_count(&tx, &id)?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(Project {
        id,
        name: input.name,
        path: input.path,
        description: input.description,
        session_count,
        memory_library_ids,
        created_at,
        updated_at: now,
    })
}

/// 删除项目（级联删除关联的会话和消息）
#[tauri::command]
pub fn delete_project(id: String) -> Result<(), String> {
    let conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;
    repair_memory_search_indexes(&conn).map_err(|e| format!("修复记忆搜索索引失败: {}", e))?;

    conn.execute("DELETE FROM projects WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn clear_project_runtime_data(
    project_id: String,
) -> Result<ProjectRuntimeCleanupResult, String> {
    let mut conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;
    repair_memory_search_indexes(&conn).map_err(|e| format!("修复记忆搜索索引失败: {}", e))?;

    let session_ids = {
        let mut stmt = conn
            .prepare("SELECT id FROM sessions WHERE project_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([&project_id], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    let cleared_sessions = session_ids.len();
    let cleared_messages: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE session_id IN (SELECT id FROM sessions WHERE project_id = ?1)",
            [&project_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let cleared_plans: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM plans WHERE project_id = ?1",
            [&project_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let cleared_tasks: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 OR plan_id IN (SELECT id FROM plans WHERE project_id = ?1)",
            [&project_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let cleared_plan_split_logs: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM plan_split_logs WHERE plan_id IN (SELECT id FROM plans WHERE project_id = ?1)",
            [&project_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let cleared_plan_split_sessions: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM task_split_sessions WHERE plan_id IN (SELECT id FROM plans WHERE project_id = ?1)",
            [&project_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let cleared_execution_results: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM task_execution_results WHERE plan_id IN (SELECT id FROM plans WHERE project_id = ?1)",
            [&project_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let cleared_execution_logs: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM task_execution_logs WHERE task_id IN (
                SELECT id FROM tasks WHERE project_id = ?1 OR plan_id IN (SELECT id FROM plans WHERE project_id = ?1)
            )",
            [&project_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let now = now_rfc3339();
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "DELETE FROM window_session_locks WHERE session_id IN (SELECT id FROM sessions WHERE project_id = ?1)",
        [&project_id],
    )
    .map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM sessions WHERE project_id = ?1", [&project_id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM tasks WHERE project_id = ?1", [&project_id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM plans WHERE project_id = ?1", [&project_id])
        .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &project_id],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    for session_id in &session_ids {
        remove_session_uploads(session_id)?;
    }

    Ok(ProjectRuntimeCleanupResult {
        project_id,
        cleared_sessions,
        cleared_messages,
        cleared_plans,
        cleared_tasks,
        cleared_plan_split_logs,
        cleared_plan_split_sessions,
        cleared_execution_results,
        cleared_execution_logs,
    })
}

/// 路径验证结果
#[derive(Debug, Serialize)]
pub struct PathValidationResult {
    pub valid: bool,
    pub error: Option<String>,
}

/// 验证项目路径
#[tauri::command]
pub fn validate_project_path(path: String) -> Result<PathValidationResult, String> {
    // 空路径是有效的（将使用默认路径）
    if path.trim().is_empty() {
        return Ok(PathValidationResult {
            valid: true,
            error: None,
        });
    }

    let path = std::path::Path::new(path.trim());

    // 检查路径是否为绝对路径（或以 ~ 开头的路径）
    let path_str = path.to_string_lossy();
    if !path.is_absolute() && !path_str.starts_with('~') {
        return Ok(PathValidationResult {
            valid: false,
            error: Some("请输入绝对路径或使用 ~ 表示用户目录".to_string()),
        });
    }

    // 如果是 ~ 开头的路径，展开后验证
    let resolved_path = if let Some(rest) = path_str.strip_prefix('~') {
        // 简单的波浪号展开验证
        let home = dirs::home_dir().ok_or_else(|| "无法获取用户主目录".to_string())?;
        let rest = rest.strip_prefix('/').unwrap_or(rest);
        home.join(rest)
    } else {
        path.to_path_buf()
    };

    // 如果路径存在，检查是否为目录
    if resolved_path.exists() {
        if !resolved_path.is_dir() {
            return Ok(PathValidationResult {
                valid: false,
                error: Some("路径存在但不是目录".to_string()),
            });
        }
        // 检查是否有写入权限
        let metadata = resolved_path.metadata().map_err(|e| e.to_string())?;
        if metadata.permissions().readonly() {
            return Ok(PathValidationResult {
                valid: false,
                error: Some("目录没有写入权限".to_string()),
            });
        }
    }

    Ok(PathValidationResult {
        valid: true,
        error: None,
    })
}

/// 需要忽略的目录和文件模式
const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "target",
    "dist",
    "build",
    ".idea",
    ".vscode",
    ".DS_Store",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    "venv",
    ".venv",
    "env",
    ".env",
];

const IGNORED_FILES: &[&str] = &[".DS_Store", "Thumbs.db", ".gitignore", ".gitattributes"];

/// 递归扫描目录获取文件树
#[allow(dead_code)]
fn scan_directory_recursive(
    dir_path: &PathBuf,
    max_depth: usize,
    current_depth: usize,
) -> Result<Vec<FileTreeNode>, String> {
    if current_depth >= max_depth {
        return Ok(Vec::new());
    }

    let mut nodes = Vec::new();

    let entries = fs::read_dir(dir_path).map_err(|e| format!("无法读取目录: {}", e))?;

    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();

    // 排序：目录优先，然后按名称排序
    entries.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        if a_is_dir == b_is_dir {
            a.file_name().cmp(&b.file_name())
        } else if a_is_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    for entry in entries {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // 跳过隐藏文件和目录（以 . 开头，但不是当前/父目录）
        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            // 跳过忽略的目录
            if IGNORED_DIRS.contains(&name.as_str()) {
                continue;
            }

            // 递归扫描子目录
            let children = if current_depth + 1 < max_depth {
                Some(scan_directory_recursive(
                    &path,
                    max_depth,
                    current_depth + 1,
                )?)
            } else {
                None
            };

            nodes.push(FileTreeNode {
                name,
                path: path.to_string_lossy().to_string(),
                node_type: FileNodeType::Directory,
                children,
                extension: None,
            });
        } else if path.is_file() {
            // 跳过忽略的文件
            if IGNORED_FILES.contains(&name.as_str()) {
                continue;
            }

            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_string());

            nodes.push(FileTreeNode {
                name,
                path: path.to_string_lossy().to_string(),
                node_type: FileNodeType::File,
                children: None,
                extension,
            });
        }
    }

    Ok(nodes)
}

/// 解析路径（处理 ~ 开头的路径）
fn resolve_path(path_str: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path_str);

    // 处理 ~ 开头的路径
    let resolved_path = if let Some(rest) = path_str.strip_prefix('~') {
        let home = dirs::home_dir().ok_or_else(|| "无法获取用户主目录".to_string())?;
        let rest = rest.strip_prefix('/').unwrap_or(rest);
        home.join(rest)
    } else {
        path
    };

    Ok(resolved_path)
}

/// 扫描单层目录（用于懒加载）
fn scan_single_directory(dir_path: &PathBuf) -> Result<Vec<FileTreeNode>, String> {
    let mut nodes = Vec::new();

    let entries = fs::read_dir(dir_path).map_err(|e| format!("无法读取目录: {}", e))?;

    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();

    // 排序：目录优先，然后按名称排序
    entries.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        if a_is_dir == b_is_dir {
            a.file_name().cmp(&b.file_name())
        } else if a_is_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    for entry in entries {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // 跳过隐藏文件和目录（以 . 开头，但不是当前/父目录）
        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            // 跳过忽略的目录
            if IGNORED_DIRS.contains(&name.as_str()) {
                continue;
            }

            // 目录的 children 设为 None，序列化后为 null
            // Naive UI 的懒加载需要 children 为 null/undefined 才会触发 onLoad
            nodes.push(FileTreeNode {
                name,
                path: path.to_string_lossy().to_string(),
                node_type: FileNodeType::Directory,
                children: None, // null 表示需要懒加载
                extension: None,
            });
        } else if path.is_file() {
            // 跳过忽略的文件
            if IGNORED_FILES.contains(&name.as_str()) {
                continue;
            }

            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_string());

            nodes.push(FileTreeNode {
                name,
                path: path.to_string_lossy().to_string(),
                node_type: FileNodeType::File,
                children: None,
                extension,
            });
        }
    }

    Ok(nodes)
}

/// 列出项目目录的文件树（懒加载模式，只加载第一层）
#[tauri::command]
pub fn list_project_files(project_path: String) -> Result<Vec<FileTreeNode>, String> {
    let resolved_path = resolve_path(&project_path)?;

    if !resolved_path.exists() {
        return Err(format!("项目路径不存在: {}", project_path));
    }

    if !resolved_path.is_dir() {
        return Err(format!("项目路径不是目录: {}", project_path));
    }

    scan_single_directory(&resolved_path)
}

/// 懒加载目录的子节点
#[tauri::command]
pub fn load_directory_children(dir_path: String) -> Result<Vec<FileTreeNode>, String> {
    let resolved_path = resolve_path(&dir_path)?;

    if !resolved_path.exists() {
        return Err(format!("目录不存在: {}", dir_path));
    }

    if !resolved_path.is_dir() {
        return Err(format!("路径不是目录: {}", dir_path));
    }

    scan_single_directory(&resolved_path)
}

/// 扁平化的文件信息（用于 @ 文件引用）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatFileInfo {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub node_type: FileNodeType,
    pub extension: Option<String>,
}

#[derive(Debug)]
struct RankedFileMentionResult {
    score: i32,
    result: FileMentionSearchResult,
}

/// 递归收集所有文件到扁平列表
fn collect_files_flat(
    dir_path: &PathBuf,
    base_path: &PathBuf,
    result: &mut Vec<FlatFileInfo>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir_path).map_err(|e| format!("无法读取目录: {}", e))?;

    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();

    // 排序：目录优先，然后按名称排序
    entries.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        if a_is_dir == b_is_dir {
            a.file_name().cmp(&b.file_name())
        } else if a_is_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    for entry in entries {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // 跳过隐藏文件和目录
        if name.starts_with('.') {
            continue;
        }

        // 计算相对路径
        let relative_path = path
            .strip_prefix(base_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string();

        if path.is_dir() {
            // 跳过忽略的目录
            if IGNORED_DIRS.contains(&name.as_str()) {
                continue;
            }

            // 添加目录项
            result.push(FlatFileInfo {
                name: name.clone(),
                path: path.to_string_lossy().to_string(),
                relative_path: relative_path.clone(),
                node_type: FileNodeType::Directory,
                extension: None,
            });

            // 递归处理子目录
            collect_files_flat(&path, base_path, result)?;
        } else if path.is_file() {
            // 跳过忽略的文件
            if IGNORED_FILES.contains(&name.as_str()) {
                continue;
            }

            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_string());

            result.push(FlatFileInfo {
                name,
                path: path.to_string_lossy().to_string(),
                relative_path,
                node_type: FileNodeType::File,
                extension,
            });
        }
    }

    Ok(())
}

fn get_project_file_index(project_path: &PathBuf) -> Result<Vec<FlatFileInfo>, String> {
    let cache_key = project_path.to_string_lossy().to_string();

    {
        let cache = PROJECT_FILE_CACHE.lock();
        if let Some(entry) = cache.get(&cache_key) {
            if entry.cached_at.elapsed() < PROJECT_FILE_CACHE_TTL {
                return Ok(entry.files.clone());
            }
        }
    }

    let mut files = Vec::new();
    collect_files_flat(project_path, project_path, &mut files)?;

    PROJECT_FILE_CACHE.lock().insert(
        cache_key,
        ProjectFileCacheEntry {
            files: files.clone(),
            cached_at: Instant::now(),
        },
    );

    Ok(files)
}

fn normalize_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_FILE_MENTION_LIMIT)
        .clamp(1, MAX_FILE_MENTION_LIMIT)
}

fn shorten_home_path(path: &Path) -> String {
    if let Some(home_dir) = dirs::home_dir() {
        if let Ok(relative) = path.strip_prefix(&home_dir) {
            let relative_str = relative.to_string_lossy();
            if relative_str.is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", relative_str);
        }
    }

    path.to_string_lossy().to_string()
}

fn path_depth(path: &str) -> usize {
    path.chars().filter(|ch| *ch == '/' || *ch == '\\').count()
}

fn compute_search_score(name: &str, display_path: &str, query: &str) -> i32 {
    let normalized_query = query.trim().to_lowercase();
    if normalized_query.is_empty() {
        return 100 - path_depth(display_path) as i32;
    }

    let name_lower = name.to_lowercase();
    let display_lower = display_path.to_lowercase();
    let mut score = 0;

    if name_lower == normalized_query {
        score += 420;
    } else if name_lower.starts_with(&normalized_query) {
        score += 280;
    } else if name_lower.contains(&normalized_query) {
        score += 190;
    }

    if display_lower == normalized_query {
        score += 180;
    } else if display_lower.starts_with(&normalized_query) {
        score += 120;
    } else if display_lower.contains(&normalized_query) {
        score += 80;
    }

    score -= path_depth(display_path) as i32 * 3;
    score -= (display_path.len() / 24) as i32;
    score
}

fn build_project_search_result(file: &FlatFileInfo) -> FileMentionSearchResult {
    FileMentionSearchResult {
        name: file.name.clone(),
        path: file.path.clone(),
        insert_path: file.relative_path.clone(),
        display_path: file.relative_path.clone(),
        node_type: file.node_type.clone(),
        extension: file.extension.clone(),
        scope: FileMentionScope::Project,
    }
}

fn metadata_to_file_node_type(metadata: &fs::Metadata) -> Option<FileNodeType> {
    if metadata.is_dir() {
        Some(FileNodeType::Directory)
    } else if metadata.is_file() {
        Some(FileNodeType::File)
    } else {
        None
    }
}

fn is_ignored_global_path(path: &Path) -> bool {
    let Some(name) = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
    else {
        return false;
    };

    IGNORED_DIRS.contains(&name.as_str()) || IGNORED_FILES.contains(&name.as_str())
}

fn rank_project_mentions(
    files: Vec<FlatFileInfo>,
    query: &str,
    limit: usize,
) -> Vec<FileMentionSearchResult> {
    let normalized_query = query.trim().to_lowercase();
    let mut ranked = files
        .into_iter()
        .filter(|file| {
            if normalized_query.is_empty() {
                return true;
            }

            let name = file.name.to_lowercase();
            let relative_path = file.relative_path.to_lowercase();
            name.contains(&normalized_query) || relative_path.contains(&normalized_query)
        })
        .map(|file| {
            let score = compute_search_score(&file.name, &file.relative_path, &normalized_query);
            RankedFileMentionResult {
                score,
                result: build_project_search_result(&file),
            }
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| {
                left.result
                    .display_path
                    .len()
                    .cmp(&right.result.display_path.len())
            })
            .then_with(|| left.result.display_path.cmp(&right.result.display_path))
    });

    ranked
        .into_iter()
        .take(limit)
        .map(|entry| entry.result)
        .collect()
}

#[allow(dead_code)]
fn build_global_search_result(path: &Path, node_type: FileNodeType) -> FileMentionSearchResult {
    let display_path = shorten_home_path(path);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string());

    let insert_path = path.to_string_lossy().to_string();

    FileMentionSearchResult {
        name: path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        path: path.to_string_lossy().to_string(),
        insert_path,
        display_path,
        node_type,
        extension,
        scope: FileMentionScope::Global,
    }
}

fn global_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(home_dir) = dirs::home_dir() {
        roots.push(home_dir);
    }

    for candidate in [
        "/Applications",
        "/System/Applications",
        "/Library",
        "/opt",
        "/usr/local",
        "/Users/Shared",
    ] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            roots.push(path);
        }
    }

    let mut seen = HashSet::new();
    roots
        .into_iter()
        .filter(|path| seen.insert(path.to_string_lossy().to_string()))
        .collect()
}

fn global_entry_from_path(path: &Path) -> Option<GlobalFileIndexEntry> {
    if is_ignored_global_path(path) {
        return None;
    }

    let metadata = fs::metadata(path).ok()?;
    let node_type = metadata_to_file_node_type(&metadata)?;
    let display_path = shorten_home_path(path);
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| display_path.clone());
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string());

    Some(GlobalFileIndexEntry {
        name,
        path: path.to_string_lossy().to_string(),
        display_path,
        node_type,
        extension,
    })
}

fn ensure_global_cache_initialized(cache: &mut GlobalFileSearchCache) {
    if cache.initialized {
        return;
    }

    for root in global_search_roots() {
        let root_key = root.to_string_lossy().to_string();
        if !cache.visited_dirs.insert(root_key.clone()) {
            continue;
        }

        if let Some(entry) = global_entry_from_path(&root) {
            cache.entries.push(entry);
        }
        cache.pending_dirs.push_back(root);
    }

    cache.initialized = true;
}

fn scan_global_cache_step(cache: &mut GlobalFileSearchCache, max_dirs: usize) {
    ensure_global_cache_initialized(cache);
    if cache.completed {
        return;
    }

    let mut scanned_dirs = 0usize;

    while scanned_dirs < max_dirs {
        let Some(dir_path) = cache.pending_dirs.pop_front() else {
            cache.completed = true;
            break;
        };

        let Ok(entries) = fs::read_dir(&dir_path) else {
            scanned_dirs += 1;
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if is_ignored_global_path(&path) {
                continue;
            }

            let Ok(metadata) = entry.metadata() else {
                continue;
            };

            if let Some(index_entry) = global_entry_from_path(&path) {
                cache.entries.push(index_entry);
            }

            if metadata.is_dir() {
                let dir_key = path.to_string_lossy().to_string();
                if cache.visited_dirs.insert(dir_key) {
                    cache.pending_dirs.push_back(path);
                }
            }
        }

        scanned_dirs += 1;
    }
}

fn search_global_cache_entries(
    cache: &GlobalFileSearchCache,
    query: &str,
    limit: usize,
) -> Vec<FileMentionSearchResult> {
    let normalized_query = query.trim().to_lowercase();

    let mut ranked = cache
        .entries
        .iter()
        .filter(|entry| {
            entry.name.to_lowercase().contains(&normalized_query)
                || entry
                    .display_path
                    .to_lowercase()
                    .contains(&normalized_query)
        })
        .map(|entry| RankedFileMentionResult {
            score: compute_search_score(&entry.name, &entry.display_path, &normalized_query),
            result: FileMentionSearchResult {
                name: entry.name.clone(),
                path: entry.path.clone(),
                insert_path: entry.path.clone(),
                display_path: entry.display_path.clone(),
                node_type: entry.node_type.clone(),
                extension: entry.extension.clone(),
                scope: FileMentionScope::Global,
            },
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| {
                left.result
                    .display_path
                    .len()
                    .cmp(&right.result.display_path.len())
            })
            .then_with(|| left.result.display_path.cmp(&right.result.display_path))
    });

    ranked
        .into_iter()
        .take(limit)
        .map(|entry| entry.result)
        .collect()
}

fn collect_global_fallback_results(query: &str, limit: usize) -> Vec<FileMentionSearchResult> {
    let normalized_query = query.trim().to_lowercase();
    if normalized_query.len() < 2 {
        return Vec::new();
    }

    let mut cache = GLOBAL_FILE_CACHE.lock();

    for _ in 0..4 {
        let current = search_global_cache_entries(&cache, &normalized_query, limit);
        if current.len() >= limit || cache.completed {
            return current;
        }

        scan_global_cache_step(&mut cache, 120);
    }

    search_global_cache_entries(&cache, &normalized_query, limit)
}

#[cfg(target_os = "macos")]
fn search_global_mentions_indexed(
    query: &str,
    limit: usize,
) -> Result<Vec<FileMentionSearchResult>, String> {
    let normalized_query = query.trim();
    if normalized_query.len() < 2 {
        return Ok(Vec::new());
    }

    let escaped_query = normalized_query.replace('\\', "\\\\").replace('"', "\\\"");
    let md_query = format!(
        "(kMDItemFSName == \"*{q}*\"cd || kMDItemPath == \"*{q}*\"cd)",
        q = escaped_query
    );

    let shell_query = md_query.replace('\'', "'\\''");
    let shell_command = format!("/usr/bin/mdfind -limit {} '{}'", limit, shell_query);

    let output = Command::new("/bin/sh")
        .args(["-lc", &shell_command])
        .output()
        .map_err(|error| format!("执行全局文件搜索失败: {}", error))?;

    if !output.status.success() {
        return Ok(collect_global_fallback_results(query, limit));
    }

    let mut seen = HashSet::new();
    let mut ranked = Vec::new();

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let path = PathBuf::from(trimmed);
        if !path.exists() || is_ignored_global_path(&path) {
            continue;
        }

        let path_key = path.to_string_lossy().to_string();
        if !seen.insert(path_key) {
            continue;
        }

        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };

        let Some(node_type) = metadata_to_file_node_type(&metadata) else {
            continue;
        };

        let result = build_global_search_result(&path, node_type);
        let score = compute_search_score(&result.name, &result.display_path, normalized_query);
        ranked.push(RankedFileMentionResult { score, result });
    }

    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| {
                left.result
                    .display_path
                    .len()
                    .cmp(&right.result.display_path.len())
            })
            .then_with(|| left.result.display_path.cmp(&right.result.display_path))
    });

    let results = ranked
        .into_iter()
        .take(limit)
        .map(|entry| entry.result)
        .collect::<Vec<_>>();

    if results.is_empty() {
        return Ok(collect_global_fallback_results(query, limit));
    }

    Ok(results)
}

#[cfg(not(target_os = "macos"))]
fn search_global_mentions_indexed(
    query: &str,
    limit: usize,
) -> Result<Vec<FileMentionSearchResult>, String> {
    Ok(collect_global_fallback_results(query, limit))
}

/// 列出项目所有文件的扁平列表（用于 @ 文件引用）
#[tauri::command]
pub fn list_all_project_files_flat(project_path: String) -> Result<Vec<FlatFileInfo>, String> {
    let resolved_path = resolve_path(&project_path)?;

    if !resolved_path.exists() {
        return Err(format!("项目路径不存在: {}", project_path));
    }

    if !resolved_path.is_dir() {
        return Err(format!("项目路径不是目录: {}", project_path));
    }

    get_project_file_index(&resolved_path)
}

#[tauri::command]
pub fn search_file_mentions(
    input: SearchFileMentionsInput,
) -> Result<Vec<FileMentionSearchResult>, String> {
    let limit = normalize_limit(input.limit);

    match input.scope {
        FileMentionScope::Project => {
            let project_path = input
                .project_path
                .ok_or_else(|| "项目范围搜索缺少 projectPath".to_string())?;
            let resolved_path = resolve_path(&project_path)?;

            if !resolved_path.exists() {
                return Err(format!("项目路径不存在: {}", project_path));
            }

            if !resolved_path.is_dir() {
                return Err(format!("项目路径不是目录: {}", project_path));
            }

            let files = get_project_file_index(&resolved_path)?;
            Ok(rank_project_mentions(files, &input.query, limit))
        }
        FileMentionScope::Global => search_global_mentions_indexed(&input.query, limit),
    }
}

#[tauri::command]
pub fn warm_project_file_index(project_path: String) -> Result<usize, String> {
    let resolved_path = resolve_path(&project_path)?;

    if !resolved_path.exists() {
        return Err(format!("项目路径不存在: {}", project_path));
    }

    if !resolved_path.is_dir() {
        return Err(format!("项目路径不是目录: {}", project_path));
    }

    let files = get_project_file_index(&resolved_path)?;
    Ok(files.len())
}

fn validate_create_entry_name(name: &str) -> Result<&str, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("名称不能为空".to_string());
    }

    if trimmed == "." || trimmed == ".." || trimmed.contains('/') || trimmed.contains('\\') {
        return Err("名称不能包含路径分隔符".to_string());
    }

    Ok(trimmed)
}

/// 在指定目录下创建文件或文件夹。
/// 参数：`parent_path` 为父目录，`name` 为新条目名称，`entry_type` 指定创建文件或目录。
/// 返回：文件操作结果，成功时附带新路径。
#[tauri::command]
pub fn create_entry(input: CreateEntryInput) -> Result<FileOperationResult, String> {
    let parent_path = resolve_path(&input.parent_path)?;
    let name = validate_create_entry_name(&input.name)?;

    if !parent_path.exists() {
        return Ok(FileOperationResult {
            success: false,
            message: Some(format!("父目录不存在: {}", input.parent_path)),
            new_path: None,
        });
    }

    if !parent_path.is_dir() {
        return Ok(FileOperationResult {
            success: false,
            message: Some("目标父路径不是目录".to_string()),
            new_path: None,
        });
    }

    let new_path = parent_path.join(name);
    if new_path.exists() {
        return Ok(FileOperationResult {
            success: false,
            message: Some(format!("已存在同名文件或目录: {}", name)),
            new_path: None,
        });
    }

    match input.entry_type {
        CreateEntryType::File => {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&new_path)
                .map_err(|e| format!("创建文件失败: {}", e))?;
        }
        CreateEntryType::Directory => {
            fs::create_dir(&new_path).map_err(|e| format!("创建目录失败: {}", e))?;
        }
    }

    invalidate_project_file_cache_for_paths(&[parent_path.as_path(), new_path.as_path()]);

    Ok(FileOperationResult {
        success: true,
        message: None,
        new_path: Some(new_path.to_string_lossy().to_string()),
    })
}

/// 重命名文件/文件夹
#[tauri::command]
pub fn rename_file(input: RenameFileInput) -> Result<FileOperationResult, String> {
    let old_path = resolve_path(&input.old_path)?;

    if !old_path.exists() {
        return Ok(FileOperationResult {
            success: false,
            message: Some(format!("文件或目录不存在: {}", input.old_path)),
            new_path: None,
        });
    }

    // 获取父目录和新路径
    let parent = old_path
        .parent()
        .ok_or_else(|| "无法获取父目录".to_string())?;
    let new_path = parent.join(&input.new_name);

    // 检查新名称是否为空
    if input.new_name.trim().is_empty() {
        return Ok(FileOperationResult {
            success: false,
            message: Some("名称不能为空".to_string()),
            new_path: None,
        });
    }

    // 检查目标是否已存在
    if new_path.exists() {
        return Ok(FileOperationResult {
            success: false,
            message: Some(format!("已存在同名文件或目录: {}", input.new_name)),
            new_path: None,
        });
    }

    // 执行重命名
    fs::rename(&old_path, &new_path).map_err(|e| format!("重命名失败: {}", e))?;
    invalidate_project_file_cache_for_paths(&[old_path.as_path(), parent.as_ref(), new_path.as_path()]);

    Ok(FileOperationResult {
        success: true,
        message: None,
        new_path: Some(new_path.to_string_lossy().to_string()),
    })
}

/// 删除单个文件/文件夹
#[tauri::command]
pub fn delete_file(path: String) -> Result<FileOperationResult, String> {
    let resolved_path = resolve_path(&path)?;

    if !resolved_path.exists() {
        return Ok(FileOperationResult {
            success: false,
            message: Some(format!("文件或目录不存在: {}", path)),
            new_path: None,
        });
    }

    let parent_path = resolved_path.parent().map(Path::to_path_buf);

    if resolved_path.is_dir() {
        // 递归删除目录
        fs::remove_dir_all(&resolved_path).map_err(|e| format!("删除目录失败: {}", e))?;
    } else {
        // 删除文件
        fs::remove_file(&resolved_path).map_err(|e| format!("删除文件失败: {}", e))?;
    }

    invalidate_project_file_cache_for_path(&resolved_path);
    if let Some(parent_path) = parent_path.as_ref() {
        invalidate_project_file_cache_for_path(parent_path);
    }

    Ok(FileOperationResult {
        success: true,
        message: None,
        new_path: None,
    })
}

/// 批量删除文件/文件夹
#[tauri::command]
pub fn batch_delete_files(input: BatchDeleteInput) -> Result<FileOperationResult, String> {
    let mut failed_count = 0;
    let mut error_messages = Vec::new();
    let mut touched_paths: Vec<PathBuf> = Vec::new();

    for path_str in input.paths {
        let resolved_path = resolve_path(&path_str)?;
        let parent_path = resolved_path.parent().map(Path::to_path_buf);

        if !resolved_path.exists() {
            failed_count += 1;
            error_messages.push(format!("文件或目录不存在: {}", path_str));
            continue;
        }

        if resolved_path.is_dir() {
            match fs::remove_dir_all(&resolved_path) {
                Ok(_) => {
                    touched_paths.push(resolved_path.clone());
                    if let Some(parent_path) = parent_path.clone() {
                        touched_paths.push(parent_path);
                    }
                }
                Err(e) => {
                    failed_count += 1;
                    error_messages.push(format!("删除目录 {} 失败: {}", path_str, e));
                }
            }
        } else {
            match fs::remove_file(&resolved_path) {
                Ok(_) => {
                    touched_paths.push(resolved_path.clone());
                    if let Some(parent_path) = parent_path.clone() {
                        touched_paths.push(parent_path);
                    }
                }
                Err(e) => {
                    failed_count += 1;
                    error_messages.push(format!("删除文件 {} 失败: {}", path_str, e));
                }
            }
        }
    }

    for path in &touched_paths {
        invalidate_project_file_cache_for_path(path);
    }

    if failed_count == 0 {
        Ok(FileOperationResult {
            success: true,
            message: None,
            new_path: None,
        })
    } else {
        Ok(FileOperationResult {
            success: false,
            message: Some(format!(
                "删除完成，但有 {} 项失败: {}",
                failed_count,
                error_messages.join("; ")
            )),
            new_path: None,
        })
    }
}

/// 移动文件/文件夹
#[tauri::command]
pub fn move_file(input: MoveFileInput) -> Result<FileOperationResult, String> {
    let source_path = resolve_path(&input.source_path)?;
    let target_path = resolve_path(&input.target_path)?;

    if !source_path.exists() {
        return Ok(FileOperationResult {
            success: false,
            message: Some(format!("源文件或目录不存在: {}", input.source_path)),
            new_path: None,
        });
    }

    if !target_path.exists() {
        return Ok(FileOperationResult {
            success: false,
            message: Some(format!("目标目录不存在: {}", input.target_path)),
            new_path: None,
        });
    }

    if !target_path.is_dir() {
        return Ok(FileOperationResult {
            success: false,
            message: Some("目标路径不是目录".to_string()),
            new_path: None,
        });
    }

    // 获取源文件名
    let file_name = source_path
        .file_name()
        .ok_or_else(|| "无法获取文件名".to_string())?;
    let new_path = target_path.join(file_name);
    let source_parent = source_path.parent().map(Path::to_path_buf);

    // 检查目标是否已存在同名文件
    if new_path.exists() {
        return Ok(FileOperationResult {
            success: false,
            message: Some(format!(
                "目标目录已存在同名文件或目录: {}",
                file_name.to_string_lossy()
            )),
            new_path: None,
        });
    }

    // 执行移动
    fs::rename(&source_path, &new_path).map_err(|e| format!("移动失败: {}", e))?;
    invalidate_project_file_cache_for_path(&source_path);
    if let Some(source_parent) = source_parent.as_ref() {
        invalidate_project_file_cache_for_path(source_parent);
    }
    invalidate_project_file_cache_for_path(&target_path);
    invalidate_project_file_cache_for_path(&new_path);

    Ok(FileOperationResult {
        success: true,
        message: None,
        new_path: Some(new_path.to_string_lossy().to_string()),
    })
}
