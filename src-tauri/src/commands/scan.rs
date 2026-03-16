use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;

use crate::commands::scan_session_shared::{
    clean_display_text, collect_jsonl_files, delete_cli_session_path,
    extract_jsonl_message_content, extract_jsonl_message_type, extract_jsonl_project_path,
    extract_jsonl_role,
};

/// MCP 传输类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum McpTransportType {
    Stdio,
    Sse,
    Http,
}

/// MCP 配置范围
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum McpConfigScope {
    User,
    Local,
    Project,
}

/// 扫描到的 MCP 配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedMcpServer {
    pub name: String,
    pub transport: McpTransportType,
    pub scope: McpConfigScope,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub url: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Skill 子目录信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSubdirectories {
    pub has_scripts: bool,
    pub has_references: bool,
    pub has_assets: bool,
}

/// 扫描到的 Skill 配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedSkill {
    /// 从 YAML frontmatter 解析的名称
    pub name: String,
    /// Skill 目录路径
    pub path: String,
    /// 从 YAML frontmatter 解析的描述
    pub description: Option<String>,
    /// YAML frontmatter 中的原始名称（可能与目录名不同）
    pub frontmatter_name: Option<String>,
    /// 子目录信息
    pub subdirectories: SkillSubdirectories,
}

/// Plugin 子目录信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSubdirectories {
    pub has_agents: bool,
    pub has_commands: bool,
    pub has_skills: bool,
    pub has_hooks: bool,
    pub has_scripts: bool,
}

/// 扫描到的 Plugin 配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedPlugin {
    /// 插件名称（从 plugin.json 解析，否则使用目录名）
    pub name: String,
    /// 插件路径
    pub path: String,
    /// 是否启用
    pub enabled: bool,
    /// 从 plugin.json 解析的版本
    pub version: Option<String>,
    /// 从 plugin.json 解析的描述
    pub description: Option<String>,
    /// 作者信息
    pub author: Option<String>,
    /// 子目录信息
    pub subdirectories: PluginSubdirectories,
}

/// Claude CLI 配置扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeConfigScanResult {
    pub claude_dir: String,
    pub mcp_servers: Vec<ScannedMcpServer>,
    pub skills: Vec<ScannedSkill>,
    pub plugins: Vec<ScannedPlugin>,
    pub scan_success: bool,
    pub error_message: Option<String>,
}

/// 根据 CLI 路径获取配置目录和信息
fn get_cli_config_dir(cli_path: Option<&str>) -> Result<(PathBuf, PathBuf, String), String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    // 如果没有提供 cliPath，默认使用 Claude
    let cli_name = if let Some(path) = cli_path {
        // 从路径中提取 CLI 名称
        std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("claude")
            .to_lowercase()
    } else {
        "claude".to_string()
    };

    match cli_name.as_str() {
        "claude" | "claude-code" => {
            // Claude CLI: 配置在 ~/.claude/ 目录，配置文件是 ~/.claude.json
            let config_dir = home_dir.join(".claude");
            let config_file = home_dir.join(".claude.json");
            Ok((config_dir, config_file, "claude".to_string()))
        }
        "codex" => {
            // Codex CLI: 配置在 ~/.codex/config.toml
            let config_dir = home_dir.join(".codex");
            let config_file = home_dir.join(".codex").join("config.toml");
            Ok((config_dir, config_file, "codex".to_string()))
        }
        "qwen" | "qwen-code" => {
            // Qwen Code: 配置在 ~/.qwen/settings.json
            let config_dir = home_dir.join(".qwen");
            let config_file = home_dir.join(".qwen").join("settings.json");
            Ok((config_dir, config_file, "qwen".to_string()))
        }
        _ => {
            // 默认使用 Claude 配置
            let config_dir = home_dir.join(".claude");
            let config_file = home_dir.join(".claude.json");
            Ok((config_dir, config_file, "claude".to_string()))
        }
    }
}

/// 获取 Claude 配置目录路径
#[allow(dead_code)]
fn get_claude_config_dir() -> Result<PathBuf> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    Ok(home_dir.join(".claude"))
}

/// 解析单个 MCP 服务器配置
fn transport_from_config_value(value: &str) -> Option<McpTransportType> {
    match value.to_lowercase().as_str() {
        "sse" => Some(McpTransportType::Sse),
        "http" => Some(McpTransportType::Http),
        "stdio" => Some(McpTransportType::Stdio),
        _ => None,
    }
}

fn infer_transport_from_fields(
    url: Option<&String>,
    command: Option<&String>,
) -> Option<McpTransportType> {
    if let Some(url_str) = url {
        return Some(if url_str.contains("/sse") || url_str.contains("sse") {
            McpTransportType::Sse
        } else {
            McpTransportType::Http
        });
    }

    command.map(|_| McpTransportType::Stdio)
}

pub(crate) fn parse_mcp_server_config(
    name: &str,
    config_obj: &serde_json::Map<String, serde_json::Value>,
    scope: McpConfigScope,
) -> Option<ScannedMcpServer> {
    // 解析 url 字段
    let url = config_obj
        .get("url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 解析 command 字段
    let command = config_obj
        .get("command")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 解析 args 字段
    let args = config_obj
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        });

    // 解析 env 字段
    let env = crate::commands::scan_shared::parse_string_map(config_obj.get("env"));

    // 解析 headers 字段
    let headers = crate::commands::scan_shared::parse_string_map(config_obj.get("headers"));

    // 推断传输类型
    let transport = config_obj
        .get("transport")
        .and_then(|value| value.as_str())
        .and_then(transport_from_config_value)
        .or_else(|| infer_transport_from_fields(url.as_ref(), command.as_ref()))?;

    Some(ScannedMcpServer {
        name: name.to_string(),
        transport,
        scope,
        command,
        args,
        env,
        url,
        headers,
    })
}

/// 扫描 MCP 配置
fn scan_mcp_config(config_dir: &PathBuf, config_file: &PathBuf) -> Result<Vec<ScannedMcpServer>> {
    let mut servers = Vec::new();

    // 1. 首先尝试从 ~/.claude.json (或对应 CLI 的配置文件) 读取用户级 MCP 配置 (user scope)
    crate::commands::scan_shared::scan_mcp_source_file(
        config_file,
        McpConfigScope::User,
        &mut servers,
    );

    // 2. 尝试从 config_dir/settings.json 读取 MCP 配置 (user scope)
    let settings_path = config_dir.join("settings.json");
    crate::commands::scan_shared::scan_mcp_source_file(
        &settings_path,
        McpConfigScope::User,
        &mut servers,
    );

    // 3. 尝试从 .mcp.json 读取项目级配置 (local scope)
    let mcp_json_path = config_dir.join(".mcp.json");
    crate::commands::scan_shared::scan_mcp_source_file(
        &mcp_json_path,
        McpConfigScope::Local,
        &mut servers,
    );

    Ok(servers)
}

/// 解析 YAML frontmatter 中的字段
fn parse_yaml_frontmatter(content: &str) -> (Option<String>, Option<String>) {
    // YAML frontmatter 格式:
    // ---
    // name: skill-name
    // description: skill description
    // ---
    let lines: Vec<&str> = content.lines().collect();

    // 查找 frontmatter 边界
    let start_idx = lines.iter().position(|line| line.trim() == "---");
    let end_idx = if let Some(start) = start_idx {
        lines
            .iter()
            .skip(start + 1)
            .position(|line| line.trim() == "---")
            .map(|idx| start + 1 + idx)
    } else {
        None
    };

    if let (Some(start), Some(end)) = (start_idx, end_idx) {
        let frontmatter_lines = &lines[start + 1..end];
        let mut name: Option<String> = None;
        let mut description: Option<String> = None;

        for line in frontmatter_lines {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "name" => name = Some(value.to_string()),
                    "description" => description = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        (name, description)
    } else {
        (None, None)
    }
}

/// 检查 Skill 目录的子目录结构
fn check_skill_subdirectories(skill_path: &PathBuf) -> SkillSubdirectories {
    SkillSubdirectories {
        has_scripts: skill_path.join("scripts").exists(),
        has_references: skill_path.join("references").exists(),
        has_assets: skill_path.join("assets").exists(),
    }
}

/// 扫描 Skills 目录
fn scan_skills_directory(claude_dir: &PathBuf) -> Result<Vec<ScannedSkill>> {
    let mut skills = Vec::new();
    let skills_dir = claude_dir.join("skills");

    if !skills_dir.exists() {
        return Ok(skills);
    }

    let entries = fs::read_dir(&skills_dir)?;
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();

            // 检查是否为符号链接，如果是则获取目标路径
            let actual_path = if path.is_symlink() {
                match fs::read_link(&path) {
                    Ok(target) => {
                        // 如果是相对路径，转换为绝对路径
                        if target.is_relative() {
                            path.parent()
                                .map(|p| p.join(&target))
                                .unwrap_or(path.clone())
                        } else {
                            target
                        }
                    }
                    Err(_) => path.clone(),
                }
            } else {
                path.clone()
            };

            if actual_path.is_dir() {
                // 使用目录名作为默认名称
                let dir_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                // 尝试读取 SKILL.md（优先）或 skill.md
                let skill_md = actual_path.join("SKILL.md");
                let skill_md_lower = actual_path.join("skill.md");

                let md_path = if skill_md.exists() {
                    Some(skill_md)
                } else if skill_md_lower.exists() {
                    Some(skill_md_lower)
                } else {
                    None
                };

                let (frontmatter_name, description) = if let Some(md_path) = md_path {
                    if let Ok(content) = fs::read_to_string(&md_path) {
                        parse_yaml_frontmatter(&content)
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };

                // 使用 frontmatter 中的 name 作为显示名称，否则使用目录名
                let display_name = frontmatter_name.clone().unwrap_or_else(|| dir_name.clone());

                // 检查子目录
                let subdirectories = check_skill_subdirectories(&actual_path);

                skills.push(ScannedSkill {
                    name: display_name,
                    path: path.to_string_lossy().to_string(),
                    description,
                    frontmatter_name,
                    subdirectories,
                });
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                // 单个 .md 文件作为 skill（不支持 frontmatter，保持向后兼容）
                let name = path
                    .file_stem()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let description = fs::read_to_string(&path)
                    .ok()
                    .and_then(|content| content.lines().next().map(|s| s.trim().to_string()));

                skills.push(ScannedSkill {
                    name,
                    path: path.to_string_lossy().to_string(),
                    description,
                    frontmatter_name: None,
                    subdirectories: SkillSubdirectories {
                        has_scripts: false,
                        has_references: false,
                        has_assets: false,
                    },
                });
            }
        }
    }

    Ok(skills)
}

/// 解析 plugin.json 文件
fn parse_plugin_json(
    plugin_json_path: &PathBuf,
) -> (Option<String>, Option<String>, Option<String>) {
    if let Ok(content) = fs::read_to_string(plugin_json_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            let version = json
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let description = json
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let author = crate::commands::scan_shared::parse_author_value(json.get("author"));

            return (version, description, author);
        }
    }
    (None, None, None)
}

/// 检查 Plugin 目录的子目录结构
fn check_plugin_subdirectories(plugin_path: &PathBuf) -> PluginSubdirectories {
    PluginSubdirectories {
        has_agents: plugin_path.join("agents").exists(),
        has_commands: plugin_path.join("commands").exists(),
        has_skills: plugin_path.join("skills").exists(),
        has_hooks: plugin_path.join("hooks").exists(),
        has_scripts: plugin_path.join("scripts").exists(),
    }
}

/// 扫描 Plugins 目录
fn scan_plugins_directory(claude_dir: &PathBuf) -> Result<Vec<ScannedPlugin>> {
    let mut plugins = Vec::new();
    let plugins_dir = claude_dir.join("plugins");

    if !plugins_dir.exists() {
        return Ok(plugins);
    }

    // 尝试从 installed_plugins.json 读取已安装的插件
    let installed_plugins_path = plugins_dir.join("installed_plugins.json");
    if installed_plugins_path.exists() {
        if let Ok(content) = fs::read_to_string(&installed_plugins_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // 解析 installed_plugins.json 格式:
                // { "version": 2, "plugins": { "name@source": [ { "installPath": "...", "version": "...", ... } ] } }
                if let Some(plugins_obj) = json.get("plugins").and_then(|v| v.as_object()) {
                    for (plugin_key, plugin_entries) in plugins_obj {
                        // 获取第一个安装条目（通常只有一个）
                        if let Some(entries) = plugin_entries.as_array() {
                            if let Some(first_entry) = entries.first() {
                                // 获取安装路径
                                if let Some(install_path_str) =
                                    first_entry.get("installPath").and_then(|v| v.as_str())
                                {
                                    let install_path = PathBuf::from(install_path_str);

                                    if install_path.exists() {
                                        // 解析 plugin.json（位于 .claude-plugin/plugin.json）
                                        let plugin_json_path =
                                            install_path.join(".claude-plugin").join("plugin.json");
                                        let (version, description, author) =
                                            parse_plugin_json(&plugin_json_path);

                                        // 检查子目录
                                        let subdirectories =
                                            check_plugin_subdirectories(&install_path);

                                        // 从 plugin_key 中提取名称（格式: name@source）
                                        let display_name =
                                            plugin_key.split('@').next().unwrap_or(plugin_key);

                                        // 检查是否启用（检查 scope 是否为 user）
                                        let enabled = first_entry
                                            .get("scope")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s == "user")
                                            .unwrap_or(true);

                                        plugins.push(ScannedPlugin {
                                            name: display_name.to_string(),
                                            path: install_path.to_string_lossy().to_string(),
                                            enabled,
                                            version,
                                            description,
                                            author,
                                            subdirectories,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 如果没有从 installed_plugins.json 解析到插件，则扫描目录（向后兼容）
    if plugins.is_empty() {
        let entries = fs::read_dir(&plugins_dir)?;
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    // 检查是否启用（可以通过存在 .disabled 文件来判断）
                    let disabled_marker = path.join(".disabled");
                    let enabled = !disabled_marker.exists();

                    // 尝试解析 plugin.json
                    let plugin_json_path = path.join(".claude-plugin").join("plugin.json");
                    let (version, description, author) = parse_plugin_json(&plugin_json_path);

                    // 检查子目录
                    let subdirectories = check_plugin_subdirectories(&path);

                    plugins.push(ScannedPlugin {
                        name,
                        path: path.to_string_lossy().to_string(),
                        enabled,
                        version,
                        description,
                        author,
                        subdirectories,
                    });
                }
            }
        }
    }

    Ok(plugins)
}

/// 扫描 CLI 配置 (Tauri 命令)
#[tauri::command]
pub fn scan_cli_config(cli_path: Option<String>) -> Result<ClaudeConfigScanResult, String> {
    // 获取 CLI 配置目录和配置文件路径
    let (config_dir, config_file, cli_name) = match get_cli_config_dir(cli_path.as_deref()) {
        Ok(result) => result,
        Err(e) => {
            return Ok(ClaudeConfigScanResult {
                claude_dir: String::new(),
                mcp_servers: Vec::new(),
                skills: Vec::new(),
                plugins: Vec::new(),
                scan_success: false,
                error_message: Some(format!("无法确定配置目录: {}", e)),
            });
        }
    };

    let config_dir_str = config_dir.to_string_lossy().to_string();

    // 检查配置文件是否存在
    if !config_file.exists() && !config_dir.exists() {
        return Ok(ClaudeConfigScanResult {
            claude_dir: config_dir_str,
            mcp_servers: Vec::new(),
            skills: Vec::new(),
            plugins: Vec::new(),
            scan_success: false,
            error_message: Some(format!("{} 配置不存在", cli_name)),
        });
    }

    // 扫描 MCP 配置
    let mcp_servers = scan_mcp_config(&config_dir, &config_file).unwrap_or_default();

    // 扫描 Skills 目录
    let skills = scan_skills_directory(&config_dir).unwrap_or_default();

    // 扫描 Plugins 目录
    let plugins = scan_plugins_directory(&config_dir).unwrap_or_default();

    Ok(ClaudeConfigScanResult {
        claude_dir: config_dir_str,
        mcp_servers,
        skills,
        plugins,
        scan_success: true,
        error_message: None,
    })
}

/// 尝试通过 claude mcp list 命令获取 MCP 配置
#[tauri::command]
pub fn scan_claude_mcp_list() -> Result<Vec<ScannedMcpServer>, String> {
    let output = Command::new("claude").arg("mcp").arg("list").output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // 解析输出，格式可能是：
                // server-name: command args
                let mut servers = Vec::new();
                for line in stdout.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }

                    // 尝试解析 "name: command args" 格式
                    if let Some((name, rest)) = line.split_once(':') {
                        let name = name.trim().to_string();
                        let rest = rest.trim();

                        // 简单分割命令和参数
                        let parts: Vec<&str> = rest.split_whitespace().collect();
                        if !parts.is_empty() {
                            let command = parts[0].to_string();
                            let args: Vec<String> =
                                parts[1..].iter().map(|s| s.to_string()).collect();

                            servers.push(ScannedMcpServer {
                                name,
                                transport: McpTransportType::Stdio,
                                scope: McpConfigScope::User,
                                command: Some(command),
                                args: if args.is_empty() { None } else { Some(args) },
                                env: None,
                                url: None,
                                headers: None,
                            });
                        }
                    }
                }
                Ok(servers)
            } else {
                // 命令执行失败，返回空列表
                Ok(Vec::new())
            }
        }
        Err(_) => {
            // claude 命令不存在，返回空列表
            Ok(Vec::new())
        }
    }
}

/// 扫描到的智能体会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedCliSession {
    /// 会话ID
    pub session_id: String,
    /// 会话文件路径
    pub session_path: String,
    /// 项目路径（如果适用）
    pub project_path: Option<String>,
    /// 首条消息（作为会话名称的参考）
    pub first_message: Option<String>,
    /// 消息数量
    pub message_count: i32,
    /// 创建时间（文件修改时间）
    pub created_at: String,
    /// 更新时间（文件修改时间）
    pub updated_at: String,
}

/// 扫描智能体会话历史的输入参数
#[derive(Debug, Deserialize)]
pub struct ScanCliSessionsInput {
    /// CLI路径（用于确定智能体类型）
    pub cli_path: Option<String>,
    /// 项目路径（可选，用于筛选特定项目的会话）
    pub project_path: Option<String>,
}

/// 扫描智能体会话历史的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCliSessionsResult {
    /// CLI名称
    pub cli_name: String,
    /// 配置目录
    pub config_dir: String,
    /// 扫描到的会话列表
    pub sessions: Vec<ScannedCliSession>,
    /// 扫描是否成功
    pub scan_success: bool,
    /// 错误信息
    pub error_message: Option<String>,
}

const FAST_SESSION_SCAN_LINE_LIMIT: usize = 120;
const FULL_SESSION_SCAN_SIZE_THRESHOLD: u64 = 256 * 1024;
const UNKNOWN_MESSAGE_COUNT: i32 = -1;

fn extract_session_project_path(session_path: &PathBuf) -> Option<String> {
    let file = fs::File::open(session_path).ok()?;
    let reader = BufReader::new(file);

    for (index, line) in reader.lines().enumerate() {
        let Ok(line) = line else {
            continue;
        };

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(project_path) = extract_jsonl_project_path(&json) {
                return Some(project_path);
            }
        }

        if index >= FAST_SESSION_SCAN_LINE_LIMIT {
            break;
        }
    }

    None
}

/// 从会话jsonl文件中提取会话信息
fn extract_session_info(session_path: &PathBuf, force_fast_scan: bool) -> Option<ScannedCliSession> {
    let file_name = session_path.file_stem()?.to_string_lossy().to_string();
    let metadata = fs::metadata(session_path).ok()?;
    let modified = metadata
        .modified()
        .ok()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let modified_str = chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339();
    let fast_scan_only = force_fast_scan || metadata.len() > FULL_SESSION_SCAN_SIZE_THRESHOLD;

    let file = fs::File::open(session_path).ok()?;
    let reader = BufReader::new(file);

    let mut first_message: Option<String> = None;
    let mut message_count = if fast_scan_only {
        UNKNOWN_MESSAGE_COUNT
    } else {
        0
    };
    let mut project_path: Option<String> = None;
    let mut earliest_timestamp: Option<String> = None;
    let mut latest_timestamp: Option<String> = None;

    for (index, line) in reader.lines().enumerate() {
        let Ok(line) = line else {
            continue;
        };

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            let msg_type = extract_jsonl_message_type(&json);

            if !fast_scan_only && (msg_type == "user" || msg_type == "assistant") {
                message_count += 1;
            }

            if first_message.is_none() && msg_type == "user" {
                if let Some(text) = extract_jsonl_message_content(&json) {
                    let preview: String = text.chars().take(100).collect();
                    first_message = clean_display_text(&preview);
                }
            }

            if project_path.is_none() {
                project_path = extract_jsonl_project_path(&json);
            }

            if let Some(timestamp) = json.get("timestamp").and_then(|v| v.as_str()) {
                let ts = timestamp.to_string();
                if earliest_timestamp.is_none() || &ts < earliest_timestamp.as_ref().unwrap() {
                    earliest_timestamp = Some(ts.clone());
                }
                if !fast_scan_only
                    && (latest_timestamp.is_none() || &ts > latest_timestamp.as_ref().unwrap())
                {
                    latest_timestamp = Some(ts);
                }
            }
        }

        if fast_scan_only
            && index >= FAST_SESSION_SCAN_LINE_LIMIT
            && first_message.is_some()
            && project_path.is_some()
        {
            break;
        }
    }

    Some(ScannedCliSession {
        session_id: file_name,
        session_path: session_path.to_string_lossy().to_string(),
        project_path,
        first_message,
        message_count,
        created_at: earliest_timestamp.unwrap_or_else(|| modified_str.clone()),
        updated_at: latest_timestamp.unwrap_or(modified_str),
    })
}

fn list_cli_session_project_paths(cli_name: &str, config_dir: &PathBuf) -> Vec<String> {
    let mut project_paths = Vec::new();

    match cli_name {
        "claude" | "claude-code" => {
            let projects_dir = config_dir.join("projects");
            if let Ok(project_entries) = fs::read_dir(&projects_dir) {
                for project_entry in project_entries.flatten() {
                    let project_dir = project_entry.path();
                    if !project_dir.is_dir() {
                        continue;
                    }

                    if let Ok(session_entries) = fs::read_dir(&project_dir) {
                        for session_entry in session_entries.flatten() {
                            let session_path = session_entry.path();
                            if session_path
                                .extension()
                                .map(|ext| ext == "jsonl")
                                .unwrap_or(false)
                            {
                                if let Some(project_path) =
                                    extract_session_project_path(&session_path)
                                {
                                    project_paths.push(project_path);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        "codex" => {
            let sessions_dir = config_dir.join("sessions");
            if sessions_dir.exists() {
                let mut session_files = Vec::new();
                collect_jsonl_files(&sessions_dir, &mut session_files);

                for session_file in session_files {
                    if let Some(project_path) = extract_session_project_path(&session_file) {
                        project_paths.push(project_path);
                    }
                }
            }
        }
        _ => {}
    }

    project_paths.sort();
    project_paths.dedup();
    project_paths
}

/// 根据 agentId 获取 agent 的 cli_path
fn get_agent_cli_path(agent_id: &str) -> Result<String, String> {
    let persistence_dir = super::get_persistence_dir_path().map_err(|e| e.to_string())?;
    let db_path = persistence_dir.join("data").join("easy-agent.db");

    let conn = Connection::open(&db_path).map_err(|e| format!("无法打开数据库: {}", e))?;

    let cli_path: Option<String> = conn
        .query_row(
            "SELECT cli_path FROM agents WHERE id = ?1",
            [&agent_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(|e| format!("无法获取 agent 的 cli_path: {}", e))?;

    cli_path.ok_or_else(|| "该 agent 没有配置 cli_path".to_string())
}

/// Agent CLI 会话列表结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCliSessionsResult {
    /// Agent ID
    pub agent_id: String,
    /// CLI 名称
    pub cli_name: String,
    /// 会话根目录
    pub session_root: String,
    /// 会话列表
    pub sessions: Vec<ScannedCliSession>,
    /// 项目路径列表（用于筛选下拉框）
    pub project_paths: Vec<String>,
}

/// Agent CLI 会话项目列表结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCliSessionProjectsResult {
    /// Agent ID
    pub agent_id: String,
    /// CLI 名称
    pub cli_name: String,
    /// 会话根目录
    pub session_root: String,
    /// 项目路径列表
    pub project_paths: Vec<String>,
}

/// 列出指定 Agent 的 CLI 会话项目列表 (Tauri 命令)
#[tauri::command]
pub async fn list_agent_cli_session_projects(
    agent_id: String,
) -> Result<AgentCliSessionProjectsResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        list_agent_cli_session_projects_impl(agent_id)
    })
    .await
    .map_err(|e| format!("加载会话项目列表失败: {}", e))?
}

fn list_agent_cli_session_projects_impl(
    agent_id: String,
) -> Result<AgentCliSessionProjectsResult, String> {
    let cli_path = get_agent_cli_path(&agent_id)?;
    let (config_dir, _, cli_name) =
        get_cli_config_dir(Some(cli_path.as_str())).map_err(|e| format!("无法确定配置目录: {}", e))?;

    Ok(AgentCliSessionProjectsResult {
        agent_id,
        cli_name: cli_name.clone(),
        session_root: config_dir.to_string_lossy().to_string(),
        project_paths: list_cli_session_project_paths(&cli_name, &config_dir),
    })
}

/// 列出指定 Agent 的 CLI 会话 (Tauri 命令)
#[tauri::command]
pub async fn list_agent_cli_sessions(
    agent_id: String,
    project_path: Option<String>,
) -> Result<AgentCliSessionsResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        list_agent_cli_sessions_impl(agent_id, project_path)
    })
    .await
    .map_err(|e| format!("加载会话列表失败: {}", e))?
}

fn list_agent_cli_sessions_impl(
    agent_id: String,
    project_path: Option<String>,
) -> Result<AgentCliSessionsResult, String> {
    let cli_path = get_agent_cli_path(&agent_id)?;
    let project_listing = list_agent_cli_session_projects_impl(agent_id.clone())?;
    let prefer_fast_scan = project_path.is_none();
    let session_result = scan_cli_sessions_internal(
        ScanCliSessionsInput {
            cli_path: Some(cli_path),
            project_path,
        },
        prefer_fast_scan,
    )?;

    Ok(AgentCliSessionsResult {
        agent_id,
        cli_name: session_result.cli_name,
        session_root: session_result.config_dir,
        sessions: session_result.sessions,
        project_paths: project_listing.project_paths,
    })
}

/// CLI 会话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliSessionMessage {
    /// 行号
    pub line_no: i32,
    /// 消息类型
    pub message_type: String,
    /// 角色
    pub role: Option<String>,
    /// 时间戳
    pub timestamp: Option<String>,
    /// 消息内容
    pub content: Option<String>,
    /// 原始 JSON
    pub raw_json: String,
}

/// CLI 会话详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliSessionDetail {
    /// 会话ID
    pub session_id: String,
    /// 会话文件路径
    pub session_path: String,
    /// 项目路径
    pub project_path: Option<String>,
    /// 首条消息
    pub first_message: Option<String>,
    /// 消息数量
    pub message_count: i32,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 消息列表
    pub messages: Vec<CliSessionMessage>,
}

/// 读取 CLI 会话详情 (Tauri 命令)
#[tauri::command]
pub async fn read_cli_session_detail(
    agent_id: String,
    session_path: String,
) -> Result<CliSessionDetail, String> {
    tauri::async_runtime::spawn_blocking(move || {
        read_cli_session_detail_impl(agent_id, session_path)
    })
    .await
    .map_err(|e| format!("读取会话详情失败: {}", e))?
}

fn read_cli_session_detail_impl(
    _agent_id: String,
    session_path: String,
) -> Result<CliSessionDetail, String> {
    let path = PathBuf::from(&session_path);

    if !path.exists() {
        return Err(format!("会话文件不存在: {}", session_path));
    }

    let file_name = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let content = fs::read_to_string(&path).map_err(|e| format!("无法读取会话文件: {}", e))?;

    let mut messages = Vec::new();
    let mut first_message: Option<String> = None;
    let mut message_count = 0;
    let mut project_path: Option<String> = None;
    let mut earliest_timestamp: Option<String> = None;
    let mut latest_timestamp: Option<String> = None;

    for (line_no, line) in content.lines().enumerate() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            let msg_type = extract_jsonl_message_type(&json);

            // 统计消息数量
            if msg_type == "user" || msg_type == "assistant" {
                message_count += 1;
            }

            // 获取第一条用户消息
            if first_message.is_none() && msg_type == "user" {
                if let Some(text) = extract_jsonl_message_content(&json) {
                    let preview: String = text.chars().take(100).collect();
                    first_message = clean_display_text(&preview);
                }
            }

            // 获取项目路径
            if project_path.is_none() {
                project_path = extract_jsonl_project_path(&json);
            }

            // 获取时间戳
            let timestamp = json
                .get("timestamp")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            if let Some(ref ts) = timestamp {
                if earliest_timestamp.is_none() || ts < earliest_timestamp.as_ref().unwrap() {
                    earliest_timestamp = Some(ts.clone());
                }
                if latest_timestamp.is_none() || ts > latest_timestamp.as_ref().unwrap() {
                    latest_timestamp = Some(ts.clone());
                }
            }

            // 获取角色
            let role = extract_jsonl_role(&json);

            let content = extract_jsonl_message_content(&json);

            messages.push(CliSessionMessage {
                line_no: line_no as i32 + 1,
                message_type: msg_type,
                role,
                timestamp,
                content,
                raw_json: line.to_string(),
            });
        }
    }

    // 获取文件元数据作为备选时间
    let metadata = fs::metadata(&path).map_err(|e| format!("无法获取文件元数据: {}", e))?;
    let modified = metadata
        .modified()
        .map_err(|e| format!("无法获取修改时间: {}", e))?;
    let modified_str = chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339();

    Ok(CliSessionDetail {
        session_id: file_name,
        session_path,
        project_path,
        first_message,
        message_count,
        created_at: earliest_timestamp.unwrap_or_else(|| modified_str.clone()),
        updated_at: latest_timestamp.unwrap_or(modified_str),
        messages,
    })
}

/// 删除 CLI 会话 (Tauri 命令)
#[tauri::command]
pub fn delete_cli_session(
    _agent_id: String,
    session_path: String,
    cleanup_empty_dirs: bool,
) -> Result<(), String> {
    let path = PathBuf::from(&session_path);
    delete_cli_session_path(&path, cleanup_empty_dirs)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCliSessionsResult {
    pub deleted_count: usize,
    pub failed_paths: Vec<String>,
}

#[tauri::command]
pub async fn delete_cli_sessions(
    _agent_id: String,
    session_paths: Vec<String>,
    cleanup_empty_dirs: bool,
) -> Result<DeleteCliSessionsResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if session_paths.is_empty() {
            return Err("未提供要删除的会话".to_string());
        }

        let mut deleted_count = 0;
        let mut failed_paths = Vec::new();

        for session_path in session_paths {
            let path = PathBuf::from(&session_path);
            match delete_cli_session_path(&path, cleanup_empty_dirs) {
                Ok(()) => deleted_count += 1,
                Err(error) => {
                    failed_paths.push(format!("{}: {}", session_path, error));
                }
            }
        }

        Ok(DeleteCliSessionsResult {
            deleted_count,
            failed_paths,
        })
    })
    .await
    .map_err(|e| format!("批量删除会话失败: {}", e))?
}

/// 扫描智能体会话历史 (Tauri 命令)
#[tauri::command]
pub fn scan_cli_sessions(input: ScanCliSessionsInput) -> Result<ScanCliSessionsResult, String> {
    scan_cli_sessions_internal(input, false)
}

fn scan_cli_sessions_internal(
    input: ScanCliSessionsInput,
    prefer_fast_scan: bool,
) -> Result<ScanCliSessionsResult, String> {
    // 获取 CLI 配置目录
    let (config_dir, _, cli_name) = match get_cli_config_dir(input.cli_path.as_deref()) {
        Ok(result) => result,
        Err(e) => {
            return Ok(ScanCliSessionsResult {
                cli_name: String::new(),
                config_dir: String::new(),
                sessions: Vec::new(),
                scan_success: false,
                error_message: Some(format!("无法确定配置目录: {}", e)),
            });
        }
    };

    let config_dir_str = config_dir.to_string_lossy().to_string();

    // 检查配置目录是否存在
    if !config_dir.exists() {
        return Ok(ScanCliSessionsResult {
            cli_name: cli_name.clone(),
            config_dir: config_dir_str,
            sessions: Vec::new(),
            scan_success: false,
            error_message: Some(format!("{} 配置目录不存在", cli_name)),
        });
    }

    let mut sessions = Vec::new();

    // 根据CLI类型扫描会话
    match cli_name.as_str() {
        "claude" | "claude-code" => {
            // Claude CLI: 会话存储在 ~/.claude/projects/<project-path>/ 目录下
            let projects_dir = config_dir.join("projects");
            if projects_dir.exists() {
                if let Some(filter_project) = &input.project_path {
                    // 如果指定了项目路径，只扫描该项目的会话
                    // Claude CLI 将路径中的分隔符替换为 "-"，Windows 路径需要同时处理 \ 和 /
                    let project_dir_name = filter_project
                        .replace('\\', "-")
                        .replace('/', "-")
                        .replace(':', "-"); // Windows 盘符如 C:
                    let project_session_dir = projects_dir.join(&project_dir_name);
                    if project_session_dir.exists() {
                        if let Ok(entries) = fs::read_dir(&project_session_dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                                    if let Some(session) = extract_session_info(&path, false) {
                                        sessions.push(session);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // 扫描所有项目的会话
                    if let Ok(project_entries) = fs::read_dir(&projects_dir) {
                        for project_entry in project_entries.flatten() {
                            let project_path = project_entry.path();
                            if project_path.is_dir() {
                                if let Ok(session_entries) = fs::read_dir(&project_path) {
                                    for session_entry in session_entries.flatten() {
                                        let session_path = session_entry.path();
                                        if session_path
                                            .extension()
                                            .map(|e| e == "jsonl")
                                            .unwrap_or(false)
                                        {
                                            if let Some(session) =
                                                extract_session_info(&session_path, prefer_fast_scan)
                                            {
                                                sessions.push(session);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        "codex" => {
            // Codex CLI: 会话存储在 ~/.codex/sessions/YYYY/MM/DD/*.jsonl
            let sessions_dir = config_dir.join("sessions");
            if sessions_dir.exists() {
                let mut session_files = Vec::new();
                collect_jsonl_files(&sessions_dir, &mut session_files);

                for session_file in session_files {
                    if let Some(session) = extract_session_info(&session_file, prefer_fast_scan) {
                        let should_keep = input
                            .project_path
                            .as_ref()
                            .map(|project| session.project_path.as_ref() == Some(project))
                            .unwrap_or(true);

                        if should_keep {
                            sessions.push(session);
                        }
                    }
                }
            }
        }
        "qwen" | "qwen-code" => {
            // Qwen Code: 会话存储格式可能不同，需要根据实际情况调整
            // 暂时返回空列表
        }
        _ => {
            // 未知CLI类型
        }
    }

    // 按更新时间排序（最新的在前）
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(ScanCliSessionsResult {
        cli_name,
        config_dir: config_dir_str,
        sessions,
        scan_success: true,
        error_message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_codex_user_message_from_payload_content() {
        let json = json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "修复会话详情渲染"
                    }
                ]
            }
        });

        assert_eq!(extract_jsonl_message_type(&json), "user");
        assert_eq!(
            extract_jsonl_message_content(&json).as_deref(),
            Some("修复会话详情渲染")
        );
        assert_eq!(extract_jsonl_role(&json).as_deref(), Some("user"));
    }

    #[test]
    fn strips_separator_only_lines_from_display_text() {
        let text = "---\nname: demo\n-----\n|------|------|\nvalue";
        assert_eq!(
            clean_display_text(text).as_deref(),
            Some("name: demo\nvalue")
        );
    }

    #[test]
    fn extracts_codex_assistant_output_text() {
        let json = json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [
                    {
                        "type": "output_text",
                        "text": "会话详情恢复正常"
                    }
                ]
            }
        });

        assert_eq!(extract_jsonl_message_type(&json), "assistant");
        assert_eq!(
            extract_jsonl_message_content(&json).as_deref(),
            Some("会话详情恢复正常")
        );
    }

    #[test]
    fn extracts_codex_tool_call_and_output() {
        let call = json!({
            "type": "response_item",
            "payload": {
                "type": "function_call",
                "name": "exec_command",
                "arguments": "{\"cmd\":\"pwd\"}"
            }
        });
        let output = json!({
            "type": "response_item",
            "payload": {
                "type": "function_call_output",
                "call_id": "call_123",
                "output": "Chunk ID: abc\nProcess exited with code 0"
            }
        });

        assert_eq!(extract_jsonl_message_type(&call), "tool_use");
        assert!(extract_jsonl_message_content(&call)
            .unwrap_or_default()
            .contains("exec_command"));

        assert_eq!(extract_jsonl_message_type(&output), "tool_result");
        assert!(extract_jsonl_message_content(&output)
            .unwrap_or_default()
            .contains("call_123"));
    }

    #[test]
    fn extracts_codex_project_path_from_payload() {
        let json = json!({
            "type": "session_meta",
            "payload": {
                "cwd": "/tmp/demo-project",
                "cli_version": "0.112.0"
            }
        });

        assert_eq!(
            extract_jsonl_project_path(&json).as_deref(),
            Some("/tmp/demo-project")
        );
        assert_eq!(extract_jsonl_message_type(&json), "system");
    }
}
