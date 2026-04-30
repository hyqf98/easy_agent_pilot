use anyhow::Result;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

use crate::commands::cli_config::{
    get_cli_config_paths_internal, read_cli_config, write_cli_config, McpServerConfig,
};
use crate::commands::mcpmarket_source::{
    get_marketplace_source_strategy, MarketListResponse, MarketplaceSourceQuery, McpSourceDetail,
    McpSourceListItem,
};

pub type McpMarketItem = McpSourceListItem;
pub type McpMarketDetail = McpSourceDetail;
pub type McpMarketListResponse = MarketListResponse<McpMarketItem>;
pub type McpMarketQuery = MarketplaceSourceQuery;

fn sanitize_mcp_key(name: &str) -> String {
    let sanitized = name
        .trim()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "")
        .to_lowercase();

    if sanitized.is_empty() {
        "mcp-server".to_string()
    } else {
        sanitized
    }
}

fn build_backup_path(config_path: &Path) -> PathBuf {
    match config_path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if !ext.is_empty() => config_path.with_extension(format!("{}.backup", ext)),
        _ => config_path.with_extension("backup"),
    }
}

fn create_config_backup(config_path: &Path) -> Result<Option<String>, String> {
    if !config_path.exists() {
        return Ok(None);
    }

    let backup_path = build_backup_path(config_path);
    fs::copy(config_path, &backup_path).map_err(|e| format!("创建备份失败: {}", e))?;
    Ok(Some(backup_path.to_string_lossy().to_string()))
}

fn resolve_cli_identifier_from_config_path(config_path: &str) -> Result<String, String> {
    for cli in ["claude", "codex", "opencode"] {
        let paths = get_cli_config_paths_internal(cli, None)?;
        if paths.config_file == config_path {
            return Ok(cli.to_string());
        }
    }

    Err(format!("Unsupported MCP config path: {}", config_path))
}

fn build_installed_mcp_config(
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
) -> McpServerConfig {
    McpServerConfig {
        command: Some(command),
        args: (!args.is_empty()).then_some(args),
        env: (!env.is_empty()).then_some(env),
        url: None,
        headers: None,
        disabled: false,
    }
}

fn resolve_project_root(project_path: Option<&str>) -> Result<PathBuf, String> {
    let raw_path = project_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "项目级安装需要指定项目路径".to_string())?;
    let path = PathBuf::from(raw_path);

    if !path.exists() {
        return Err(format!("项目路径不存在: {}", raw_path));
    }
    if !path.is_dir() {
        return Err(format!("项目路径不是目录: {}", raw_path));
    }

    Ok(path)
}

fn resolve_project_mcp_config_path(cli_type: &str, project_root: &Path) -> Result<PathBuf, String> {
    match cli_type {
        "claude" => Ok(project_root.join(".mcp.json")),
        "opencode" => Ok(project_root.join("opencode.json")),
        "codex" => {
            Err("Codex CLI 当前没有可用的官方项目级 MCP 配置文件，暂仅支持全局安装".to_string())
        }
        other => Err(format!("当前 CLI 暂不支持项目级 MCP 安装: {}", other)),
    }
}

fn read_json_mcp_config(
    config_path: &Path,
) -> Result<crate::commands::cli_config::ClaudeCliConfig, String> {
    if !config_path.exists() {
        return Ok(crate::commands::cli_config::ClaudeCliConfig::default());
    }

    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))
}

fn write_json_mcp_config(
    config_path: &Path,
    config: crate::commands::cli_config::ClaudeCliConfig,
) -> Result<(), String> {
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    fs::write(config_path, content).map_err(|e| format!("Failed to write config file: {}", e))
}

fn read_opencode_project_mcp_config(
    config_path: &Path,
) -> Result<crate::commands::cli_config::ClaudeCliConfig, String> {
    if !config_path.exists() {
        return Ok(crate::commands::cli_config::ClaudeCliConfig::default());
    }

    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read opencode config: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::json!({}));

    let mut mcp_servers = HashMap::new();
    if let Some(mcp_obj) = json.get("mcp").and_then(|value| value.as_object()) {
        for (name, value) in mcp_obj {
            let server_type = value
                .get("type")
                .and_then(|item| item.as_str())
                .unwrap_or("local");

            let (command, args, env, url, headers, disabled) = if server_type == "remote" {
                let url = value
                    .get("url")
                    .and_then(|item| item.as_str())
                    .map(|item| item.to_string());
                let headers =
                    value
                        .get("headers")
                        .and_then(|item| item.as_object())
                        .map(|object| {
                            object
                                .iter()
                                .filter_map(|(key, item)| {
                                    item.as_str().map(|entry| (key.clone(), entry.to_string()))
                                })
                                .collect()
                        });
                (
                    None,
                    None,
                    None,
                    url,
                    headers,
                    !value
                        .get("enabled")
                        .and_then(|item| item.as_bool())
                        .unwrap_or(true),
                )
            } else {
                let command_array = value.get("command").and_then(|item| item.as_array());
                let command = command_array
                    .and_then(|items| items.first())
                    .and_then(|item| item.as_str())
                    .map(|item| item.to_string());
                let args = command_array.map(|items| {
                    items
                        .iter()
                        .skip(1)
                        .filter_map(|item| item.as_str().map(|entry| entry.to_string()))
                        .collect()
                });
                let env = value
                    .get("environment")
                    .and_then(|item| item.as_object())
                    .map(|object| {
                        object
                            .iter()
                            .filter_map(|(key, item)| {
                                item.as_str().map(|entry| (key.clone(), entry.to_string()))
                            })
                            .collect()
                    });
                (
                    command,
                    args,
                    env,
                    None,
                    None,
                    !value
                        .get("enabled")
                        .and_then(|item| item.as_bool())
                        .unwrap_or(true),
                )
            };

            mcp_servers.insert(
                name.clone(),
                McpServerConfig {
                    command,
                    args,
                    env,
                    url,
                    headers,
                    disabled,
                },
            );
        }
    }

    let mut other = std::collections::BTreeMap::new();
    if let Some(object) = json.as_object() {
        for (key, value) in object {
            if key != "mcp" {
                other.insert(key.clone(), value.clone());
            }
        }
    }

    Ok(crate::commands::cli_config::ClaudeCliConfig {
        mcpServers: (!mcp_servers.is_empty()).then_some(mcp_servers),
        other,
    })
}

fn write_opencode_project_mcp_config(
    config_path: &Path,
    config: crate::commands::cli_config::ClaudeCliConfig,
) -> Result<(), String> {
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let mut json = if config_path.exists() {
        let content = fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read opencode config: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let mcp_servers = config.mcpServers.unwrap_or_default();
    if !mcp_servers.is_empty() {
        let mut mcp_obj = serde_json::Map::new();
        for (name, server) in &mcp_servers {
            let mut entry = serde_json::Map::new();
            if let Some(url) = &server.url {
                entry.insert("type".to_string(), serde_json::json!("remote"));
                entry.insert("url".to_string(), serde_json::json!(url));
                if let Some(headers) = &server.headers {
                    entry.insert("headers".to_string(), serde_json::json!(headers));
                }
            } else {
                entry.insert("type".to_string(), serde_json::json!("local"));
                let mut command = Vec::new();
                if let Some(binary) = &server.command {
                    command.push(serde_json::json!(binary));
                }
                if let Some(args) = &server.args {
                    for arg in args {
                        command.push(serde_json::json!(arg));
                    }
                }
                entry.insert("command".to_string(), serde_json::json!(command));
                if let Some(env) = &server.env {
                    entry.insert("environment".to_string(), serde_json::json!(env));
                }
            }
            entry.insert("enabled".to_string(), serde_json::json!(!server.disabled));
            mcp_obj.insert(name.clone(), serde_json::Value::Object(entry));
        }
        if let Some(object) = json.as_object_mut() {
            object.insert("mcp".to_string(), serde_json::Value::Object(mcp_obj));
        }
    } else if let Some(object) = json.as_object_mut() {
        object.remove("mcp");
    }

    let content = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize opencode config: {}", e))?;
    fs::write(config_path, content).map_err(|e| format!("Failed to write opencode config: {}", e))
}

fn read_project_mcp_config(
    cli_type: &str,
    config_path: &Path,
) -> Result<crate::commands::cli_config::ClaudeCliConfig, String> {
    match cli_type {
        "claude" => read_json_mcp_config(config_path),
        "opencode" => read_opencode_project_mcp_config(config_path),
        "codex" => {
            Err("Codex CLI 当前没有可用的官方项目级 MCP 配置文件，暂仅支持全局安装".to_string())
        }
        other => Err(format!("当前 CLI 暂不支持项目级 MCP 安装: {}", other)),
    }
}

fn write_project_mcp_config(
    cli_type: &str,
    config_path: &Path,
    config: crate::commands::cli_config::ClaudeCliConfig,
) -> Result<(), String> {
    match cli_type {
        "claude" => write_json_mcp_config(config_path, config),
        "opencode" => write_opencode_project_mcp_config(config_path, config),
        "codex" => {
            Err("Codex CLI 当前没有可用的官方项目级 MCP 配置文件，暂仅支持全局安装".to_string())
        }
        other => Err(format!("当前 CLI 暂不支持项目级 MCP 安装: {}", other)),
    }
}

fn perform_rollback(settings_path: &Path, backup_path: &Option<String>) -> (bool, String) {
    if let Some(backup) = backup_path {
        let backup = PathBuf::from(backup);
        if backup.exists() {
            match fs::copy(&backup, settings_path) {
                Ok(_) => {
                    let _ = fs::remove_file(&backup);
                    (true, "已恢复备份文件".to_string())
                }
                Err(e) => (false, format!("恢复备份失败: {}", e)),
            }
        } else {
            (false, "备份文件不存在".to_string())
        }
    } else if settings_path.exists() {
        match fs::remove_file(settings_path) {
            Ok(_) => (true, "已删除新创建的配置文件".to_string()),
            Err(e) => (false, format!("删除配置文件失败: {}", e)),
        }
    } else {
        (true, "无需回滚（文件不存在）".to_string())
    }
}

fn delete_installed_mcp_test_result(config_path: &str, mcp_name: &str) {
    if let Ok(conn) = get_db_connection() {
        let _ = conn.execute(
            "DELETE FROM installed_mcp_test_results WHERE config_path = ?1 AND mcp_name = ?2",
            rusqlite::params![config_path, mcp_name],
        );
    }
}

/// Fetch MCP market items from the active marketplace source.
#[tauri::command]
pub async fn fetch_mcp_market(
    app: AppHandle,
    query: McpMarketQuery,
) -> Result<McpMarketListResponse, String> {
    let strategy = get_marketplace_source_strategy(query.source_market.as_deref())?;
    strategy.fetch_mcp_list(&app, &query).await
}

/// Fetch MCP market detail by slug.
#[tauri::command]
pub async fn fetch_mcp_market_detail(
    app: AppHandle,
    mcp_id: String,
    source_market: Option<String>,
) -> Result<McpMarketDetail, String> {
    let strategy = get_marketplace_source_strategy(source_market.as_deref())?;
    strategy.fetch_mcp_detail(&app, &mcp_id).await
}

/// MCP installation input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInstallInput {
    pub mcp_id: String,
    pub mcp_name: String,
    pub cli_path: String,
    pub scope: String,
    pub project_path: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// MCP installation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInstallResult {
    pub success: bool,
    pub message: String,
    pub config_path: Option<String>,
    pub backup_path: Option<String>,
    pub rollback_performed: bool,
    pub rollback_message: Option<String>,
}

/// Install MCP to Claude/Codex CLI config.
#[tauri::command]
pub async fn install_mcp_to_cli(input: McpInstallInput) -> Result<McpInstallResult, String> {
    let paths = get_cli_config_paths_internal(&input.cli_path, None)?;
    let config_path = if input.scope == "project" {
        let project_root = resolve_project_root(input.project_path.as_deref())?;
        resolve_project_mcp_config_path(&paths.cli_type, &project_root)?
    } else {
        PathBuf::from(&paths.config_file)
    };
    let backup_path = create_config_backup(&config_path)?;

    let mut config = if input.scope == "project" {
        read_project_mcp_config(&paths.cli_type, &config_path)?
    } else {
        read_cli_config(input.cli_path.clone(), None)?
    };
    if config.mcpServers.is_none() {
        config.mcpServers = Some(HashMap::new());
    }

    if let Some(ref mut servers) = config.mcpServers {
        servers.insert(
            sanitize_mcp_key(&input.mcp_name),
            build_installed_mcp_config(
                input.command.clone(),
                input.args.clone(),
                input.env.clone(),
            ),
        );
    }

    let write_result = if input.scope == "project" {
        write_project_mcp_config(&paths.cli_type, &config_path, config)
    } else {
        write_cli_config(input.cli_path.clone(), None, config)
    };

    if let Err(error) = write_result {
        let (rollback_success, rollback_message) = perform_rollback(&config_path, &backup_path);
        return Ok(McpInstallResult {
            success: false,
            message: error,
            config_path: Some(config_path.to_string_lossy().to_string()),
            backup_path: None,
            rollback_performed: rollback_success,
            rollback_message: Some(rollback_message),
        });
    }

    let config_path_str = config_path.to_string_lossy().to_string();
    if let Err(error) = save_install_history(
        input.mcp_id.clone(),
        sanitize_mcp_key(&input.mcp_name),
        input.cli_path.clone(),
        config_path_str.clone(),
        backup_path.clone(),
        input.scope.clone(),
    ) {
        eprintln!("Warning: failed to save MCP install history: {}", error);
    }

    Ok(McpInstallResult {
        success: true,
        message: format!("成功安装 {} 到 {}", input.mcp_name, paths.cli_type),
        config_path: Some(config_path_str),
        backup_path,
        rollback_performed: false,
        rollback_message: None,
    })
}

/// Rollback MCP installation
#[tauri::command]
pub async fn rollback_mcp_install(
    config_path: String,
    backup_path: Option<String>,
) -> Result<McpInstallResult, String> {
    let settings_path = PathBuf::from(&config_path);

    if let Some(backup) = &backup_path {
        let backup = PathBuf::from(backup);
        if backup.exists() {
            fs::copy(&backup, &settings_path).map_err(|e| format!("恢复备份失败: {}", e))?;
            fs::remove_file(&backup).map_err(|e| format!("删除备份文件失败: {}", e))?;

            return Ok(McpInstallResult {
                success: true,
                message: "安装已成功回滚".to_string(),
                config_path: Some(config_path),
                backup_path: None,
                rollback_performed: true,
                rollback_message: Some("已恢复备份文件并清理".to_string()),
            });
        }
    }

    if settings_path.exists() {
        fs::remove_file(&settings_path).map_err(|e| format!("删除配置文件失败: {}", e))?;
    }

    Ok(McpInstallResult {
        success: true,
        message: "安装已回滚（配置文件已删除）".to_string(),
        config_path: Some(config_path),
        backup_path: None,
        rollback_performed: true,
        rollback_message: Some("已删除新创建的配置文件".to_string()),
    })
}

/// Installed MCP item (from CLI config file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMcp {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub disabled: bool,
    pub source_cli: String,
    pub source_cli_path: String,
    pub config_path: String,
    pub scope: String,
    pub installed_at: Option<String>,
    pub has_update: bool,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub tool_count: Option<i32>,
}

struct CliConfigInfo {
    cli_name: String,
    cli_path: String,
    config_path: PathBuf,
    scope: String,
}

fn get_all_cli_configs() -> Vec<CliConfigInfo> {
    ["claude", "codex", "opencode"]
        .iter()
        .filter_map(|cli| {
            let paths = get_cli_config_paths_internal(cli, None).ok()?;
            let config_path = PathBuf::from(&paths.config_file);
            config_path.exists().then_some(CliConfigInfo {
                cli_name: (*cli).to_string(),
                cli_path: (*cli).to_string(),
                config_path,
                scope: "global".to_string(),
            })
        })
        .collect()
}

/// List all installed MCPs from Claude/Codex CLI config files.
#[tauri::command]
pub fn list_installed_mcps() -> Result<Vec<InstalledMcp>, String> {
    let mut installed_mcps = Vec::new();

    for config_info in get_all_cli_configs() {
        let cli_config = match read_cli_config(config_info.cli_path.clone(), None) {
            Ok(config) => config,
            Err(error) => {
                eprintln!(
                    "Failed to read MCP config for {}: {}",
                    config_info.cli_name, error
                );
                continue;
            }
        };

        let installed_at = fs::metadata(&config_info.config_path)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .map(|time| {
                let datetime: chrono::DateTime<chrono::Utc> = time.into();
                datetime.to_rfc3339()
            });

        for (name, config) in cli_config.mcpServers.unwrap_or_default() {
            let command = config.command.unwrap_or_default();
            let args = config.args.unwrap_or_default();
            let env = config.env.unwrap_or_default();
            let disabled = config.disabled;
            let current_version = extract_version_from_args(&args);

            installed_mcps.push(InstalledMcp {
                name,
                command,
                args,
                env,
                disabled,
                source_cli: config_info.cli_name.clone(),
                source_cli_path: config_info.cli_path.clone(),
                config_path: config_info.config_path.to_string_lossy().to_string(),
                scope: config_info.scope.clone(),
                installed_at: installed_at.clone(),
                has_update: false,
                current_version,
                latest_version: None,
                tool_count: None,
            });
        }
    }

    Ok(installed_mcps)
}

fn extract_version_from_args(args: &[String]) -> Option<String> {
    for arg in args {
        if arg.starts_with('@') && arg.contains('/') {
            if let Some(at_pos) = arg.rfind('@') {
                if at_pos > 0 {
                    let version = &arg[at_pos + 1..];
                    if !version.is_empty() && !version.contains('/') {
                        return Some(version.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Toggle installed MCP enabled/disabled status.
#[tauri::command]
pub fn toggle_installed_mcp(
    config_path: String,
    mcp_name: String,
    disabled: bool,
) -> Result<(), String> {
    let cli_path = resolve_cli_identifier_from_config_path(&config_path)?;
    let mut config = read_cli_config(cli_path.clone(), None)?;

    if let Some(ref mut servers) = config.mcpServers {
        if let Some(server_config) = servers.get_mut(&mcp_name) {
            server_config.disabled = disabled;
        }
    }

    write_cli_config(cli_path, None, config)
}

/// Uninstall MCP from CLI config file.
#[tauri::command]
pub fn uninstall_mcp(config_path: String, mcp_name: String) -> Result<McpInstallResult, String> {
    let cli_path = resolve_cli_identifier_from_config_path(&config_path)?;
    let mut config = read_cli_config(cli_path.clone(), None)?;
    let backup_path = create_config_backup(Path::new(&config_path))?;

    if let Some(ref mut servers) = config.mcpServers {
        servers.remove(&mcp_name);
    }

    write_cli_config(cli_path, None, config)?;
    delete_installed_mcp_test_result(&config_path, &mcp_name);

    Ok(McpInstallResult {
        success: true,
        message: format!("Successfully uninstalled {}", mcp_name),
        config_path: Some(config_path),
        backup_path,
        rollback_performed: false,
        rollback_message: None,
    })
}

/// MCP update check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpUpdateCheckResult {
    pub mcp_name: String,
    pub has_update: bool,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub update_notes: Option<String>,
}

/// Check for MCP updates.
#[tauri::command]
pub async fn check_mcp_updates(
    mcp_names: Vec<String>,
) -> Result<Vec<McpUpdateCheckResult>, String> {
    Ok(mcp_names
        .into_iter()
        .map(|mcp_name| McpUpdateCheckResult {
            mcp_name,
            has_update: false,
            current_version: None,
            latest_version: None,
            update_notes: Some("MCP Market 当前未暴露稳定版本元数据".to_string()),
        })
        .collect())
}

/// Update MCP to latest version (reinstall).
#[tauri::command]
pub async fn update_mcp(input: McpInstallInput) -> Result<McpInstallResult, String> {
    install_mcp_to_cli(input).await
}

/// Update installed MCP configuration.
#[tauri::command]
pub fn update_installed_mcp(
    config_path: String,
    old_name: String,
    new_name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
) -> Result<McpInstallResult, String> {
    let cli_path = resolve_cli_identifier_from_config_path(&config_path)?;
    let mut config = read_cli_config(cli_path.clone(), None)?;
    let backup_path = create_config_backup(Path::new(&config_path))?;

    if config.mcpServers.is_none() {
        config.mcpServers = Some(HashMap::new());
    }

    if let Some(ref mut servers) = config.mcpServers {
        if old_name != new_name {
            servers.remove(&old_name);
        }
        servers.insert(
            new_name.clone(),
            build_installed_mcp_config(command, args, env),
        );
    }

    write_cli_config(cli_path, None, config)?;

    Ok(McpInstallResult {
        success: true,
        message: format!("Successfully updated {}", new_name),
        config_path: Some(config_path),
        backup_path,
        rollback_performed: false,
        rollback_message: None,
    })
}

/// MCP 连接测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTestResult {
    pub success: bool,
    pub message: String,
    pub tool_count: Option<i32>,
}

/// 已安装 MCP 测试结果（包含数据库中的测试信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMcpTestResult {
    pub success: bool,
    pub message: String,
    pub tool_count: Option<i32>,
    pub tested_at: String,
}

/// 获取数据库连接
fn get_db_connection() -> Result<rusqlite::Connection, String> {
    let persistence_dir = crate::commands::get_persistence_dir_path().map_err(|e| e.to_string())?;
    let db_path = persistence_dir.join("data").join("easy-agent.db");
    rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())
}

/// 发送 JSON-RPC 请求并读取响应
fn send_jsonrpc_request(
    writer: &mut std::io::BufWriter<std::process::ChildStdin>,
    reader: &mut std::io::BufReader<std::process::ChildStdout>,
    method: &str,
    params: serde_json::Value,
    request_id: i64,
) -> Result<serde_json::Value, String> {
    use std::io::{BufRead, Read, Write};

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params
    });

    let request_str = serde_json::to_string(&request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?;

    writeln!(
        writer,
        "Content-Length: {}\r\n\r\n{}",
        request_str.len(),
        request_str
    )
    .map_err(|e| format!("Failed to write request: {}", e))?;

    writer
        .flush()
        .map_err(|e| format!("Failed to flush writer: {}", e))?;

    let mut header_line = String::new();
    reader
        .read_line(&mut header_line)
        .map_err(|e| format!("Failed to read header: {}", e))?;

    let content_length: usize = header_line
        .strip_prefix("Content-Length: ")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| "Invalid Content-Length header".to_string())?;

    let mut empty_line = String::new();
    reader
        .read_line(&mut empty_line)
        .map_err(|e| format!("Failed to read separator: {}", e))?;

    let mut response_body = vec![0u8; content_length];
    reader
        .read_exact(&mut response_body)
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let response_str = String::from_utf8(response_body)
        .map_err(|e| format!("Invalid UTF-8 in response: {}", e))?;

    serde_json::from_str(&response_str).map_err(|e| format!("Failed to parse response JSON: {}", e))
}

/// 测试已安装 MCP 服务器连接 (Tauri 命令)
#[tauri::command]
pub fn test_installed_mcp_connection(
    config_path: String,
    mcp_name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
) -> Result<InstalledMcpTestResult, String> {
    use chrono::Utc;
    use std::io::{BufReader, BufWriter, Write};
    use std::process::{Command, Stdio};
    use uuid::Uuid;

    let mut child = Command::new(&command)
        .args(&args)
        .envs(env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start MCP process: {}", e))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Failed to get stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to get stdout".to_string())?;

    let mut writer = BufWriter::new(stdin);
    let mut reader = BufReader::new(stdout);

    let result = (|| {
        let init_params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "easy-agent",
                "version": "1.1.0"
            }
        });

        let init_response =
            send_jsonrpc_request(&mut writer, &mut reader, "initialize", init_params, 1)?;

        if let Some(error) = init_response.get("error") {
            return Err(format!("Initialize error: {}", error));
        }

        let initialized_notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let notification_str = serde_json::to_string(&initialized_notification)
            .map_err(|e| format!("Failed to serialize notification: {}", e))?;
        writeln!(
            writer,
            "Content-Length: {}\r\n\r\n{}",
            notification_str.len(),
            notification_str
        )
        .map_err(|e| format!("Failed to write notification: {}", e))?;
        writer
            .flush()
            .map_err(|e| format!("Failed to flush writer: {}", e))?;

        let tools_response = send_jsonrpc_request(
            &mut writer,
            &mut reader,
            "tools/list",
            serde_json::json!({}),
            2,
        )?;

        if let Some(error) = tools_response.get("error") {
            return Err(format!("Tools list error: {}", error));
        }

        let tool_count = tools_response
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array())
            .map(|t| t.len() as i32);

        Ok(McpTestResult {
            success: true,
            message: format!("连接成功，服务器「{}」可用", mcp_name),
            tool_count,
        })
    })();

    let _ = child.kill();
    let _ = child.wait();

    let now = Utc::now().to_rfc3339();
    let test_result = match result {
        Ok(r) => r,
        Err(e) => McpTestResult {
            success: false,
            message: format!("连接失败: {}", e),
            tool_count: None,
        },
    };

    let conn = get_db_connection()?;
    let status = if test_result.success {
        "success"
    } else {
        "failed"
    };
    let result_id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT OR REPLACE INTO installed_mcp_test_results (id, config_path, mcp_name, test_status, test_message, tool_count, tested_at)
         VALUES (
            COALESCE((SELECT id FROM installed_mcp_test_results WHERE config_path = ?1 AND mcp_name = ?2), ?3),
            ?1, ?2, ?4, ?5, ?6, ?7
         )",
        rusqlite::params![
            config_path,
            mcp_name,
            result_id,
            status,
            test_result.message.clone(),
            test_result.tool_count,
            now
        ],
    )
    .map_err(|e| format!("Failed to save test result: {}", e))?;

    Ok(InstalledMcpTestResult {
        success: test_result.success,
        message: test_result.message,
        tool_count: test_result.tool_count,
        tested_at: now,
    })
}

/// 获取已安装 MCP 的测试结果
#[tauri::command]
pub fn get_installed_mcp_test_result(
    config_path: String,
    mcp_name: String,
) -> Result<Option<InstalledMcpTestResult>, String> {
    let conn = get_db_connection()?;

    let mut stmt = conn
        .prepare(
            "SELECT test_status, test_message, tool_count, tested_at FROM installed_mcp_test_results WHERE config_path = ?1 AND mcp_name = ?2"
        )
        .map_err(|e| e.to_string())?;

    let result = stmt
        .query_row(rusqlite::params![config_path, mcp_name], |row| {
            Ok(InstalledMcpTestResult {
                success: row.get::<_, String>(0)? == "success",
                message: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                tool_count: row.get(2)?,
                tested_at: row.get(3)?,
            })
        })
        .optional()
        .map_err(|e: rusqlite::Error| e.to_string())?;

    Ok(result)
}

/// MCP 安装历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInstallHistory {
    pub id: String,
    pub mcp_id: String,
    pub mcp_name: String,
    pub cli_path: String,
    pub config_path: String,
    pub backup_path: Option<String>,
    pub scope: String,
    pub status: String,
    pub created_at: String,
    pub rolled_back_at: Option<String>,
}

/// 保存安装历史
#[tauri::command]
pub fn save_install_history(
    mcp_id: String,
    mcp_name: String,
    cli_path: String,
    config_path: String,
    backup_path: Option<String>,
    scope: String,
) -> Result<McpInstallHistory, String> {
    use chrono::Utc;
    use uuid::Uuid;

    let conn = get_db_connection()?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO mcp_install_history (id, mcp_id, mcp_name, cli_path, config_path, backup_path, scope, status, created_at, rolled_back_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL)",
        rusqlite::params![
            id,
            mcp_id,
            mcp_name,
            cli_path,
            config_path,
            backup_path,
            scope,
            "completed",
            now
        ],
    )
    .map_err(|e| format!("Failed to save install history: {}", e))?;

    Ok(McpInstallHistory {
        id,
        mcp_id,
        mcp_name,
        cli_path,
        config_path,
        backup_path,
        scope,
        status: "completed".to_string(),
        created_at: now,
        rolled_back_at: None,
    })
}

/// 获取安装历史列表
#[tauri::command]
pub fn get_install_history(limit: Option<i32>) -> Result<Vec<McpInstallHistory>, String> {
    let conn = get_db_connection()?;
    let limit = limit.unwrap_or(50);

    let mut stmt = conn
        .prepare(
            "SELECT id, mcp_id, mcp_name, cli_path, config_path, backup_path, scope, status, created_at, rolled_back_at
             FROM mcp_install_history
             ORDER BY created_at DESC
             LIMIT ?1"
        )
        .map_err(|e| e.to_string())?;

    let histories = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(McpInstallHistory {
                id: row.get(0)?,
                mcp_id: row.get(1)?,
                mcp_name: row.get(2)?,
                cli_path: row.get(3)?,
                config_path: row.get(4)?,
                backup_path: row.get(5)?,
                scope: row.get(6)?,
                status: row.get(7)?,
                created_at: row.get(8)?,
                rolled_back_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(histories)
}

/// 手动回滚安装
#[tauri::command]
pub async fn manual_rollback_install(history_id: String) -> Result<McpInstallResult, String> {
    use chrono::Utc;

    let conn = get_db_connection()?;

    let history: McpInstallHistory = conn
        .query_row(
            "SELECT id, mcp_id, mcp_name, cli_path, config_path, backup_path, scope, status, created_at, rolled_back_at
             FROM mcp_install_history WHERE id = ?1",
            rusqlite::params![history_id],
            |row| {
                Ok(McpInstallHistory {
                    id: row.get(0)?,
                    mcp_id: row.get(1)?,
                    mcp_name: row.get(2)?,
                    cli_path: row.get(3)?,
                    config_path: row.get(4)?,
                    backup_path: row.get(5)?,
                    scope: row.get(6)?,
                    status: row.get(7)?,
                    created_at: row.get(8)?,
                    rolled_back_at: row.get(9)?,
                })
            },
        )
        .map_err(|e| format!("安装历史记录不存在: {}", e))?;

    if history.status == "rolled_back" {
        return Err("此安装已经回滚".to_string());
    }

    let rollback_result =
        rollback_mcp_install(history.config_path.clone(), history.backup_path.clone()).await?;

    if rollback_result.success {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE mcp_install_history SET status = 'rolled_back', rolled_back_at = ?1 WHERE id = ?2",
            rusqlite::params![now, history_id],
        )
        .map_err(|e| format!("更新历史记录失败: {}", e))?;
    }

    Ok(rollback_result)
}
