pub mod agent;
pub mod agent_cli_usage;
pub mod agent_config;
pub mod agent_team;
pub mod app_state;
pub mod builtin_mcp;
pub mod cli;
pub mod cli_config;
pub mod cli_installer;
pub(crate) mod cli_support;
pub mod conversation;
pub mod data;
pub mod file_editor;
pub(crate) mod git_install;
pub mod install;
pub mod lsp;
pub mod mcp;
pub(crate) mod mcp_shared;
pub mod memory;
pub mod message;
pub mod mini_panel;
#[cfg(target_os = "macos")]
pub(crate) mod mini_panel_macos_shortcut;
#[cfg(target_os = "windows")]
pub(crate) mod mini_panel_windows_shortcut;
pub mod plan;
pub mod plan_split;
pub mod project;
pub mod project_access;
pub mod provider_profile;
pub mod runtime_log;
pub mod scan;
pub(crate) mod scan_session_shared;
pub(crate) mod scan_shared;
pub mod session;
pub mod settings;
pub mod skill_plugin;
pub mod solo;
pub mod support;
pub mod task;
pub mod task_execution;
pub mod terminal;
pub mod unattended;
pub mod window;

use anyhow::Result;
use once_cell::sync::Lazy;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

static CUSTOM_PERSISTENCE_PATH: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

/// 获取开发模式下默认的持久化路径（当前项目下的 data 目录）
#[cfg(debug_assertions)]
fn get_dev_default_persistence_path() -> Option<PathBuf> {
    std::env::current_dir().ok().map(|cwd| cwd.join("data"))
}

/// 获取生产模式下默认的持久化路径（~/.easy-agent）
fn get_prod_default_persistence_path() -> Result<PathBuf> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    Ok(home_dir.join(".easy-agent"))
}

/// 获取持久化目录路径
pub fn get_persistence_dir_path() -> Result<PathBuf> {
    let custom = CUSTOM_PERSISTENCE_PATH
        .lock()
        .expect("persistence path poisoned");
    if let Some(ref path) = *custom {
        return Ok(path.clone());
    }
    drop(custom);

    #[cfg(debug_assertions)]
    {
        if let Some(dev_path) = get_dev_default_persistence_path() {
            return Ok(dev_path);
        }
    }

    get_prod_default_persistence_path()
}

/// 设置自定义持久化路径（仅在运行时切换）
pub fn set_persistence_dir_path(path: &PathBuf) {
    let mut custom = CUSTOM_PERSISTENCE_PATH
        .lock()
        .expect("persistence path poisoned");
    *custom = Some(path.clone());
}

/// 初始化持久化目录结构
pub fn init_persistence_dirs() -> Result<()> {
    let base_dir = get_persistence_dir_path()?;

    fs::create_dir_all(&base_dir)?;

    let sub_dirs = ["data", "logs", "cache", "data/session-uploads"];
    for dir in sub_dirs {
        fs::create_dir_all(base_dir.join(dir))?;
    }

    fs::create_dir_all(base_dir.join("tools").join("lsp"))?;

    crate::logging::write_log(
        "INFO",
        "bootstrap",
        &format!(
            "Persistence directories initialized at: {}",
            base_dir.display()
        ),
    );
    Ok(())
}

#[tauri::command]
pub fn get_persistence_dir() -> Result<String, String> {
    get_persistence_dir_path()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn check_database_exists() -> Result<bool, String> {
    let db_path = get_persistence_dir_path()
        .map(|p| p.join("data").join("easy-agent.db"))
        .map_err(|e| e.to_string())?;

    Ok(db_path.exists())
}

#[derive(Debug, serde::Serialize)]
pub struct PersistenceMigrationResult {
    pub success: bool,
    pub old_path: String,
    pub new_path: String,
    pub migrated_files: usize,
    pub message: String,
}

#[tauri::command]
pub fn migrate_persistence_path(new_path: String) -> Result<PersistenceMigrationResult, String> {
    let new_dir = PathBuf::from(&new_path);
    if !new_dir.exists() {
        fs::create_dir_all(&new_dir).map_err(|e| format!("创建目标目录失败: {}", e))?;
    }

    let old_dir = get_persistence_dir_path().map_err(|e| e.to_string())?;
    let old_dir_str = old_dir.to_string_lossy().to_string();
    let new_dir_str = new_dir.to_string_lossy().to_string();

    if old_dir_str == new_dir_str {
        return Ok(PersistenceMigrationResult {
            success: true,
            old_path: old_dir_str,
            new_path: new_dir_str,
            migrated_files: 0,
            message: "路径相同，无需迁移".to_string(),
        });
    }

    let sub_dirs = ["data", "logs", "cache", "tools", "data/session-uploads"];
    for dir in sub_dirs {
        let target = new_dir.join(dir);
        if !target.exists() {
            fs::create_dir_all(&target).map_err(|e| format!("创建子目录 {} 失败: {}", dir, e))?;
        }
    }

    let mut migrated = 0usize;
    let entries_to_copy = ["data", "logs", "cache", "tools"];
    for entry_name in entries_to_copy {
        let src = old_dir.join(entry_name);
        let dst = new_dir.join(entry_name);
        if src.exists() {
            if let Err(e) = copy_dir_recursive(&src, &dst, &mut migrated) {
                return Err(format!("迁移 {} 失败: {}", entry_name, e));
            }
        }
    }

    set_persistence_dir_path(&new_dir);

    crate::logging::write_log(
        "INFO",
        "migration",
        &format!(
            "Persistence migrated: {} -> {}, {} files",
            old_dir_str, new_dir_str, migrated
        ),
    );

    Ok(PersistenceMigrationResult {
        success: true,
        old_path: old_dir_str,
        new_path: new_dir_str,
        migrated_files: migrated,
        message: format!("成功迁移 {} 个文件", migrated),
    })
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf, count: &mut usize) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path, count)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
            *count += 1;
        }
    }
    Ok(())
}
