use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_EDITABLE_FILE_SIZE: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedLanguage {
    pub language_id: String,
    pub strategy_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFileContent {
    pub content: String,
    pub size_bytes: u64,
    pub line_count: usize,
}

fn resolve_path(path: &str) -> Result<PathBuf, String> {
    if let Some(rest) = path.strip_prefix('~') {
        let home = dirs::home_dir().ok_or_else(|| "无法获取用户主目录".to_string())?;
        let rest = rest.strip_prefix('/').unwrap_or(rest);
        return Ok(home.join(rest));
    }

    Ok(PathBuf::from(path))
}

fn canonicalize_existing(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path).map_err(|e| format!("无法解析路径 {}: {}", path.display(), e))
}

fn validate_project_file(project_path: &str, file_path: &str) -> Result<PathBuf, String> {
    let project_root = resolve_path(project_path)?;
    if !project_root.exists() {
        return Err(format!("项目路径不存在: {}", project_path));
    }
    if !project_root.is_dir() {
        return Err(format!("项目路径不是目录: {}", project_path));
    }

    let target_path = resolve_path(file_path)?;
    if !target_path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }
    if !target_path.is_file() {
        return Err(format!("路径不是文件: {}", file_path));
    }

    let canonical_root = canonicalize_existing(&project_root)?;
    let canonical_target = canonicalize_existing(&target_path)?;

    if !canonical_target.starts_with(&canonical_root) {
        return Err("目标文件不在项目目录内".to_string());
    }

    Ok(canonical_target)
}

fn detect_language_from_path(path: &str) -> DetectedLanguage {
    let lower = path.to_lowercase();

    if lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".mts")
        || lower.ends_with(".cts")
        || lower.ends_with(".d.ts")
    {
        return DetectedLanguage {
            language_id: "typescript".to_string(),
            strategy_id: "typescript".to_string(),
        };
    }

    if lower.ends_with(".js")
        || lower.ends_with(".jsx")
        || lower.ends_with(".mjs")
        || lower.ends_with(".cjs")
    {
        return DetectedLanguage {
            language_id: "javascript".to_string(),
            strategy_id: "javascript".to_string(),
        };
    }

    if lower.ends_with(".vue") {
        return DetectedLanguage {
            language_id: "html".to_string(),
            strategy_id: "vue".to_string(),
        };
    }

    if lower.ends_with(".json") || lower.ends_with(".jsonc") {
        return DetectedLanguage {
            language_id: "json".to_string(),
            strategy_id: "json".to_string(),
        };
    }

    if lower.ends_with(".md") || lower.ends_with(".mdx") {
        return DetectedLanguage {
            language_id: "markdown".to_string(),
            strategy_id: "markdown".to_string(),
        };
    }

    if lower.ends_with(".py") {
        return DetectedLanguage {
            language_id: "python".to_string(),
            strategy_id: "python".to_string(),
        };
    }

    if lower.ends_with(".java") {
        return DetectedLanguage {
            language_id: "java".to_string(),
            strategy_id: "java".to_string(),
        };
    }

    if lower.ends_with(".rs") {
        return DetectedLanguage {
            language_id: "rust".to_string(),
            strategy_id: "rust".to_string(),
        };
    }

    if lower.ends_with(".html")
        || lower.ends_with(".htm")
        || lower.ends_with(".xml")
        || lower.ends_with(".jsp")
        || lower.ends_with(".jspf")
        || lower.ends_with(".tag")
    {
        return DetectedLanguage {
            language_id: "html".to_string(),
            strategy_id: "html".to_string(),
        };
    }

    if lower.ends_with(".css")
        || lower.ends_with(".scss")
        || lower.ends_with(".sass")
        || lower.ends_with(".less")
    {
        return DetectedLanguage {
            language_id: "css".to_string(),
            strategy_id: "css".to_string(),
        };
    }

    if lower.ends_with(".sh") || lower.ends_with(".bash") || lower.ends_with(".zsh") {
        return DetectedLanguage {
            language_id: "shell".to_string(),
            strategy_id: "shell".to_string(),
        };
    }

    if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        return DetectedLanguage {
            language_id: "yaml".to_string(),
            strategy_id: "yaml".to_string(),
        };
    }

    DetectedLanguage {
        language_id: "plaintext".to_string(),
        strategy_id: "plaintext".to_string(),
    }
}

#[tauri::command]
pub fn read_project_file(
    project_path: String,
    file_path: String,
) -> Result<ProjectFileContent, String> {
    let path = validate_project_file(&project_path, &file_path)?;
    let metadata = fs::metadata(&path).map_err(|e| format!("读取文件元信息失败: {}", e))?;

    if metadata.len() > MAX_EDITABLE_FILE_SIZE {
        return Err(format!(
            "文件过大（{} bytes），当前仅支持编辑 {} bytes 以内的文本文件",
            metadata.len(),
            MAX_EDITABLE_FILE_SIZE
        ));
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取文件失败（仅支持 UTF-8 文本文件）: {}", e))?;

    Ok(ProjectFileContent {
        line_count: content.lines().count(),
        size_bytes: metadata.len(),
        content,
    })
}

#[tauri::command]
pub fn write_project_file(
    project_path: String,
    file_path: String,
    content: String,
) -> Result<(), String> {
    let path = validate_project_file(&project_path, &file_path)?;

    fs::write(&path, content).map_err(|e| format!("写入文件失败: {}", e))
}

#[tauri::command]
pub fn detect_file_language(file_path: String) -> Result<DetectedLanguage, String> {
    Ok(detect_language_from_path(&file_path))
}
