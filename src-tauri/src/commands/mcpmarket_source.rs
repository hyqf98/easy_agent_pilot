use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::process::Command;

use crate::commands::cli_support::find_cli_executable;

pub const DEFAULT_MARKETPLACE_SOURCE_ID: &str = "mcpmarket";
const MCPMARKET_SOURCE_LABEL: &str = "MCP Market";
const MODELSCOPE_SOURCE_ID: &str = "modelscope";
const MODELSCOPE_SOURCE_LABEL: &str = "ModelScope MCP";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketplaceResourceType {
    Mcp,
    Skills,
    Plugins,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSourceOption {
    pub id: String,
    pub label: String,
    pub supported_resources: Vec<MarketplaceResourceType>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MarketplaceSourceQuery {
    pub page: Option<u32>,
    pub category: Option<String>,
    pub category_slug: Option<String>,
    pub search: Option<String>,
    pub source_market: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketCategory {
    pub label: String,
    pub value: String,
    pub slug: Option<String>,
    pub count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketListResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub has_more: bool,
    pub categories: Vec<MarketCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSourceListItem {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub transport_type: String,
    pub install_command: Option<String>,
    pub install_args: Vec<String>,
    pub env_template: HashMap<String, String>,
    pub logo: Option<String>,
    pub downloads: Option<u64>,
    pub rating: Option<f64>,
    pub repository_url: Option<String>,
    pub source_market: String,
    pub stars: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSourceDetail {
    #[serde(flatten)]
    pub item: McpSourceListItem,
    pub full_description: String,
    pub readme_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSourceListItem {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub author: String,
    pub category: String,
    pub category_slug: Option<String>,
    pub tags: Vec<String>,
    pub trigger_scenario: String,
    pub source_market: String,
    pub repository_url: Option<String>,
    pub downloads: Option<u64>,
    pub rating: Option<f64>,
    pub stars: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSourceDetail {
    #[serde(flatten)]
    pub item: SkillSourceListItem,
    pub full_description: String,
    pub skill_content: Option<String>,
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillArchivePayload {
    pub slug: String,
    pub download_url: String,
    pub file_name: String,
    pub zip_base64: String,
}

#[async_trait]
pub trait MarketplaceSourceStrategy: Send + Sync {
    async fn fetch_mcp_list(
        &self,
        app: &AppHandle,
        query: &MarketplaceSourceQuery,
    ) -> Result<MarketListResponse<McpSourceListItem>, String>;

    async fn fetch_mcp_detail(
        &self,
        app: &AppHandle,
        slug: &str,
    ) -> Result<McpSourceDetail, String>;

    async fn fetch_skill_list(
        &self,
        app: &AppHandle,
        query: &MarketplaceSourceQuery,
    ) -> Result<MarketListResponse<SkillSourceListItem>, String>;

    async fn fetch_skill_detail(
        &self,
        app: &AppHandle,
        slug: &str,
    ) -> Result<SkillSourceDetail, String>;

    async fn download_skill_archive(
        &self,
        app: &AppHandle,
        slug: &str,
    ) -> Result<SkillArchivePayload, String>;
}

fn build_source_options() -> Vec<MarketplaceSourceOption> {
    vec![
        MarketplaceSourceOption {
            id: DEFAULT_MARKETPLACE_SOURCE_ID.to_string(),
            label: MCPMARKET_SOURCE_LABEL.to_string(),
            supported_resources: vec![
                MarketplaceResourceType::Mcp,
                MarketplaceResourceType::Skills,
            ],
        },
        MarketplaceSourceOption {
            id: MODELSCOPE_SOURCE_ID.to_string(),
            label: MODELSCOPE_SOURCE_LABEL.to_string(),
            supported_resources: vec![MarketplaceResourceType::Mcp],
        },
    ]
}

#[tauri::command]
pub fn list_marketplace_source_options() -> Vec<MarketplaceSourceOption> {
    build_source_options()
}

pub fn get_marketplace_source_strategy(
    source_market: Option<&str>,
) -> Result<Box<dyn MarketplaceSourceStrategy>, String> {
    let source = source_market.unwrap_or(DEFAULT_MARKETPLACE_SOURCE_ID);
    match source {
        DEFAULT_MARKETPLACE_SOURCE_ID => Ok(Box::new(McpMarketSourceStrategy)),
        MODELSCOPE_SOURCE_ID => Ok(Box::new(ModelScopeSourceStrategy)),
        other => Err(format!("Unsupported marketplace source: {}", other)),
    }
}

fn push_if_exists(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if path.exists() {
        candidates.push(path);
    }
}

fn resolve_helper_script_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();

    if let Ok(current_dir) = env::current_dir() {
        push_if_exists(
            &mut candidates,
            current_dir.join("scripts").join("mcpmarket-fetcher.mjs"),
        );
        push_if_exists(
            &mut candidates,
            current_dir
                .join("..")
                .join("scripts")
                .join("mcpmarket-fetcher.mjs"),
        );
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        push_if_exists(
            &mut candidates,
            resource_dir.join("scripts").join("mcpmarket-fetcher.mjs"),
        );
        push_if_exists(&mut candidates, resource_dir.join("mcpmarket-fetcher.mjs"));
    }

    candidates
        .into_iter()
        .next()
        .ok_or_else(|| "Cannot locate marketplace helper script".to_string())
}

fn find_playwright_core_in_node_modules(base_dir: &Path) -> Option<PathBuf> {
    let direct = base_dir
        .join("node_modules")
        .join("playwright-core")
        .join("index.mjs");
    if direct.exists() {
        return Some(direct);
    }

    let pnpm_root = base_dir.join("node_modules").join(".pnpm");
    let entries = std::fs::read_dir(&pnpm_root).ok()?;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if !file_name.starts_with("playwright-core@") {
            continue;
        }

        let candidate = entry
            .path()
            .join("node_modules")
            .join("playwright-core")
            .join("index.mjs");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn resolve_playwright_core_path(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(current_dir) = env::current_dir() {
        if let Some(path) = find_playwright_core_in_node_modules(&current_dir) {
            return Some(path);
        }

        let parent_dir = current_dir.join("..");
        if let Some(path) = find_playwright_core_in_node_modules(&parent_dir) {
            return Some(path);
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("playwright-core").join("index.mjs");
        if bundled.exists() {
            return Some(bundled);
        }
    }

    None
}

fn resolve_node_binary() -> String {
    if let Some(explicit) = env::var_os("EASY_AGENT_NODE_PATH") {
        let explicit = PathBuf::from(explicit);
        if explicit.exists() {
            return explicit.to_string_lossy().to_string();
        }
    }

    let mut extra_dirs = Vec::new();

    #[cfg(target_os = "macos")]
    {
        extra_dirs.push(PathBuf::from("/opt/homebrew/bin"));
        extra_dirs.push(PathBuf::from("/usr/local/bin"));
        extra_dirs.push(PathBuf::from("/usr/bin"));
        if let Some(home_dir) = dirs::home_dir() {
            extra_dirs.push(home_dir.join(".volta").join("bin"));
            extra_dirs.push(home_dir.join(".nvm").join("current").join("bin"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        extra_dirs.push(PathBuf::from("/usr/local/bin"));
        extra_dirs.push(PathBuf::from("/usr/bin"));
        if let Some(home_dir) = dirs::home_dir() {
            extra_dirs.push(home_dir.join(".local").join("bin"));
            extra_dirs.push(home_dir.join(".volta").join("bin"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(program_files) = env::var_os("ProgramFiles") {
            extra_dirs.push(PathBuf::from(program_files).join("nodejs"));
        }
        if let Some(program_files_x86) = env::var_os("ProgramFiles(x86)") {
            extra_dirs.push(PathBuf::from(program_files_x86).join("nodejs"));
        }
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            extra_dirs.push(
                PathBuf::from(local_app_data)
                    .join("Programs")
                    .join("nodejs"),
            );
        }
    }

    find_cli_executable("node", &extra_dirs)
        .unwrap_or_else(|| PathBuf::from("node"))
        .to_string_lossy()
        .to_string()
}

async fn run_marketplace_helper<T>(
    app: &AppHandle,
    mode: &str,
    payload: serde_json::Value,
) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let script_path = resolve_helper_script_path(app)?;
    let node_binary = resolve_node_binary();
    let payload_json = serde_json::to_string(&payload)
        .map_err(|e| format!("Failed to serialize helper payload: {}", e))?;

    let mut command = Command::new(&node_binary);
    command.arg(script_path).arg(mode).arg(payload_json);

    if let Some(playwright_core_path) = resolve_playwright_core_path(app) {
        command.env("EASY_AGENT_PLAYWRIGHT_CORE", playwright_core_path);
    }

    let output = command
        .output()
        .await
        .map_err(|e| format!("Failed to start marketplace helper: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Marketplace helper failed".to_string()
        } else {
            stderr
        });
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to decode marketplace helper output: {}", e))?;
    serde_json::from_str::<T>(stdout.trim())
        .map_err(|e| format!("Failed to parse marketplace helper output: {}", e))
}

pub struct McpMarketSourceStrategy;

#[async_trait]
impl MarketplaceSourceStrategy for McpMarketSourceStrategy {
    async fn fetch_mcp_list(
        &self,
        app: &AppHandle,
        query: &MarketplaceSourceQuery,
    ) -> Result<MarketListResponse<McpSourceListItem>, String> {
        let page = query.page.unwrap_or(1).max(1);
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let category_slug = query
            .category_slug
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let category = query
            .category
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        run_marketplace_helper(
            app,
            "mcp-list",
            serde_json::json!({
                "page": page,
                "search": search,
                "category_slug": if search.is_some() { None::<String> } else { category_slug.map(ToOwned::to_owned) },
                "category": if search.is_some() { None::<String> } else { category.map(ToOwned::to_owned) },
            }),
        )
        .await
    }

    async fn fetch_mcp_detail(
        &self,
        app: &AppHandle,
        slug: &str,
    ) -> Result<McpSourceDetail, String> {
        run_marketplace_helper(app, "mcp-detail", serde_json::json!({ "slug": slug })).await
    }

    async fn fetch_skill_list(
        &self,
        app: &AppHandle,
        query: &MarketplaceSourceQuery,
    ) -> Result<MarketListResponse<SkillSourceListItem>, String> {
        let page = query.page.unwrap_or(1).max(1);
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let category_slug = query
            .category_slug
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let category = query
            .category
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        run_marketplace_helper(
            app,
            "skill-list",
            serde_json::json!({
                "page": page,
                "search": search,
                "category_slug": if search.is_some() { None::<String> } else { category_slug.map(ToOwned::to_owned) },
                "category": if search.is_some() { None::<String> } else { category.map(ToOwned::to_owned) },
            }),
        )
        .await
    }

    async fn fetch_skill_detail(
        &self,
        app: &AppHandle,
        slug: &str,
    ) -> Result<SkillSourceDetail, String> {
        run_marketplace_helper(app, "skill-detail", serde_json::json!({ "slug": slug })).await
    }

    async fn download_skill_archive(
        &self,
        app: &AppHandle,
        slug: &str,
    ) -> Result<SkillArchivePayload, String> {
        run_marketplace_helper(app, "skill-archive", serde_json::json!({ "slug": slug })).await
    }
}

pub struct ModelScopeSourceStrategy;

#[async_trait]
impl MarketplaceSourceStrategy for ModelScopeSourceStrategy {
    async fn fetch_mcp_list(
        &self,
        app: &AppHandle,
        query: &MarketplaceSourceQuery,
    ) -> Result<MarketListResponse<McpSourceListItem>, String> {
        let page = query.page.unwrap_or(1).max(1);
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let category_slug = query
            .category_slug
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        run_marketplace_helper(
            app,
            "modelscope-mcp-list",
            serde_json::json!({
                "page": page,
                "search": search,
                "category_slug": if search.is_some() { None::<String> } else { category_slug.map(ToOwned::to_owned) },
            }),
        )
        .await
    }

    async fn fetch_mcp_detail(
        &self,
        app: &AppHandle,
        slug: &str,
    ) -> Result<McpSourceDetail, String> {
        run_marketplace_helper(
            app,
            "modelscope-mcp-detail",
            serde_json::json!({ "slug": slug }),
        )
        .await
    }

    async fn fetch_skill_list(
        &self,
        _app: &AppHandle,
        _query: &MarketplaceSourceQuery,
    ) -> Result<MarketListResponse<SkillSourceListItem>, String> {
        Err("ModelScope source does not provide Skills".to_string())
    }

    async fn fetch_skill_detail(
        &self,
        _app: &AppHandle,
        _slug: &str,
    ) -> Result<SkillSourceDetail, String> {
        Err("ModelScope source does not provide Skills".to_string())
    }

    async fn download_skill_archive(
        &self,
        _app: &AppHandle,
        _slug: &str,
    ) -> Result<SkillArchivePayload, String> {
        Err("ModelScope source does not provide skill archives".to_string())
    }
}
