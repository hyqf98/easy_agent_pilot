use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Skill 文件内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFileContent {
    pub path: String,
    pub content: String,
    pub file_type: String, // "markdown", "text", "code"
}

/// Reference 文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceFile {
    pub name: String,
    pub path: String,
    pub file_type: String,
    pub size: u64,
}

/// Reference 文件内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceFileContent {
    pub name: String,
    pub path: String,
    pub content: String,
    pub file_type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSkillReferenceInput {
    pub title: String,
    pub summary: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCliSkillInput {
    pub cli_path: String,
    pub cli_type: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub instructions: String,
    #[serde(default)]
    pub references: Vec<CreateSkillReferenceInput>,
    #[serde(default)]
    pub include_scripts_dir: bool,
    #[serde(default)]
    pub include_assets_dir: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCliSkillResult {
    pub skill_path: String,
    pub skill_file_path: String,
    pub references_path: Option<String>,
    pub scripts_path: Option<String>,
    pub assets_path: Option<String>,
}

/// Plugin 内部项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalItem {
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub item_type: String, // "skill", "command", "agent"
}

/// Plugin 详细信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDetails {
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub install_source: Option<String>,
    pub internal_skills: Vec<InternalItem>,
    pub internal_commands: Vec<InternalItem>,
    pub internal_agents: Vec<InternalItem>,
}

/// 根据文件扩展名获取文件类型
fn get_file_type(path: &Path) -> String {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match extension.as_deref() {
        Some("md") => "markdown".to_string(),
        Some("js") | Some("ts") | Some("jsx") | Some("tsx") => "javascript".to_string(),
        Some("py") => "python".to_string(),
        Some("rs") => "rust".to_string(),
        Some("json") => "json".to_string(),
        Some("yaml") | Some("yml") => "yaml".to_string(),
        Some("toml") => "toml".to_string(),
        Some("html") => "html".to_string(),
        Some("css") | Some("scss") => "css".to_string(),
        Some("sh") | Some("bash") => "shell".to_string(),
        _ => "text".to_string(),
    }
}

fn slugify_name(value: &str, fallback: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        fallback.to_string()
    } else {
        slug
    }
}

fn unique_markdown_path(base_dir: &Path, desired_stem: &str) -> PathBuf {
    let safe_stem = slugify_name(desired_stem, "reference");
    let mut candidate = base_dir.join(format!("{}.md", safe_stem));
    let mut index = 2;

    while candidate.exists() {
        candidate = base_dir.join(format!("{}-{}.md", safe_stem, index));
        index += 1;
    }

    candidate
}

fn render_directory_tree(
    skill_dir_name: &str,
    reference_files: &[String],
    include_scripts_dir: bool,
    include_assets_dir: bool,
) -> String {
    let mut lines = vec![format!("{}/", skill_dir_name), "├── SKILL.md".to_string()];

    if !reference_files.is_empty() {
        lines.push("├── references/".to_string());
        for (index, file_name) in reference_files.iter().enumerate() {
            let prefix = if index + 1 == reference_files.len()
                && !include_scripts_dir
                && !include_assets_dir
            {
                "│   └──"
            } else {
                "│   ├──"
            };
            lines.push(format!("{} {}", prefix, file_name));
        }
    }

    if include_scripts_dir {
        let is_last = !include_assets_dir;
        lines.push(format!("{} scripts/", if is_last { "└──" } else { "├──" }));
    }

    if include_assets_dir {
        lines.push("└── assets/".to_string());
    }

    lines.join("\n")
}

fn render_skill_markdown(
    name: &str,
    description: Option<&str>,
    instructions: &str,
    references: &[(String, String, Option<String>)],
    include_scripts_dir: bool,
    include_assets_dir: bool,
    skill_dir_name: &str,
) -> String {
    let clean_name = name.trim();
    let clean_description = description.unwrap_or("").trim();
    let clean_instructions = instructions.trim();
    let reference_files = references
        .iter()
        .map(|(_, file_name, _)| file_name.clone())
        .collect::<Vec<_>>();
    let tree = render_directory_tree(
        skill_dir_name,
        &reference_files,
        include_scripts_dir,
        include_assets_dir,
    );

    let mut sections = vec![format!(
        "---\nname: {}\ndescription: {}\n---",
        clean_name, clean_description
    )];
    sections.push(format!("# {}", clean_name));

    if !clean_description.is_empty() {
        sections.push(format!("## 技能概述\n\n{}", clean_description));
    }

    sections.push(format!("## 工作说明\n\n{}", clean_instructions));

    if !references.is_empty() {
        let reference_lines = references
            .iter()
            .map(|(title, file_name, summary)| match summary.as_deref() {
                Some(text) if !text.trim().is_empty() => {
                    format!("- [{}](references/{}) - {}", title, file_name, text.trim())
                }
                _ => format!("- [{}](references/{})", title, file_name),
            })
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!(
            "## 参考文档\n\n如需更详细上下文，优先打开这些文档：\n{}",
            reference_lines
        ));
    }

    sections.push(format!("## 文件结构\n\n```text\n{}\n```", tree));
    sections.join("\n\n")
}

/// 创建 Skills 标准结构
#[tauri::command]
pub fn create_cli_skill_scaffold(
    input: CreateCliSkillInput,
) -> Result<CreateCliSkillResult, String> {
    let paths = crate::commands::cli_config::get_cli_config_paths_internal(
        &input.cli_path,
        input.cli_type.as_deref(),
    )?;

    let skills_dir = PathBuf::from(paths.skills_dir);
    fs::create_dir_all(&skills_dir)
        .map_err(|e| format!("Failed to create skills directory: {}", e))?;

    let skill_dir_name = slugify_name(&input.name, "custom-skill");
    let skill_dir = skills_dir.join(&skill_dir_name);

    if skill_dir.exists() {
        return Err(format!(
            "Skill already exists: {}",
            skill_dir.to_string_lossy()
        ));
    }

    fs::create_dir_all(&skill_dir)
        .map_err(|e| format!("Failed to create skill directory: {}", e))?;

    let mut created_references: Vec<(String, String, Option<String>)> = Vec::new();
    let references_path = if input.references.is_empty() {
        None
    } else {
        let references_dir = skill_dir.join("references");
        fs::create_dir_all(&references_dir)
            .map_err(|e| format!("Failed to create references directory: {}", e))?;

        for reference in &input.references {
            let file_path = unique_markdown_path(&references_dir, &reference.title);
            let file_name = file_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("reference.md")
                .to_string();
            let summary = reference
                .summary
                .clone()
                .filter(|text| !text.trim().is_empty());
            let content = format!(
                "# {}\n\n{}",
                reference.title.trim(),
                reference.content.trim()
            );

            fs::write(&file_path, content)
                .map_err(|e| format!("Failed to write reference file: {}", e))?;

            created_references.push((reference.title.trim().to_string(), file_name, summary));
        }

        Some(references_dir.to_string_lossy().to_string())
    };

    let scripts_path = if input.include_scripts_dir {
        let scripts_dir = skill_dir.join("scripts");
        fs::create_dir_all(&scripts_dir)
            .map_err(|e| format!("Failed to create scripts directory: {}", e))?;
        Some(scripts_dir.to_string_lossy().to_string())
    } else {
        None
    };

    let assets_path = if input.include_assets_dir {
        let assets_dir = skill_dir.join("assets");
        fs::create_dir_all(&assets_dir)
            .map_err(|e| format!("Failed to create assets directory: {}", e))?;
        Some(assets_dir.to_string_lossy().to_string())
    } else {
        None
    };

    let skill_markdown = render_skill_markdown(
        &input.name,
        input.description.as_deref(),
        &input.instructions,
        &created_references,
        input.include_scripts_dir,
        input.include_assets_dir,
        &skill_dir_name,
    );

    let skill_file_path = skill_dir.join("SKILL.md");
    fs::write(&skill_file_path, skill_markdown)
        .map_err(|e| format!("Failed to write SKILL.md: {}", e))?;

    Ok(CreateCliSkillResult {
        skill_path: skill_dir.to_string_lossy().to_string(),
        skill_file_path: skill_file_path.to_string_lossy().to_string(),
        references_path,
        scripts_path,
        assets_path,
    })
}

/// 读取 Skill 文件内容 (SKILL.md 或 skill.md)
#[tauri::command]
pub fn read_skill_file(skill_path: String) -> Result<SkillFileContent, String> {
    let skill_dir = PathBuf::from(&skill_path);

    if !skill_dir.exists() {
        return Err(format!("Skill directory does not exist: {}", skill_path));
    }

    // 尝试读取 SKILL.md（优先）或 skill.md
    let skill_md = skill_dir.join("SKILL.md");
    let skill_md_lower = skill_dir.join("skill.md");

    let md_path = if skill_md.exists() {
        skill_md
    } else if skill_md_lower.exists() {
        skill_md_lower
    } else {
        return Err(format!("No SKILL.md or skill.md found in: {}", skill_path));
    };

    let content =
        fs::read_to_string(&md_path).map_err(|e| format!("Failed to read skill file: {}", e))?;

    Ok(SkillFileContent {
        path: md_path.to_string_lossy().to_string(),
        content,
        file_type: "markdown".to_string(),
    })
}

/// 列出 Skill references 目录下的文件
#[tauri::command]
pub fn list_skill_references(skill_path: String) -> Result<Vec<ReferenceFile>, String> {
    let skill_dir = PathBuf::from(&skill_path);
    let references_dir = skill_dir.join("references");

    if !references_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();

    fn scan_directory(dir: &Path, files: &mut Vec<ReferenceFile>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // 递归扫描子目录
                scan_directory(&path, files)?;
            } else if path.is_file() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let metadata = fs::metadata(&path).ok();
                let size = metadata.map(|m| m.len()).unwrap_or(0);

                files.push(ReferenceFile {
                    name,
                    path: path.to_string_lossy().to_string(),
                    file_type: get_file_type(&path),
                    size,
                });
            }
        }

        Ok(())
    }

    scan_directory(&references_dir, &mut files)?;

    // 按文件名排序
    files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(files)
}

/// 读取 reference 文件内容
#[tauri::command]
pub fn read_reference_file(file_path: String) -> Result<ReferenceFileContent, String> {
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;

    Ok(ReferenceFileContent {
        name,
        path: file_path,
        content,
        file_type: get_file_type(&path),
    })
}

/// 解析 plugin.json 获取插件元信息
fn parse_plugin_json_for_details(
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

            let author = if let Some(author_obj) = json.get("author") {
                if let Some(author_str) = author_obj.as_str() {
                    Some(author_str.to_string())
                } else if let Some(author_obj) = author_obj.as_object() {
                    author_obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            } else {
                None
            };

            return (version, description, author);
        }
    }
    (None, None, None)
}

fn resolve_plugin_manifest_path_for_detail(plugin_dir: &Path) -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let codex_plugins_dir = home_dir.join(".codex").join("plugins");
    let manifest_dirs = if plugin_dir.starts_with(&codex_plugins_dir) {
        [".codex-plugin", ".claude-plugin"]
    } else {
        [".claude-plugin", ".codex-plugin"]
    };

    manifest_dirs
        .iter()
        .map(|dir_name| plugin_dir.join(dir_name).join("plugin.json"))
        .find(|candidate| candidate.exists())
}

/// 解析 skill.md 文件获取描述
fn parse_skill_description(skill_md_path: &PathBuf) -> Option<String> {
    if let Ok(content) = fs::read_to_string(skill_md_path) {
        // 尝试从 YAML frontmatter 解析 description
        let lines: Vec<&str> = content.lines().collect();

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

            for line in frontmatter_lines {
                let line = line.trim();
                if let Some((key, value)) = line.split_once(':') {
                    if key.trim() == "description" {
                        return Some(value.trim().to_string());
                    }
                }
            }
        }

        // 如果没有 frontmatter，返回第一行非空内容
        for line in lines {
            let line = line.trim();
            if !line.is_empty() && line != "---" {
                return Some(line.to_string());
            }
        }
    }
    None
}

fn first_existing_description_path(base_path: &Path, file_names: &[&str]) -> Option<PathBuf> {
    file_names
        .iter()
        .map(|name| base_path.join(name))
        .find(|path| path.exists())
}

fn collect_internal_items(
    base_dir: &Path,
    item_type: &str,
    description_files: &[&str],
) -> Vec<InternalItem> {
    if !base_dir.exists() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(base_dir) else {
        return Vec::new();
    };

    let mut items = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = if path.is_dir() {
            path.file_name()
                .map(|item| item.to_string_lossy().to_string())
                .unwrap_or_default()
        } else if path.extension().is_some_and(|extension| extension == "md") {
            path.file_stem()
                .map(|item| item.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            continue;
        };

        let description = if path.is_dir() {
            first_existing_description_path(&path, description_files)
                .and_then(|description_path| parse_skill_description(&description_path))
        } else {
            parse_skill_description(&path)
        };

        items.push(InternalItem {
            name,
            path: path.to_string_lossy().to_string(),
            description,
            item_type: item_type.to_string(),
        });
    }

    items
}

/// 扫描 Plugin 内部的 skills/commands/agents 目录
fn scan_plugin_internal_items(
    plugin_path: &Path,
) -> (Vec<InternalItem>, Vec<InternalItem>, Vec<InternalItem>) {
    (
        collect_internal_items(
            &plugin_path.join("skills"),
            "skill",
            &["skill.md", "SKILL.md"],
        ),
        collect_internal_items(
            &plugin_path.join("commands"),
            "command",
            &["command.md", "COMMAND.md"],
        ),
        collect_internal_items(
            &plugin_path.join("agents"),
            "agent",
            &["agent.md", "AGENT.md"],
        ),
    )
}

/// 尝试从 installed_plugins.json 获取安装来源
fn get_install_source(plugin_name: &str) -> Option<String> {
    let persistence_dir = crate::commands::get_persistence_dir_path().ok()?;
    let installed_plugins_path = persistence_dir.join("plugins.json");

    if !installed_plugins_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&installed_plugins_path).ok()?;
    let plugins = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    let items = plugins.as_array()?;

    items.iter().find_map(|item| {
        let matches_name = item
            .get("name")
            .and_then(|value| value.as_str())
            .map(|value| value == plugin_name)
            .unwrap_or(false);

        if !matches_name {
            return None;
        }

        item.get("source_market")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
    })
}

fn write_json_file_pretty(path: &Path, value: &JsonValue) -> Result<(), String> {
    let content = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Failed to serialize JSON file: {}", error))?;
    fs::write(path, format!("{}\n", content))
        .map_err(|error| format!("Failed to write JSON file: {}", error))
}

fn prune_cli_installed_plugins_file(plugin_dir: &Path) -> Result<(), String> {
    let Some(plugins_dir) = plugin_dir.parent() else {
        return Ok(());
    };
    let installed_plugins_path = plugins_dir.join("installed_plugins.json");
    if !installed_plugins_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&installed_plugins_path)
        .map_err(|error| format!("Failed to read installed_plugins.json: {}", error))?;
    let mut json = serde_json::from_str::<JsonValue>(&content)
        .map_err(|error| format!("Failed to parse installed_plugins.json: {}", error))?;
    let plugin_dir_str = plugin_dir.to_string_lossy().to_string();

    let Some(root) = json.as_object_mut() else {
        return Ok(());
    };
    let Some(plugins_obj) = root
        .get_mut("plugins")
        .and_then(|value| value.as_object_mut())
    else {
        return Ok(());
    };

    let keys = plugins_obj.keys().cloned().collect::<Vec<_>>();
    for key in keys {
        let should_remove_key = plugins_obj
            .get_mut(&key)
            .and_then(|value| value.as_array_mut())
            .map(|entries| {
                entries.retain(|entry| {
                    entry
                        .get("installPath")
                        .and_then(|value| value.as_str())
                        .map(|path| path != plugin_dir_str)
                        .unwrap_or(true)
                });
                entries.is_empty()
            })
            .unwrap_or(false);

        if should_remove_key {
            plugins_obj.remove(&key);
        }
    }

    write_json_file_pretty(&installed_plugins_path, &json)
}

fn prune_app_installed_plugins_file(plugin_dir: &Path) -> Result<(), String> {
    let persistence_dir = crate::commands::get_persistence_dir_path().map_err(|e| e.to_string())?;
    let installed_plugins_path = persistence_dir.join("plugins.json");
    if !installed_plugins_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&installed_plugins_path)
        .map_err(|error| format!("Failed to read plugins.json: {}", error))?;
    let mut json = serde_json::from_str::<JsonValue>(&content)
        .map_err(|error| format!("Failed to parse plugins.json: {}", error))?;

    let Some(items) = json.as_array_mut() else {
        return Ok(());
    };

    items.retain(|item| {
        let matches_component = item
            .get("components")
            .and_then(|value| value.as_array())
            .map(|components| {
                components.iter().any(|component| {
                    component
                        .get("target_path")
                        .and_then(|value| value.as_str())
                        .map(|path| {
                            let target_path = Path::new(path);
                            target_path == plugin_dir || target_path.starts_with(plugin_dir)
                        })
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        !matches_component
    });

    write_json_file_pretty(&installed_plugins_path, &json)
}

/// 获取 Plugin 详细信息
#[tauri::command]
pub fn get_plugin_details(plugin_path: String) -> Result<PluginDetails, String> {
    let plugin_dir = PathBuf::from(&plugin_path);

    if !plugin_dir.exists() {
        return Err(format!("Plugin directory does not exist: {}", plugin_path));
    }

    // 获取插件名称
    let name = plugin_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // 尝试读取 plugin.json
    let (version, description, author) = resolve_plugin_manifest_path_for_detail(&plugin_dir)
        .map(|plugin_json_path| parse_plugin_json_for_details(&plugin_json_path))
        .unwrap_or((None, None, None));

    // 获取安装来源
    let install_source = get_install_source(&name);

    // 扫描内部 items
    let (internal_skills, internal_commands, internal_agents) =
        scan_plugin_internal_items(&plugin_dir);

    Ok(PluginDetails {
        name,
        path: plugin_path,
        version,
        description,
        author,
        install_source,
        internal_skills,
        internal_commands,
        internal_agents,
    })
}

/// 删除 Skill 目录
#[tauri::command]
pub fn delete_skill_directory(skill_path: String) -> Result<(), String> {
    let skill_dir = PathBuf::from(&skill_path);

    if !skill_dir.exists() {
        return Err(format!("Skill path does not exist: {}", skill_path));
    }

    // 安全检查：确保路径在 skills 目录下
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    let skills_dir = home_dir.join(".claude").join("skills");
    let codex_skills_dir = home_dir.join(".codex").join("skills");
    let qwen_skills_dir = home_dir.join(".qwen").join("skills");
    let opencode_skills_dir = home_dir.join(".config").join("opencode").join("skills");

    let is_valid_path = skill_dir.starts_with(&skills_dir)
        || skill_dir.starts_with(&codex_skills_dir)
        || skill_dir.starts_with(&qwen_skills_dir)
        || skill_dir.starts_with(&opencode_skills_dir);

    if !is_valid_path {
        return Err(
            "Invalid skill path: skill must be in a valid CLI skills directory".to_string(),
        );
    }

    let metadata = fs::symlink_metadata(&skill_dir)
        .map_err(|e| format!("Failed to inspect skill path: {}", e))?;

    if metadata.is_dir() {
        fs::remove_dir_all(&skill_dir)
            .map_err(|e| format!("Failed to delete skill directory: {}", e))?;
    } else {
        fs::remove_file(&skill_dir).map_err(|e| format!("Failed to delete skill file: {}", e))?;
    }

    Ok(())
}

/// 删除 Plugin 目录
#[tauri::command]
pub fn delete_plugin_directory(plugin_path: String) -> Result<(), String> {
    let plugin_dir = PathBuf::from(&plugin_path);

    if !plugin_dir.exists() {
        return Err(format!("Plugin directory does not exist: {}", plugin_path));
    }

    // 安全检查：确保路径在 plugins 目录下
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    let plugins_dir = home_dir.join(".claude").join("plugins");
    let codex_plugins_dir = home_dir.join(".codex").join("plugins");
    let qwen_plugins_dir = home_dir.join(".qwen").join("plugins");
    let opencode_plugins_dir = home_dir.join(".config").join("opencode").join("plugins");

    let is_valid_path = plugin_dir.starts_with(&plugins_dir)
        || plugin_dir.starts_with(&codex_plugins_dir)
        || plugin_dir.starts_with(&qwen_plugins_dir)
        || plugin_dir.starts_with(&opencode_plugins_dir);

    if !is_valid_path {
        return Err(
            "Invalid plugin path: plugin must be in a valid CLI plugins directory".to_string(),
        );
    }

    // 删除插件目录
    fs::remove_dir_all(&plugin_dir)
        .map_err(|e| format!("Failed to delete plugin directory: {}", e))?;

    prune_cli_installed_plugins_file(&plugin_dir)?;
    prune_app_installed_plugins_file(&plugin_dir)?;

    Ok(())
}

/// 目录文件信息

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryFile {
    pub name: String,
    pub path: String,
}

/// 读取文件内容
#[tauri::command]
pub fn read_file_content(file_path: String) -> Result<String, String> {
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", file_path));
    }

    fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))
}

/// 写入文件内容
#[tauri::command]
pub fn write_file_content(file_path: String, content: String) -> Result<(), String> {
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", file_path));
    }

    // 安全检查：确保路径在允许的目录下
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    let claude_dir = home_dir.join(".claude");
    let codex_dir = home_dir.join(".codex");
    let qwen_dir = home_dir.join(".qwen");
    let opencode_dir = home_dir.join(".config").join("opencode");

    let is_valid_path = path.starts_with(&claude_dir)
        || path.starts_with(&codex_dir)
        || path.starts_with(&qwen_dir)
        || path.starts_with(&opencode_dir);

    if !is_valid_path {
        return Err("Invalid file path: file must be in a valid CLI directory".to_string());
    }

    fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))
}

/// 列出目录下指定扩展名的文件
#[tauri::command]
pub fn list_directory_files(
    dir_path: String,
    extension: Option<String>,
) -> Result<Vec<DirectoryFile>, String> {
    let dir = PathBuf::from(&dir_path);

    if !dir.exists() {
        return Err(format!("Directory does not exist: {}", dir_path));
    }

    if !dir.is_dir() {
        return Err(format!("Path is not a directory: {}", dir_path));
    }

    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file() {
                // 检查扩展名
                let matches_extension = if let Some(ref ext) = extension {
                    path.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e == ext.trim_start_matches('.'))
                        .unwrap_or(false)
                } else {
                    true
                };

                if matches_extension {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    files.push(DirectoryFile {
                        name,
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }

    // 按文件名排序
    files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(files)
}

// ───── Plugin Slash Commands ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSlashCommand {
    pub name: String,
    pub plugin_name: String,
    pub cli_type: String,
    pub description: Option<String>,
    pub argument_hint: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub disable_model_invocation: bool,
}

struct CommandFrontmatter {
    description: Option<String>,
    argument_hint: Option<String>,
    allowed_tools: Option<Vec<String>>,
    disable_model_invocation: bool,
}

fn parse_command_frontmatter(content: &str) -> CommandFrontmatter {
    let lines: Vec<&str> = content.lines().collect();

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

    let mut description = None;
    let mut argument_hint = None;
    let mut allowed_tools = None;
    let mut disable_model_invocation = false;

    if let (Some(start), Some(end)) = (start_idx, end_idx) {
        let frontmatter_lines = &lines[start + 1..end];

        for line in frontmatter_lines {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "description" => {
                        description = Some(unquote_yaml_string(value));
                    }
                    "argument-hint" => {
                        argument_hint = Some(unquote_yaml_string(value));
                    }
                    "allowed-tools" => {
                        let trimmed = value.trim();
                        if trimmed.starts_with('[') && trimmed.ends_with(']') {
                            let inner = trimmed[1..trimmed.len() - 1].trim();
                            allowed_tools = Some(
                                inner
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect(),
                            );
                        } else {
                            allowed_tools = Some(
                                value
                                    .split(|c: char| c == ',' || c.is_whitespace())
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect(),
                            );
                        }
                    }
                    "disable-model-invocation" => {
                        disable_model_invocation = value == "true";
                    }
                    _ => {}
                }
            }
        }
    }

    CommandFrontmatter {
        description,
        argument_hint,
        allowed_tools,
        disable_model_invocation,
    }
}

fn unquote_yaml_string(value: &str) -> String {
    let trimmed = value.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn resolve_cli_plugins_dir(cli_type: &str) -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    match cli_type {
        "claude" | "claude-code" => Some(home_dir.join(".claude").join("plugins")),
        "codex" => Some(home_dir.join(".codex").join("plugins")),
        "opencode" => Some(home_dir.join(".config").join("opencode").join("plugins")),
        _ => None,
    }
}

fn resolve_cli_skills_dir(cli_type: &str) -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    match cli_type {
        "opencode" => Some(home_dir.join(".config").join("opencode").join("skill")),
        _ => None,
    }
}

fn scan_skill_commands(skills_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir(skills_dir) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.is_empty() || name.starts_with('.') {
            continue;
        }
        let skill_file = path.join("SKILL.md");
        if skill_file.exists() {
            result.push((name, skill_file));
        }
    }
    result
}

fn collect_slash_commands_from_skills(
    skills: &[(String, PathBuf)],
    cli_name: &str,
    commands: &mut Vec<PluginSlashCommand>,
    seen_names: &mut HashSet<String>,
) {
    for (skill_name, skill_file) in skills {
        if !seen_names.insert(skill_name.clone()) {
            continue;
        }

        let frontmatter = fs::read_to_string(skill_file)
            .map(|content| parse_command_frontmatter(&content))
            .unwrap_or(CommandFrontmatter {
                description: None,
                argument_hint: None,
                allowed_tools: None,
                disable_model_invocation: false,
            });

        commands.push(PluginSlashCommand {
            name: skill_name.clone(),
            plugin_name: "skill".to_string(),
            cli_type: cli_name.to_string(),
            description: frontmatter.description,
            argument_hint: frontmatter.argument_hint,
            allowed_tools: frontmatter.allowed_tools,
            disable_model_invocation: frontmatter.disable_model_invocation,
        });
    }
}

fn scan_plugins_with_commands(plugins_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut result = Vec::new();

    let installed_path = plugins_dir.join("installed_plugins.json");
    if installed_path.exists() {
        if let Ok(content) = fs::read_to_string(&installed_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(plugins_obj) = json.get("plugins").and_then(|v| v.as_object()) {
                    for (plugin_key, plugin_entries) in plugins_obj {
                        if let Some(first_entry) = plugin_entries
                            .as_array()
                            .and_then(|entries| entries.first())
                        {
                            if let Some(install_path_str) =
                                first_entry.get("installPath").and_then(|v| v.as_str())
                            {
                                let install_path = PathBuf::from(install_path_str);
                                if install_path.exists()
                                    && install_path.join("commands").exists()
                                {
                                    let display_name = plugin_key
                                        .split('@')
                                        .next()
                                        .unwrap_or(plugin_key)
                                        .to_string();
                                    result.push((display_name, install_path));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if result.is_empty() {
        if let Ok(entries) = fs::read_dir(plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("commands").exists() {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    result.push((name, path));
                }
            }
        }
    }

    result
}

fn collect_command_files(commands_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir(commands_dir) else {
        return result;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if !name.is_empty() && !name.starts_with('.') {
                result.push((name, path));
            }
        } else if path.extension().is_some_and(|ext| ext == "md") {
            let name = path
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if !name.is_empty() {
                result.push((name, path));
            }
        }
    }

    result
}

fn resolve_command_content_path(command_path: &Path) -> Option<PathBuf> {
    if command_path.is_file() {
        return Some(command_path.to_path_buf());
    }
    if command_path.is_dir() {
        for candidate in &["command.md", "COMMAND.md"] {
            let file_path = command_path.join(candidate);
            if file_path.exists() {
                return Some(file_path);
            }
        }
    }
    None
}

fn collect_slash_commands_from_plugins(
    plugins: &[(String, PathBuf)],
    cli_name: &str,
    commands: &mut Vec<PluginSlashCommand>,
    seen_names: &mut HashSet<String>,
) {
    for (plugin_name, plugin_path) in plugins {
        let commands_dir = plugin_path.join("commands");
        let command_files = collect_command_files(&commands_dir);

        for (cmd_name, cmd_path) in command_files {
            let unique_key = format!("{}:{}", plugin_name, cmd_name);
            if !seen_names.insert(unique_key.clone()) {
                continue;
            }

            let content_path = match resolve_command_content_path(&cmd_path) {
                Some(p) => p,
                None => continue,
            };

            let frontmatter = fs::read_to_string(&content_path)
                .map(|content| parse_command_frontmatter(&content))
                .unwrap_or(CommandFrontmatter {
                    description: None,
                    argument_hint: None,
                    allowed_tools: None,
                    disable_model_invocation: false,
                });

            commands.push(PluginSlashCommand {
                name: cmd_name,
                plugin_name: plugin_name.clone(),
                cli_type: cli_name.to_string(),
                description: frontmatter.description,
                argument_hint: frontmatter.argument_hint,
                allowed_tools: frontmatter.allowed_tools,
                disable_model_invocation: frontmatter.disable_model_invocation,
            });
        }
    }
}

#[tauri::command]
pub fn scan_plugin_slash_commands(
    cli_type: String,
    project_path: Option<String>,
) -> Result<Vec<PluginSlashCommand>, String> {
    let cli_name = cli_type.trim().to_lowercase();
    let plugins_dir = match resolve_cli_plugins_dir(&cli_name) {
        Some(dir) => dir,
        None => return Err(format!("Unknown CLI type: {}", cli_name)),
    };

    let mut commands = Vec::new();
    let mut seen_names = HashSet::new();

    if plugins_dir.exists() {
        let plugins = scan_plugins_with_commands(&plugins_dir);
        collect_slash_commands_from_plugins(&plugins, &cli_name, &mut commands, &mut seen_names);
    }

    if let Some(skills_dir) = resolve_cli_skills_dir(&cli_name) {
        if skills_dir.exists() {
            let skills = scan_skill_commands(&skills_dir);
            collect_slash_commands_from_skills(&skills, &cli_name, &mut commands, &mut seen_names);
        }
    }

    let mut commands = Vec::new();
    let mut seen_names = HashSet::new();

    let plugins = scan_plugins_with_commands(&plugins_dir);
    collect_slash_commands_from_plugins(&plugins, &cli_name, &mut commands, &mut seen_names);

    if let Some(skills_dir) = resolve_cli_skills_dir(&cli_name) {
        if skills_dir.exists() {
            let skills = scan_skill_commands(&skills_dir);
            collect_slash_commands_from_skills(&skills, &cli_name, &mut commands, &mut seen_names);
        }
    }

    if let Some(ref project_root) = project_path {
        let project_root = PathBuf::from(project_root.trim());
        if project_root.exists() {
            let project_plugins_dir = match cli_name.as_str() {
                "opencode" => Some(project_root.join(".opencode").join("plugins")),
                _ => None,
            };

            if let Some(pp_dir) = project_plugins_dir {
                if pp_dir.exists() {
                    let project_plugins = scan_plugins_with_commands(&pp_dir);
                    collect_slash_commands_from_plugins(
                        &project_plugins,
                        &cli_name,
                        &mut commands,
                        &mut seen_names,
                    );
                }
            }

            let project_skills_dir = match cli_name.as_str() {
                "opencode" => Some(project_root.join(".opencode").join("skill")),
                _ => None,
            };

            if let Some(ps_dir) = project_skills_dir {
                if ps_dir.exists() {
                    let project_skills = scan_skill_commands(&ps_dir);
                    collect_slash_commands_from_skills(
                        &project_skills,
                        &cli_name,
                        &mut commands,
                        &mut seen_names,
                    );
                }
            }
        }
    }

    Ok(commands)
}
