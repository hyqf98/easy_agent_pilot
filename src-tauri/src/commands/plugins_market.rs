use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::commands::git_install::{
    cleanup_checkout, clone_repository, copy_dir_recursive, normalize_lookup_name,
};

/// Plugin component type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum PluginComponentType {
    Skill,
    Mcp,
    Prompt,
    Agent,
    Workflow,
    Other,
}

impl From<&str> for PluginComponentType {
    fn from(s: &str) -> Self {
        match s {
            "skill" => PluginComponentType::Skill,
            "mcp" => PluginComponentType::Mcp,
            "prompt" => PluginComponentType::Prompt,
            "agent" => PluginComponentType::Agent,
            "workflow" => PluginComponentType::Workflow,
            _ => PluginComponentType::Other,
        }
    }
}

impl std::fmt::Display for PluginComponentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginComponentType::Skill => write!(f, "skill"),
            PluginComponentType::Mcp => write!(f, "mcp"),
            PluginComponentType::Prompt => write!(f, "prompt"),
            PluginComponentType::Agent => write!(f, "agent"),
            PluginComponentType::Workflow => write!(f, "workflow"),
            PluginComponentType::Other => write!(f, "other"),
        }
    }
}

/// Plugin component info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginComponent {
    pub name: String,
    pub component_type: String,
    pub description: String,
    pub version: String,
}

/// Plugin version history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVersion {
    pub version: String,
    pub release_notes: String,
    pub released_at: String,
}

/// Plugin configuration option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigOption {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

/// Plugin market item from a marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub source_market: String,
    pub author: String,
    pub component_types: Vec<String>,
    pub tags: Vec<String>,
    pub repository_url: Option<String>,
    pub homepage_url: Option<String>,
    pub downloads: u64,
    pub rating: f64,
    pub created_at: String,
    pub updated_at: String,
}

/// Plugin detail info with full information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketDetail {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub full_description: String,
    pub source_market: String,
    pub author: String,
    pub component_types: Vec<String>,
    pub tags: Vec<String>,
    pub repository_url: Option<String>,
    pub homepage_url: Option<String>,
    pub license: String,
    pub downloads: u64,
    pub rating: f64,
    pub components: Vec<PluginComponent>,
    pub version_history: Vec<PluginVersion>,
    pub config_options: Vec<PluginConfigOption>,
    pub created_at: String,
    pub updated_at: String,
}

/// Plugin market list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketListResponse {
    pub items: Vec<PluginMarketItem>,
    pub total: u64,
}

/// Plugin market query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketQuery {
    pub category: Option<String>,
    pub search: Option<String>,
}

/// Get database connection
fn get_db_connection() -> Result<Connection, String> {
    crate::commands::market_source_support::open_market_db_connection()
}

/// Market source response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MarketSourceResponse {
    plugins: Vec<PluginMarketItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PluginMarketSourceBundle {
    #[serde(default)]
    plugins: Vec<PluginMarketItem>,
    #[serde(default)]
    plugin_details: Vec<PluginMarketDetail>,
    #[serde(default)]
    plugin_files: HashMap<String, Vec<PluginFileContent>>,
}

/// Fetch plugins from all enabled market sources
#[tauri::command]
pub async fn fetch_plugins_market(
    query: PluginMarketQuery,
) -> Result<PluginMarketListResponse, String> {
    let conn = get_db_connection()?;
    let sources = get_enabled_plugin_sources(&conn)?;

    if sources.is_empty() {
        return Ok(PluginMarketListResponse {
            items: Vec::new(),
            total: 0,
        });
    }

    let client = crate::commands::market_source_support::build_market_http_client()?;

    let mut all_items: Vec<PluginMarketItem> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    for source in sources {
        match fetch_plugins_from_source(&client, &source.url_or_path, &source.name).await {
            Ok(items) => {
                for item in items {
                    if !seen_ids.contains(&item.id) {
                        seen_ids.insert(item.id.clone());
                        all_items.push(item);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch from source {}: {}", source.name, e);
            }
        }
    }

    let mut filtered_items = all_items;

    // Filter by category (component type)
    if let Some(category) = &query.category {
        if !category.is_empty() && category != "all" {
            filtered_items.retain(|item| item.component_types.contains(category));
        }
    }

    // Filter by search
    if let Some(search) = &query.search {
        if !search.is_empty() {
            let search_lower = search.to_lowercase();
            filtered_items.retain(|item| {
                item.name.to_lowercase().contains(&search_lower)
                    || item.description.to_lowercase().contains(&search_lower)
                    || item.author.to_lowercase().contains(&search_lower)
                    || item
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&search_lower))
            });
        }
    }

    let total = filtered_items.len() as u64;

    Ok(PluginMarketListResponse {
        items: filtered_items,
        total,
    })
}

/// Fetch plugin detail by ID
#[tauri::command]
pub async fn fetch_plugin_detail(plugin_id: String) -> Result<PluginMarketDetail, String> {
    let conn = get_db_connection()?;
    let sources = get_enabled_plugin_sources(&conn)?;
    if sources.is_empty() {
        return Err("No plugin market source configured".to_string());
    }

    let client = crate::commands::market_source_support::build_market_http_client()?;

    for source in sources {
        let payload = match crate::commands::market_source_support::read_market_source_payload(
            &client,
            &source.url_or_path,
            "plugins.json",
        )
        .await
        {
            Ok(payload) => payload,
            Err(error) => {
                eprintln!("Failed to read plugin source {}: {}", source.name, error);
                continue;
            }
        };

        let bundle = match parse_plugin_market_payload(&payload, &source.name) {
            Ok(bundle) => bundle,
            Err(error) => {
                eprintln!("Failed to parse plugin source {}: {}", source.name, error);
                continue;
            }
        };

        if let Some(detail) = bundle
            .plugin_details
            .into_iter()
            .find(|item| item.id == plugin_id)
        {
            return Ok(detail);
        }

        if let Some(item) = bundle.plugins.into_iter().find(|item| item.id == plugin_id) {
            return Ok(synthesize_plugin_detail(item));
        }
    }

    Err(format!(
        "Plugin not found in configured sources: {}",
        plugin_id
    ))
}

/// Plugin source from database
struct PluginSource {
    name: String,
    url_or_path: String,
}

/// Get enabled plugin sources from database
fn get_enabled_plugin_sources(conn: &Connection) -> Result<Vec<PluginSource>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT name, url_or_path FROM market_sources WHERE enabled = 1 ORDER BY created_at DESC"
        )
        .map_err(|e| e.to_string())?;

    let sources = stmt
        .query_map([], |row| {
            Ok(PluginSource {
                name: row.get(0)?,
                url_or_path: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(sources)
}

/// Fetch plugins from a single source URL
async fn fetch_plugins_from_source(
    client: &reqwest::Client,
    url: &str,
    source_name: &str,
) -> Result<Vec<PluginMarketItem>, String> {
    let text = crate::commands::market_source_support::read_market_source_payload(
        client,
        url,
        "plugins.json",
    )
    .await?;
    let bundle = parse_plugin_market_payload(&text, source_name)?;
    Ok(bundle.plugins)
}

fn parse_plugin_market_payload(
    text: &str,
    source_name: &str,
) -> Result<PluginMarketSourceBundle, String> {
    if let Ok(mut bundle) = serde_json::from_str::<PluginMarketSourceBundle>(text) {
        for item in &mut bundle.plugins {
            item.source_market = source_name.to_string();
        }
        for detail in &mut bundle.plugin_details {
            detail.source_market = source_name.to_string();
        }
        return Ok(bundle);
    }

    if let Ok(market_response) = serde_json::from_str::<MarketSourceResponse>(text) {
        let plugins = market_response
            .plugins
            .into_iter()
            .map(|mut item| {
                item.source_market = source_name.to_string();
                item
            })
            .collect();

        return Ok(PluginMarketSourceBundle {
            plugins,
            ..PluginMarketSourceBundle::default()
        });
    }

    if let Ok(items) = serde_json::from_str::<Vec<PluginMarketItem>>(text) {
        let plugins = items
            .into_iter()
            .map(|mut item| {
                item.source_market = source_name.to_string();
                item
            })
            .collect();

        return Ok(PluginMarketSourceBundle {
            plugins,
            ..PluginMarketSourceBundle::default()
        });
    }

    Err("Failed to parse plugin market payload".to_string())
}

fn synthesize_plugin_detail(item: PluginMarketItem) -> PluginMarketDetail {
    PluginMarketDetail {
        id: item.id,
        name: item.name,
        version: item.version,
        description: item.description.clone(),
        full_description: item.description,
        source_market: item.source_market,
        author: item.author,
        component_types: item.component_types,
        tags: item.tags,
        repository_url: item.repository_url,
        homepage_url: item.homepage_url,
        license: "Unknown".to_string(),
        downloads: item.downloads,
        rating: item.rating,
        components: Vec::new(),
        version_history: Vec::new(),
        config_options: Vec::new(),
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

async fn load_plugin_install_payload(
    plugin_id: &str,
) -> Result<(PluginMarketDetail, Vec<PluginFileContent>), String> {
    let conn = get_db_connection()?;
    let sources = get_enabled_plugin_sources(&conn)?;
    if sources.is_empty() {
        return Err("No plugin market source configured".to_string());
    }

    let client = crate::commands::market_source_support::build_market_http_client()?;

    for source in sources {
        let payload = match crate::commands::market_source_support::read_market_source_payload(
            &client,
            &source.url_or_path,
            "plugins.json",
        )
        .await
        {
            Ok(payload) => payload,
            Err(error) => {
                eprintln!("Failed to read plugin source {}: {}", source.name, error);
                continue;
            }
        };

        let bundle = match parse_plugin_market_payload(&payload, &source.name) {
            Ok(bundle) => bundle,
            Err(error) => {
                eprintln!("Failed to parse plugin source {}: {}", source.name, error);
                continue;
            }
        };

        let plugin_files = bundle.plugin_files.get(plugin_id).cloned();
        let detail = bundle
            .plugin_details
            .into_iter()
            .find(|item| item.id == plugin_id)
            .or_else(|| {
                bundle
                    .plugins
                    .into_iter()
                    .find(|item| item.id == plugin_id)
                    .map(synthesize_plugin_detail)
            });

        if let Some(detail) = detail {
            let files = plugin_files.ok_or_else(|| {
                format!(
                    "Plugin source '{}' does not provide installable files for {}",
                    source.name, plugin_id
                )
            })?;
            return Ok((detail, files));
        }
    }

    Err(format!(
        "Plugin {} was not found in configured plugin market sources",
        plugin_id
    ))
}

fn plugin_manifest_path(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join(".claude-plugin").join("plugin.json")
}

fn parse_plugin_directory_metadata(
    plugin_dir: &Path,
) -> Result<(Option<String>, Option<String>, Option<String>), String> {
    let manifest_path = plugin_manifest_path(plugin_dir);
    if !manifest_path.exists() {
        return Ok((None, None, None));
    }

    let content = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("Failed to read {}: {}", manifest_path.display(), error))?;
    let json = serde_json::from_str::<serde_json::Value>(&content)
        .map_err(|error| format!("Failed to parse plugin.json: {}", error))?;

    let name = json
        .get("name")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let version = json
        .get("version")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let description = json
        .get("description")
        .and_then(|value| value.as_str())
        .map(str::to_string);

    Ok((name, version, description))
}

fn has_plugin_manifest(plugin_dir: &Path) -> bool {
    plugin_manifest_path(plugin_dir).exists()
}

fn collect_plugin_candidate_dirs(
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

        if has_plugin_manifest(&path) {
            candidates.push(path.clone());
        }

        collect_plugin_candidate_dirs(&path, depth + 1, candidates)?;
    }

    Ok(())
}

fn find_matching_plugin_directory(repo_dir: &Path, plugin_name: &str) -> Result<PathBuf, String> {
    let expected_name = normalize_lookup_name(plugin_name);
    if expected_name.is_empty() {
        return Err("Plugin 名称不能为空".to_string());
    }

    let mut candidates = Vec::new();
    collect_plugin_candidate_dirs(repo_dir, 0, &mut candidates)?;

    let mut matched_paths = candidates
        .into_iter()
        .filter(|candidate| {
            let dir_name = candidate
                .file_name()
                .and_then(|name| name.to_str())
                .map(normalize_lookup_name)
                .unwrap_or_default();
            let manifest_name = parse_plugin_directory_metadata(candidate)
                .ok()
                .and_then(|(name, _, _)| name)
                .map(|name| normalize_lookup_name(&name))
                .unwrap_or_default();

            dir_name == expected_name || manifest_name == expected_name
        })
        .collect::<Vec<_>>();

    matched_paths.sort();
    matched_paths.dedup();

    match matched_paths.len() {
        0 => Err(format!("仓库中未找到名为 '{}' 的 Plugin", plugin_name)),
        1 => Ok(matched_paths.remove(0)),
        _ => Err(format!("仓库中存在多个同名 Plugin '{}'", plugin_name)),
    }
}

fn get_cli_plugins_dir(cli_type: &str) -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    match cli_type.to_lowercase().as_str() {
        "claude" => Ok(home_dir.join(".claude").join("plugins")),
        "codex" => Ok(home_dir.join(".codex").join("plugins")),
        "opencode" => Ok(home_dir.join(".config").join("opencode").join("plugins")),
        other => Err(format!("Unsupported CLI type for plugins: {}", other)),
    }
}

fn build_plugin_backup_path(plugin_dir: &Path) -> PathBuf {
    let backup_name = format!(
        "{}.backup",
        plugin_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("plugin")
    );
    plugin_dir.with_file_name(backup_name)
}

fn move_dir_with_overwrite(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        fs::remove_dir_all(target)
            .map_err(|error| format!("Failed to clear target dir: {}", error))?;
    }

    fs::rename(source, target)
        .map_err(|error| format!("Failed to move plugin directory: {}", error))
}

fn create_plugin_backup(plugin_dir: &Path) -> Result<Option<String>, String> {
    if !plugin_dir.exists() {
        return Ok(None);
    }

    let backup_path = build_plugin_backup_path(plugin_dir);
    move_dir_with_overwrite(plugin_dir, &backup_path)?;
    Ok(Some(backup_path.to_string_lossy().to_string()))
}

fn restore_plugin_backup(plugin_dir: &Path, backup_path: &Option<String>) -> Result<(), String> {
    if plugin_dir.exists() {
        fs::remove_dir_all(plugin_dir)
            .map_err(|error| format!("Failed to clear plugin dir: {}", error))?;
    }

    if let Some(backup_path) = backup_path {
        let backup_path = PathBuf::from(backup_path);
        if backup_path.exists() {
            fs::rename(&backup_path, plugin_dir)
                .map_err(|error| format!("Failed to restore plugin backup: {}", error))?;
        }
    }

    Ok(())
}

fn finalize_plugin_backup(backup_path: &Option<String>) {
    if let Some(backup_path) = backup_path {
        let backup_path = PathBuf::from(backup_path);
        if backup_path.exists() {
            let _ = fs::remove_dir_all(backup_path);
        }
    }
}

fn slugify_plugin_identifier(value: &str) -> String {
    let normalized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    normalized
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ============== Plugin Installation Types ==============

/// Plugin install input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstallInput {
    pub plugin_id: String,
    pub plugin_name: String,
    pub plugin_version: String,
    pub cli_path: String,
    pub scope: String, // "global" or "project"
    pub project_path: Option<String>,
    pub selected_components: Vec<String>, // component names to install
    pub config_values: std::collections::HashMap<String, String>, // config option values
}

/// Git plugin install input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitPluginInstallInput {
    pub repository_url: String,
    pub git_ref: Option<String>,
    pub plugin_name: String,
    pub cli_type: String,
    pub cli_path: String,
}

/// Plugin install result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstallResult {
    pub success: bool,
    pub message: String,
    pub plugin_id: String,
    pub installed_components: Vec<InstalledPluginComponent>,
    pub backup_path: Option<String>,
    pub plugins_json_path: Option<String>,
}

/// Installed plugin component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPluginComponent {
    pub name: String,
    pub component_type: String,
    pub target_path: String,
}

/// Installed plugin info (from plugins.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source_market: String,
    pub cli_path: String,
    pub scope: String,
    pub components: Vec<InstalledPluginComponent>,
    pub enabled: bool,
    pub installed_at: String,
    pub config_values: std::collections::HashMap<String, String>,
}

/// Plugin file content for download
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginFileContent {
    pub relative_path: String, // e.g., "skills/example-skill.md"
    pub content: String,
}

// ============== Plugin Installation Commands ==============

/// Get CLI config directory path
fn get_cli_config_dir(
    cli_path: &str,
    scope: &str,
    project_path: Option<&str>,
) -> Result<PathBuf, String> {
    if scope == "project" {
        if let Some(path) = project_path {
            let project_dir = PathBuf::from(path);
            return Ok(project_dir.join(".claude"));
        }
        return Err("Project path is required for project scope".to_string());
    }

    // Global scope - use home directory
    let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    // Determine CLI type from path
    let cli_name = PathBuf::from(cli_path)
        .file_stem()
        .map(|s: &std::ffi::OsStr| s.to_string_lossy().to_string())
        .unwrap_or_default();

    match cli_name.to_lowercase().as_str() {
        "claude" => Ok(home.join(".claude")),
        _ => Ok(home.join(".claude")), // Default to claude
    }
}

/// Get plugins.json path
fn get_plugins_json_path() -> Result<PathBuf, String> {
    let persistence_dir = crate::commands::get_persistence_dir_path().map_err(|e| e.to_string())?;
    Ok(persistence_dir.join("plugins.json"))
}

/// Load installed plugins from plugins.json
fn load_installed_plugins() -> Result<Vec<InstalledPlugin>, String> {
    let plugins_json_path = get_plugins_json_path()?;

    if !plugins_json_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&plugins_json_path)
        .map_err(|e| format!("Failed to read plugins.json: {}", e))?;

    let plugins: Vec<InstalledPlugin> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse plugins.json: {}", e))?;

    Ok(plugins)
}

/// Save installed plugins to plugins.json
fn save_installed_plugins(plugins: &[InstalledPlugin]) -> Result<PathBuf, String> {
    let plugins_json_path = get_plugins_json_path()?;

    // Ensure parent directory exists
    if let Some(parent) = plugins_json_path.parent() {
        fs::create_dir_all(parent).map_err(|e: std::io::Error| e.to_string())?;
    }

    let content = serde_json::to_string_pretty(plugins)
        .map_err(|e| format!("Failed to serialize plugins: {}", e))?;

    fs::write(&plugins_json_path, content)
        .map_err(|e| format!("Failed to write plugins.json: {}", e))?;

    Ok(plugins_json_path)
}

fn install_plugin_directory_to_cli(
    source_dir: &Path,
    input: &GitPluginInstallInput,
) -> Result<PluginInstallResult, String> {
    let plugins_dir = get_cli_plugins_dir(&input.cli_type)?;
    fs::create_dir_all(&plugins_dir)
        .map_err(|error| format!("Failed to create plugins dir: {}", error))?;

    let target_name = source_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| input.plugin_name.clone());
    let target_dir = plugins_dir.join(&target_name);
    let backup_path = create_plugin_backup(&target_dir)?;

    if let Err(error) = copy_dir_recursive(source_dir, &target_dir) {
        restore_plugin_backup(&target_dir, &backup_path)?;
        return Ok(PluginInstallResult {
            success: false,
            message: error,
            plugin_id: String::new(),
            installed_components: Vec::new(),
            backup_path: None,
            plugins_json_path: None,
        });
    }

    if !has_plugin_manifest(&target_dir) {
        restore_plugin_backup(&target_dir, &backup_path)?;
        return Ok(PluginInstallResult {
            success: false,
            message: "仓库中的插件目录未包含 .claude-plugin/plugin.json".to_string(),
            plugin_id: String::new(),
            installed_components: Vec::new(),
            backup_path: None,
            plugins_json_path: None,
        });
    }

    let (display_name, version, description) = parse_plugin_directory_metadata(&target_dir)?;
    let installed_name = display_name.unwrap_or_else(|| input.plugin_name.clone());
    let installed_components = vec![InstalledPluginComponent {
        name: installed_name.clone(),
        component_type: "plugin_directory".to_string(),
        target_path: target_dir.to_string_lossy().to_string(),
    }];

    let plugin_id = format!(
        "git-{}-{}",
        input.cli_type,
        slugify_plugin_identifier(&format!("{}-{}", input.repository_url, input.plugin_name))
    );
    let now = chrono::Utc::now().to_rfc3339();
    let mut plugins = load_installed_plugins()?;

    if let Some(existing) = plugins.iter_mut().find(|plugin| plugin.id == plugin_id) {
        existing.name = installed_name.clone();
        existing.version = version.clone().unwrap_or_else(|| "unknown".to_string());
        existing.source_market = "git".to_string();
        existing.cli_path = input.cli_path.clone();
        existing.scope = "global".to_string();
        existing.components = installed_components.clone();
        existing.enabled = true;
        existing.installed_at = now.clone();
        existing.config_values = HashMap::from([
            ("repository_url".to_string(), input.repository_url.clone()),
            (
                "git_ref".to_string(),
                input.git_ref.clone().unwrap_or_default(),
            ),
            ("description".to_string(), description.unwrap_or_default()),
        ]);
    } else {
        plugins.push(InstalledPlugin {
            id: plugin_id.clone(),
            name: installed_name.clone(),
            version: version.unwrap_or_else(|| "unknown".to_string()),
            source_market: "git".to_string(),
            cli_path: input.cli_path.clone(),
            scope: "global".to_string(),
            components: installed_components.clone(),
            enabled: true,
            installed_at: now,
            config_values: HashMap::from([
                ("repository_url".to_string(), input.repository_url.clone()),
                (
                    "git_ref".to_string(),
                    input.git_ref.clone().unwrap_or_default(),
                ),
                ("description".to_string(), description.unwrap_or_default()),
            ]),
        });
    }

    let plugins_json_path = save_installed_plugins(&plugins)?;
    finalize_plugin_backup(&backup_path);

    Ok(PluginInstallResult {
        success: true,
        message: format!("Plugin '{}' installed successfully", installed_name),
        plugin_id,
        installed_components,
        backup_path,
        plugins_json_path: Some(plugins_json_path.to_string_lossy().to_string()),
    })
}

/// Install plugin to CLI
#[tauri::command]
pub async fn install_plugin(input: PluginInstallInput) -> Result<PluginInstallResult, String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let backup_base = crate::commands::install::get_backup_base_dir().map_err(|e| e.to_string())?;
    let backup_dir = backup_base.join(&session_id);
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    let (plugin_detail, plugin_files) = load_plugin_install_payload(&input.plugin_id).await?;

    let config_dir =
        get_cli_config_dir(&input.cli_path, &input.scope, input.project_path.as_deref())?;
    fs::create_dir_all(&config_dir).map_err(|e| format!("Failed to create config dir: {}", e))?;

    let files_to_install: Vec<_> = plugin_files
        .into_iter()
        .filter(|f| {
            let file_name = PathBuf::from(&f.relative_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            input.selected_components.contains(&file_name) || input.selected_components.is_empty()
        })
        .collect();

    if files_to_install.is_empty() {
        return Ok(PluginInstallResult {
            success: false,
            message: "No components selected for installation".to_string(),
            plugin_id: input.plugin_id,
            installed_components: vec![],
            backup_path: None,
            plugins_json_path: None,
        });
    }

    let mut installed_components: Vec<InstalledPluginComponent> = Vec::new();
    let mut operations: Vec<crate::commands::install::InstallOperation> = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    // Install each file
    for file in &files_to_install {
        let relative_path = &file.relative_path;
        let target_path = config_dir.join(relative_path);

        // Create parent directories
        if let Some(parent) = target_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
            }
        }

        // Backup existing file if it exists
        let backup_path = if target_path.exists() {
            let backup_file = backup_dir.join(format!(
                "{}_{}",
                uuid::Uuid::new_v4(),
                relative_path.replace('/', "_")
            ));
            fs::copy(&target_path, &backup_file).map_err(|e| format!("Failed to backup: {}", e))?;
            Some(backup_file.to_string_lossy().to_string())
        } else {
            None
        };

        // Write file
        fs::write(&target_path, &file.content)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        // Determine component type from path
        let component_type = relative_path
            .split('/')
            .next()
            .unwrap_or("other")
            .to_string();
        let component_name = PathBuf::from(relative_path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        installed_components.push(InstalledPluginComponent {
            name: component_name,
            component_type,
            target_path: target_path.to_string_lossy().to_string(),
        });

        operations.push(crate::commands::install::InstallOperation {
            sequence: operations.len() as u32 + 1,
            operation_type: if backup_path.is_some() {
                crate::commands::install::InstallOperationType::ModifyFile
            } else {
                crate::commands::install::InstallOperationType::CreateFile
            },
            target_path: target_path.to_string_lossy().to_string(),
            backup_path,
            timestamp: now.clone(),
        });
    }

    // Handle MCP components - add to settings.json
    let has_mcp = installed_components
        .iter()
        .any(|c| c.component_type == "mcp");
    let _settings_backup_path = if has_mcp {
        let settings_path = config_dir.join("settings.json");
        if settings_path.exists() {
            let backup_file = backup_dir.join(format!("{}_settings.json", uuid::Uuid::new_v4()));
            fs::copy(&settings_path, &backup_file)
                .map_err(|e| format!("Failed to backup settings: {}", e))?;
            Some(backup_file.to_string_lossy().to_string())
        } else {
            None
        }
    } else {
        None
    };

    // Update plugins.json
    let mut plugins = load_installed_plugins()?;

    // Check if already installed
    if plugins.iter().any(|p| p.id == input.plugin_id) {
        // Update existing
        if let Some(plugin) = plugins.iter_mut().find(|p| p.id == input.plugin_id) {
            plugin.version = input.plugin_version.clone();
            plugin.source_market = plugin_detail.source_market.clone();
            plugin.components = installed_components.clone();
            plugin.enabled = true;
        }
    } else {
        plugins.push(InstalledPlugin {
            id: input.plugin_id.clone(),
            name: input.plugin_name.clone(),
            version: input.plugin_version.clone(),
            source_market: plugin_detail.source_market,
            cli_path: input.cli_path.clone(),
            scope: input.scope.clone(),
            components: installed_components.clone(),
            enabled: true,
            installed_at: now.clone(),
            config_values: input.config_values.clone(),
        });
    }

    let plugins_json_path = save_installed_plugins(&plugins)?;

    Ok(PluginInstallResult {
        success: true,
        message: format!(
            "Successfully installed {} components",
            installed_components.len()
        ),
        plugin_id: input.plugin_id,
        installed_components,
        backup_path: Some(backup_dir.to_string_lossy().to_string()),
        plugins_json_path: Some(plugins_json_path.to_string_lossy().to_string()),
    })
}

/// Install a plugin from a Git repository into the target CLI native plugins directory.
#[tauri::command]
pub async fn install_plugin_from_git(
    input: GitPluginInstallInput,
) -> Result<PluginInstallResult, String> {
    let checkout = clone_repository(&input.repository_url, input.git_ref.as_deref())?;
    let result = (|| {
        let source_dir = find_matching_plugin_directory(&checkout.repo_dir, &input.plugin_name)?;
        install_plugin_directory_to_cli(&source_dir, &input)
    })();
    cleanup_checkout(&checkout);
    result
}

/// List installed plugins
#[tauri::command]
pub fn list_installed_plugins() -> Result<Vec<InstalledPlugin>, String> {
    load_installed_plugins()
}

/// Toggle plugin enable/disable
#[tauri::command]
pub fn toggle_plugin(plugin_id: String, enabled: bool) -> Result<InstalledPlugin, String> {
    let mut plugins = load_installed_plugins()?;

    let plugin = plugins
        .iter_mut()
        .find(|p| p.id == plugin_id)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

    plugin.enabled = enabled;

    // Disable/enable component files by renaming
    for component in &plugin.components {
        let path = PathBuf::from(&component.target_path);
        if path.is_dir() {
            let disabled_marker = path.join(".disabled");
            if enabled {
                if disabled_marker.exists() {
                    fs::remove_file(&disabled_marker).map_err(|e: std::io::Error| e.to_string())?;
                }
            } else if !disabled_marker.exists() {
                fs::write(&disabled_marker, b"disabled")
                    .map_err(|e: std::io::Error| e.to_string())?;
            }
            continue;
        }

        if component.component_type == "mcp" {
            // MCP components are toggled in settings.json - skip for now
            continue;
        }

        let disabled_path = PathBuf::from(format!("{}.disabled", component.target_path));

        if enabled {
            // Enable: rename .disabled to original
            if disabled_path.exists() {
                fs::rename(&disabled_path, &path).map_err(|e: std::io::Error| e.to_string())?;
            }
        } else {
            // Disable: rename to .disabled
            if path.exists() {
                fs::rename(&path, &disabled_path).map_err(|e: std::io::Error| e.to_string())?;
            }
        }
    }

    save_installed_plugins(&plugins)?;

    // Clone before returning to avoid borrow issues
    let plugin = plugins
        .iter()
        .find(|p| p.id == plugin_id)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?
        .clone();

    Ok(plugin)
}

/// Uninstall plugin
#[tauri::command]
pub fn uninstall_plugin(plugin_id: String) -> Result<PluginInstallResult, String> {
    let mut plugins = load_installed_plugins()?;

    let plugin_idx = plugins
        .iter()
        .position(|p| p.id == plugin_id)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

    let plugin = plugins.remove(plugin_idx);

    // Remove component files
    for component in &plugin.components {
        let path = PathBuf::from(&component.target_path);
        if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| format!("Failed to remove plugin dir: {}", e))?;
            continue;
        }

        let disabled_path = PathBuf::from(format!("{}.disabled", component.target_path));

        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Failed to remove file: {}", e))?;
        }
        if disabled_path.exists() {
            fs::remove_file(&disabled_path)
                .map_err(|e| format!("Failed to remove disabled file: {}", e))?;
        }
    }

    save_installed_plugins(&plugins)?;

    Ok(PluginInstallResult {
        success: true,
        message: format!("Plugin {} uninstalled successfully", plugin.name),
        plugin_id: plugin_id.clone(),
        installed_components: plugin.components,
        backup_path: None,
        plugins_json_path: Some(get_plugins_json_path()?.to_string_lossy().to_string()),
    })
}
