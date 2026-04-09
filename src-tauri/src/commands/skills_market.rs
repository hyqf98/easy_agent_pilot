use anyhow::Result;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use tauri::AppHandle;
use zip::ZipArchive;

use crate::commands::git_install::{
    cleanup_checkout, clone_repository, copy_dir_recursive, normalize_lookup_name,
};
use crate::commands::mcpmarket_source::{
    get_marketplace_source_strategy, MarketListResponse, MarketplaceSourceQuery,
    SkillArchivePayload, SkillSourceDetail, SkillSourceListItem,
};

pub type SkillMarketItem = SkillSourceListItem;
pub type SkillMarketDetail = SkillSourceDetail;
pub type SkillMarketListResponse = MarketListResponse<SkillMarketItem>;
pub type SkillMarketQuery = MarketplaceSourceQuery;

/// Skill install input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInstallInput {
    pub skill_id: String,
    pub skill_name: String,
    pub cli_type: String,
    pub scope: String,
    pub project_path: Option<String>,
    pub source_market: Option<String>,
}

/// Git skill install input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSkillInstallInput {
    pub repository_url: String,
    pub git_ref: Option<String>,
    pub skill_name: String,
    pub cli_type: String,
}

/// Skill install result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInstallResult {
    pub success: bool,
    pub message: String,
    pub skill_path: Option<String>,
    pub backup_path: Option<String>,
}

/// Installed skill from native CLI skills directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkill {
    pub name: String,
    pub file_name: String,
    pub path: String,
    pub disabled: bool,
    pub source_cli: String,
    pub source_cli_path: String,
    pub scope: String,
    pub description: Option<String>,
    pub installed_at: Option<String>,
    pub triggers: Vec<String>,
}

/// Fetch skills from the active marketplace source.
#[tauri::command]
pub async fn fetch_skills_market(
    app: AppHandle,
    query: SkillMarketQuery,
) -> Result<SkillMarketListResponse, String> {
    let strategy = get_marketplace_source_strategy(query.source_market.as_deref())?;
    strategy.fetch_skill_list(&app, &query).await
}

/// Fetch detailed skill information by slug.
#[tauri::command]
pub async fn fetch_skill_market_detail(
    app: AppHandle,
    skill_id: String,
    source_market: Option<String>,
) -> Result<SkillMarketDetail, String> {
    let strategy = get_marketplace_source_strategy(source_market.as_deref())?;
    strategy.fetch_skill_detail(&app, &skill_id).await
}

fn slugify_skill_name(name: &str) -> String {
    let value = name
        .trim()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "")
        .to_lowercase();

    if value.is_empty() {
        "skill".to_string()
    } else {
        value
    }
}

fn get_cli_skills_dir(cli_type: &str) -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    match cli_type.to_lowercase().as_str() {
        "claude" => Ok(home_dir.join(".claude").join("skills")),
        "codex" => Ok(home_dir.join(".codex").join("skills")),
        "opencode" => Ok(home_dir.join(".config").join("opencode").join("skills")),
        other => Err(format!("Unsupported CLI type for skills: {}", other)),
    }
}

fn build_dir_backup_path(skill_dir: &Path) -> PathBuf {
    let backup_name = format!(
        "{}.backup",
        skill_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("skill")
    );
    skill_dir.with_file_name(backup_name)
}

fn move_dir_with_overwrite(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        fs::remove_dir_all(target).map_err(|e| format!("Failed to clear target dir: {}", e))?;
    }

    fs::rename(source, target).map_err(|e| format!("Failed to move skill directory: {}", e))
}

fn create_skill_backup(skill_dir: &Path) -> Result<Option<String>, String> {
    if !skill_dir.exists() {
        return Ok(None);
    }

    let backup_path = build_dir_backup_path(skill_dir);
    move_dir_with_overwrite(skill_dir, &backup_path)?;
    Ok(Some(backup_path.to_string_lossy().to_string()))
}

fn restore_skill_backup(skill_dir: &Path, backup_path: &Option<String>) -> Result<(), String> {
    if skill_dir.exists() {
        fs::remove_dir_all(skill_dir).map_err(|e| format!("Failed to clear skill dir: {}", e))?;
    }

    if let Some(backup) = backup_path {
        let backup_path = PathBuf::from(backup);
        if backup_path.exists() {
            fs::rename(&backup_path, skill_dir)
                .map_err(|e| format!("Failed to restore skill backup: {}", e))?;
        }
    }

    Ok(())
}

fn finalize_skill_backup(backup_path: &Option<String>) {
    if let Some(backup) = backup_path {
        let backup_path = PathBuf::from(backup);
        if backup_path.exists() {
            let _ = fs::remove_dir_all(backup_path);
        }
    }
}

fn decode_archive(payload: &SkillArchivePayload) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD
        .decode(payload.zip_base64.as_bytes())
        .map_err(|e| format!("Failed to decode skill archive: {}", e))
}

fn detect_common_prefix(entries: &[String]) -> Option<String> {
    let mut prefixes = entries
        .iter()
        .filter_map(|name| {
            Path::new(name)
                .components()
                .next()
                .and_then(|component| match component {
                    Component::Normal(value) => value.to_str().map(|value| value.to_string()),
                    _ => None,
                })
        })
        .collect::<Vec<_>>();

    prefixes.sort();
    prefixes.dedup();

    (prefixes.len() == 1).then(|| prefixes.remove(0))
}

fn unzip_skill_archive(zip_bytes: &[u8], target_dir: &Path) -> Result<(), String> {
    let mut archive = ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|e| format!("Failed to open zip: {}", e))?;

    let entry_names = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|file| file.name().to_string())
        })
        .collect::<Vec<_>>();
    let common_prefix = detect_common_prefix(&entry_names);

    fs::create_dir_all(target_dir).map_err(|e| format!("Failed to create skill dir: {}", e))?;

    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;
        let raw_name = file.name().to_string();
        let normalized = if let Some(prefix) = &common_prefix {
            raw_name
                .strip_prefix(prefix)
                .and_then(|value| value.strip_prefix('/'))
                .map(|value| value.to_string())
                .unwrap_or(raw_name.clone())
        } else {
            raw_name.clone()
        };

        if normalized.is_empty() {
            continue;
        }

        let out_path = target_dir.join(&normalized);
        if !out_path.starts_with(target_dir) {
            return Err("Invalid zip archive path".to_string());
        }

        if file.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create extracted directory: {}", e))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create extracted parent dir: {}", e))?;
        }

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Failed to read extracted file: {}", e))?;
        fs::write(&out_path, buffer)
            .map_err(|e| format!("Failed to write extracted file: {}", e))?;
    }

    Ok(())
}

type ParsedSkillMetadata = (Option<String>, Option<String>, Vec<String>);

fn resolve_skill_markdown_path(skill_dir: &Path) -> Option<PathBuf> {
    ["SKILL.md", "skill.md"]
        .into_iter()
        .map(|file_name| skill_dir.join(file_name))
        .find(|path| path.exists() && path.is_file())
}

fn has_skill_markdown(skill_dir: &Path) -> bool {
    resolve_skill_markdown_path(skill_dir).is_some()
}

fn parse_skill_metadata(skill_dir: &Path) -> Result<ParsedSkillMetadata, String> {
    let Some(skill_md_path) = resolve_skill_markdown_path(skill_dir) else {
        return Ok((None, None, Vec::new()));
    };

    let content = fs::read_to_string(&skill_md_path)
        .map_err(|e| format!("Failed to read {}: {}", skill_md_path.display(), e))?;
    let map = extract_frontmatter_map(&content);
    let name = map.get("name").cloned();
    let description = map.get("description").cloned();
    let triggers = extract_skill_triggers(&content);

    Ok((name, description, triggers))
}

fn extract_frontmatter_map(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return map;
    }

    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            map.insert(
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            );
        }
    }

    map
}

fn extract_skill_triggers(content: &str) -> Vec<String> {
    let direct = content
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .starts_with("triggers:")
                .then(|| trimmed.trim_start_matches("triggers:").trim().to_string())
        })
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .split(',')
                .map(|item| item.trim().trim_matches('"').to_string())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
        });

    if let Some(values) = direct {
        return values;
    }

    let mut triggers = Vec::new();
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "triggers:" {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix('-') {
            let value = value.trim().trim_matches('"');
            if !value.is_empty() {
                triggers.push(value.to_string());
            }
            continue;
        }
        if !trimmed.is_empty() {
            break;
        }
    }

    triggers
}

fn collect_skill_candidate_dirs(
    current_dir: &Path,
    depth: usize,
    candidates: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if depth > 6 {
        return Ok(());
    }

    let entries = fs::read_dir(current_dir).map_err(|error| {
        format!(
            "Failed to read directory {}: {}",
            current_dir.display(),
            error
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| format!("Failed to read directory entry: {}", error))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == ".git")
        {
            continue;
        }

        if has_skill_markdown(&path) {
            candidates.push(path.clone());
        }

        collect_skill_candidate_dirs(&path, depth + 1, candidates)?;
    }

    Ok(())
}

fn find_matching_skill_directory(repo_dir: &Path, skill_name: &str) -> Result<PathBuf, String> {
    let expected_name = normalize_lookup_name(skill_name);
    if expected_name.is_empty() {
        return Err("Skill 名称不能为空".to_string());
    }

    let mut candidates = Vec::new();
    collect_skill_candidate_dirs(repo_dir, 0, &mut candidates)?;

    let mut matched_paths = candidates
        .into_iter()
        .filter(|candidate| {
            let dir_name = candidate
                .file_name()
                .and_then(|name| name.to_str())
                .map(normalize_lookup_name)
                .unwrap_or_default();
            let frontmatter_name = parse_skill_metadata(candidate)
                .ok()
                .and_then(|(name, _, _)| name)
                .map(|name| normalize_lookup_name(&name))
                .unwrap_or_default();

            dir_name == expected_name || frontmatter_name == expected_name
        })
        .collect::<Vec<_>>();

    matched_paths.sort();
    matched_paths.dedup();

    match matched_paths.len() {
        0 => Err(format!("仓库中未找到名为 '{}' 的 Skill", skill_name)),
        1 => Ok(matched_paths.remove(0)),
        _ => Err(format!("仓库中存在多个同名 Skill '{}'", skill_name)),
    }
}

fn install_skill_directory_to_cli(
    source_dir: &Path,
    skill_name: &str,
    cli_type: &str,
) -> Result<SkillInstallResult, String> {
    let skills_dir = get_cli_skills_dir(cli_type)?;
    fs::create_dir_all(&skills_dir).map_err(|e| format!("Failed to create skills dir: {}", e))?;

    let target_name = source_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(slugify_skill_name)
        .unwrap_or_else(|| slugify_skill_name(skill_name));
    let target_dir = skills_dir.join(target_name);
    let backup_path = create_skill_backup(&target_dir)?;

    if let Err(error) = copy_dir_recursive(source_dir, &target_dir) {
        restore_skill_backup(&target_dir, &backup_path)?;
        return Ok(SkillInstallResult {
            success: false,
            message: error,
            skill_path: Some(target_dir.to_string_lossy().to_string()),
            backup_path: None,
        });
    }

    if !has_skill_markdown(&target_dir) {
        restore_skill_backup(&target_dir, &backup_path)?;
        return Ok(SkillInstallResult {
            success: false,
            message: "仓库中的技能目录未包含 SKILL.md".to_string(),
            skill_path: Some(target_dir.to_string_lossy().to_string()),
            backup_path: None,
        });
    }

    finalize_skill_backup(&backup_path);

    Ok(SkillInstallResult {
        success: true,
        message: format!("Skill '{}' installed successfully", skill_name),
        skill_path: Some(target_dir.to_string_lossy().to_string()),
        backup_path,
    })
}

fn list_skill_dirs(base_dir: &Path, cli_type: &str) -> Result<Vec<InstalledSkill>, String> {
    if !base_dir.exists() {
        return Ok(Vec::new());
    }

    let mut installed = Vec::new();
    let entries =
        fs::read_dir(base_dir).map_err(|e| format!("Failed to read skills dir: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        let disabled = dir_name.ends_with(".disabled");
        let real_path = path.clone();
        let has_skill = real_path.join("SKILL.md").exists();
        if !has_skill {
            continue;
        }

        let installed_at = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .map(|time| {
                let datetime: chrono::DateTime<chrono::Utc> = time.into();
                datetime.to_rfc3339()
            });

        let clean_name = dir_name.trim_end_matches(".disabled").to_string();
        let (display_name, description, triggers) =
            parse_skill_metadata(&path).unwrap_or((None, None, Vec::new()));

        installed.push(InstalledSkill {
            name: display_name.unwrap_or_else(|| clean_name.clone()),
            file_name: clean_name,
            path: path.to_string_lossy().to_string(),
            disabled,
            source_cli: cli_type.to_string(),
            source_cli_path: base_dir.to_string_lossy().to_string(),
            scope: "global".to_string(),
            description,
            installed_at,
            triggers,
        });
    }

    installed.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(installed)
}

async fn download_skill_payload(
    app: &AppHandle,
    skill_id: &str,
    source_market: Option<&str>,
) -> Result<SkillArchivePayload, String> {
    let strategy = get_marketplace_source_strategy(source_market)?;
    strategy.download_skill_archive(app, skill_id).await
}

/// Install a skill to the target CLI native skills directory.
#[tauri::command]
pub async fn install_skill_to_cli(
    app: AppHandle,
    input: SkillInstallInput,
) -> Result<SkillInstallResult, String> {
    if input.scope == "project" {
        return Err("当前版本的市场技能安装仅支持全局目录".to_string());
    }

    let archive =
        download_skill_payload(&app, &input.skill_id, input.source_market.as_deref()).await?;
    let zip_bytes = decode_archive(&archive)?;
    let skills_dir = get_cli_skills_dir(&input.cli_type)?;
    fs::create_dir_all(&skills_dir).map_err(|e| format!("Failed to create skills dir: {}", e))?;

    let skill_dir_name = slugify_skill_name(&archive.slug);
    let skill_dir = skills_dir.join(&skill_dir_name);
    let backup_path = create_skill_backup(&skill_dir)?;

    if let Err(error) = unzip_skill_archive(&zip_bytes, &skill_dir) {
        restore_skill_backup(&skill_dir, &backup_path)?;
        return Ok(SkillInstallResult {
            success: false,
            message: error,
            skill_path: Some(skill_dir.to_string_lossy().to_string()),
            backup_path: None,
        });
    }

    if !has_skill_markdown(&skill_dir) {
        restore_skill_backup(&skill_dir, &backup_path)?;
        return Ok(SkillInstallResult {
            success: false,
            message: "下载的技能包中未找到 SKILL.md".to_string(),
            skill_path: Some(skill_dir.to_string_lossy().to_string()),
            backup_path: None,
        });
    }

    finalize_skill_backup(&backup_path);

    Ok(SkillInstallResult {
        success: true,
        message: format!("Skill '{}' installed successfully", input.skill_name),
        skill_path: Some(skill_dir.to_string_lossy().to_string()),
        backup_path,
    })
}

/// Install a skill from a Git repository into the target CLI native skills directory.
#[tauri::command]
pub async fn install_skill_from_git(
    input: GitSkillInstallInput,
) -> Result<SkillInstallResult, String> {
    let checkout = clone_repository(&input.repository_url, input.git_ref.as_deref())?;
    let result = (|| {
        let source_dir = find_matching_skill_directory(&checkout.repo_dir, &input.skill_name)?;
        install_skill_directory_to_cli(&source_dir, &input.skill_name, &input.cli_type)
    })();
    cleanup_checkout(&checkout);
    result
}

/// List installed skills from Claude/Codex native skills directories.
#[tauri::command]
pub async fn list_installed_skills() -> Result<Vec<InstalledSkill>, String> {
    let mut installed = Vec::new();
    for cli_type in ["claude", "codex", "opencode"] {
        let dir = get_cli_skills_dir(cli_type)?;
        installed.extend(list_skill_dirs(&dir, cli_type)?);
    }
    Ok(installed)
}

/// Toggle installed skill (enable/disable) by renaming the skill directory.
#[tauri::command]
pub async fn toggle_installed_skill(
    skill_path: String,
    disable: bool,
) -> Result<SkillInstallResult, String> {
    let path = PathBuf::from(&skill_path);
    if !path.exists() {
        return Err(format!("Skill directory not found: {}", skill_path));
    }

    let dir_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Invalid skill directory name".to_string())?;

    let target_path = if disable {
        if dir_name.ends_with(".disabled") {
            return Ok(SkillInstallResult {
                success: true,
                message: "Skill is already disabled".to_string(),
                skill_path: Some(skill_path),
                backup_path: None,
            });
        }
        path.with_file_name(format!("{}.disabled", dir_name))
    } else if dir_name.ends_with(".disabled") {
        path.with_file_name(dir_name.trim_end_matches(".disabled"))
    } else {
        return Ok(SkillInstallResult {
            success: true,
            message: "Skill is already enabled".to_string(),
            skill_path: Some(skill_path),
            backup_path: None,
        });
    };

    fs::rename(&path, &target_path).map_err(|e| format!("Failed to rename skill dir: {}", e))?;

    Ok(SkillInstallResult {
        success: true,
        message: if disable {
            "Skill disabled successfully".to_string()
        } else {
            "Skill enabled successfully".to_string()
        },
        skill_path: Some(target_path.to_string_lossy().to_string()),
        backup_path: None,
    })
}

/// Uninstall a skill by removing its directory after making a deleted backup copy.
#[tauri::command]
pub async fn uninstall_skill(skill_path: String) -> Result<SkillInstallResult, String> {
    let path = PathBuf::from(&skill_path);
    if !path.exists() {
        return Err(format!("Skill directory not found: {}", skill_path));
    }

    let backup_path = path.with_file_name(format!(
        "{}.deleted",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("skill")
    ));

    move_dir_with_overwrite(&path, &backup_path)?;

    Ok(SkillInstallResult {
        success: true,
        message: "Skill uninstalled successfully".to_string(),
        skill_path: None,
        backup_path: Some(backup_path.to_string_lossy().to_string()),
    })
}

/// Skill update check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpdateCheckResult {
    pub skill_name: String,
    pub has_update: bool,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub update_notes: Option<String>,
}

/// Check for skill updates.
#[tauri::command]
pub async fn check_skill_updates(
    skill_names: Vec<String>,
) -> Result<Vec<SkillUpdateCheckResult>, String> {
    Ok(skill_names
        .into_iter()
        .map(|skill_name| SkillUpdateCheckResult {
            skill_name,
            has_update: false,
            current_version: None,
            latest_version: None,
            update_notes: Some("MCP Market 技能包未暴露版本元数据".to_string()),
        })
        .collect())
}

/// Skill update input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpdateInput {
    pub skill_path: String,
    pub skill_id: String,
    pub skill_name: String,
}

fn infer_cli_type_from_skill_path(skill_path: &Path) -> Result<String, String> {
    let path = skill_path.to_string_lossy();
    if path.contains("/.claude/skills/") {
        Ok("claude".to_string())
    } else if path.contains("/.codex/skills/") {
        Ok("codex".to_string())
    } else {
        Err(format!("Unsupported skill path: {}", path))
    }
}

/// Update a skill to the latest archive from MCP Market.
#[tauri::command]
pub async fn update_skill(
    app: AppHandle,
    input: SkillUpdateInput,
) -> Result<SkillInstallResult, String> {
    let skill_dir = PathBuf::from(&input.skill_path);
    if !skill_dir.exists() {
        return Err(format!("Skill directory not found: {}", input.skill_path));
    }

    let cli_type = infer_cli_type_from_skill_path(&skill_dir)?;
    let result = install_skill_to_cli(
        app,
        SkillInstallInput {
            skill_id: input.skill_id,
            skill_name: input.skill_name,
            cli_type,
            scope: "global".to_string(),
            project_path: None,
            source_market: None,
        },
    )
    .await?;

    Ok(result)
}
