use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::cli_support::resolve_cli_name;
use crate::commands::mcp_shared::parse_args_string;
use crate::commands::scan_session_shared::{
    clean_display_text, collect_jsonl_files, delete_cli_session_path,
    extract_jsonl_message_content, extract_jsonl_message_type, extract_jsonl_project_path,
    extract_jsonl_role, format_json_value,
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
    pub disabled: bool,
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

fn build_cli_config_scan_result(
    config_dir: String,
    mcp_servers: Vec<ScannedMcpServer>,
    skills: Vec<ScannedSkill>,
    plugins: Vec<ScannedPlugin>,
) -> ClaudeConfigScanResult {
    ClaudeConfigScanResult {
        claude_dir: config_dir,
        mcp_servers,
        skills,
        plugins,
        scan_success: true,
        error_message: None,
    }
}

fn build_cli_config_scan_error(
    config_dir: String,
    error_message: String,
) -> ClaudeConfigScanResult {
    ClaudeConfigScanResult {
        claude_dir: config_dir,
        mcp_servers: Vec::new(),
        skills: Vec::new(),
        plugins: Vec::new(),
        scan_success: false,
        error_message: Some(error_message),
    }
}

fn get_cli_config_dir(
    cli_path: Option<&str>,
    cli_type_hint: Option<&str>,
) -> Result<(PathBuf, PathBuf, String), String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    let cli_name = resolve_cli_name(cli_path, cli_type_hint, "claude");

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
        "opencode" => {
            // OpenCode CLI: 配置在 ~/.config/opencode/opencode.json
            let config_dir = home_dir.join(".config").join("opencode");
            let config_file = config_dir.join("opencode.json");
            Ok((config_dir, config_file, "opencode".to_string()))
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

fn parse_disabled_flag(config_obj: &serde_json::Map<String, serde_json::Value>) -> bool {
    if let Some(enabled) = config_obj.get("enabled").and_then(|value| value.as_bool()) {
        return !enabled;
    }

    config_obj
        .get("disabled")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
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
    let headers = crate::commands::scan_shared::parse_string_map(
        config_obj
            .get("http_headers")
            .or_else(|| config_obj.get("headers")),
    );

    let disabled = parse_disabled_flag(config_obj);

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
        disabled,
    })
}

fn normalize_project_config_key(raw: &str) -> String {
    let normalized = raw.trim().replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/').to_string();
    let bytes = trimmed.as_bytes();

    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return trimmed.to_lowercase();
    }

    trimmed
}

fn scan_claude_project_mcp_from_main_config(
    config_file: &Path,
    project_root: &Path,
    servers: &mut Vec<ScannedMcpServer>,
) {
    let Some(json) = crate::commands::scan_shared::read_json_file(config_file) else {
        return;
    };
    let global_mcp_servers = json.get("mcpServers").and_then(|value| value.as_object());
    let Some(projects) = json.get("projects").and_then(|value| value.as_object()) else {
        return;
    };

    let mut lookup_keys = vec![normalize_project_config_key(
        project_root.to_string_lossy().as_ref(),
    )];

    if let Ok(canonical_path) = project_root.canonicalize() {
        lookup_keys.push(normalize_project_config_key(
            canonical_path.to_string_lossy().as_ref(),
        ));
    }

    lookup_keys.sort();
    lookup_keys.dedup();

    let project_entry = projects.iter().find_map(|(project_key, value)| {
        let normalized_key = normalize_project_config_key(project_key);
        if lookup_keys.iter().any(|candidate| candidate == &normalized_key) {
            value.as_object()
        } else {
            None
        }
    });

    let Some(project_obj) = project_entry else {
        return;
    };
    let Some(mcp_servers) = project_obj
        .get("mcpServers")
        .or_else(|| project_obj.get("mcp_servers"))
        .and_then(|value| value.as_object())
    else {
        return;
    };

    for (name, config) in mcp_servers {
        if servers
            .iter()
            .any(|server| server.name == *name && server.scope == McpConfigScope::Project)
        {
            continue;
        }

        let Some(config_obj) = config.as_object() else {
            continue;
        };

        if let Some(server) = parse_mcp_server_config(name, config_obj, McpConfigScope::Project) {
            servers.push(server);
            continue;
        }

        let Some(global_config_obj) = global_mcp_servers
            .and_then(|items| items.get(name))
            .and_then(|value| value.as_object())
        else {
            continue;
        };

        let mut merged_config = global_config_obj.clone();
        for (key, value) in config_obj {
            merged_config.insert(key.clone(), value.clone());
        }

        if let Some(server) = parse_mcp_server_config(
            name,
            &merged_config,
            McpConfigScope::Project,
        ) {
            servers.push(server);
        }
    }
}

fn scan_opencode_mcp_source_file(
    path: &Path,
    scope: McpConfigScope,
    servers: &mut Vec<ScannedMcpServer>,
) {
    let Some(json) = crate::commands::scan_shared::read_json_file(path) else {
        return;
    };
    let Some(mcp_obj) = json.get("mcp").and_then(|value| value.as_object()) else {
        return;
    };

    for (name, config) in mcp_obj {
        if servers
            .iter()
            .any(|server| server.name == *name && server.scope == scope)
        {
            continue;
        }

        let Some(config_obj) = config.as_object() else {
            continue;
        };

        let server_type = config_obj
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or("local");
        let enabled = config_obj
            .get("enabled")
            .and_then(|value| value.as_bool())
            .unwrap_or(true);

        if server_type == "remote" {
            let url = config_obj
                .get("url")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            let transport = infer_transport_from_fields(url.as_ref(), None).unwrap_or(McpTransportType::Http);

            servers.push(ScannedMcpServer {
                name: name.clone(),
                transport,
                scope: scope.clone(),
                command: None,
                args: None,
                env: None,
                url,
                headers: crate::commands::scan_shared::parse_string_map(config_obj.get("headers")),
                disabled: !enabled,
            });
            continue;
        }

        let command_array = config_obj.get("command").and_then(|value| value.as_array());
        let command = command_array
            .and_then(|values| values.first())
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());
        let args = command_array.map(|values| {
            values
                .iter()
                .skip(1)
                .filter_map(|value| value.as_str().map(|entry| entry.to_string()))
                .collect::<Vec<String>>()
        });

        servers.push(ScannedMcpServer {
            name: name.clone(),
            transport: McpTransportType::Stdio,
            scope: scope.clone(),
            command,
            args,
            env: crate::commands::scan_shared::parse_string_map(config_obj.get("environment")),
            url: None,
            headers: None,
            disabled: !enabled,
        });
    }
}

/// 扫描 MCP 配置
fn scan_mcp_config(
    cli_name: &str,
    config_dir: &Path,
    config_file: &Path,
    project_path: Option<&Path>,
) -> Result<Vec<ScannedMcpServer>> {
    let mut servers = Vec::new();

    // 1. 首先尝试从 ~/.claude.json (或对应 CLI 的配置文件) 读取用户级 MCP 配置 (user scope)
    if cli_name == "opencode" {
        scan_opencode_mcp_source_file(config_file, McpConfigScope::User, &mut servers);
    } else {
        crate::commands::scan_shared::scan_mcp_source_file(
            config_file,
            McpConfigScope::User,
            &mut servers,
        );
    }

    // 2. 尝试从 config_dir/settings.json 读取 MCP 配置 (user scope)
    let settings_path = config_dir.join("settings.json");
    crate::commands::scan_shared::scan_mcp_source_file(
        &settings_path,
        McpConfigScope::User,
        &mut servers,
    );

    if let Some(project_root) = project_path {
        match cli_name {
            "claude" => {
                let mcp_json_path = project_root.join(".mcp.json");
                crate::commands::scan_shared::scan_mcp_source_file(
                    &mcp_json_path,
                    McpConfigScope::Local,
                    &mut servers,
                );
                scan_claude_project_mcp_from_main_config(config_file, project_root, &mut servers);
            }
            "opencode" => {
                let project_config_path = project_root.join("opencode.json");
                scan_opencode_mcp_source_file(
                    &project_config_path,
                    McpConfigScope::Project,
                    &mut servers,
                );
            }
            _ => {}
        }
    }

    Ok(servers)
}

fn is_jsonl_file(path: &Path) -> bool {
    path.extension().map(|ext| ext == "jsonl").unwrap_or(false)
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
fn check_skill_subdirectories(skill_path: &Path) -> SkillSubdirectories {
    SkillSubdirectories {
        has_scripts: skill_path.join("scripts").exists(),
        has_references: skill_path.join("references").exists(),
        has_assets: skill_path.join("assets").exists(),
    }
}

fn resolve_scan_entry_path(path: &Path) -> PathBuf {
    if !path.is_symlink() {
        return path.to_path_buf();
    }

    match fs::read_link(path) {
        Ok(target) if target.is_relative() => path
            .parent()
            .map(|parent| parent.join(&target))
            .unwrap_or_else(|| path.to_path_buf()),
        Ok(target) => target,
        Err(_) => path.to_path_buf(),
    }
}

fn find_skill_markdown_path(skill_dir: &Path) -> Option<PathBuf> {
    ["SKILL.md", "skill.md"]
        .iter()
        .map(|name| skill_dir.join(name))
        .find(|path| path.exists())
}

fn build_directory_skill(path: &Path, actual_path: &Path) -> ScannedSkill {
    let dir_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let (frontmatter_name, description) = find_skill_markdown_path(actual_path)
        .and_then(|md_path| fs::read_to_string(md_path).ok())
        .map(|content| parse_yaml_frontmatter(&content))
        .unwrap_or((None, None));

    ScannedSkill {
        name: frontmatter_name.clone().unwrap_or_else(|| dir_name.clone()),
        path: path.to_string_lossy().to_string(),
        description,
        frontmatter_name,
        subdirectories: check_skill_subdirectories(actual_path),
    }
}

fn build_markdown_skill(path: &PathBuf) -> ScannedSkill {
    let name = path
        .file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let description = fs::read_to_string(path)
        .ok()
        .and_then(|content| content.lines().next().map(|line| line.trim().to_string()));

    ScannedSkill {
        name,
        path: path.to_string_lossy().to_string(),
        description,
        frontmatter_name: None,
        subdirectories: SkillSubdirectories {
            has_scripts: false,
            has_references: false,
            has_assets: false,
        },
    }
}

/// 扫描 Skills 目录
fn scan_skills_directory_at(skills_dir: &Path) -> Result<Vec<ScannedSkill>> {
    let mut skills = Vec::new();

    if !skills_dir.exists() {
        return Ok(skills);
    }

    let entries = fs::read_dir(&skills_dir)?;
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };

        let path = entry.path();
        let actual_path = resolve_scan_entry_path(&path);

        if actual_path.is_dir() {
            skills.push(build_directory_skill(&path, &actual_path));
            continue;
        }

        if path.extension().is_some_and(|extension| extension == "md") {
            skills.push(build_markdown_skill(&path));
        }
    }

    Ok(skills)
}

fn scan_skills_directory(
    config_dir: &Path,
    cli_name: &str,
    project_path: Option<&Path>,
) -> Result<Vec<ScannedSkill>> {
    let mut skills = scan_skills_directory_at(&config_dir.join("skills"))?;

    if let Some(project_root) = project_path {
        let project_skills_dir = match cli_name {
            "claude" => Some(project_root.join(".claude").join("skills")),
            "opencode" => Some(project_root.join(".opencode").join("skills")),
            _ => None,
        };

        if let Some(skills_dir) = project_skills_dir {
            for skill in scan_skills_directory_at(&skills_dir)? {
                if skills.iter().any(|existing| existing.path == skill.path) {
                    continue;
                }
                skills.push(skill);
            }
        }
    }

    Ok(skills)
}

/// 解析 plugin.json 文件
fn parse_plugin_json(plugin_json_path: &Path) -> (Option<String>, Option<String>, Option<String>) {
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

fn plugin_manifest_dir_candidates(cli_name: &str) -> &'static [&'static str] {
    match cli_name {
        "codex" => &[".codex-plugin", ".claude-plugin"],
        _ => &[".claude-plugin", ".codex-plugin"],
    }
}

fn resolve_plugin_manifest_path(plugin_path: &Path, cli_name: &str) -> Option<PathBuf> {
    plugin_manifest_dir_candidates(cli_name)
        .iter()
        .map(|dir_name| plugin_path.join(dir_name).join("plugin.json"))
        .find(|candidate| candidate.exists())
}

/// 检查 Plugin 目录的子目录结构
fn check_plugin_subdirectories(plugin_path: &Path) -> PluginSubdirectories {
    PluginSubdirectories {
        has_agents: plugin_path.join("agents").exists(),
        has_commands: plugin_path.join("commands").exists(),
        has_skills: plugin_path.join("skills").exists(),
        has_hooks: plugin_path.join("hooks").exists(),
        has_scripts: plugin_path.join("scripts").exists(),
    }
}

fn build_scanned_plugin(
    plugin_path: &Path,
    name: String,
    enabled: bool,
    cli_name: &str,
) -> ScannedPlugin {
    let (version, description, author) = resolve_plugin_manifest_path(plugin_path, cli_name)
        .map(|plugin_json_path| parse_plugin_json(&plugin_json_path))
        .unwrap_or((None, None, None));

    ScannedPlugin {
        name,
        path: plugin_path.to_string_lossy().to_string(),
        enabled,
        version,
        description,
        author,
        subdirectories: check_plugin_subdirectories(plugin_path),
    }
}

fn scan_installed_plugins_file(plugins_dir: &Path, cli_name: &str) -> Vec<ScannedPlugin> {
    let installed_plugins_path = plugins_dir.join("installed_plugins.json");
    let Ok(content) = fs::read_to_string(installed_plugins_path) else {
        return Vec::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        return Vec::new();
    };
    let Some(plugins_obj) = json.get("plugins").and_then(|value| value.as_object()) else {
        return Vec::new();
    };
    let mut plugins = Vec::new();

    for (plugin_key, plugin_entries) in plugins_obj {
        let Some(first_entry) = plugin_entries
            .as_array()
            .and_then(|entries| entries.first())
        else {
            continue;
        };
        let Some(install_path_str) = first_entry
            .get("installPath")
            .and_then(|value| value.as_str())
        else {
            continue;
        };

        let install_path = PathBuf::from(install_path_str);
        if !install_path.exists() {
            continue;
        }

        let display_name = plugin_key
            .split('@')
            .next()
            .unwrap_or(plugin_key)
            .to_string();
        let enabled = first_entry
            .get("scope")
            .and_then(|value| value.as_str())
            .map(|scope| scope == "user")
            .unwrap_or(true);

        plugins.push(build_scanned_plugin(
            &install_path,
            display_name,
            enabled,
            cli_name,
        ));
    }

    plugins
}

fn scan_plugin_directories(plugins_dir: &Path, cli_name: &str) -> Result<Vec<ScannedPlugin>> {
    let mut plugins = Vec::new();

    for entry in fs::read_dir(plugins_dir)? {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let has_manifest = resolve_plugin_manifest_path(&path, cli_name).is_some();
        let subdirectories = check_plugin_subdirectories(&path);
        if !has_manifest
            && !subdirectories.has_agents
            && !subdirectories.has_commands
            && !subdirectories.has_skills
            && !subdirectories.has_hooks
            && !subdirectories.has_scripts
        {
            continue;
        }

        let name = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        let enabled = !path.join(".disabled").exists();
        plugins.push(build_scanned_plugin(&path, name, enabled, cli_name));
    }

    Ok(plugins)
}

/// 扫描 Plugins 目录
fn scan_plugins_directory_at(plugins_dir: &Path, cli_name: &str) -> Result<Vec<ScannedPlugin>> {
    if !plugins_dir.exists() {
        return Ok(Vec::new());
    }

    let plugins = scan_installed_plugins_file(&plugins_dir, cli_name);
    if plugins.is_empty() {
        return scan_plugin_directories(&plugins_dir, cli_name);
    }

    Ok(plugins)
}

fn scan_plugins_directory(
    config_dir: &Path,
    cli_name: &str,
    project_path: Option<&Path>,
) -> Result<Vec<ScannedPlugin>> {
    let mut plugins = scan_plugins_directory_at(&config_dir.join("plugins"), cli_name)?;

    if let Some(project_root) = project_path {
        let project_plugins_dir = match cli_name {
            "opencode" => Some(project_root.join(".opencode").join("plugins")),
            _ => None,
        };

        if let Some(plugins_dir) = project_plugins_dir {
            for plugin in scan_plugins_directory_at(&plugins_dir, cli_name)? {
                if plugins.iter().any(|existing| existing.path == plugin.path) {
                    continue;
                }
                plugins.push(plugin);
            }
        }
    }

    Ok(plugins)
}

/// 扫描 CLI 配置 (Tauri 命令)
#[tauri::command]
pub fn scan_cli_config(
    cli_path: Option<String>,
    cli_type: Option<String>,
    project_path: Option<String>,
) -> Result<ClaudeConfigScanResult, String> {
    let (config_dir, config_file, cli_name) =
        match get_cli_config_dir(cli_path.as_deref(), cli_type.as_deref()) {
            Ok(result) => result,
            Err(e) => {
                return Ok(build_cli_config_scan_error(
                    String::new(),
                    format!("无法确定配置目录: {}", e),
                ));
            }
        };

    let config_dir_str = config_dir.to_string_lossy().to_string();

    if !config_file.exists() && !config_dir.exists() {
        return Ok(build_cli_config_scan_error(
            config_dir_str,
            format!("{} 配置不存在", cli_name),
        ));
    }

    let project_root = project_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .filter(|path| path.exists() && path.is_dir());

    let mcp_servers =
        scan_mcp_config(&cli_name, &config_dir, &config_file, project_root.as_deref())
            .unwrap_or_default();
    let skills =
        scan_skills_directory(&config_dir, &cli_name, project_root.as_deref()).unwrap_or_default();
    let plugins = scan_plugins_directory(&config_dir, &cli_name, project_root.as_deref())
        .unwrap_or_default();

    Ok(build_cli_config_scan_result(
        config_dir_str,
        mcp_servers,
        skills,
        plugins,
    ))
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

                        let parts = parse_args_string(Some(rest));
                        if !parts.is_empty() {
                            let command = parts[0].clone();
                            let args: Vec<String> = parts[1..].to_vec();

                            servers.push(ScannedMcpServer {
                                name,
                                transport: McpTransportType::Stdio,
                                scope: McpConfigScope::User,
                                command: Some(command),
                                args: if args.is_empty() { None } else { Some(args) },
                                env: None,
                                url: None,
                                headers: None,
                                disabled: false,
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

fn build_session_scan_result(
    cli_name: String,
    config_dir: String,
    mut sessions: Vec<ScannedCliSession>,
) -> ScanCliSessionsResult {
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    ScanCliSessionsResult {
        cli_name,
        config_dir,
        sessions,
        scan_success: true,
        error_message: None,
    }
}

fn build_session_scan_error(
    cli_name: String,
    config_dir: String,
    error_message: String,
) -> ScanCliSessionsResult {
    ScanCliSessionsResult {
        cli_name,
        config_dir,
        sessions: Vec::new(),
        scan_success: false,
        error_message: Some(error_message),
    }
}

const FAST_SESSION_SCAN_LINE_LIMIT: usize = 120;
const FULL_SESSION_SCAN_SIZE_THRESHOLD: u64 = 256 * 1024;
const UNKNOWN_MESSAGE_COUNT: i32 = -1;

fn extract_session_project_path(session_path: &Path) -> Option<String> {
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

fn normalized_project_dir_name(project_path: &str) -> String {
    project_path
        .replace('\\', "-")
        .replace('/', "-")
        .replace(':', "-")
}

fn should_keep_session(session: &ScannedCliSession, project_filter: Option<&str>) -> bool {
    project_filter
        .map(|project| session.project_path.as_deref() == Some(project))
        .unwrap_or(true)
}

fn collect_claude_session_files(projects_dir: &Path, project_filter: Option<&str>) -> Vec<PathBuf> {
    let mut session_files = Vec::new();

    if let Some(filter_project) = project_filter {
        let project_session_dir = projects_dir.join(normalized_project_dir_name(filter_project));
        if let Ok(entries) = fs::read_dir(project_session_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if is_jsonl_file(&path) {
                    session_files.push(path);
                }
            }
        }
        return session_files;
    }

    if let Ok(project_entries) = fs::read_dir(projects_dir) {
        for project_entry in project_entries.flatten() {
            let project_path = project_entry.path();
            if !project_path.is_dir() {
                continue;
            }

            if let Ok(session_entries) = fs::read_dir(project_path) {
                for session_entry in session_entries.flatten() {
                    let session_path = session_entry.path();
                    if is_jsonl_file(&session_path) {
                        session_files.push(session_path);
                    }
                }
            }
        }
    }

    session_files
}

fn collect_cli_sessions_from_files(
    session_files: Vec<PathBuf>,
    prefer_fast_scan: bool,
    project_filter: Option<&str>,
) -> Vec<ScannedCliSession> {
    session_files
        .into_iter()
        .filter_map(|session_path| extract_session_info(&session_path, prefer_fast_scan))
        .filter(|session| should_keep_session(session, project_filter))
        .collect()
}

/// 从会话jsonl文件中提取会话信息
fn extract_session_info(session_path: &Path, force_fast_scan: bool) -> Option<ScannedCliSession> {
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

fn list_cli_session_project_paths(cli_name: &str, config_dir: &Path) -> Vec<String> {
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
                            if is_jsonl_file(&session_path) {
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
        "opencode" => {
            if let Ok(paths) = list_opencode_session_project_paths(config_dir) {
                project_paths.extend(paths);
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
    tauri::async_runtime::spawn_blocking(move || list_agent_cli_session_projects_impl(agent_id))
        .await
        .map_err(|e| format!("加载会话项目列表失败: {}", e))?
}

fn list_agent_cli_session_projects_impl(
    agent_id: String,
) -> Result<AgentCliSessionProjectsResult, String> {
    let cli_path = get_agent_cli_path(&agent_id)?;
    let (config_dir, _, cli_name) = get_cli_config_dir(Some(cli_path.as_str()), None)
        .map_err(|e| format!("无法确定配置目录: {}", e))?;

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
    if is_opencode_session_path(&session_path) {
        return read_opencode_session_detail(&session_path);
    }

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
    agent_id: String,
    session_path: String,
    cleanup_empty_dirs: bool,
) -> Result<(), String> {
    if is_opencode_session_path(&session_path) {
        return delete_opencode_session(&agent_id, &session_path);
    }

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
    agent_id: String,
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
            if is_opencode_session_path(&session_path) {
                match delete_opencode_session(&agent_id, &session_path) {
                    Ok(()) => deleted_count += 1,
                    Err(error) => {
                        failed_paths.push(format!("{}: {}", session_path, error));
                    }
                }
                continue;
            }

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

const OPENCODE_SESSION_PATH_PREFIX: &str = "opencode://session/";

fn opencode_data_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".local").join("share").join("opencode"))
}

fn opencode_db_path() -> Option<PathBuf> {
    opencode_data_dir().map(|d| d.join("opencode.db"))
}

fn is_opencode_session_path(session_path: &str) -> bool {
    session_path.starts_with(OPENCODE_SESSION_PATH_PREFIX)
}

fn extract_opencode_session_id(session_path: &str) -> Option<&str> {
    session_path
        .strip_prefix(OPENCODE_SESSION_PATH_PREFIX)
        .filter(|s| !s.is_empty())
}

fn list_opencode_session_project_paths(config_dir: &Path) -> Result<Vec<String>, String> {
    let db_path = if config_dir.join("opencode.db").exists() {
        config_dir.join("opencode.db")
    } else {
        opencode_db_path().ok_or_else(|| "无法确定 OpenCode 数据目录".to_string())?
    };

    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let conn = Connection::open(&db_path)
        .map_err(|e| format!("无法打开 OpenCode 数据库: {}", e))?;

    let mut stmt = conn
        .prepare("SELECT DISTINCT directory FROM session ORDER BY directory")
        .map_err(|e| format!("查询 OpenCode 会话项目失败: {}", e))?;

    let paths = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("读取 OpenCode 项目列表失败: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(paths)
}

fn scan_opencode_sessions(
    config_dir: &Path,
    project_filter: Option<&str>,
) -> Vec<ScannedCliSession> {
    let db_path = if config_dir.join("opencode.db").exists() {
        config_dir.join("opencode.db")
    } else {
        match opencode_db_path() {
            Some(p) if p.exists() => p,
            _ => return Vec::new(),
        }
    };

    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("无法打开 OpenCode 数据库: {}", e);
            return Vec::new();
        }
    };

    let sql = match project_filter {
        Some(_) => "SELECT id, directory, title, time_created, time_updated FROM session WHERE directory = ?1 ORDER BY time_updated DESC",
        None => "SELECT id, directory, title, time_created, time_updated FROM session ORDER BY time_updated DESC",
    };

    let mut sessions = Vec::new();

    let rows_result: Result<Vec<(String, String, Option<String>, i64, i64)>, rusqlite::Error> =
        if let Some(filter) = project_filter {
            let mut stmt = match conn.prepare(sql) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("OpenCode 会话查询失败: {}", e);
                    return Vec::new();
                }
            };
            stmt.query_map([filter], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
        } else {
            let mut stmt = match conn.prepare(sql) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("OpenCode 会话查询失败: {}", e);
                    return Vec::new();
                }
            };
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
        };

    let rows = match rows_result {
        Ok(r) => r,
        Err(e) => {
            eprintln!("OpenCode 会话查询失败: {}", e);
            return Vec::new();
        }
    };

    for (session_id, directory, title, time_created, time_updated) in rows {
        let created_at = format_unix_timestamp_ms(time_created);
        let updated_at = format_unix_timestamp_ms(time_updated);

        let message_count = match conn.query_row(
            "SELECT count(*) FROM message WHERE session_id = ?1 AND json_extract(data, '$.role') IN ('user', 'assistant')",
            [&session_id],
            |r| r.get::<_, i32>(0),
        ) {
            Ok(c) => c,
            Err(_) => 0,
        };

        let first_message = title.unwrap_or_default();

        sessions.push(ScannedCliSession {
            session_id: session_id.clone(),
            session_path: format!("{}{}", OPENCODE_SESSION_PATH_PREFIX, session_id),
            project_path: Some(directory),
            first_message: if first_message.is_empty() { None } else { Some(first_message) },
            message_count,
            created_at,
            updated_at,
        });
    }

    sessions
}

fn format_unix_timestamp_ms(ts_ms: i64) -> String {
    let secs = ts_ms / 1000;
    chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_default()
        .to_rfc3339()
}

fn read_opencode_session_detail(session_path: &str) -> Result<CliSessionDetail, String> {
    let session_id = extract_opencode_session_id(session_path)
        .ok_or_else(|| "无效的 OpenCode 会话路径".to_string())?;

    let db_path = opencode_db_path()
        .filter(|p| p.exists())
        .ok_or_else(|| "无法找到 OpenCode 数据库".to_string())?;

    let conn = Connection::open(&db_path)
        .map_err(|e| format!("无法打开 OpenCode 数据库: {}", e))?;

    let (directory, title, time_created, time_updated) = conn
        .query_row(
            "SELECT directory, title, time_created, time_updated FROM session WHERE id = ?1",
            [session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .map_err(|e| format!("查询会话失败: {}", e))?;

    let created_at = format_unix_timestamp_ms(time_created);
    let updated_at = format_unix_timestamp_ms(time_updated);

    let first_message = title.filter(|t| !t.is_empty());

    let mut messages = Vec::new();
    let mut msg_count = 0;

    let mut stmt = conn
        .prepare(
            "SELECT id, data FROM message WHERE session_id = ?1 ORDER BY time_created",
        )
        .map_err(|e| format!("查询消息失败: {}", e))?;

    let rows = stmt
        .query_map([session_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ))
        })
        .map_err(|e| format!("读取消息失败: {}", e))?;

    for row in rows {
        let (msg_id, data_str) = match row {
            Ok(r) => r,
            Err(_) => continue,
        };

        let data: serde_json::Value = match serde_json::from_str(&data_str) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let role = data
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        if role == "user" || role == "assistant" {
            msg_count += 1;
        }

        let timestamp = data
            .get("time")
            .and_then(|t| t.get("created"))
            .and_then(|v| v.as_i64())
            .map(|ts| format_unix_timestamp_ms(ts));

        let extraction = extract_opencode_message_content(&conn, &msg_id, &data);
        let msg_type = match role.as_str() {
            "user" => "user".to_string(),
            "assistant" => extraction
                .preferred_message_type
                .clone()
                .unwrap_or_else(|| "assistant".to_string()),
            _ => extraction
                .preferred_message_type
                .clone()
                .unwrap_or_else(|| "system".to_string()),
        };

        messages.push(CliSessionMessage {
            line_no: messages.len() as i32 + 1,
            message_type: msg_type,
            role: Some(role),
            timestamp,
            content: extraction.content,
            raw_json: data_str,
        });
    }

    Ok(CliSessionDetail {
        session_id: session_id.to_string(),
        session_path: session_path.to_string(),
        project_path: Some(directory),
        first_message,
        message_count: msg_count,
        created_at,
        updated_at,
        messages,
    })
}

#[derive(Default)]
struct OpenCodeMessageExtraction {
    content: Option<String>,
    preferred_message_type: Option<String>,
}

fn extract_opencode_tool_part_content(part: &serde_json::Value) -> Option<String> {
    let tool_name = part
        .get("tool")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let status = part
        .get("state")
        .and_then(|state| state.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let input = part
        .get("state")
        .and_then(|state| state.get("input"))
        .and_then(format_json_value);
    let output = part
        .get("state")
        .and_then(|state| state.get("output"))
        .and_then(format_json_value);

    if let Some(output_text) = output.filter(|text| !text.trim().is_empty()) {
        return Some(format!("[Tool Result] {}\n{}", tool_name, output_text));
    }

    if let Some(input_text) = input.filter(|text| !text.trim().is_empty()) {
        return Some(format!("[Tool Use] {}\n{}", tool_name, input_text));
    }

    if status.is_empty() {
        Some(format!("[Tool Use] {}", tool_name))
    } else {
        Some(format!("[Tool Use] {} ({})", tool_name, status))
    }
}

fn extract_opencode_message_content(
    conn: &Connection,
    msg_id: &str,
    message_data: &serde_json::Value,
) -> OpenCodeMessageExtraction {
    let mut stmt = conn
        .prepare("SELECT data FROM part WHERE message_id = ?1 ORDER BY time_created")
        .ok();

    let mut parts = Vec::new();
    let mut has_assistant_text = false;
    let mut has_tool_activity = false;
    let mut has_step_text = false;

    if let Some(stmt) = stmt.as_mut() {
        if let Ok(rows) = stmt.query_map([msg_id], |row| row.get::<_, String>(0)) {
            for row in rows {
                let Ok(result) = row else {
                    continue;
                };

                let Ok(data) = serde_json::from_str::<serde_json::Value>(&result) else {
                    continue;
                };

                let part_type = data.get("type").and_then(|v| v.as_str()).unwrap_or_default();
                let text = match part_type {
                    "text" => {
                        has_assistant_text = true;
                        data.get("text").and_then(|v| v.as_str()).and_then(clean_display_text)
                    }
                    "reasoning" => {
                        has_assistant_text = true;
                        data.get("text")
                            .and_then(|v| v.as_str())
                            .and_then(clean_display_text)
                            .map(|value| format!("[Thinking]\n{}", value))
                    }
                    "step-start" => data
                        .get("text")
                        .and_then(|v| v.as_str())
                        .and_then(clean_display_text)
                        .map(|value| {
                            has_step_text = true;
                            format!("[Step]\n{}", value)
                        }),
                    "tool" => {
                        has_tool_activity = true;
                        extract_opencode_tool_part_content(&data)
                    }
                    _ => data
                        .get("text")
                        .and_then(|v| v.as_str())
                        .and_then(clean_display_text),
                };

                if let Some(value) = text {
                    parts.push(value);
                }
            }
        }
    }

    let content = if !parts.is_empty() {
        Some(parts.join("\n\n"))
    } else {
        message_data
        .get("error")
        .and_then(|error| error.get("data"))
        .and_then(|data| data.get("message"))
        .and_then(|v| v.as_str())
        .and_then(clean_display_text)
        .map(|value| format!("[Error]\n{}", value))
        .or_else(|| {
            message_data
                .get("summary")
                .and_then(format_json_value)
        })
    };

    let preferred_message_type = if has_assistant_text {
        Some("assistant".to_string())
    } else if has_tool_activity {
        Some("tool_use".to_string())
    } else if has_step_text {
        Some("system".to_string())
    } else if content.is_some() {
        Some("assistant".to_string())
    } else {
        None
    };

    OpenCodeMessageExtraction {
        content,
        preferred_message_type,
    }
}

fn delete_opencode_session(_agent_id: &str, session_path: &str) -> Result<(), String> {
    let session_id = extract_opencode_session_id(session_path)
        .ok_or_else(|| "无效的 OpenCode 会话路径".to_string())?;

    let db_path = opencode_db_path()
        .filter(|p| p.exists())
        .ok_or_else(|| "无法找到 OpenCode 数据库".to_string())?;

    let conn = Connection::open(&db_path)
        .map_err(|e| format!("无法打开 OpenCode 数据库: {}", e))?;

    conn.execute("DELETE FROM part WHERE session_id = ?1", [session_id])
        .map_err(|e| format!("删除会话 part 失败: {}", e))?;

    conn.execute("DELETE FROM message WHERE session_id = ?1", [session_id])
        .map_err(|e| format!("删除会话 message 失败: {}", e))?;

    conn.execute("DELETE FROM session WHERE id = ?1", [session_id])
        .map_err(|e| format!("删除会话记录失败: {}", e))?;

    Ok(())
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
    let (config_dir, _, cli_name) = match get_cli_config_dir(input.cli_path.as_deref(), None) {
        Ok(result) => result,
        Err(e) => {
            return Ok(build_session_scan_error(
                String::new(),
                String::new(),
                format!("无法确定配置目录: {}", e),
            ));
        }
    };

    let config_dir_str = config_dir.to_string_lossy().to_string();

    if !config_dir.exists() {
        return Ok(build_session_scan_error(
            cli_name.clone(),
            config_dir_str,
            format!("{} 配置目录不存在", cli_name),
        ));
    }

    let project_filter = input.project_path.as_deref();
    let sessions = match cli_name.as_str() {
        "claude" | "claude-code" => {
            let projects_dir = config_dir.join("projects");
            collect_cli_sessions_from_files(
                collect_claude_session_files(&projects_dir, project_filter),
                prefer_fast_scan && project_filter.is_none(),
                project_filter,
            )
        }
        "codex" => {
            let sessions_dir = config_dir.join("sessions");
            let mut session_files = Vec::new();
            if sessions_dir.exists() {
                collect_jsonl_files(&sessions_dir, &mut session_files);
            }
            collect_cli_sessions_from_files(session_files, prefer_fast_scan, project_filter)
        }
        "opencode" => scan_opencode_sessions(&config_dir, project_filter),
        "qwen" | "qwen-code" => Vec::new(),
        _ => Vec::new(),
    };

    Ok(build_session_scan_result(
        cli_name,
        config_dir_str,
        sessions,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(prefix: &str) -> Self {
            let unique = format!(
                "{}-{}",
                prefix,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let path = std::env::temp_dir().join(unique);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

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
    fn unwraps_claude_wrapped_user_prompt() {
        let json = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": "system:\n内部说明\n\ndeveloper:\n开发说明\n\nuser:\n请修复会话详情显示"
            }
        });

        assert_eq!(extract_jsonl_message_type(&json), "user");
        assert_eq!(
            extract_jsonl_message_content(&json).as_deref(),
            Some("请修复会话详情显示")
        );
        assert_eq!(extract_jsonl_role(&json).as_deref(), Some("user"));
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

    #[test]
    fn normalizes_claude_project_dir_names() {
        assert_eq!(
            normalized_project_dir_name("C:\\work/demo-project"),
            "C--work-demo-project"
        );
        assert_eq!(
            normalized_project_dir_name("/Users/demo/project"),
            "-Users-demo-project"
        );
    }

    #[test]
    fn filters_sessions_by_project_path() {
        let session = ScannedCliSession {
            session_id: "session-1".to_string(),
            session_path: "/tmp/session-1.jsonl".to_string(),
            project_path: Some("/tmp/project-a".to_string()),
            first_message: Some("hello".to_string()),
            message_count: 2,
            created_at: "2026-03-20T10:00:00Z".to_string(),
            updated_at: "2026-03-20T10:00:00Z".to_string(),
        };

        assert!(should_keep_session(&session, None));
        assert!(should_keep_session(&session, Some("/tmp/project-a")));
        assert!(!should_keep_session(&session, Some("/tmp/project-b")));
    }

    #[test]
    fn collects_claude_session_files_for_filtered_project() {
        let temp_dir = TestDir::new("scan-claude-projects");
        let projects_dir = temp_dir.path().join("projects");
        let project_dir = projects_dir.join(normalized_project_dir_name("/tmp/project-a"));
        let other_dir = projects_dir.join(normalized_project_dir_name("/tmp/project-b"));

        fs::create_dir_all(&project_dir).unwrap();
        fs::create_dir_all(&other_dir).unwrap();
        fs::write(project_dir.join("keep.jsonl"), "{}\n").unwrap();
        fs::write(project_dir.join("ignore.txt"), "not a session").unwrap();
        fs::write(other_dir.join("other.jsonl"), "{}\n").unwrap();

        let files = collect_claude_session_files(&projects_dir, Some("/tmp/project-a"));

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("keep.jsonl"));
    }

    #[test]
    fn collects_sessions_from_files_and_applies_project_filter() {
        let temp_dir = TestDir::new("scan-session-files");
        let first = temp_dir.path().join("session-a.jsonl");
        let second = temp_dir.path().join("session-b.jsonl");

        fs::write(
            &first,
            concat!(
                "{\"type\":\"session_meta\",\"payload\":{\"cwd\":\"/tmp/project-a\"}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"first task\"}]},\"timestamp\":\"2026-03-20T10:00:00Z\"}\n"
            ),
        )
        .unwrap();
        fs::write(
            &second,
            concat!(
                "{\"type\":\"session_meta\",\"payload\":{\"cwd\":\"/tmp/project-b\"}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"second task\"}]},\"timestamp\":\"2026-03-20T11:00:00Z\"}\n"
            ),
        )
        .unwrap();

        let sessions =
            collect_cli_sessions_from_files(vec![first, second], false, Some("/tmp/project-b"));

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].project_path.as_deref(), Some("/tmp/project-b"));
        assert_eq!(sessions[0].first_message.as_deref(), Some("second task"));
        assert_eq!(sessions[0].message_count, 1);
    }

    #[test]
    fn sorts_session_scan_results_by_updated_time_descending() {
        let result = build_session_scan_result(
            "codex".to_string(),
            "/tmp/.codex".to_string(),
            vec![
                ScannedCliSession {
                    session_id: "older".to_string(),
                    session_path: "/tmp/older.jsonl".to_string(),
                    project_path: None,
                    first_message: None,
                    message_count: 0,
                    created_at: "2026-03-20T09:00:00Z".to_string(),
                    updated_at: "2026-03-20T09:00:00Z".to_string(),
                },
                ScannedCliSession {
                    session_id: "newer".to_string(),
                    session_path: "/tmp/newer.jsonl".to_string(),
                    project_path: None,
                    first_message: None,
                    message_count: 0,
                    created_at: "2026-03-20T12:00:00Z".to_string(),
                    updated_at: "2026-03-20T12:00:00Z".to_string(),
                },
            ],
        );

        assert_eq!(result.sessions[0].session_id, "newer");
        assert_eq!(result.sessions[1].session_id, "older");
    }

    #[test]
    fn scans_claude_project_level_mcp_from_main_config_projects_map() {
        let temp_dir = TestDir::new("scan-claude-project-mcp");
        let config_file = temp_dir.path().join(".claude.json");
        let project_root = temp_dir.path().join("demo-project");

        fs::create_dir_all(&project_root).unwrap();
        fs::write(
            &config_file,
            format!(
                r#"{{
  "projects": {{
    "{}": {{
      "mcpServers": {{
        "laya_mcp_server": {{
          "command": "npx",
          "args": ["-y", "@acme/laya-mcp"]
        }}
      }}
    }}
  }}
}}"#,
                project_root.to_string_lossy()
            ),
        )
        .unwrap();

        let servers = scan_mcp_config("claude", temp_dir.path(), &config_file, Some(&project_root))
            .expect("scan mcp config");

        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "laya_mcp_server");
        assert_eq!(servers[0].scope, McpConfigScope::Project);
        assert_eq!(servers[0].command.as_deref(), Some("npx"));
    }

    #[test]
    fn scans_claude_project_level_mcp_reference_from_main_config_projects_map() {
        let temp_dir = TestDir::new("scan-claude-project-mcp-ref");
        let config_file = temp_dir.path().join(".claude.json");
        let project_root = temp_dir.path().join("demo-project");

        fs::create_dir_all(&project_root).unwrap();
        fs::write(
            &config_file,
            format!(
                r#"{{
  "mcpServers": {{
    "laya_mcp_server": {{
      "command": "npx",
      "args": ["-y", "@acme/laya-mcp"]
    }}
  }},
  "projects": {{
    "{}": {{
      "mcpServers": {{
        "laya_mcp_server": {{}}
      }}
    }}
  }}
}}"#,
                project_root.to_string_lossy()
            ),
        )
        .unwrap();

        let servers = scan_mcp_config("claude", temp_dir.path(), &config_file, Some(&project_root))
            .expect("scan mcp config");

        assert_eq!(servers.len(), 2);
        let project_server = servers
            .iter()
            .find(|server| server.scope == McpConfigScope::Project)
            .expect("project server");
        assert_eq!(project_server.name, "laya_mcp_server");
        assert_eq!(project_server.command.as_deref(), Some("npx"));
        assert_eq!(
            project_server.args.as_ref(),
            Some(&vec!["-y".to_string(), "@acme/laya-mcp".to_string()])
        );
    }
}
