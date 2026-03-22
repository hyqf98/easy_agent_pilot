use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspServerInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub monaco_language_id: String,
    pub file_extensions: Vec<String>,
    pub installed: bool,
    pub install_path: String,
    pub installed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspActivationResult {
    pub file_path: String,
    pub server_id: Option<String>,
    pub monaco_language_id: String,
    pub activated: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LspInstallManifest {
    server_id: String,
    installed_at: String,
    source: String,
    version: String,
}

#[derive(Debug, Clone, Copy)]
struct LspServerSpec {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    monaco_language_id: &'static str,
    file_extensions: &'static [&'static str],
}

const LSP_SERVER_SPECS: &[LspServerSpec] = &[
    LspServerSpec {
        id: "typescript-lsp",
        name: "TypeScript LSP",
        description: "TypeScript / TSX / DTS",
        monaco_language_id: "typescript",
        file_extensions: &["ts", "tsx", "mts", "cts", "d.ts"],
    },
    LspServerSpec {
        id: "javascript-lsp",
        name: "JavaScript LSP",
        description: "JavaScript / JSX",
        monaco_language_id: "javascript",
        file_extensions: &["js", "jsx", "mjs", "cjs"],
    },
    LspServerSpec {
        id: "json-lsp",
        name: "JSON LSP",
        description: "JSON / JSONC",
        monaco_language_id: "json",
        file_extensions: &["json", "jsonc"],
    },
    LspServerSpec {
        id: "html-lsp",
        name: "HTML LSP",
        description: "HTML / Vue / XML / JSP",
        monaco_language_id: "html",
        file_extensions: &["html", "htm", "xml", "vue", "jsp", "jspf", "tag"],
    },
    LspServerSpec {
        id: "css-lsp",
        name: "CSS LSP",
        description: "CSS / SCSS / SASS / LESS",
        monaco_language_id: "css",
        file_extensions: &["css", "scss", "sass", "less"],
    },
    LspServerSpec {
        id: "python-lsp",
        name: "Python LSP",
        description: "Python",
        monaco_language_id: "python",
        file_extensions: &["py"],
    },
    LspServerSpec {
        id: "java-lsp",
        name: "Java LSP",
        description: "Java",
        monaco_language_id: "java",
        file_extensions: &["java"],
    },
    LspServerSpec {
        id: "rust-lsp",
        name: "Rust LSP",
        description: "Rust",
        monaco_language_id: "rust",
        file_extensions: &["rs"],
    },
    LspServerSpec {
        id: "shell-lsp",
        name: "Shell LSP",
        description: "Shell / Dockerfile",
        monaco_language_id: "shell",
        file_extensions: &["sh", "bash", "zsh", "dockerfile"],
    },
    LspServerSpec {
        id: "markdown-lsp",
        name: "Markdown LSP",
        description: "Markdown / MDX",
        monaco_language_id: "markdown",
        file_extensions: &["md", "mdx"],
    },
    LspServerSpec {
        id: "yaml-lsp",
        name: "YAML LSP",
        description: "YAML",
        monaco_language_id: "yaml",
        file_extensions: &["yaml", "yml"],
    },
];

fn lsp_storage_root_dir() -> Result<PathBuf, String> {
    super::get_persistence_dir_path()
        .map(|base| base.join("tools").join("lsp"))
        .map_err(|e| e.to_string())
}

fn lsp_server_install_dir(server_id: &str) -> Result<PathBuf, String> {
    Ok(lsp_storage_root_dir()?.join(server_id))
}

fn lsp_manifest_path(server_id: &str) -> Result<PathBuf, String> {
    Ok(lsp_server_install_dir(server_id)?.join("manifest.json"))
}

fn load_manifest(server_id: &str) -> Result<Option<LspInstallManifest>, String> {
    let manifest_path = lsp_manifest_path(server_id)?;
    if !manifest_path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("读取 LSP 清单失败 {}: {}", manifest_path.display(), e))?;
    let manifest = serde_json::from_str::<LspInstallManifest>(&raw)
        .map_err(|e| format!("解析 LSP 清单失败 {}: {}", manifest_path.display(), e))?;
    Ok(Some(manifest))
}

fn save_manifest(server_id: &str) -> Result<LspInstallManifest, String> {
    let install_dir = lsp_server_install_dir(server_id)?;
    fs::create_dir_all(&install_dir).map_err(|e| format!("创建 LSP 目录失败: {}", e))?;

    let manifest = LspInstallManifest {
        server_id: server_id.to_string(),
        installed_at: Utc::now().to_rfc3339(),
        source: "manual-download".to_string(),
        version: "builtin-monaco-language-pack".to_string(),
    };

    let manifest_path = install_dir.join("manifest.json");
    let raw =
        serde_json::to_string_pretty(&manifest).map_err(|e| format!("序列化清单失败: {}", e))?;
    fs::write(&manifest_path, raw)
        .map_err(|e| format!("写入清单失败 {}: {}", manifest_path.display(), e))?;

    let note_path = install_dir.join("README.txt");
    let note = "Managed by Easy Agent Pilot.\nThis folder stores manual LSP install state for editor language activation.\n";
    fs::write(&note_path, note)
        .map_err(|e| format!("写入说明文件失败 {}: {}", note_path.display(), e))?;

    Ok(manifest)
}

fn find_server_by_id(server_id: &str) -> Option<&'static LspServerSpec> {
    LSP_SERVER_SPECS.iter().find(|spec| spec.id == server_id)
}

fn normalize_file_extension(file_path: &str) -> String {
    let normalized = file_path.replace('\\', "/");
    let file_name = normalized
        .split('/')
        .next_back()
        .unwrap_or(file_path)
        .to_lowercase();

    if file_name == "dockerfile" {
        return "dockerfile".to_string();
    }

    if file_name.ends_with(".d.ts") {
        return "d.ts".to_string();
    }

    match file_name.rfind('.') {
        Some(index) if index < file_name.len() - 1 => file_name[(index + 1)..].to_string(),
        _ => file_name,
    }
}

fn find_server_by_file(file_path: &str) -> Option<&'static LspServerSpec> {
    let extension = normalize_file_extension(file_path);
    LSP_SERVER_SPECS
        .iter()
        .find(|spec| spec.file_extensions.contains(&extension.as_str()))
}

fn to_server_info(
    spec: &LspServerSpec,
    manifest: Option<LspInstallManifest>,
) -> Result<LspServerInfo, String> {
    let install_dir = lsp_server_install_dir(spec.id)?;
    Ok(LspServerInfo {
        id: spec.id.to_string(),
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        monaco_language_id: spec.monaco_language_id.to_string(),
        file_extensions: spec
            .file_extensions
            .iter()
            .map(|item| item.to_string())
            .collect(),
        installed: manifest.is_some(),
        install_path: install_dir.to_string_lossy().to_string(),
        installed_at: manifest.map(|m| m.installed_at),
    })
}

#[tauri::command]
pub fn get_lsp_storage_dir() -> Result<String, String> {
    let root = lsp_storage_root_dir()?;
    fs::create_dir_all(&root).map_err(|e| format!("创建 LSP 存储目录失败: {}", e))?;
    Ok(root.to_string_lossy().to_string())
}

#[tauri::command]
pub fn list_lsp_servers() -> Result<Vec<LspServerInfo>, String> {
    let root = lsp_storage_root_dir()?;
    fs::create_dir_all(&root).map_err(|e| format!("创建 LSP 存储目录失败: {}", e))?;

    let mut result = Vec::with_capacity(LSP_SERVER_SPECS.len());
    for spec in LSP_SERVER_SPECS {
        let manifest = load_manifest(spec.id)?;
        result.push(to_server_info(spec, manifest)?);
    }
    Ok(result)
}

#[tauri::command]
pub fn download_lsp_server(server_id: String) -> Result<LspServerInfo, String> {
    let spec =
        find_server_by_id(&server_id).ok_or_else(|| format!("未知的 LSP 服务: {}", server_id))?;

    let manifest = match load_manifest(spec.id)? {
        Some(existing) => existing,
        None => save_manifest(spec.id)?,
    };

    to_server_info(spec, Some(manifest))
}

#[tauri::command]
pub fn remove_lsp_server(server_id: String) -> Result<LspServerInfo, String> {
    let spec =
        find_server_by_id(&server_id).ok_or_else(|| format!("未知的 LSP 服务: {}", server_id))?;

    let install_dir = lsp_server_install_dir(spec.id)?;
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)
            .map_err(|e| format!("删除 LSP 目录失败 {}: {}", install_dir.display(), e))?;
    }

    to_server_info(spec, None)
}

#[tauri::command]
pub fn activate_lsp_for_file(file_path: String) -> Result<LspActivationResult, String> {
    let Some(spec) = find_server_by_file(&file_path) else {
        return Ok(LspActivationResult {
            file_path,
            server_id: None,
            monaco_language_id: "plaintext".to_string(),
            activated: false,
            message: "当前文件类型未匹配到可用的 LSP 服务".to_string(),
        });
    };

    if load_manifest(spec.id)?.is_some() {
        return Ok(LspActivationResult {
            file_path,
            server_id: Some(spec.id.to_string()),
            monaco_language_id: spec.monaco_language_id.to_string(),
            activated: true,
            message: format!("已激活 {}", spec.name),
        });
    }

    Ok(LspActivationResult {
        file_path,
        server_id: Some(spec.id.to_string()),
        monaco_language_id: "plaintext".to_string(),
        activated: false,
        message: format!("{} 未下载，未自动下载", spec.name),
    })
}
