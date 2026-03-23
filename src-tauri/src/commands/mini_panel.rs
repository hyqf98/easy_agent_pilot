use anyhow::Result;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

use super::support::{now_rfc3339, open_db_connection, open_db_connection_with_foreign_keys};

pub const MINI_PANEL_WINDOW_LABEL: &str = "mini-panel";
pub const MINI_PANEL_PROJECT_ID_KEY: &str = "miniPanelProjectId";
pub const MINI_PANEL_SESSION_ID_KEY: &str = "miniPanelSessionId";
pub const MINI_PANEL_WORKING_DIRECTORY_KEY: &str = "miniPanelWorkingDirectory";

const MINI_PANEL_PROJECT_NAME: &str = "__EasyAgent Mini Panel__";
const MINI_PANEL_SESSION_NAME: &str = "Mini Panel";
const MINI_PANEL_PROJECT_DESCRIPTION: &str = "__ea_system_mini_panel__";
const MINI_PANEL_DEFAULT_AGENT_TYPE: &str = "claude";

#[cfg(target_os = "windows")]
const MINI_PANEL_DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+Space";

#[cfg(not(target_os = "windows"))]
const MINI_PANEL_DEFAULT_SHORTCUT: &str = "Alt+Space";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniPanelState {
    pub project_id: String,
    pub session_id: String,
    pub working_directory: String,
    pub default_shortcut: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniPanelDirectoryResult {
    pub working_directory: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestMiniPanelDirectoriesInput {
    pub current_directory: Option<String>,
    pub partial_path: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniPanelDirectorySuggestion {
    pub value: String,
    pub display_value: String,
    pub insert_value: String,
}

fn set_setting(conn: &rusqlite::Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
        [key, value, &now_rfc3339()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn get_setting(conn: &rusqlite::Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [key],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn home_directory() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "无法获取用户主目录".to_string())
}

fn mini_panel_project_path() -> Result<PathBuf, String> {
    let path = super::get_persistence_dir_path()
        .map_err(|e| e.to_string())?
        .join("data")
        .join("mini-panel-workspace");
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

fn resolve_path_input(input: &str, current_directory: &Path) -> Result<PathBuf, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(current_directory.to_path_buf());
    }

    let candidate = if let Some(stripped) = trimmed.strip_prefix('~') {
        let home = home_directory()?;
        let rest = stripped.strip_prefix('/').unwrap_or(stripped);
        home.join(rest)
    } else {
        let path = PathBuf::from(trimmed);
        if path.is_absolute() {
            path
        } else {
            current_directory.join(path)
        }
    };

    let resolved = candidate
        .canonicalize()
        .map_err(|e| format!("无法切换到路径: {}", e))?;

    if !resolved.is_dir() {
        return Err("路径存在但不是目录".to_string());
    }

    Ok(resolved)
}

fn preferred_separator(input: &str) -> char {
    if input.contains('\\') {
        '\\'
    } else {
        std::path::MAIN_SEPARATOR
    }
}

fn normalize_display_path(path: &str, separator: char) -> String {
    if separator == '\\' {
        path.replace('/', "\\")
    } else {
        path.replace('\\', "/")
    }
}

fn resolve_partial_path_context(
    partial_path: &str,
    current_directory: &Path,
) -> Result<(PathBuf, String, char), String> {
    let trimmed = partial_path.trim();
    let separator = preferred_separator(trimmed);

    if trimmed.is_empty() {
        return Ok((current_directory.to_path_buf(), String::new(), separator));
    }

    let expanded = if let Some(stripped) = trimmed.strip_prefix('~') {
        let home = home_directory()?;
        let rest = stripped.strip_prefix('/').unwrap_or(stripped);
        home.join(rest)
    } else {
        let path = PathBuf::from(trimmed);
        if path.is_absolute() {
            path
        } else {
            current_directory.join(path)
        }
    };

    let ends_with_separator = trimmed.ends_with('/') || trimmed.ends_with('\\');
    if ends_with_separator {
        return Ok((expanded, String::new(), separator));
    }

    let parent = expanded
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| current_directory.to_path_buf());
    let prefix = expanded
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok((parent, prefix, separator))
}

fn build_suggestion_value(
    base_directory: &Path,
    full_path: &Path,
    partial_path: &str,
    separator: char,
) -> String {
    let trimmed = partial_path.trim();

    let raw = if trimmed.starts_with('~') {
        if let Ok(home) = home_directory() {
            if let Ok(relative) = full_path.strip_prefix(&home) {
                let relative = normalize_display_path(&relative.to_string_lossy(), separator);
                if relative.is_empty() {
                    "~".to_string()
                } else {
                    format!("~{}{}", separator, relative)
                }
            } else {
                normalize_display_path(&full_path.to_string_lossy(), separator)
            }
        } else {
            normalize_display_path(&full_path.to_string_lossy(), separator)
        }
    } else if PathBuf::from(trimmed).is_absolute() {
        normalize_display_path(&full_path.to_string_lossy(), separator)
    } else if let Ok(relative) = full_path.strip_prefix(base_directory) {
        normalize_display_path(&relative.to_string_lossy(), separator)
    } else {
        normalize_display_path(&full_path.to_string_lossy(), separator)
    };

    if raw.ends_with(separator) {
        raw
    } else {
        format!("{}{}", raw, separator)
    }
}

fn project_exists(conn: &rusqlite::Connection, project_id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM projects WHERE id = ?1 LIMIT 1",
        [project_id],
        |_row| Ok(()),
    )
    .optional()
    .map(|result| result.is_some())
    .map_err(|e| e.to_string())
}

fn session_exists(conn: &rusqlite::Connection, session_id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM sessions WHERE id = ?1 LIMIT 1",
        [session_id],
        |_row| Ok(()),
    )
    .optional()
    .map(|result| result.is_some())
    .map_err(|e| e.to_string())
}

fn ensure_mini_panel_project(conn: &rusqlite::Connection) -> Result<String, String> {
    if let Some(project_id) = get_setting(conn, MINI_PANEL_PROJECT_ID_KEY)? {
        if project_exists(conn, &project_id)? {
            return Ok(project_id);
        }
    }

    let project_id = Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let path = mini_panel_project_path()?;
    let path_str = path.to_string_lossy().to_string();

    conn.execute(
        "INSERT INTO projects (id, name, path, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            &project_id,
            MINI_PANEL_PROJECT_NAME,
            &path_str,
            MINI_PANEL_PROJECT_DESCRIPTION,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    set_setting(conn, MINI_PANEL_PROJECT_ID_KEY, &project_id)?;
    Ok(project_id)
}

fn ensure_mini_panel_session(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> Result<String, String> {
    if let Some(session_id) = get_setting(conn, MINI_PANEL_SESSION_ID_KEY)? {
        if session_exists(conn, &session_id)? {
            return Ok(session_id);
        }
    }

    let session_id = Uuid::new_v4().to_string();
    let now = now_rfc3339();

    conn.execute(
        "INSERT INTO sessions (id, project_id, name, agent_type, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 'idle', ?5, ?6)",
        rusqlite::params![
            &session_id,
            project_id,
            MINI_PANEL_SESSION_NAME,
            MINI_PANEL_DEFAULT_AGENT_TYPE,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    set_setting(conn, MINI_PANEL_SESSION_ID_KEY, &session_id)?;
    Ok(session_id)
}

fn ensure_working_directory(conn: &rusqlite::Connection) -> Result<String, String> {
    let home = home_directory()?;
    let home_str = home.to_string_lossy().to_string();
    let stored = get_setting(conn, MINI_PANEL_WORKING_DIRECTORY_KEY)?;
    let next = stored
        .as_ref()
        .filter(|path| Path::new(path).is_dir())
        .cloned()
        .unwrap_or_else(|| home_str.clone());

    if stored.as_deref() != Some(next.as_str()) {
        set_setting(conn, MINI_PANEL_WORKING_DIRECTORY_KEY, &next)?;
    }

    Ok(next)
}

fn ensure_mini_panel_state_internal(conn: &rusqlite::Connection) -> Result<MiniPanelState, String> {
    let project_id = ensure_mini_panel_project(conn)?;
    let session_id = ensure_mini_panel_session(conn, &project_id)?;
    let working_directory = ensure_working_directory(conn)?;

    Ok(MiniPanelState {
        project_id,
        session_id,
        working_directory,
        default_shortcut: MINI_PANEL_DEFAULT_SHORTCUT.to_string(),
    })
}

fn ensure_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(MINI_PANEL_WINDOW_LABEL) {
        return Ok(window);
    }

    WebviewWindowBuilder::new(
        app,
        MINI_PANEL_WINDOW_LABEL,
        WebviewUrl::App("/mini-panel".into()),
    )
    .title("Easy Agent Mini Panel")
    .inner_size(920.0, 680.0)
    .min_inner_size(720.0, 520.0)
    .resizable(true)
    .visible(false)
    .always_on_top(true)
    .build()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ensure_mini_panel_state() -> Result<MiniPanelState, String> {
    let conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;
    ensure_mini_panel_state_internal(&conn)
}

#[tauri::command]
pub fn set_mini_panel_working_directory(
    path: String,
    current_directory: Option<String>,
) -> Result<MiniPanelDirectoryResult, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let base = current_directory
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(home_directory()?);
    let resolved = resolve_path_input(&path, &base)?;
    let resolved_str = resolved.to_string_lossy().to_string();

    set_setting(&conn, MINI_PANEL_WORKING_DIRECTORY_KEY, &resolved_str)?;

    Ok(MiniPanelDirectoryResult {
        working_directory: resolved_str,
    })
}

#[tauri::command]
pub fn get_mini_panel_default_shortcut() -> Result<String, String> {
    Ok(MINI_PANEL_DEFAULT_SHORTCUT.to_string())
}

#[tauri::command]
pub fn suggest_mini_panel_directories(
    input: SuggestMiniPanelDirectoriesInput,
) -> Result<Vec<MiniPanelDirectorySuggestion>, String> {
    let base_directory = input
        .current_directory
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(home_directory()?);
    let (search_directory, prefix, separator) =
        resolve_partial_path_context(&input.partial_path, &base_directory)?;

    if !search_directory.exists() || !search_directory.is_dir() {
        return Ok(Vec::new());
    }

    let include_hidden = prefix.starts_with('.');
    let normalized_prefix = prefix.to_lowercase();
    let mut entries = fs::read_dir(&search_directory)
        .map_err(|e| format!("无法读取目录: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return false;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            if !include_hidden && name.starts_with('.') {
                return false;
            }

            normalized_prefix.is_empty() || name.to_lowercase().starts_with(&normalized_prefix)
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| left.file_name().cmp(&right.file_name()));

    let limit = input.limit.unwrap_or(24).clamp(1, 80);
    Ok(entries
        .into_iter()
        .take(limit)
        .map(|path| {
            let value =
                build_suggestion_value(&base_directory, &path, &input.partial_path, separator);
            let display_value = path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| value.clone());

            MiniPanelDirectorySuggestion {
                value: value.clone(),
                display_value: format!("{}{}", display_value, separator),
                insert_value: value,
            }
        })
        .collect())
}

#[tauri::command]
pub fn register_mini_panel_windows_shortcut(
    app: AppHandle,
    shortcut: String,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        super::mini_panel_windows_shortcut::register_shortcut(app, shortcut)
    }

    #[cfg(target_os = "macos")]
    {
        super::mini_panel_macos_shortcut::register_shortcut(app, shortcut)
    }

    #[cfg(not(target_os = "windows"))]
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, shortcut);
        Err("NATIVE_SHORTCUT_OVERRIDE_UNSUPPORTED".to_string())
    }
}

#[tauri::command]
pub fn unregister_mini_panel_windows_shortcut() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        super::mini_panel_windows_shortcut::unregister_shortcut()
    }

    #[cfg(target_os = "macos")]
    {
        super::mini_panel_macos_shortcut::unregister_shortcut()
    }

    #[cfg(not(target_os = "windows"))]
    #[cfg(not(target_os = "macos"))]
    {
        Err("NATIVE_SHORTCUT_OVERRIDE_UNSUPPORTED".to_string())
    }
}

#[tauri::command]
pub fn capture_mini_panel_native_shortcut_once(timeout_ms: Option<u64>) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        super::mini_panel_macos_shortcut::capture_shortcut_once(timeout_ms)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = timeout_ms;
        Err("NATIVE_SHORTCUT_OVERRIDE_UNSUPPORTED".to_string())
    }
}

#[tauri::command]
pub fn show_mini_panel(app: AppHandle) -> Result<(), String> {
    let window = ensure_window(&app)?;
    window.center().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    window
        .emit("mini-panel:focus-input", true)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn hide_mini_panel(app: AppHandle) -> Result<(), String> {
    let main_was_visible = app
        .get_webview_window("main")
        .map(|window| window.is_visible().map_err(|e| e.to_string()))
        .transpose()?
        .unwrap_or(false);

    if let Some(window) = app.get_webview_window(MINI_PANEL_WINDOW_LABEL) {
        window.hide().map_err(|e| e.to_string())?;
    }

    if !main_was_visible {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
    }

    Ok(())
}

#[tauri::command]
pub fn toggle_mini_panel(app: AppHandle) -> Result<bool, String> {
    let window = ensure_window(&app)?;
    let visible = window.is_visible().map_err(|e| e.to_string())?;

    if visible {
        let main_was_visible = app
            .get_webview_window("main")
            .map(|main_window| main_window.is_visible().map_err(|e| e.to_string()))
            .transpose()?
            .unwrap_or(false);

        window.hide().map_err(|e| e.to_string())?;

        if !main_was_visible {
            if let Some(main_window) = app.get_webview_window("main") {
                let _ = main_window.hide();
            }
        }

        return Ok(false);
    }

    window.center().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    window
        .emit("mini-panel:focus-input", true)
        .map_err(|e| e.to_string())?;
    Ok(true)
}
