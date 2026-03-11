use anyhow::Result;
use reqwest::Url;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::commands::mcp_shared::{
    ensure_mcp_servers_object, read_json_config_or_default, write_json_config_pretty,
};

const MCP_MARKET_BASE_URL: &str = "https://modelscope.cn/api/v1/mcp";

/// MCP market item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMarketItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub rating: f64,
    pub category: String,
    pub repository_url: Option<String>,
    pub install_command: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// MCP version history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpVersion {
    pub version: String,
    pub release_notes: String,
    pub released_at: String,
}

/// MCP market detail (full information)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMarketDetail {
    pub id: String,
    pub name: String,
    pub description: String,
    pub full_description: String,
    pub author: String,
    pub author_url: Option<String>,
    pub license: String,
    pub homepage_url: Option<String>,
    pub repository_url: Option<String>,
    pub downloads: u64,
    pub rating: f64,
    pub category: String,
    pub install_command: Option<String>,
    pub config_example: String,
    pub tags: Vec<String>,
    pub version_history: Vec<McpVersion>,
    pub requirements: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// MCP market list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMarketListResponse {
    pub items: Vec<McpMarketItem>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub has_more: bool,
}

/// MCP market query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpMarketQuery {
    pub page: u32,
    pub page_size: u32,
    pub category: Option<String>,
    pub search: Option<String>,
}

impl Default for McpMarketQuery {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
            category: None,
            search: None,
        }
    }
}

fn build_mcp_market_url(query: &McpMarketQuery) -> Result<Url, String> {
    let mut url = Url::parse(MCP_MARKET_BASE_URL)
        .map_err(|e| format!("Invalid MCP market URL: {}", e))?;

    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("page", &query.page.to_string());
        pairs.append_pair("page_size", &query.page_size.to_string());

        if let Some(category) = &query.category {
            pairs.append_pair("category", category);
        }

        if let Some(search) = &query.search {
            pairs.append_pair("search", search);
        }
    }

    Ok(url)
}

fn sanitize_mcp_key(name: &str) -> String {
    let sanitized = name
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
        .to_lowercase();

    if sanitized.is_empty() {
        "mcp-server".to_string()
    } else {
        sanitized
    }
}

fn create_settings_backup(settings_path: &PathBuf, error_prefix: &str) -> Result<Option<String>, String> {
    if !settings_path.exists() {
        return Ok(None);
    }

    let backup = settings_path.with_extension("json.backup");
    fs::copy(settings_path, &backup)
        .map_err(|e| format!("{error_prefix}: {}", e))?;
    Ok(Some(backup.to_string_lossy().to_string()))
}

fn load_settings_config(settings_path: &PathBuf) -> Result<serde_json::Value, String> {
    read_json_config_or_default(settings_path, serde_json::json!({}))
}

fn ensure_settings_parent_dir(settings_path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = settings_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
        }
    }
    Ok(())
}

fn build_installed_mcp_config(
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
) -> serde_json::Value {
    let mut config = serde_json::Map::new();
    config.insert("command".to_string(), serde_json::json!(command));
    if !args.is_empty() {
        config.insert("args".to_string(), serde_json::json!(args));
    }
    if !env.is_empty() {
        config.insert("env".to_string(), serde_json::json!(env));
    }
    serde_json::Value::Object(config)
}

/// Fetch MCP market items from ModelScope API
#[tauri::command]
pub async fn fetch_mcp_market(query: McpMarketQuery) -> Result<McpMarketListResponse, String> {
    let client = crate::commands::market_source_support::build_market_http_client()?;
    let url = build_mcp_market_url(&query)?;

    match client.get(url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<McpMarketListResponse>().await {
                    Ok(data) => Ok(data),
                    Err(e) => Err(format!("Failed to parse MCP market response: {}", e)),
                }
            } else {
                Err(format!(
                    "MCP market request failed with status {}",
                    response.status()
                ))
            }
        }
        Err(e) => Err(format!("Failed to fetch MCP market: {}", e)),
    }
}

/// Fetch MCP market detail by ID
#[tauri::command]
pub async fn fetch_mcp_market_detail(mcp_id: String) -> Result<McpMarketDetail, String> {
    let client = crate::commands::market_source_support::build_market_http_client()?;

    let url = format!("{}/{}", MCP_MARKET_BASE_URL, mcp_id);

    match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<McpMarketDetail>().await {
                    Ok(data) => Ok(data),
                    Err(e) => Err(format!("Failed to parse MCP detail response: {}", e)),
                }
            } else {
                Err(format!(
                    "MCP detail request failed with status {}",
                    response.status()
                ))
            }
        }
        Err(e) => Err(format!("Failed to fetch MCP detail: {}", e)),
    }
}

/// MCP installation input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInstallInput {
    pub mcp_id: String,
    pub mcp_name: String,
    pub cli_path: String,
    pub scope: String, // "global" or "project"
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
    /// Whether a rollback was performed due to installation failure
    pub rollback_performed: bool,
    /// Message describing the rollback result (if any)
    pub rollback_message: Option<String>,
}

/// Get CLI settings.json path
fn get_cli_settings_path(
    cli_path: &str,
    scope: &str,
    project_path: Option<&str>,
) -> Result<PathBuf, String> {
    let cli = PathBuf::from(cli_path);
    let cli_name = cli
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "claude".to_string());

    let settings_path = if scope == "project" {
        // Project-level settings
        if let Some(proj_path) = project_path {
            PathBuf::from(proj_path)
                .join(".claude")
                .join("settings.json")
        } else {
            return Err("Project path is required for project-level installation".to_string());
        }
    } else {
        // Global settings
        let home = dirs::home_dir().ok_or("Cannot determine home directory")?;

        match cli_name.as_str() {
            "claude" => home.join(".claude").join("settings.json"),
            "codex" => home.join(".codex").join("settings.json"),
            _ => return Err(format!("Unknown CLI: {}", cli_name)),
        }
    };

    Ok(settings_path)
}

/// Internal helper function to perform rollback
fn perform_rollback(settings_path: &PathBuf, backup_path: &Option<String>) -> (bool, String) {
    if let Some(backup) = backup_path {
        let backup = PathBuf::from(backup);
        if backup.exists() {
            // Restore from backup
            match fs::copy(&backup, settings_path) {
                Ok(_) => {
                    // Clean up backup file
                    let _ = fs::remove_file(&backup);
                    (true, "已恢复备份文件".to_string())
                }
                Err(e) => (false, format!("恢复备份失败: {}", e)),
            }
        } else {
            (false, "备份文件不存在".to_string())
        }
    } else {
        // No backup means file was newly created, so delete it
        if settings_path.exists() {
            match fs::remove_file(settings_path) {
                Ok(_) => (true, "已删除新创建的配置文件".to_string()),
                Err(e) => (false, format!("删除配置文件失败: {}", e)),
            }
        } else {
            (true, "无需回滚（文件不存在）".to_string())
        }
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

/// Install MCP to CLI settings.json
#[tauri::command]
pub async fn install_mcp_to_cli(input: McpInstallInput) -> Result<McpInstallResult, String> {
    // Get the settings file path
    let settings_path =
        get_cli_settings_path(&input.cli_path, &input.scope, input.project_path.as_deref())?;
    ensure_settings_parent_dir(&settings_path)?;

    // Read existing settings or create new
    let mut settings = load_settings_config(&settings_path)?;

    // Create backup
    let backup_path = create_settings_backup(&settings_path, "创建备份失败")?;

    // Build MCP server config
    let mcp_config = build_installed_mcp_config(input.command.clone(), input.args.clone(), input.env.clone());

    let write_result = {
        let mcp_servers = ensure_mcp_servers_object(&mut settings)?;
        mcp_servers.insert(sanitize_mcp_key(&input.mcp_name), mcp_config);
        write_json_config_pretty(&settings_path, &settings)
    };

    if let Err(e) = write_result {
        let (rollback_success, rollback_msg) = perform_rollback(&settings_path, &backup_path);
        return Ok(McpInstallResult {
            success: false,
            message: e,
            config_path: Some(settings_path.to_string_lossy().to_string()),
            backup_path: None,
            rollback_performed: rollback_success,
            rollback_message: Some(rollback_msg),
        });
    }

    // Save install history to database for manual rollback
    let config_path_str = settings_path.to_string_lossy().to_string();
    let backup_path_str = backup_path.clone();
    if let Err(e) = save_install_history(
        input.mcp_id.clone(),
        input.mcp_name.clone(),
        input.cli_path.clone(),
        config_path_str.clone(),
        backup_path_str,
        input.scope.clone(),
    ) {
        // Log error but don't fail the installation
        println!("Warning: Failed to save install history: {}", e);
    }

    // Note: We keep the backup file for potential manual rollback
    // The backup will be cleaned up when:
    // 1. User manually rolls back the installation
    // 2. A new installation overwrites it

    Ok(McpInstallResult {
        success: true,
        message: format!("成功安装 {} 到 {}", input.mcp_name, input.cli_path),
        config_path: Some(config_path_str),
        backup_path: backup_path,
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

    // If no backup, just delete the settings file if it exists
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

/// CLI config info for scanning
struct CliConfigInfo {
    cli_name: String,
    cli_path: String,
    config_path: PathBuf,
    scope: String,
}

/// Get all CLI config files to scan for installed MCPs
fn get_all_cli_configs() -> Vec<CliConfigInfo> {
    let mut configs = Vec::new();
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return configs,
    };

    // Standard CLI config paths
    let cli_configs = vec![
        (
            "claude",
            "claude",
            home.join(".claude").join("settings.json"),
            "global",
        ),
        (
            "codex",
            "codex",
            home.join(".codex").join("settings.json"),
            "global",
        ),
    ];

    for (cli_name, cli_path, config_path, scope) in cli_configs {
        if config_path.exists() {
            configs.push(CliConfigInfo {
                cli_name: cli_name.to_string(),
                cli_path: cli_path.to_string(),
                config_path,
                scope: scope.to_string(),
            });
        }
    }

    configs
}

/// List all installed MCPs from all CLI config files
#[tauri::command]
pub fn list_installed_mcps() -> Result<Vec<InstalledMcp>, String> {
    let configs = get_all_cli_configs();
    let mut installed_mcps = Vec::new();

    for config_info in configs {
        let content = match fs::read_to_string(&config_info.config_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let settings: serde_json::Value = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let mcp_servers = match settings.get("mcpServers").and_then(|v| v.as_object()) {
            Some(obj) => obj,
            None => continue,
        };

        // Get file metadata for installed_at
        let metadata = fs::metadata(&config_info.config_path).ok();
        let installed_at = metadata.and_then(|m| {
            m.modified().ok().and_then(|t| {
                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                Some(datetime.to_rfc3339())
            })
        });

        for (name, config) in mcp_servers {
            let config_obj = match config.as_object() {
                Some(obj) => obj,
                None => continue,
            };

            let command = config_obj
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let args: Vec<String> = config_obj
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let env = config_obj
                .get("env")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();

            let disabled = config_obj
                .get("disabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Try to extract version from args (e.g., npx -y @scope/package@1.0.0)
            let current_version = extract_version_from_args(&args);

            installed_mcps.push(InstalledMcp {
                name: name.clone(),
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

/// Extract version from npm package args
fn extract_version_from_args(args: &[String]) -> Option<String> {
    for arg in args {
        if arg.starts_with('@') && arg.contains('/') {
            // Format: @scope/package or @scope/package@version
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

/// Toggle installed MCP enabled/disabled status
#[tauri::command]
pub fn toggle_installed_mcp(
    config_path: String,
    mcp_name: String,
    disabled: bool,
) -> Result<(), String> {
    let settings_path = PathBuf::from(&config_path);
    let mut settings = load_settings_config(&settings_path)?;
    let mcp_servers = ensure_mcp_servers_object(&mut settings)?;

    if let Some(mcp_config) = mcp_servers.get_mut(&mcp_name) {
        if let Some(config_obj) = mcp_config.as_object_mut() {
            if disabled {
                config_obj.insert("disabled".to_string(), serde_json::json!(true));
            } else {
                config_obj.remove("disabled");
            }
        }
    }

    write_json_config_pretty(&settings_path, &settings)
        .map_err(|e| format!("Failed to persist settings file: {}", e))?;

    Ok(())
}

/// Uninstall MCP from CLI config file
#[tauri::command]
pub fn uninstall_mcp(config_path: String, mcp_name: String) -> Result<McpInstallResult, String> {
    let settings_path = PathBuf::from(&config_path);
    let mut settings = load_settings_config(&settings_path)?;

    // Create backup
    let backup_path = create_settings_backup(&settings_path, "Failed to create backup")?;
    let mcp_servers = ensure_mcp_servers_object(&mut settings)?;
    mcp_servers.remove(&mcp_name);
    write_json_config_pretty(&settings_path, &settings)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

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

/// Check for MCP updates by comparing with market versions
#[tauri::command]
pub async fn check_mcp_updates(
    mcp_names: Vec<String>,
) -> Result<Vec<McpUpdateCheckResult>, String> {
    let mut results = Vec::new();

    for mcp_name in mcp_names {
        // Try to fetch market detail to get latest version
        let market_detail =
            fetch_mcp_market_detail(format!("mcp-{}", sanitize_mcp_key(&mcp_name))).await;

        let (latest_version, update_notes) = match market_detail {
            Ok(detail) => {
                let latest = detail.version_history.first().map(|v| v.version.clone());
                let notes = detail
                    .version_history
                    .first()
                    .map(|v| v.release_notes.clone());
                (latest, notes)
            }
            Err(_) => (None, None),
        };

        results.push(McpUpdateCheckResult {
            mcp_name: mcp_name.clone(),
            has_update: false, // Will be updated by caller with current version
            current_version: None,
            latest_version,
            update_notes,
        });
    }

    Ok(results)
}

/// Update MCP to latest version (reinstall)
#[tauri::command]
pub async fn update_mcp(input: McpInstallInput) -> Result<McpInstallResult, String> {
    // Update is essentially a reinstall that overwrites existing config
    install_mcp_to_cli(input).await
}

/// Update installed MCP configuration
#[tauri::command]
pub fn update_installed_mcp(
    config_path: String,
    old_name: String,
    new_name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
) -> Result<McpInstallResult, String> {
    let settings_path = PathBuf::from(&config_path);
    let mut settings = load_settings_config(&settings_path)?;

    // Create backup
    let backup_path = create_settings_backup(&settings_path, "Failed to create backup")?;
    let mcp_servers = ensure_mcp_servers_object(&mut settings)
        .map_err(|e| format!("Failed to get mcpServers object: {}", e))?;
    let new_config = build_installed_mcp_config(command, args, env);

    // If name changed, remove old entry
    if old_name != new_name {
        mcp_servers.remove(&old_name);
    }

    // Insert/update the new config
    mcp_servers.insert(new_name.clone(), new_config);
    write_json_config_pretty(&settings_path, &settings)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

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

    // 构造 JSON-RPC 请求
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params
    });

    // 发送请求
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

    // 读取响应头
    let mut header_line = String::new();
    reader
        .read_line(&mut header_line)
        .map_err(|e| format!("Failed to read header: {}", e))?;

    // 解析 Content-Length
    let content_length: usize = header_line
        .strip_prefix("Content-Length: ")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| "Invalid Content-Length header".to_string())?;

    // 读取空行
    let mut empty_line = String::new();
    reader
        .read_line(&mut empty_line)
        .map_err(|e| format!("Failed to read separator: {}", e))?;

    // 读取响应体
    let mut response_body = vec![0u8; content_length];
    reader
        .read_exact(&mut response_body)
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let response_str = String::from_utf8(response_body)
        .map_err(|e| format!("Invalid UTF-8 in response: {}", e))?;

    let response: serde_json::Value = serde_json::from_str(&response_str)
        .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

    Ok(response)
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
    use std::io::{BufReader, BufWriter};
    use std::process::{Command, Stdio};
    use uuid::Uuid;

    // 启动 MCP 进程
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
        // 发送 initialize 请求
        let init_params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "easy-agent",
                "version": "1.0.0"
            }
        });

        let init_response =
            send_jsonrpc_request(&mut writer, &mut reader, "initialize", init_params, 1)?;

        // 检查是否有错误
        if let Some(error) = init_response.get("error") {
            return Err(format!("Initialize error: {}", error));
        }

        // 发送 initialized 通知
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

        // 发送 tools/list 请求
        let tools_response = send_jsonrpc_request(
            &mut writer,
            &mut reader,
            "tools/list",
            serde_json::json!({}),
            2,
        )?;

        // 检查是否有错误
        if let Some(error) = tools_response.get("error") {
            return Err(format!("Tools list error: {}", error));
        }

        // 获取工具数量
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

    // 关闭进程
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

    // 保存测试结果到数据库
    let conn = get_db_connection()?;
    let status = if test_result.success {
        "success"
    } else {
        "failed"
    };
    let result_id = Uuid::new_v4().to_string();

    // 使用 UPSERT（INSERT OR REPLACE）
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

// ===================== MCP 安装历史 =====================

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
    pub status: String, // "completed" | "rolled_back"
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

    // 获取历史记录
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

    // 检查是否已经回滚
    if history.status == "rolled_back" {
        return Err("此安装已经回滚".to_string());
    }

    // 执行回滚
    let config_path = history.config_path.clone();
    let backup_path = history.backup_path.clone();

    let rollback_result = rollback_mcp_install(config_path, backup_path).await?;

    if rollback_result.success {
        // 更新历史记录状态
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE mcp_install_history SET status = 'rolled_back', rolled_back_at = ?1 WHERE id = ?2",
            rusqlite::params![now, history_id],
        )
        .map_err(|e| format!("更新历史记录失败: {}", e))?;
    }

    Ok(rollback_result)
}
