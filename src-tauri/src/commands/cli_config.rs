use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use tauri_plugin_opener::OpenerExt;

use crate::commands::cli_support::resolve_cli_name;

/// CLI 能力信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliCapabilities {
    /// 是否支持 MCP
    pub supports_mcp: bool,
    /// 是否支持 Skills
    pub supports_skills: bool,
    /// 是否支持 Plugins
    pub supports_plugins: bool,
    /// MCP 添加命令（如果支持）
    pub mcp_add_command: Option<String>,
}

/// CLI 配置路径信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliConfigPaths {
    /// 配置目录路径 (如 ~/.claude/)
    pub config_dir: String,
    /// 主配置文件路径 (如 ~/.claude.json)
    pub config_file: String,
    /// CLI 类型名称 (claude, codex, qwen)
    pub cli_type: String,
    /// Skills 安装目录路径
    pub skills_dir: String,
}

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpServerConfig {
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub url: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub disabled: bool,
}

/// CLI 配置结构 (Claude CLI 格式)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(non_snake_case)]
pub struct ClaudeCliConfig {
    /// MCP 服务器配置（支持 mcpServers 和 mcp_servers 两种格式）
    #[serde(skip_serializing_if = "Option::is_none", alias = "mcp_servers")]
    pub mcpServers: Option<HashMap<String, McpServerConfig>>,
    #[serde(flatten, default)]
    pub other: BTreeMap<String, JsonValue>,
}

/// MCP 配置更新输入
#[derive(Debug, Clone, Deserialize)]
pub struct McpConfigUpdateInput {
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub url: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CliSyncConfigType {
    Mcp,
    Skills,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CliSyncConflictPolicy {
    Skip,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliSyncInput {
    pub source_cli_path: String,
    pub target_cli_path: String,
    pub source_cli_type: Option<String>,
    pub target_cli_type: Option<String>,
    pub config_type: CliSyncConfigType,
    pub item_names: Vec<String>,
    pub conflict_policy: CliSyncConflictPolicy,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliSyncItemIssue {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CliSyncResult {
    pub success_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub created_items: Vec<String>,
    pub skipped_items: Vec<CliSyncItemIssue>,
    pub failed_items: Vec<CliSyncItemIssue>,
}

fn parse_mcp_servers_from_toml(
    toml_value: &toml::Value,
) -> Option<HashMap<String, McpServerConfig>> {
    let mcp = toml_value
        .get("mcp_servers")
        .or_else(|| toml_value.get("mcpServers"))?;

    let mcp_obj = mcp.as_table()?;
    let mut servers = HashMap::new();

    for (name, config) in mcp_obj {
        if let Some(config_obj) = config.as_table() {
            let server_config = McpServerConfig {
                command: config_obj
                    .get("command")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                args: config_obj
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    }),
                env: config_obj.get("env").and_then(|v| v.as_table()).map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                }),
                url: config_obj
                    .get("url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                headers: config_obj
                    .get("headers")
                    .and_then(|v| v.as_table())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    }),
                disabled: config_obj
                    .get("disabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            };
            servers.insert(name.clone(), server_config);
        }
    }

    Some(servers)
}

fn json_to_toml_value(value: JsonValue) -> Result<toml::Value, String> {
    match value {
        JsonValue::Null => Err("Null values are not supported in TOML config".to_string()),
        JsonValue::Bool(value) => Ok(toml::Value::Boolean(value)),
        JsonValue::Number(value) => {
            if let Some(integer) = value.as_i64() {
                Ok(toml::Value::Integer(integer))
            } else if let Some(float) = value.as_f64() {
                Ok(toml::Value::Float(float))
            } else {
                Err("Unsupported numeric value in TOML config".to_string())
            }
        }
        JsonValue::String(value) => Ok(toml::Value::String(value)),
        JsonValue::Array(values) => {
            let mut array = Vec::with_capacity(values.len());
            for value in values {
                array.push(json_to_toml_value(value)?);
            }
            Ok(toml::Value::Array(array))
        }
        JsonValue::Object(values) => {
            let mut table = toml::map::Map::new();
            for (key, value) in values {
                if value.is_null() {
                    continue;
                }
                table.insert(key, json_to_toml_value(value)?);
            }
            Ok(toml::Value::Table(table))
        }
    }
}

fn json_map_to_toml_table(
    values: &BTreeMap<String, JsonValue>,
) -> Result<toml::map::Map<String, toml::Value>, String> {
    let mut table = toml::map::Map::new();

    for (key, value) in values {
        if value.is_null() {
            continue;
        }

        table.insert(key.clone(), json_to_toml_value(value.clone())?);
    }

    Ok(table)
}

fn build_toml_mcp_servers(
    servers: HashMap<String, McpServerConfig>,
) -> toml::map::Map<String, toml::Value> {
    let mut mcp_table = toml::map::Map::new();

    for (name, server_config) in servers {
        let mut server_table = toml::map::Map::new();
        if let Some(cmd) = server_config.command {
            server_table.insert("command".to_string(), toml::Value::String(cmd));
        }
        if let Some(args) = server_config.args {
            server_table.insert(
                "args".to_string(),
                toml::Value::Array(args.into_iter().map(toml::Value::String).collect()),
            );
        }
        if let Some(env) = server_config.env {
            let env_table: toml::map::Map<String, toml::Value> = env
                .into_iter()
                .map(|(k, v)| (k, toml::Value::String(v)))
                .collect();
            server_table.insert("env".to_string(), toml::Value::Table(env_table));
        }
        if let Some(url) = server_config.url {
            server_table.insert("url".to_string(), toml::Value::String(url));
        }
        if let Some(headers) = server_config.headers {
            let headers_table: toml::map::Map<String, toml::Value> = headers
                .into_iter()
                .map(|(k, v)| (k, toml::Value::String(v)))
                .collect();
            server_table.insert("headers".to_string(), toml::Value::Table(headers_table));
        }
        if server_config.disabled {
            server_table.insert("disabled".to_string(), toml::Value::Boolean(true));
        }
        mcp_table.insert(name, toml::Value::Table(server_table));
    }

    mcp_table
}

fn is_supported_sync_cli(cli_type: &str) -> bool {
    matches!(cli_type, "claude" | "codex" | "opencode")
}

fn validate_sync_cli_paths(
    source_cli_path: &str,
    target_cli_path: &str,
    source_cli_type: Option<&str>,
    target_cli_type: Option<&str>,
) -> Result<(CliConfigPaths, CliConfigPaths), String> {
    let source_paths = get_cli_config_paths_internal(source_cli_path, source_cli_type)?;
    let target_paths = get_cli_config_paths_internal(target_cli_path, target_cli_type)?;

    if !is_supported_sync_cli(&source_paths.cli_type)
        || !is_supported_sync_cli(&target_paths.cli_type)
    {
        return Err("Only Claude CLI, Codex CLI and OpenCode CLI are supported for sync".to_string());
    }

    if source_paths.cli_type == target_paths.cli_type {
        return Err("Source CLI and target CLI must be different".to_string());
    }

    if source_paths.config_file == target_paths.config_file {
        return Err("Source CLI and target CLI must be different".to_string());
    }

    Ok((source_paths, target_paths))
}

fn ensure_path_within(base: &Path, path: &Path, label: &str) -> Result<(), String> {
    if path.starts_with(base) {
        Ok(())
    } else {
        Err(format!(
            "{} path is outside of the expected CLI directory",
            label
        ))
    }
}

fn copy_directory_recursive(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|e| {
        format!(
            "Failed to create target directory {}: {}",
            target.display(),
            e
        )
    })?;

    for entry in fs::read_dir(source).map_err(|e| {
        format!(
            "Failed to read source directory {}: {}",
            source.display(),
            e
        )
    })? {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if source_path.is_dir() {
            copy_directory_recursive(&source_path, &target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Failed to create target directory {}: {}",
                        parent.display(),
                        e
                    )
                })?;
            }
            fs::copy(&source_path, &target_path).map_err(|e| {
                format!(
                    "Failed to copy file from {} to {}: {}",
                    source_path.display(),
                    target_path.display(),
                    e
                )
            })?;
        }
    }

    Ok(())
}

fn sync_mcp_items(
    source_cli_path: &str,
    target_cli_path: &str,
    source_cli_type: Option<&str>,
    target_cli_type: Option<&str>,
    item_names: &[String],
    _conflict_policy: &CliSyncConflictPolicy,
) -> Result<CliSyncResult, String> {
    let source_config = read_cli_config_internal(source_cli_path, source_cli_type)?;
    let mut target_config = read_cli_config_internal(target_cli_path, target_cli_type)?;
    let source_servers = source_config.mcpServers.unwrap_or_default();
    let target_servers = target_config.mcpServers.get_or_insert_with(HashMap::new);
    let mut result = CliSyncResult::default();

    for item_name in item_names {
        let Some(source_server) = source_servers.get(item_name) else {
            result.failed_count += 1;
            result.failed_items.push(CliSyncItemIssue {
                name: item_name.clone(),
                reason: "Source MCP config was not found".to_string(),
            });
            continue;
        };

        if target_servers.contains_key(item_name) {
            result.skipped_count += 1;
            result.skipped_items.push(CliSyncItemIssue {
                name: item_name.clone(),
                reason: "Target CLI already contains an MCP config with the same name".to_string(),
            });
            continue;
        }

        target_servers.insert(item_name.clone(), source_server.clone());
        result.success_count += 1;
        result.created_items.push(item_name.clone());
    }

    if result.success_count > 0 {
        write_cli_config_internal(target_cli_path, target_cli_type, target_config)?;
    }

    Ok(result)
}

fn sync_skill_items(
    source_cli_path: &str,
    target_cli_path: &str,
    source_cli_type: Option<&str>,
    target_cli_type: Option<&str>,
    item_names: &[String],
    _conflict_policy: &CliSyncConflictPolicy,
) -> Result<CliSyncResult, String> {
    let (source_paths, target_paths) = validate_sync_cli_paths(
        source_cli_path,
        target_cli_path,
        source_cli_type,
        target_cli_type,
    )?;
    let source_scan = crate::commands::scan::scan_cli_config(
        Some(source_cli_path.to_string()),
        source_cli_type.map(str::to_string),
    )?;
    let target_scan = crate::commands::scan::scan_cli_config(
        Some(target_cli_path.to_string()),
        target_cli_type.map(str::to_string),
    )?;
    let source_skills_dir = PathBuf::from(&source_paths.config_dir).join("skills");
    let target_skills_dir = PathBuf::from(&target_paths.config_dir).join("skills");
    let mut result = CliSyncResult::default();

    fs::create_dir_all(&target_skills_dir).map_err(|e| {
        format!(
            "Failed to create target skills directory {}: {}",
            target_skills_dir.display(),
            e
        )
    })?;

    let source_by_name: HashMap<_, _> = source_scan
        .skills
        .into_iter()
        .map(|skill| (skill.name.clone(), skill))
        .collect();
    let mut target_names: HashMap<String, String> = target_scan
        .skills
        .into_iter()
        .map(|skill| (skill.name.clone(), skill.path))
        .collect();

    for item_name in item_names {
        let Some(source_skill) = source_by_name.get(item_name) else {
            result.failed_count += 1;
            result.failed_items.push(CliSyncItemIssue {
                name: item_name.clone(),
                reason: "Source Skill was not found".to_string(),
            });
            continue;
        };

        if target_names.contains_key(item_name) {
            result.skipped_count += 1;
            result.skipped_items.push(CliSyncItemIssue {
                name: item_name.clone(),
                reason: "Target CLI already contains a Skill with the same name".to_string(),
            });
            continue;
        }

        let source_path = PathBuf::from(&source_skill.path);
        ensure_path_within(&source_skills_dir, &source_path, "Source skill")?;

        let Some(file_name) = source_path.file_name() else {
            result.failed_count += 1;
            result.failed_items.push(CliSyncItemIssue {
                name: item_name.clone(),
                reason: "Source Skill path is invalid".to_string(),
            });
            continue;
        };

        let target_path = target_skills_dir.join(file_name);
        ensure_path_within(&target_skills_dir, &target_path, "Target skill")?;

        if target_path.exists() {
            result.skipped_count += 1;
            result.skipped_items.push(CliSyncItemIssue {
                name: item_name.clone(),
                reason: "Target CLI already contains a Skill at the same path".to_string(),
            });
            continue;
        }

        let copy_result = if source_path.is_dir() {
            copy_directory_recursive(&source_path, &target_path)
        } else if source_path.is_file() {
            fs::copy(&source_path, &target_path)
                .map(|_| ())
                .map_err(|e| {
                    format!(
                        "Failed to copy Skill from {} to {}: {}",
                        source_path.display(),
                        target_path.display(),
                        e
                    )
                })
        } else {
            Err("Source Skill path does not exist".to_string())
        };

        match copy_result {
            Ok(()) => {
                result.success_count += 1;
                result.created_items.push(item_name.clone());
                target_names.insert(item_name.clone(), target_path.to_string_lossy().to_string());
            }
            Err(reason) => {
                result.failed_count += 1;
                result.failed_items.push(CliSyncItemIssue {
                    name: item_name.clone(),
                    reason,
                });
            }
        }
    }

    Ok(result)
}

/// 根据 CLI 路径获取配置目录和信息
pub(crate) fn get_cli_config_paths_internal(
    cli_path: &str,
    cli_type_hint: Option<&str>,
) -> Result<CliConfigPaths, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    let cli_name = resolve_cli_name(Some(cli_path), cli_type_hint, "claude");

    match cli_name.as_str() {
        "claude" | "claude-code" => {
            let config_dir = home_dir.join(".claude");
            let config_file = home_dir.join(".claude.json");
            Ok(CliConfigPaths {
                config_dir: config_dir.to_string_lossy().to_string(),
                config_file: config_file.to_string_lossy().to_string(),
                cli_type: "claude".to_string(),
                skills_dir: config_dir.join("skills").to_string_lossy().to_string(),
            })
        }
        "codex" => {
            let config_dir = home_dir.join(".codex");
            let config_file = home_dir.join(".codex").join("config.toml");
            Ok(CliConfigPaths {
                config_dir: config_dir.to_string_lossy().to_string(),
                config_file: config_file.to_string_lossy().to_string(),
                cli_type: "codex".to_string(),
                skills_dir: config_dir.join("skills").to_string_lossy().to_string(),
            })
        }
        "opencode" => {
            let config_dir = dirs::home_dir()
                .map(|h| h.join(".config").join("opencode"))
                .ok_or_else(|| "Cannot determine home directory".to_string())?;
            let config_file = config_dir.join("opencode.json");
            Ok(CliConfigPaths {
                config_dir: config_dir.to_string_lossy().to_string(),
                config_file: config_file.to_string_lossy().to_string(),
                cli_type: "opencode".to_string(),
                skills_dir: config_dir.join("skills").to_string_lossy().to_string(),
            })
        }
        "qwen" | "qwen-code" => {
            let config_dir = home_dir.join(".qwen");
            let config_file = home_dir.join(".qwen").join("settings.json");
            Ok(CliConfigPaths {
                config_dir: config_dir.to_string_lossy().to_string(),
                config_file: config_file.to_string_lossy().to_string(),
                cli_type: "qwen".to_string(),
                skills_dir: config_dir.join("skills").to_string_lossy().to_string(),
            })
        }
        _ => {
            // 默认使用 Claude 配置
            let config_dir = home_dir.join(".claude");
            let config_file = home_dir.join(".claude.json");
            Ok(CliConfigPaths {
                config_dir: config_dir.to_string_lossy().to_string(),
                config_file: config_file.to_string_lossy().to_string(),
                cli_type: "claude".to_string(),
                skills_dir: config_dir.join("skills").to_string_lossy().to_string(),
            })
        }
    }
}

/// 获取 CLI 配置路径信息 (Tauri 命令)
#[tauri::command]
pub fn get_cli_config_paths(
    cli_path: String,
    cli_type: Option<String>,
) -> Result<CliConfigPaths, String> {
    get_cli_config_paths_internal(&cli_path, cli_type.as_deref())
}

fn read_cli_config_internal(
    cli_path: &str,
    cli_type_hint: Option<&str>,
) -> Result<ClaudeCliConfig, String> {
    let paths = get_cli_config_paths_internal(cli_path, cli_type_hint)?;
    let config_file = PathBuf::from(&paths.config_file);

    if !config_file.exists() {
        return Ok(ClaudeCliConfig::default());
    }

    if paths.cli_type == "opencode" {
        return read_opencode_config(&config_file);
    }

    if paths.cli_type == "codex" {
        // Codex 使用 TOML 格式
        let content = fs::read_to_string(&config_file)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        // 解析 TOML 并转换为 JSON 格式
        let toml_value: toml::Value =
            toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {}", e))?;
        let mut other = match serde_json::to_value(&toml_value)
            .map_err(|e| format!("Failed to convert TOML to JSON: {}", e))?
        {
            JsonValue::Object(map) => map,
            _ => JsonMap::new(),
        };
        other.remove("mcpServers");
        other.remove("mcp_servers");

        Ok(ClaudeCliConfig {
            mcpServers: parse_mcp_servers_from_toml(&toml_value),
            other: other.into_iter().collect(),
        })
    } else {
        // Claude 和 Qwen 使用 JSON 格式
        let content = fs::read_to_string(&config_file)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let config: ClaudeCliConfig =
            serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        Ok(config)
    }
}

fn read_opencode_config(config_file: &Path) -> Result<ClaudeCliConfig, String> {
    let content = fs::read_to_string(config_file)
        .map_err(|e| format!("Failed to read opencode config: {}", e))?;
    let json: serde_json::Value =
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}));

    let mut mcp_servers = HashMap::new();

    if let Some(mcp_obj) = json.get("mcp").and_then(|m| m.as_object()) {
        for (name, val) in mcp_obj {
            let server_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("local");

            let (command, args, env, url, headers, disabled) = if server_type == "remote" {
                let url = val.get("url").and_then(|u| u.as_str()).map(|s| s.to_string());
                let headers = val
                    .get("headers")
                    .and_then(|h| h.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    });
                (None, None, None, url, headers, false)
            } else {
                let cmd_array = val.get("command").and_then(|c| c.as_array());
                let command = cmd_array
                    .and_then(|arr| arr.first())
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                let args = cmd_array.map(|arr| {
                    arr.iter()
                        .skip(1)
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });
                let env = val
                    .get("environment")
                    .and_then(|e| e.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    });
                let enabled = val.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
                (command, args, env, None, None, !enabled)
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

    let mut other = BTreeMap::new();
    if let Some(obj) = json.as_object() {
        for (key, val) in obj {
            if key != "mcp" {
                other.insert(key.clone(), val.clone());
            }
        }
    }

    Ok(ClaudeCliConfig {
        mcpServers: if mcp_servers.is_empty() {
            None
        } else {
            Some(mcp_servers)
        },
        other,
    })
}

fn write_opencode_config(
    config_file: &Path,
    config: ClaudeCliConfig,
) -> Result<(), String> {
    let mut json = if config_file.exists() {
        let content = fs::read_to_string(config_file)
            .map_err(|e| format!("Failed to read opencode config: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let mcp_servers = config.mcpServers.unwrap_or_default();
    if !mcp_servers.is_empty() {
        let mut mcp_obj: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
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
                let mut cmd_array = vec![];
                if let Some(cmd) = &server.command {
                    cmd_array.push(serde_json::json!(cmd));
                }
                if let Some(args) = &server.args {
                    for arg in args {
                        cmd_array.push(serde_json::json!(arg));
                    }
                }
                entry.insert("command".to_string(), serde_json::json!(cmd_array));
                if let Some(env) = &server.env {
                    entry.insert("environment".to_string(), serde_json::json!(env));
                }
            }
            entry.insert(
                "enabled".to_string(),
                serde_json::json!(!server.disabled),
            );
            mcp_obj.insert(name.clone(), serde_json::Value::Object(entry));
        }
        json.as_object_mut()
            .map(|obj| obj.insert("mcp".to_string(), serde_json::Value::Object(mcp_obj)));
    } else {
        json.as_object_mut().map(|obj| obj.remove("mcp"));
    }

    let content = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize opencode config: {}", e))?;
    fs::write(config_file, content)
        .map_err(|e| format!("Failed to write opencode config: {}", e))?;

    Ok(())
}
/// 读取 CLI 配置文件 (Tauri 命令)
#[tauri::command]
pub fn read_cli_config(
    cli_path: String,
    cli_type: Option<String>,
) -> Result<ClaudeCliConfig, String> {
    read_cli_config_internal(&cli_path, cli_type.as_deref())
}

fn write_cli_config_internal(
    cli_path: &str,
    cli_type_hint: Option<&str>,
    config: ClaudeCliConfig,
) -> Result<(), String> {
    let paths = get_cli_config_paths_internal(cli_path, cli_type_hint)?;
    let config_file = PathBuf::from(&paths.config_file);

    // 确保父目录存在
    if let Some(parent) = config_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    if paths.cli_type == "opencode" {
        return write_opencode_config(&config_file, config);
    }

    if paths.cli_type == "codex" {
        // Codex 使用 TOML 格式
        let mut toml_table = json_map_to_toml_table(&config.other)?;

        // 转换 MCP 配置
        let mcp_servers = config.mcpServers.unwrap_or_default();
        if !mcp_servers.is_empty() {
            toml_table.insert(
                "mcp_servers".to_string(),
                toml::Value::Table(build_toml_mcp_servers(mcp_servers)),
            );
        } else {
            toml_table.remove("mcpServers");
            toml_table.remove("mcp_servers");
        }

        let content = toml::to_string_pretty(&toml::Value::Table(toml_table))
            .map_err(|e| format!("Failed to serialize TOML: {}", e))?;

        fs::write(&config_file, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
    } else {
        // Claude 和 Qwen 使用 JSON 格式
        let content = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

        fs::write(&config_file, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
    }

    Ok(())
}

/// 写入 CLI 配置文件 (Tauri 命令)
#[tauri::command]
pub fn write_cli_config(
    cli_path: String,
    cli_type: Option<String>,
    config: ClaudeCliConfig,
) -> Result<(), String> {
    write_cli_config_internal(&cli_path, cli_type.as_deref(), config)
}

/// 更新 CLI 配置中的 MCP 服务器 (Tauri 命令)
#[tauri::command]
pub fn update_cli_mcp_config(
    cli_path: String,
    cli_type: Option<String>,
    name: String,
    config: McpConfigUpdateInput,
) -> Result<(), String> {
    let mut cli_config = read_cli_config_internal(&cli_path, cli_type.as_deref())?;

    let server_config = McpServerConfig {
        command: config.command,
        args: config.args,
        env: config.env,
        url: config.url,
        headers: config.headers,
        disabled: config.disabled,
    };

    // 更新 MCP 服务器配置
    if cli_config.mcpServers.is_none() {
        cli_config.mcpServers = Some(HashMap::new());
    }

    if let Some(ref mut servers) = cli_config.mcpServers {
        servers.insert(name, server_config);
    }

    write_cli_config_internal(&cli_path, cli_type.as_deref(), cli_config)
}

/// 删除 CLI 配置中的 MCP 服务器 (Tauri 命令)
#[tauri::command]
pub fn delete_cli_mcp_config(
    cli_path: String,
    cli_type: Option<String>,
    name: String,
) -> Result<(), String> {
    let mut cli_config = read_cli_config_internal(&cli_path, cli_type.as_deref())?;

    // 从配置中删除
    if let Some(ref mut servers) = cli_config.mcpServers {
        servers.remove(&name);
    }

    write_cli_config_internal(&cli_path, cli_type.as_deref(), cli_config)
}

/// 打开配置文件 (Tauri 命令) - 使用系统默认编辑器打开
#[tauri::command]
pub async fn open_config_file(app: tauri::AppHandle, config_path: String) -> Result<(), String> {
    let path = PathBuf::from(&config_path);

    // 确保文件存在
    if !path.exists() {
        // 创建空的配置文件
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        // 根据文件扩展名创建空配置
        let content = if config_path.ends_with(".toml") {
            "# CLI Configuration\n".to_string()
        } else {
            "{\n}\n".to_string()
        };

        fs::write(&path, content).map_err(|e| format!("Failed to create config file: {}", e))?;
    }

    // 使用 tauri-plugin-opener 打开文件
    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| format!("Failed to open config file: {}", e))?;

    Ok(())
}

/// 获取 CLI 能力信息 (Tauri 命令)
#[tauri::command]
pub fn get_cli_capabilities(
    cli_path: String,
    cli_type: Option<String>,
) -> Result<CliCapabilities, String> {
    let cli_name = resolve_cli_name(Some(&cli_path), cli_type.as_deref(), "claude");

    let capabilities = match cli_name.as_str() {
        "claude" | "claude-code" => CliCapabilities {
            supports_mcp: true,
            supports_skills: true,
            supports_plugins: true,
            mcp_add_command: Some("claude mcp add".to_string()),
        },
        "codex" => CliCapabilities {
            supports_mcp: true,
            supports_skills: true,
            supports_plugins: false,
            mcp_add_command: Some("codex mcp add".to_string()),
        },
        "opencode" => CliCapabilities {
            supports_mcp: true,
            supports_skills: true,
            supports_plugins: false,
            mcp_add_command: Some("opencode mcp add".to_string()),
        },
        "qwen" | "qwen-code" => CliCapabilities {
            supports_mcp: true,
            supports_skills: true,
            supports_plugins: false,
            mcp_add_command: Some("qwen mcp add".to_string()),
        },
        _ => {
            // 默认使用 Claude 配置
            CliCapabilities {
                supports_mcp: true,
                supports_skills: true,
                supports_plugins: true,
                mcp_add_command: Some("claude mcp add".to_string()),
            }
        }
    };

    Ok(capabilities)
}

#[tauri::command]
pub fn sync_cli_items(input: CliSyncInput) -> Result<CliSyncResult, String> {
    let _ = validate_sync_cli_paths(
        &input.source_cli_path,
        &input.target_cli_path,
        input.source_cli_type.as_deref(),
        input.target_cli_type.as_deref(),
    )?;

    if input.item_names.is_empty() {
        return Ok(CliSyncResult::default());
    }

    match input.config_type {
        CliSyncConfigType::Mcp => sync_mcp_items(
            &input.source_cli_path,
            &input.target_cli_path,
            input.source_cli_type.as_deref(),
            input.target_cli_type.as_deref(),
            &input.item_names,
            &input.conflict_policy,
        ),
        CliSyncConfigType::Skills => sync_skill_items(
            &input.source_cli_path,
            &input.target_cli_path,
            input.source_cli_type.as_deref(),
            input.target_cli_type.as_deref(),
            &input.item_names,
            &input.conflict_policy,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;
    use uuid::Uuid;

    static HOME_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    struct TestHome {
        root: PathBuf,
        previous_home: Option<String>,
    }

    impl TestHome {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("easy-agent-cli-sync-{}", Uuid::new_v4()));
            fs::create_dir_all(&root).expect("create test home");

            let previous_home = std::env::var("HOME").ok();
            std::env::set_var("HOME", &root);

            Self {
                root,
                previous_home,
            }
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.join(relative)
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            if let Some(previous_home) = &self.previous_home {
                std::env::set_var("HOME", previous_home);
            } else {
                std::env::remove_var("HOME");
            }

            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn write_cli_config_uses_codex_mcp_servers_key_and_preserves_other_fields() {
        let _guard = HOME_LOCK.lock().unwrap();
        let test_home = TestHome::new();
        let codex_path = "/usr/local/bin/codex".to_string();
        let config_path = test_home.path(".codex/config.toml");

        let mut config = ClaudeCliConfig {
            mcpServers: Some(HashMap::from([(
                "docs".to_string(),
                McpServerConfig {
                    url: Some("https://developers.openai.com/mcp".to_string()),
                    ..Default::default()
                },
            )])),
            other: BTreeMap::from([
                ("model".to_string(), JsonValue::String("gpt-5".to_string())),
                (
                    "approval_policy".to_string(),
                    JsonValue::String("never".to_string()),
                ),
            ]),
        };

        write_cli_config_internal(&codex_path, Some("codex"), config.clone())
            .expect("write codex config");

        let content = fs::read_to_string(config_path).expect("read codex config");
        assert!(content.contains("[mcp_servers.docs]"));
        assert!(content.contains("url = \"https://developers.openai.com/mcp\""));
        assert!(content.contains("model = \"gpt-5\""));
        assert!(!content.contains("[mcpServers.docs]"));

        let reloaded =
            read_cli_config_internal(&codex_path, Some("codex")).expect("read codex config");
        assert_eq!(
            reloaded
                .mcpServers
                .as_ref()
                .and_then(|servers| servers.get("docs"))
                .and_then(|server| server.url.as_ref())
                .map(String::as_str),
            Some("https://developers.openai.com/mcp")
        );
        assert_eq!(
            reloaded.other.get("model").and_then(|value| value.as_str()),
            Some("gpt-5")
        );

        config.mcpServers = None;
    }

    #[test]
    fn sync_mcp_items_copies_claude_json_into_codex_toml() {
        let _guard = HOME_LOCK.lock().unwrap();
        let test_home = TestHome::new();
        let claude_config_path = test_home.path(".claude.json");
        let codex_config_path = test_home.path(".codex/config.toml");

        fs::create_dir_all(test_home.path(".codex")).expect("create codex dir");
        fs::write(
            &claude_config_path,
            r#"{
  "mcpServers": {
    "docs": {
      "url": "https://developers.openai.com/mcp"
    }
  }
}"#,
        )
        .expect("write claude config");
        fs::write(
            &codex_config_path,
            r#"model = "gpt-5"
"#,
        )
        .expect("write codex config");

        let result = sync_cli_items(CliSyncInput {
            source_cli_path: "/usr/local/bin/claude".to_string(),
            target_cli_path: "/usr/local/bin/codex".to_string(),
            source_cli_type: Some("claude".to_string()),
            target_cli_type: Some("codex".to_string()),
            config_type: CliSyncConfigType::Mcp,
            item_names: vec!["docs".to_string()],
            conflict_policy: CliSyncConflictPolicy::Skip,
        })
        .expect("sync mcp items");

        assert_eq!(result.success_count, 1);
        assert_eq!(result.skipped_count, 0);
        assert_eq!(result.failed_count, 0);

        let content = fs::read_to_string(codex_config_path).expect("read synced codex config");
        assert!(content.contains("model = \"gpt-5\""));
        assert!(content.contains("[mcp_servers.docs]"));
        assert!(content.contains("url = \"https://developers.openai.com/mcp\""));
    }

    #[test]
    fn sync_skill_items_copies_skill_directory_between_clis() {
        let _guard = HOME_LOCK.lock().unwrap();
        let test_home = TestHome::new();
        let source_skill_dir = test_home.path(".claude/skills/backend-audit");
        let target_skill_dir = test_home.path(".codex/skills/backend-audit");

        fs::create_dir_all(source_skill_dir.join("references")).expect("create source skill refs");
        fs::write(
            source_skill_dir.join("SKILL.md"),
            r#"---
name: backend-audit
description: Audit backend services
---

Use this skill for backend reviews.
"#,
        )
        .expect("write skill file");
        fs::write(
            source_skill_dir.join("references/checklist.md"),
            "# checklist",
        )
        .expect("write reference file");

        let result = sync_cli_items(CliSyncInput {
            source_cli_path: "/usr/local/bin/claude".to_string(),
            target_cli_path: "/usr/local/bin/codex".to_string(),
            source_cli_type: Some("claude".to_string()),
            target_cli_type: Some("codex".to_string()),
            config_type: CliSyncConfigType::Skills,
            item_names: vec!["backend-audit".to_string()],
            conflict_policy: CliSyncConflictPolicy::Skip,
        })
        .expect("sync skill items");

        assert_eq!(result.success_count, 1);
        assert!(target_skill_dir.join("SKILL.md").exists());
        assert!(target_skill_dir.join("references/checklist.md").exists());
        let copied_content =
            fs::read_to_string(target_skill_dir.join("SKILL.md")).expect("read copied skill file");
        assert!(copied_content.contains("backend-audit"));
    }
}
