pub mod agent;
pub mod agent_config;
pub mod agent_profile;
pub mod agent_studio;
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
pub mod install;
pub mod lsp;
pub mod market_source_support;
pub mod marketplace;
pub mod mcp;
pub mod mcp_market;
pub(crate) mod mcp_shared;
pub mod mcpmarket_source;
pub mod memory;
pub mod message;
pub mod mini_panel;
#[cfg(target_os = "macos")]
pub(crate) mod mini_panel_macos_shortcut;
#[cfg(target_os = "windows")]
pub(crate) mod mini_panel_windows_shortcut;
pub mod plan;
pub mod plan_split;
pub mod plugins_market;
pub mod project;
pub mod project_access;
pub mod provider_profile;
pub mod scan;
pub(crate) mod scan_session_shared;
pub(crate) mod scan_shared;
pub mod session;
pub mod settings;
pub mod skill_plugin;
pub mod skills_market;
pub mod support;
pub mod task;
pub mod task_execution;
pub mod window;

use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// 获取持久化目录路径
pub fn get_persistence_dir_path() -> Result<PathBuf> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    Ok(home_dir.join(".easy-agent"))
}

/// 初始化持久化目录结构
pub fn init_persistence_dirs() -> Result<()> {
    let base_dir = get_persistence_dir_path()?;

    // 创建主目录
    fs::create_dir_all(&base_dir)?;

    // 创建子目录
    let sub_dirs = ["data", "logs", "cache", "data/session-uploads"];
    for dir in sub_dirs {
        fs::create_dir_all(base_dir.join(dir))?;
    }

    // LSP 统一存储目录
    fs::create_dir_all(base_dir.join("tools").join("lsp"))?;

    println!("Persistence directories initialized at: {:?}", base_dir);
    Ok(())
}

/// 获取持久化目录路径 (Tauri 命令)
#[tauri::command]
pub fn get_persistence_dir() -> Result<String, String> {
    get_persistence_dir_path()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

/// 检查数据库是否存在 (Tauri 命令)
#[tauri::command]
pub fn check_database_exists() -> Result<bool, String> {
    let db_path = get_persistence_dir_path()
        .map(|p| p.join("data").join("easy-agent.db"))
        .map_err(|e| e.to_string())?;

    Ok(db_path.exists())
}
