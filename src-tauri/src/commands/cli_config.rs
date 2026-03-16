use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri_plugin_opener::OpenerExt;

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
    #[serde(flatten)]
    pub other: Option<serde_json::Value>,
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

/// 根据 CLI 路径获取配置目录和信息
pub(crate) fn get_cli_config_paths_internal(cli_path: &str) -> Result<CliConfigPaths, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    // 从路径中提取 CLI 名称
    let cli_name = std::path::Path::new(cli_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("claude")
        .to_lowercase();

    match cli_name.as_str() {
        "claude" | "claude-code" => {
            let config_dir = home_dir.join(".claude");
            let config_file = home_dir.join(".claude.json");
            Ok(CliConfigPaths {
                config_dir: config_dir.to_string_lossy().to_string(),
                config_file: config_file.to_string_lossy().to_string(),
                cli_type: "claude".to_string(),
            })
        }
        "codex" => {
            let config_dir = home_dir.join(".codex");
            let config_file = home_dir.join(".codex").join("config.toml");
            Ok(CliConfigPaths {
                config_dir: config_dir.to_string_lossy().to_string(),
                config_file: config_file.to_string_lossy().to_string(),
                cli_type: "codex".to_string(),
            })
        }
        "qwen" | "qwen-code" => {
            let config_dir = home_dir.join(".qwen");
            let config_file = home_dir.join(".qwen").join("settings.json");
            Ok(CliConfigPaths {
                config_dir: config_dir.to_string_lossy().to_string(),
                config_file: config_file.to_string_lossy().to_string(),
                cli_type: "qwen".to_string(),
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
            })
        }
    }
}

/// 获取 CLI 配置路径信息 (Tauri 命令)
#[tauri::command]
pub fn get_cli_config_paths(cli_path: String) -> Result<CliConfigPaths, String> {
    get_cli_config_paths_internal(&cli_path)
}

/// 读取 CLI 配置文件 (Tauri 命令)
#[tauri::command]
pub fn read_cli_config(cli_path: String) -> Result<ClaudeCliConfig, String> {
    let paths = get_cli_config_paths_internal(&cli_path)?;
    let config_file = PathBuf::from(&paths.config_file);

    if !config_file.exists() {
        // 返回空配置而不是错误
        return Ok(ClaudeCliConfig::default());
    }

    // 根据文件类型读取
    if paths.cli_type == "codex" {
        // Codex 使用 TOML 格式
        let content = fs::read_to_string(&config_file)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        // 解析 TOML 并转换为 JSON 格式
        let toml_value: toml::Value =
            toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {}", e))?;

        // 转换 MCP 配置
        let mcp_servers = if let Some(mcp) = toml_value
            .get("mcp_servers")
            .or_else(|| toml_value.get("mcpServers"))
        {
            let mut servers = HashMap::new();
            if let Some(mcp_obj) = mcp.as_table() {
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
                                    .filter_map(|(k, v)| {
                                        v.as_str().map(|s| (k.clone(), s.to_string()))
                                    })
                                    .collect()
                            }),
                            url: config_obj
                                .get("url")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            headers: config_obj.get("headers").and_then(|v| v.as_table()).map(
                                |obj| {
                                    obj.iter()
                                        .filter_map(|(k, v)| {
                                            v.as_str().map(|s| (k.clone(), s.to_string()))
                                        })
                                        .collect()
                                },
                            ),
                            disabled: config_obj
                                .get("disabled")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                        };
                        servers.insert(name.clone(), server_config);
                    }
                }
            }
            Some(servers)
        } else {
            None
        };

        Ok(ClaudeCliConfig {
            mcpServers: mcp_servers,
            other: None,
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

/// 写入 CLI 配置文件 (Tauri 命令)
#[tauri::command]
pub fn write_cli_config(cli_path: String, config: ClaudeCliConfig) -> Result<(), String> {
    let paths = get_cli_config_paths_internal(&cli_path)?;
    let config_file = PathBuf::from(&paths.config_file);

    // 确保父目录存在
    if let Some(parent) = config_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    if paths.cli_type == "codex" {
        // Codex 使用 TOML 格式
        let mut toml_table = toml::map::Map::new();

        // 转换 MCP 配置
        let mcp_servers = config.mcpServers.unwrap_or_default();
        if !mcp_servers.is_empty() {
            let mut mcp_table = toml::map::Map::new();
            for (name, server_config) in mcp_servers {
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
            toml_table.insert("mcpServers".to_string(), toml::Value::Table(mcp_table));
        }

        let content = toml::to_string_pretty(&toml_table)
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

/// 更新 CLI 配置中的 MCP 服务器 (Tauri 命令)
#[tauri::command]
pub fn update_cli_mcp_config(
    cli_path: String,
    name: String,
    config: McpConfigUpdateInput,
) -> Result<(), String> {
    let mut cli_config = read_cli_config(cli_path.clone())?;

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

    write_cli_config(cli_path, cli_config)
}

/// 删除 CLI 配置中的 MCP 服务器 (Tauri 命令)
#[tauri::command]
pub fn delete_cli_mcp_config(cli_path: String, name: String) -> Result<(), String> {
    let mut cli_config = read_cli_config(cli_path.clone())?;

    // 从配置中删除
    if let Some(ref mut servers) = cli_config.mcpServers {
        servers.remove(&name);
    }

    write_cli_config(cli_path, cli_config)
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
pub fn get_cli_capabilities(cli_path: String) -> Result<CliCapabilities, String> {
    // 从路径中提取 CLI 名称
    let cli_name = std::path::Path::new(&cli_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("claude")
        .to_lowercase();

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
            supports_plugins: false, // Codex CLI 不支持 Plugins
            mcp_add_command: Some("codex mcp add".to_string()),
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
