use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use super::support::{
    bind_optional, bind_value, bool_from_int, now_rfc3339, open_db_connection, UpdateSqlBuilder,
};

fn opencode_config_dir() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|h| h.join(".config").join("opencode"))
        .ok_or_else(|| "Cannot determine home directory".to_string())
}

fn opencode_auth_dir() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|h| h.join(".local").join("share").join("opencode"))
        .ok_or_else(|| "Cannot determine home directory".to_string())
}

const PROVIDER_PROFILE_SELECT_SQL: &str =
    "SELECT id, name, cli_type, is_active, api_key, base_url, provider_name, main_model, reasoning_model, haiku_model, sonnet_default, opus_default, codex_model, opencode_provider_models, opencode_provider_npm, created_at, updated_at FROM provider_profiles";
const PROVIDER_PROFILE_SELECT_BY_ID_SQL: &str =
    "SELECT id, name, cli_type, is_active, api_key, base_url, provider_name, main_model, reasoning_model, haiku_model, sonnet_default, opus_default, codex_model, opencode_provider_models, opencode_provider_npm, created_at, updated_at FROM provider_profiles WHERE id = ?1";
const PROVIDER_PROFILE_SELECT_ACTIVE_SQL: &str =
    "SELECT id, name, cli_type, is_active, api_key, base_url, provider_name, main_model, reasoning_model, haiku_model, sonnet_default, opus_default, codex_model, opencode_provider_models, opencode_provider_npm, created_at, updated_at FROM provider_profiles WHERE cli_type = ?1 AND is_active = 1";
const OPENCODE_DEFAULT_PROVIDER_NPM: &str = "@ai-sdk/openai-compatible";
const MODELS_DEV_PROVIDER_CATALOG_URL: &str = "https://models.dev/api.json";
const CODEX_RUNTIME_SYNC_FILES: &[&str] = &[
    "config.toml",
    "auth.json",
    "version.json",
    ".personality_migration",
    "installation_id",
];
const CODEX_RUNTIME_SYNC_DIRS: &[&str] = &["skills", "plugins", "rules", "sessions", "memories"];

fn open_conn() -> Result<Connection, String> {
    open_db_connection().map_err(|e| e.to_string())
}

fn build_current_profile(cli_type: &str, now: String) -> ProviderProfile {
    ProviderProfile {
        id: String::new(),
        name: "Current Config".to_string(),
        cli_type: cli_type.to_string(),
        is_active: true,
        api_key: None,
        base_url: None,
        provider_name: None,
        main_model: None,
        reasoning_model: None,
        haiku_model: None,
        sonnet_default: None,
        opus_default: None,
        codex_model: None,
        opencode_provider_models: None,
        opencode_provider_npm: None,
        created_at: now.clone(),
        updated_at: now,
    }
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_opencode_model_names(raw: Option<&str>, main_model: Option<&str>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut models = Vec::new();

    if let Some(raw) = raw {
        for item in raw.split([',', '\n', '\r']) {
            let model = item.trim();
            if !model.is_empty() && seen.insert(model.to_string()) {
                models.push(model.to_string());
            }
        }
    }

    if let Some(main_model) = main_model.map(str::trim).filter(|value| !value.is_empty()) {
        if seen.insert(main_model.to_string()) {
            models.push(main_model.to_string());
        }
    }

    models
}

fn serialize_opencode_model_names(models: &[String]) -> Option<String> {
    if models.is_empty() {
        None
    } else {
        Some(models.join("\n"))
    }
}

fn parse_json_object_map(
    value: Option<&serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    value
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn set_process_env_var(key: &str, value: Option<&str>) {
    let normalized = value
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match normalized {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

fn copy_directory_recursive(source: &Path, target: &Path) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }

    fs::create_dir_all(target).map_err(|e| e.to_string())?;

    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if source_path.is_dir() {
            copy_directory_recursive(&source_path, &target_path)?;
            continue;
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        fs::copy(&source_path, &target_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn copy_file_if_exists(source_root: &Path, target_root: &Path, relative_path: &str) -> Result<(), String> {
    let source_path = source_root.join(relative_path);
    if !source_path.exists() {
        return Ok(());
    }

    let target_path = target_root.join(relative_path);
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    fs::copy(&source_path, &target_path).map_err(|e| e.to_string())?;
    Ok(())
}

fn sync_codex_runtime_home_from_user_home() -> Result<(), String> {
    let Some(home_dir) = dirs::home_dir() else {
        return Ok(());
    };

    let source_root = home_dir.join(".codex");
    if !source_root.exists() {
        return Ok(());
    }

    let runtime_home = crate::commands::get_persistence_dir_path()
        .map_err(|e| e.to_string())?
        .join("cache")
        .join("codex-home");
    fs::create_dir_all(&runtime_home).map_err(|e| e.to_string())?;

    for relative_path in CODEX_RUNTIME_SYNC_FILES {
        copy_file_if_exists(&source_root, &runtime_home, relative_path)?;
    }

    for relative_path in CODEX_RUNTIME_SYNC_DIRS {
        let source_path = source_root.join(relative_path);
        if !source_path.exists() {
            continue;
        }

        copy_directory_recursive(&source_path, &runtime_home.join(relative_path))?;
    }

    Ok(())
}

fn apply_cli_runtime_state(profile: &ProviderProfile) -> Result<(), String> {
    match profile.cli_type.as_str() {
        "claude" => {
            set_process_env_var("ANTHROPIC_AUTH_TOKEN", profile.api_key.as_deref());
            set_process_env_var("ANTHROPIC_BASE_URL", profile.base_url.as_deref());
            set_process_env_var("ANTHROPIC_MODEL", profile.main_model.as_deref());
        }
        "codex" => {
            set_process_env_var("OPENAI_API_KEY", profile.api_key.as_deref());
            set_process_env_var("OPENAI_BASE_URL", profile.base_url.as_deref());
            set_process_env_var("OPENAI_MODEL", profile.codex_model.as_deref());
            sync_codex_runtime_home_from_user_home()?;
        }
        "opencode" => {}
        _ => {}
    }

    Ok(())
}

pub fn refresh_cli_runtime_state(cli_type: &str) -> Result<(), String> {
    let normalized_cli_type = cli_type.trim().to_lowercase();
    let profile = read_current_cli_config(normalized_cli_type)?;
    apply_cli_runtime_state(&profile)
}

/// Provider 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub cli_type: String,
    pub is_active: bool,
    // Claude CLI 配置字段
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub provider_name: Option<String>,
    pub main_model: Option<String>,
    pub reasoning_model: Option<String>,
    pub haiku_model: Option<String>,
    pub sonnet_default: Option<String>,
    pub opus_default: Option<String>,
    // Codex 配置字段
    pub codex_model: Option<String>,
    // OpenCode 自定义 Provider 配置字段
    pub opencode_provider_models: Option<String>,
    pub opencode_provider_npm: Option<String>,
    // 元数据
    pub created_at: String,
    pub updated_at: String,
}

/// 创建 Provider 配置输入
#[derive(Debug, Deserialize)]
pub struct CreateProviderProfileInput {
    pub name: String,
    pub cli_type: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub provider_name: Option<String>,
    pub main_model: Option<String>,
    pub reasoning_model: Option<String>,
    pub haiku_model: Option<String>,
    pub sonnet_default: Option<String>,
    pub opus_default: Option<String>,
    pub codex_model: Option<String>,
    pub opencode_provider_models: Option<String>,
    pub opencode_provider_npm: Option<String>,
}

/// 更新 Provider 配置输入
#[derive(Debug, Deserialize)]
pub struct UpdateProviderProfileInput {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub provider_name: Option<String>,
    pub main_model: Option<String>,
    pub reasoning_model: Option<String>,
    pub haiku_model: Option<String>,
    pub sonnet_default: Option<String>,
    pub opus_default: Option<String>,
    pub codex_model: Option<String>,
    pub opencode_provider_models: Option<String>,
    pub opencode_provider_npm: Option<String>,
}

fn map_provider_profile_row(row: &Row<'_>) -> rusqlite::Result<ProviderProfile> {
    Ok(ProviderProfile {
        id: row.get(0)?,
        name: row.get(1)?,
        cli_type: row.get(2)?,
        is_active: bool_from_int(row.get::<_, Option<i32>>(3)?).unwrap_or(false),
        api_key: row.get(4)?,
        base_url: row.get(5)?,
        provider_name: row.get(6)?,
        main_model: row.get(7)?,
        reasoning_model: row.get(8)?,
        haiku_model: row.get(9)?,
        sonnet_default: row.get(10)?,
        opus_default: row.get(11)?,
        codex_model: row.get(12)?,
        opencode_provider_models: row.get(13)?,
        opencode_provider_npm: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

/// 列出所有 Provider 配置
#[tauri::command]
pub fn list_provider_profiles(cli_type: Option<String>) -> Result<Vec<ProviderProfile>, String> {
    let conn = open_conn()?;

    let sql = if let Some(ref _ct) = cli_type {
        format!("{PROVIDER_PROFILE_SELECT_SQL} WHERE cli_type = ?1 ORDER BY updated_at DESC")
    } else {
        format!("{PROVIDER_PROFILE_SELECT_SQL} ORDER BY cli_type, updated_at DESC")
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let profiles = if let Some(ct) = cli_type {
        stmt.query_map([&ct], map_provider_profile_row)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map([], map_provider_profile_row)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    Ok(profiles)
}

/// 获取单个 Provider 配置
#[tauri::command]
pub fn get_provider_profile(id: String) -> Result<ProviderProfile, String> {
    let conn = open_conn()?;

    let profile = conn
        .query_row(
            PROVIDER_PROFILE_SELECT_BY_ID_SQL,
            [&id],
            map_provider_profile_row,
        )
        .map_err(|e| e.to_string())?;

    Ok(profile)
}

/// 创建 Provider 配置
#[tauri::command]
pub fn create_provider_profile(
    input: CreateProviderProfileInput,
) -> Result<ProviderProfile, String> {
    let conn = open_conn()?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();

    conn.execute(
        "INSERT INTO provider_profiles (id, name, cli_type, is_active, api_key, base_url, provider_name, main_model, reasoning_model, haiku_model, sonnet_default, opus_default, codex_model, opencode_provider_models, opencode_provider_npm, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        rusqlite::params![
            &id,
            &input.name,
            &input.cli_type,
            0,
            &input.api_key,
            &input.base_url,
            &input.provider_name,
            &input.main_model,
            &input.reasoning_model,
            &input.haiku_model,
            &input.sonnet_default,
            &input.opus_default,
            &input.codex_model,
            &input.opencode_provider_models,
            &input.opencode_provider_npm,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(ProviderProfile {
        id,
        name: input.name,
        cli_type: input.cli_type,
        is_active: false,
        api_key: input.api_key,
        base_url: input.base_url,
        provider_name: input.provider_name,
        main_model: input.main_model,
        reasoning_model: input.reasoning_model,
        haiku_model: input.haiku_model,
        sonnet_default: input.sonnet_default,
        opus_default: input.opus_default,
        codex_model: input.codex_model,
        opencode_provider_models: input.opencode_provider_models,
        opencode_provider_npm: input.opencode_provider_npm,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 更新 Provider 配置
#[tauri::command]
pub fn update_provider_profile(
    id: String,
    input: UpdateProviderProfileInput,
) -> Result<ProviderProfile, String> {
    let conn = open_conn()?;

    let now = now_rfc3339();

    let mut updates = UpdateSqlBuilder::new();
    updates.push("name", input.name.is_some());
    updates.push("api_key", input.api_key.is_some());
    updates.push("base_url", input.base_url.is_some());
    updates.push("provider_name", input.provider_name.is_some());
    updates.push("main_model", input.main_model.is_some());
    updates.push("reasoning_model", input.reasoning_model.is_some());
    updates.push("haiku_model", input.haiku_model.is_some());
    updates.push("sonnet_default", input.sonnet_default.is_some());
    updates.push("opus_default", input.opus_default.is_some());
    updates.push("codex_model", input.codex_model.is_some());
    updates.push(
        "opencode_provider_models",
        input.opencode_provider_models.is_some(),
    );
    updates.push(
        "opencode_provider_npm",
        input.opencode_provider_npm.is_some(),
    );

    let sql = updates.finish("provider_profiles", "id");

    let mut stmt = conn.prepare_cached(&sql).map_err(|e| e.to_string())?;

    let mut param_count = 1;
    bind_value(&mut stmt, &mut param_count, &now).map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.name).map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.api_key).map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.base_url).map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.provider_name).map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.main_model).map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.reasoning_model)
        .map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.haiku_model).map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.sonnet_default).map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.opus_default).map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.codex_model).map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.opencode_provider_models)
        .map_err(|e| e.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.opencode_provider_npm)
        .map_err(|e| e.to_string())?;
    bind_value(&mut stmt, &mut param_count, &id).map_err(|e| e.to_string())?;

    stmt.raw_execute().map_err(|e| e.to_string())?;

    // 获取更新后的配置
    let updated_profile = get_provider_profile_by_id(&conn, &id)?;

    // 编辑当前激活配置时，需要同步回写对应 CLI 配置文件，保证设置页无需重进即可读到最新内容。
    if updated_profile.is_active {
        write_to_cli_config(&updated_profile)?;
    }

    Ok(updated_profile)
}

/// 直接更新当前 CLI 配置文件中的默认配置。
///
/// 该命令不依赖数据库中的 provider_profiles 记录，而是读取当前 CLI 配置文件，
/// 按输入字段覆写后立即写回文件，并返回最新的文件配置快照。
#[tauri::command]
pub fn update_current_cli_config(
    cli_type: String,
    input: UpdateProviderProfileInput,
) -> Result<ProviderProfile, String> {
    let mut profile = read_current_cli_config(cli_type.clone())?;

    if let Some(api_key) = input.api_key {
        profile.api_key = Some(api_key);
    }
    if let Some(base_url) = input.base_url {
        profile.base_url = Some(base_url);
    }
    if let Some(provider_name) = input.provider_name {
        profile.provider_name = Some(provider_name);
    }
    if let Some(main_model) = input.main_model {
        profile.main_model = Some(main_model);
    }
    if let Some(reasoning_model) = input.reasoning_model {
        profile.reasoning_model = Some(reasoning_model);
    }
    if let Some(haiku_model) = input.haiku_model {
        profile.haiku_model = Some(haiku_model);
    }
    if let Some(sonnet_default) = input.sonnet_default {
        profile.sonnet_default = Some(sonnet_default);
    }
    if let Some(opus_default) = input.opus_default {
        profile.opus_default = Some(opus_default);
    }
    if let Some(codex_model) = input.codex_model {
        profile.codex_model = Some(codex_model);
    }
    if let Some(opencode_provider_models) = input.opencode_provider_models {
        profile.opencode_provider_models = Some(opencode_provider_models);
    }
    if let Some(opencode_provider_npm) = input.opencode_provider_npm {
        profile.opencode_provider_npm = Some(opencode_provider_npm);
    }

    write_to_cli_config(&profile)?;
    read_current_cli_config(cli_type)
}

/// 获取单个 Provider 配置
fn get_provider_profile_by_id(conn: &Connection, id: &str) -> Result<ProviderProfile, String> {
    conn.query_row(
        PROVIDER_PROFILE_SELECT_BY_ID_SQL,
        [id],
        map_provider_profile_row,
    )
    .map_err(|e| e.to_string())
}

/// 删除 Provider 配置
#[tauri::command]
pub fn delete_provider_profile(id: String) -> Result<(), String> {
    let conn = open_conn()?;

    conn.execute("DELETE FROM provider_profiles WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取当前激活的配置
#[tauri::command]
pub fn get_active_provider_profile(cli_type: String) -> Result<Option<ProviderProfile>, String> {
    let conn = open_conn()?;

    let result = conn.query_row(
        PROVIDER_PROFILE_SELECT_ACTIVE_SQL,
        [&cli_type],
        map_provider_profile_row,
    );

    match result {
        Ok(profile) => Ok(Some(profile)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// 一键切换 Provider 配置
#[tauri::command]
pub fn switch_provider_profile(id: String) -> Result<ProviderProfile, String> {
    let mut conn = open_conn()?;

    // 获取要切换的配置
    let profile = get_provider_profile_by_id(&conn, &id)?;

    // 开启事务
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // 将同类型其他配置设为非激活
    tx.execute(
        "UPDATE provider_profiles SET is_active = 0 WHERE cli_type = ?1",
        [&profile.cli_type],
    )
    .map_err(|e| e.to_string())?;

    // 激活当前配置
    tx.execute(
        "UPDATE provider_profiles SET is_active = 1, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now_rfc3339(), &id],
    )
    .map_err(|e| e.to_string())?;

    // 提交事务
    tx.commit().map_err(|e| e.to_string())?;

    // 写入 CLI 配置文件
    write_to_cli_config(&profile)?;

    // 返回更新后的配置
    let updated_profile = get_provider_profile_by_id(&conn, &id)?;
    Ok(updated_profile)
}

/// 写入 CLI 配置文件
fn write_to_cli_config(profile: &ProviderProfile) -> Result<(), String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    match profile.cli_type.as_str() {
        "claude" => {
            // 写入 ~/.claude/settings.json
            let claude_dir = home_dir.join(".claude");
            fs::create_dir_all(&claude_dir)
                .map_err(|e| format!("Failed to create .claude directory: {}", e))?;

            let settings_path = claude_dir.join("settings.json");

            // 读取现有配置
            let mut settings: serde_json::Value = if settings_path.exists() {
                let content = fs::read_to_string(&settings_path)
                    .map_err(|e| format!("Failed to read settings.json: {}", e))?;
                serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

            // 更新 env 配置
            if let Some(obj) = settings.as_object_mut() {
                let mut env = serde_json::Map::new();

                if let Some(ref api_key) = profile.api_key {
                    if !api_key.is_empty() {
                        env.insert(
                            "ANTHROPIC_AUTH_TOKEN".to_string(),
                            serde_json::json!(api_key),
                        );
                    }
                }
                if let Some(ref base_url) = profile.base_url {
                    if !base_url.is_empty() {
                        env.insert(
                            "ANTHROPIC_BASE_URL".to_string(),
                            serde_json::json!(base_url),
                        );
                    }
                }
                if let Some(ref main_model) = profile.main_model {
                    if !main_model.is_empty() {
                        env.insert("ANTHROPIC_MODEL".to_string(), serde_json::json!(main_model));
                    }
                }

                obj.insert("env".to_string(), serde_json::Value::Object(env));
            }

            // 写入文件
            let content = serde_json::to_string_pretty(&settings)
                .map_err(|e| format!("Failed to serialize settings: {}", e))?;
            fs::write(&settings_path, content)
                .map_err(|e| format!("Failed to write settings.json: {}", e))?;
        }
        "codex" => {
            // 写入 ~/.codex/config.toml 和 ~/.codex/auth.json
            let codex_dir = home_dir.join(".codex");
            fs::create_dir_all(&codex_dir)
                .map_err(|e| format!("Failed to create .codex directory: {}", e))?;

            let config_path = codex_dir.join("config.toml");
            let auth_path = codex_dir.join("auth.json");

            // 1. 读取并写入 auth.json (API Key)
            let mut auth: serde_json::Map<String, serde_json::Value> = if auth_path.exists() {
                let content = fs::read_to_string(&auth_path)
                    .map_err(|e| format!("Failed to read auth.json: {}", e))?;
                serde_json::from_str(&content).unwrap_or(serde_json::Map::new())
            } else {
                serde_json::Map::new()
            };

            // 更新 API Key
            if let Some(ref api_key) = profile.api_key {
                if !api_key.is_empty() {
                    // 使用 OPENAI_API_KEY 作为 env_key (Codex CLI 标准格式)
                    auth.insert("OPENAI_API_KEY".to_string(), serde_json::json!(api_key));
                }
            }

            // 写入 auth.json
            let auth_content = serde_json::to_string_pretty(&auth)
                .map_err(|e| format!("Failed to serialize auth.json: {}", e))?;
            fs::write(&auth_path, auth_content)
                .map_err(|e| format!("Failed to write auth.json: {}", e))?;

            // 2. 读取并写入 config.toml
            let mut toml_value: toml::Value = if config_path.exists() {
                let content = fs::read_to_string(&config_path)
                    .map_err(|e| format!("Failed to read config.toml: {}", e))?;
                toml::from_str(&content).unwrap_or(toml::Value::Table(toml::map::Map::new()))
            } else {
                toml::Value::Table(toml::map::Map::new())
            };

            // 更新配置
            if let toml::Value::Table(ref mut table) = toml_value {
                // 更新模型
                if let Some(ref codex_model) = profile.codex_model {
                    if !codex_model.is_empty() {
                        table.insert(
                            "model".to_string(),
                            toml::Value::String(codex_model.clone()),
                        );
                    }
                }

                // 更新或创建 model_providers 配置
                if let Some(ref base_url) = profile.base_url {
                    if !base_url.is_empty() {
                        // 获取或创建 model_providers 表
                        let providers = table
                            .entry("model_providers".to_string())
                            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

                        if let toml::Value::Table(ref mut providers_table) = providers {
                            // 获取或创建 custom_provider 配置
                            let custom_provider = providers_table
                                .entry("custom_provider".to_string())
                                .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

                            if let toml::Value::Table(ref mut provider_table) = custom_provider {
                                provider_table.insert(
                                    "name".to_string(),
                                    toml::Value::String("Custom Provider".to_string()),
                                );
                                provider_table.insert(
                                    "base_url".to_string(),
                                    toml::Value::String(base_url.clone()),
                                );
                                provider_table.insert(
                                    "env_key".to_string(),
                                    toml::Value::String("OPENAI_API_KEY".to_string()),
                                );
                                provider_table.insert(
                                    "wire_api".to_string(),
                                    toml::Value::String("responses".to_string()),
                                );
                            }
                        }

                        // 设置当前使用的 provider
                        table.insert(
                            "model_provider".to_string(),
                            toml::Value::String("custom_provider".to_string()),
                        );
                    }
                }
            }

            // 写入 config.toml
            let content = toml::to_string_pretty(&toml_value)
                .map_err(|e| format!("Failed to serialize TOML: {}", e))?;
            fs::write(&config_path, content)
                .map_err(|e| format!("Failed to write config.toml: {}", e))?;
        }
        "opencode" => {
            let config_dir = opencode_config_dir()?;
            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create opencode config directory: {}", e))?;

            let config_path = config_dir.join("opencode.json");
            let auth_dir = opencode_auth_dir()?;
            fs::create_dir_all(&auth_dir)
                .map_err(|e| format!("Failed to create opencode auth directory: {}", e))?;
            let auth_path = auth_dir.join("auth.json");

            let mut config: serde_json::Value = if config_path.exists() {
                let content = fs::read_to_string(&config_path)
                    .map_err(|e| format!("Failed to read opencode.json: {}", e))?;
                serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

            let provider_name = normalize_optional_text(profile.provider_name.as_deref());
            let main_model = normalize_optional_text(profile.main_model.as_deref());
            let base_url = normalize_optional_text(profile.base_url.as_deref());
            let api_key = normalize_optional_text(profile.api_key.as_deref());
            let provider_npm = normalize_optional_text(profile.opencode_provider_npm.as_deref())
                .unwrap_or_else(|| OPENCODE_DEFAULT_PROVIDER_NPM.to_string());
            let has_explicit_models = profile
                .opencode_provider_models
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            let provider_models = parse_opencode_model_names(
                profile.opencode_provider_models.as_deref(),
                profile.main_model.as_deref(),
            );

            if let Some(obj) = config.as_object_mut() {
                if let (Some(provider_name), Some(main_model)) =
                    (provider_name.as_ref(), main_model.as_ref())
                {
                    obj.insert(
                        "model".to_string(),
                        serde_json::json!(format!("{}/{}", provider_name, main_model)),
                    );
                }

                if let Some(provider_name) = provider_name.as_ref() {
                    let mut providers_map = parse_json_object_map(obj.get("provider"));
                    let should_write_custom_provider = base_url.is_some() || has_explicit_models;

                    if should_write_custom_provider {
                        let mut provider_config =
                            parse_json_object_map(providers_map.get(provider_name));
                        let mut provider_options =
                            parse_json_object_map(provider_config.get("options"));

                        if let Some(base_url) = base_url.as_ref() {
                            provider_options
                                .insert("baseURL".to_string(), serde_json::json!(base_url));
                        } else {
                            provider_options.remove("baseURL");
                        }

                        if let Some(api_key) = api_key.as_ref() {
                            provider_options
                                .insert("apiKey".to_string(), serde_json::json!(api_key));
                        } else {
                            provider_options.remove("apiKey");
                        }

                        if provider_options.is_empty() {
                            provider_config.remove("options");
                        } else {
                            provider_config.insert(
                                "options".to_string(),
                                serde_json::Value::Object(provider_options),
                            );
                        }

                        provider_config.insert("npm".to_string(), serde_json::json!(provider_npm));

                        let models_map = provider_models
                            .iter()
                            .map(|model| (model.clone(), serde_json::json!({ "name": model })))
                            .collect::<serde_json::Map<String, serde_json::Value>>();

                        if models_map.is_empty() {
                            provider_config.remove("models");
                        } else {
                            provider_config.insert(
                                "models".to_string(),
                                serde_json::Value::Object(models_map),
                            );
                        }

                        providers_map.insert(
                            provider_name.clone(),
                            serde_json::Value::Object(provider_config),
                        );
                    }

                    if providers_map.is_empty() {
                        obj.remove("provider");
                    } else {
                        obj.insert(
                            "provider".to_string(),
                            serde_json::Value::Object(providers_map),
                        );
                    }
                }
            }

            let content = serde_json::to_string_pretty(&config)
                .map_err(|e| format!("Failed to serialize opencode.json: {}", e))?;
            fs::write(&config_path, content)
                .map_err(|e| format!("Failed to write opencode.json: {}", e))?;

            let mut auth: serde_json::Value = if auth_path.exists() {
                let content = fs::read_to_string(&auth_path)
                    .map_err(|e| format!("Failed to read auth.json: {}", e))?;
                serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

            if let (Some(provider_name), Some(api_key)) = (provider_name.as_ref(), api_key.as_ref())
            {
                if let Some(auth_obj) = auth.as_object_mut() {
                    auth_obj.insert(
                        provider_name.clone(),
                        serde_json::json!({
                            "type": "api",
                            "key": api_key
                        }),
                    );
                }
            }

            let auth_content = serde_json::to_string_pretty(&auth)
                .map_err(|e| format!("Failed to serialize auth.json: {}", e))?;
            fs::write(&auth_path, auth_content)
                .map_err(|e| format!("Failed to write auth.json: {}", e))?;
        }
        _ => {
            return Err(format!("Unknown CLI type: {}", profile.cli_type));
        }
    }

    apply_cli_runtime_state(profile)?;

    Ok(())
}

/// 从 CLI 配置文件读取当前配置
#[tauri::command]
pub fn read_current_cli_config(cli_type: String) -> Result<ProviderProfile, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    let now = chrono::Utc::now().to_rfc3339();

    match cli_type.as_str() {
        "claude" => {
            let settings_path = home_dir.join(".claude").join("settings.json");

            let mut profile = build_current_profile("claude", now);

            if settings_path.exists() {
                let content = fs::read_to_string(&settings_path)
                    .map_err(|e| format!("Failed to read settings.json: {}", e))?;
                let settings: serde_json::Value =
                    serde_json::from_str(&content).unwrap_or(serde_json::json!({}));

                if let Some(env) = settings.get("env").and_then(|e| e.as_object()) {
                    profile.api_key = env
                        .get("ANTHROPIC_AUTH_TOKEN")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    profile.base_url = env
                        .get("ANTHROPIC_BASE_URL")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    profile.main_model = env
                        .get("ANTHROPIC_MODEL")
                        .or_else(|| env.get("CLAUDE_MODEL"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }

                // 兜底：从 settings.json 顶层 model 字段读取
                if profile.main_model.is_none() {
                    profile.main_model = settings
                        .get("model")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }

            // 如果 settings.json 未找到模型，尝试 ~/.claude.json
            if profile.main_model.is_none() {
                let claude_json_path = home_dir.join(".claude.json");
                if claude_json_path.exists() {
                    if let Ok(content) = fs::read_to_string(&claude_json_path) {
                        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(env) = config.get("env").and_then(|e| e.as_object()) {
                                profile.main_model = env
                                    .get("ANTHROPIC_MODEL")
                                    .or_else(|| env.get("CLAUDE_MODEL"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                            }
                            if profile.main_model.is_none() {
                                profile.main_model = config
                                    .get("model")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                            }
                        }
                    }
                }
            }

            Ok(profile)
        }
        "codex" => {
            let codex_dir = home_dir.join(".codex");
            let config_path = codex_dir.join("config.toml");
            let auth_path = codex_dir.join("auth.json");

            let mut profile = build_current_profile("codex", now);

            // 1. 从 auth.json 读取 API Key
            if auth_path.exists() {
                if let Ok(content) = fs::read_to_string(&auth_path) {
                    if let Ok(auth) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(obj) = auth.as_object() {
                            // 尝试读取 OPENAI_API_KEY (标准格式)
                            profile.api_key = obj
                                .get("OPENAI_API_KEY")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                    }
                }
            }

            // 2. 从 config.toml 读取模型和 base_url
            if config_path.exists() {
                let content = fs::read_to_string(&config_path)
                    .map_err(|e| format!("Failed to read config.toml: {}", e))?;
                let config: toml::Value =
                    toml::from_str(&content).unwrap_or(toml::Value::Table(toml::map::Map::new()));

                if let toml::Value::Table(ref table) = config {
                    // 读取模型
                    profile.codex_model = table
                        .get("model")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // 从 model_providers 中读取 base_url
                    if let Some(providers) = table.get("model_providers").and_then(|p| p.as_table())
                    {
                        // 获取当前使用的 provider 名称
                        let provider_name = table
                            .get("model_provider")
                            .and_then(|v| v.as_str())
                            .unwrap_or("custom_provider");

                        if let Some(provider) =
                            providers.get(provider_name).and_then(|p| p.as_table())
                        {
                            profile.base_url = provider
                                .get("base_url")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            profile.provider_name = provider
                                .get("name")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                    }
                }
            }

            Ok(profile)
        }
        "opencode" => {
            let config_dir = opencode_config_dir()?;
            let config_path = config_dir.join("opencode.json");
            let auth_dir = opencode_auth_dir()?;
            let auth_path = auth_dir.join("auth.json");

            let mut profile = build_current_profile("opencode", now);

            let mut resolved_provider: Option<String> = None;

            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(model_str) = config.get("model").and_then(|v| v.as_str()) {
                            if let Some(slash_pos) = model_str.find('/') {
                                resolved_provider = Some(model_str[..slash_pos].to_string());
                                profile.main_model = Some(model_str[slash_pos + 1..].to_string());
                            } else {
                                profile.main_model = Some(model_str.to_string());
                            }
                        }

                        if let Some(providers) = config.get("provider").and_then(|p| p.as_object())
                        {
                            if resolved_provider.is_none() {
                                resolved_provider = providers.keys().next().cloned();
                            }

                            if let Some(ref pname) = resolved_provider {
                                profile.provider_name = Some(pname.clone());

                                if let Some(pcfg) =
                                    providers.get(pname).and_then(|value| value.as_object())
                                {
                                    profile.opencode_provider_npm = normalize_optional_text(
                                        pcfg.get("npm").and_then(|v| v.as_str()),
                                    );

                                    if let Some(opts) =
                                        pcfg.get("options").and_then(|value| value.as_object())
                                    {
                                        profile.base_url = normalize_optional_text(
                                            opts.get("baseURL").and_then(|v| v.as_str()),
                                        );
                                        profile.api_key = normalize_optional_text(
                                            opts.get("apiKey").and_then(|v| v.as_str()),
                                        );
                                    }

                                    let provider_models = pcfg
                                        .get("models")
                                        .and_then(|value| value.as_object())
                                        .map(|models| models.keys().cloned().collect::<Vec<_>>())
                                        .unwrap_or_default();
                                    profile.opencode_provider_models =
                                        serialize_opencode_model_names(&provider_models);
                                }
                            }
                        }
                    }
                }
            }

            // 从 auth.json 读取 API Key
            if auth_path.exists() {
                if let Ok(content) = fs::read_to_string(&auth_path) {
                    if let Ok(auth) = serde_json::from_str::<serde_json::Value>(&content) {
                        let provider_key = resolved_provider.as_deref().unwrap_or("");
                        if !provider_key.is_empty() {
                            if profile.api_key.is_none() {
                                profile.api_key = auth
                                    .get(provider_key)
                                    .and_then(|p| p.get("key"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                            }
                        }

                        // 如果没有解析到 provider，从 auth.json 取第一个
                        if profile.provider_name.is_none() {
                            if let Some(obj) = auth.as_object() {
                                for (key, val) in obj {
                                    profile.provider_name = Some(key.clone());
                                    if profile.api_key.is_none() {
                                        profile.api_key = val
                                            .get("key")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            Ok(profile)
        }
        _ => Err(format!("Unknown CLI type: {}", cli_type)),
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliConnectionInfo {
    /// CLI 类型名称
    pub cli_type: String,
    /// CLI 显示名称
    pub display_name: String,
    /// 主配置文件路径 (如 ~/.claude.json)
    pub config_file: String,
    /// 设置文件路径 (如 ~/.claude/settings.json)
    pub settings_file: String,
    /// 当前 API Base URL
    pub base_url: Option<String>,
    /// 当前主模型
    pub main_model: Option<String>,
    /// 当前 Provider 名称 (OpenCode 用)
    pub provider_name: Option<String>,
    /// 当前 API Key (脱敏显示)
    pub api_key_masked: Option<String>,
    /// 当前 API Key (完整值，用于切换显示)
    pub api_key: Option<String>,
    /// 配置是否有效
    pub is_valid: bool,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 读取 CLI 连接信息 (Tauri 命令)
#[tauri::command]
pub fn read_cli_connection_info(cli_type: String) -> Result<CliConnectionInfo, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;

    match cli_type.as_str() {
        "claude" => {
            let config_file = home_dir.join(".claude.json");
            let settings_file = home_dir.join(".claude").join("settings.json");

            let mut info = CliConnectionInfo {
                cli_type: "claude".to_string(),
                display_name: "Claude CLI".to_string(),
                config_file: config_file.to_string_lossy().to_string(),
                settings_file: settings_file.to_string_lossy().to_string(),
                base_url: None,
                main_model: None,
                api_key_masked: None,
                api_key: None,
                is_valid: false,
                error_message: None,
                provider_name: None,
            };

            // 首先尝试从 settings.json 读取
            if settings_file.exists() {
                if let Ok(content) = fs::read_to_string(&settings_file) {
                    if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(env) = settings.get("env").and_then(|e| e.as_object()) {
                            // 读取 API Key (同时保存完整值和脱敏值)
                            if let Some(api_key) =
                                env.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str())
                            {
                                info.api_key = Some(api_key.to_string());
                                info.api_key_masked = Some(mask_api_key(api_key));
                            }
                            // 读取 Base URL
                            if let Some(base_url) =
                                env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str())
                            {
                                info.base_url = Some(base_url.to_string());
                            }
                            // 读取主模型
                            if let Some(main_model) =
                                env.get("ANTHROPIC_MODEL").and_then(|v| v.as_str())
                            {
                                info.main_model = Some(main_model.to_string());
                            }
                            info.is_valid = true;
                        }
                    }
                }
            }

            // 如果 settings.json 没有配置，尝试从 .claude.json 读取
            if !info.is_valid && config_file.exists() {
                if let Ok(content) = fs::read_to_string(&config_file) {
                    if let Ok(_config) = serde_json::from_str::<serde_json::Value>(&content) {
                        // .claude.json 主要存储 MCP 等配置，API 配置通常在 settings.json
                        // 但我们仍然检查是否有相关配置
                        info.is_valid = true;
                    }
                }
            }

            if !info.is_valid {
                info.error_message = Some("未找到有效配置".to_string());
            }

            Ok(info)
        }
        "codex" => {
            let codex_dir = home_dir.join(".codex");
            let config_file = codex_dir.join("config.toml");
            let auth_file = codex_dir.join("auth.json");

            let mut info = CliConnectionInfo {
                cli_type: "codex".to_string(),
                display_name: "Codex CLI".to_string(),
                config_file: config_file.to_string_lossy().to_string(),
                settings_file: auth_file.to_string_lossy().to_string(),
                base_url: None,
                main_model: None,
                api_key_masked: None,
                api_key: None,
                is_valid: false,
                error_message: None,
                provider_name: None,
            };

            // 1. 从 auth.json 读取 API Key
            if auth_file.exists() {
                if let Ok(content) = fs::read_to_string(&auth_file) {
                    if let Ok(auth) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(obj) = auth.as_object() {
                            // 读取 OPENAI_API_KEY (Codex CLI 标准格式)
                            if let Some(api_key) =
                                obj.get("OPENAI_API_KEY").and_then(|v| v.as_str())
                            {
                                info.api_key = Some(api_key.to_string());
                                info.api_key_masked = Some(mask_api_key(api_key));
                            }
                        }
                    }
                }
            }

            // 2. 从 config.toml 读取模型和 base_url
            if config_file.exists() {
                if let Ok(content) = fs::read_to_string(&config_file) {
                    if let Ok(toml::Value::Table(ref table)) =
                        toml::from_str::<toml::Value>(&content)
                    {
                        // 读取模型
                        if let Some(model) = table.get("model").and_then(|v| v.as_str()) {
                            info.main_model = Some(model.to_string());
                        }

                        // 从 model_providers 中读取 base_url
                        if let Some(providers) =
                            table.get("model_providers").and_then(|p| p.as_table())
                        {
                            // 获取当前使用的 provider 名称
                            let provider_name = table
                                .get("model_provider")
                                .and_then(|v| v.as_str())
                                .unwrap_or("custom_provider");

                            if let Some(provider) =
                                providers.get(provider_name).and_then(|p| p.as_table())
                            {
                                if let Some(base_url) =
                                    provider.get("base_url").and_then(|v| v.as_str())
                                {
                                    info.base_url = Some(base_url.to_string());
                                }
                            }
                        }

                        info.is_valid = true;
                    }
                }
            }

            if !info.is_valid {
                info.error_message = Some("未找到有效配置".to_string());
            }

            Ok(info)
        }
        "opencode" => {
            let config_dir = opencode_config_dir()?;
            let config_file = config_dir.join("opencode.json");
            let auth_dir = opencode_auth_dir()?;
            let auth_file = auth_dir.join("auth.json");

            let mut info = CliConnectionInfo {
                cli_type: "opencode".to_string(),
                display_name: "OpenCode CLI".to_string(),
                config_file: config_file.to_string_lossy().to_string(),
                settings_file: auth_file.to_string_lossy().to_string(),
                base_url: None,
                main_model: None,
                api_key_masked: None,
                api_key: None,
                is_valid: false,
                error_message: None,
                provider_name: None,
            };

            let mut resolved_provider: Option<String> = None;

            if config_file.exists() {
                if let Ok(content) = fs::read_to_string(&config_file) {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(model_str) = config.get("model").and_then(|v| v.as_str()) {
                            if let Some(slash_pos) = model_str.find('/') {
                                resolved_provider = Some(model_str[..slash_pos].to_string());
                                info.main_model = Some(model_str[slash_pos + 1..].to_string());
                            } else {
                                info.main_model = Some(model_str.to_string());
                            }
                        }

                        if let Some(providers) = config.get("provider").and_then(|p| p.as_object())
                        {
                            if resolved_provider.is_none() {
                                resolved_provider = providers.keys().next().cloned();
                            }

                            if let Some(ref pname) = resolved_provider {
                                if let Some(pcfg) =
                                    providers.get(pname).and_then(|value| value.as_object())
                                {
                                    if let Some(opts) =
                                        pcfg.get("options").and_then(|value| value.as_object())
                                    {
                                        info.base_url = opts
                                            .get("baseURL")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());

                                        if let Some(api_key) =
                                            opts.get("apiKey").and_then(|v| v.as_str())
                                        {
                                            info.api_key = Some(api_key.to_string());
                                            info.api_key_masked = Some(mask_api_key(api_key));
                                        }
                                    }
                                }
                            }
                        }

                        info.is_valid = true;
                    }
                }
            }

            if auth_file.exists() {
                if let Ok(content) = fs::read_to_string(&auth_file) {
                    if let Ok(auth) = serde_json::from_str::<serde_json::Value>(&content) {
                        let provider_key = resolved_provider.as_deref().unwrap_or("");
                        if !provider_key.is_empty() && info.api_key.is_none() {
                            if let Some(api_key) = auth
                                .get(provider_key)
                                .and_then(|p| p.get("key"))
                                .and_then(|v| v.as_str())
                            {
                                info.api_key = Some(api_key.to_string());
                                info.api_key_masked = Some(mask_api_key(api_key));
                                info.is_valid = true;
                            }
                        }

                        if resolved_provider.is_none() {
                            if let Some(obj) = auth.as_object() {
                                for (key, val) in obj {
                                    resolved_provider = Some(key.clone());
                                    if info.api_key.is_none() {
                                        if let Some(api_key) =
                                            val.get("key").and_then(|v| v.as_str())
                                        {
                                            info.api_key = Some(api_key.to_string());
                                            info.api_key_masked = Some(mask_api_key(api_key));
                                        }
                                    }
                                    break;
                                }
                            }
                            info.is_valid = true;
                        }
                    }
                }
            }

            if !info.is_valid {
                info.error_message = Some("未找到有效配置".to_string());
            }

            info.provider_name = resolved_provider;

            Ok(info)
        }
        _ => Err(format!("Unknown CLI type: {}", cli_type)),
    }
}
fn mask_api_key(api_key: &str) -> String {
    if api_key.len() <= 8 {
        return "*".repeat(api_key.len());
    }
    let visible_chars = 4;
    let masked_len = api_key.len() - visible_chars;
    format!("{}{}", "*".repeat(masked_len), &api_key[masked_len..])
}

/// 批量读取所有 CLI 连接信息
#[tauri::command]
pub fn read_all_cli_connections() -> Result<Vec<CliConnectionInfo>, String> {
    let mut connections = Vec::new();

    // 读取 Claude CLI 配置
    match read_cli_connection_info("claude".to_string()) {
        Ok(info) => connections.push(info),
        Err(e) => {
            connections.push(CliConnectionInfo {
                cli_type: "claude".to_string(),
                display_name: "Claude CLI".to_string(),
                config_file: String::new(),
                settings_file: String::new(),
                base_url: None,
                main_model: None,
                provider_name: None,
                api_key_masked: None,
                api_key: None,
                is_valid: false,
                error_message: Some(e),
            });
        }
    }

    // 读取 Codex CLI 配置
    match read_cli_connection_info("codex".to_string()) {
        Ok(info) => connections.push(info),
        Err(e) => {
            connections.push(CliConnectionInfo {
                cli_type: "codex".to_string(),
                display_name: "Codex CLI".to_string(),
                config_file: String::new(),
                settings_file: String::new(),
                base_url: None,
                main_model: None,
                provider_name: None,
                api_key_masked: None,
                api_key: None,
                is_valid: false,
                error_message: Some(e),
            });
        }
    }

    // 读取 OpenCode CLI 配置
    match read_cli_connection_info("opencode".to_string()) {
        Ok(info) => connections.push(info),
        Err(e) => {
            connections.push(CliConnectionInfo {
                cli_type: "opencode".to_string(),
                display_name: "OpenCode CLI".to_string(),
                config_file: String::new(),
                settings_file: String::new(),
                base_url: None,
                main_model: None,
                provider_name: None,
                api_key_masked: None,
                api_key: None,
                is_valid: false,
                error_message: Some(e),
            });
        }
    }

    Ok(connections)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeAuthProvider {
    pub id: String,
    pub display_name: String,
    pub has_key: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelsDevProviderEntry {
    #[allow(dead_code)]
    id: Option<String>,
    name: Option<String>,
}

fn load_opencode_config_json() -> Option<serde_json::Value> {
    let config_path = opencode_config_dir().ok()?.join("opencode.json");
    let content = fs::read_to_string(config_path).ok()?;
    serde_json::from_str::<serde_json::Value>(&content).ok()
}

fn load_opencode_auth_json() -> Option<serde_json::Value> {
    let auth_path = opencode_auth_dir().ok()?.join("auth.json");
    let content = fs::read_to_string(auth_path).ok()?;
    serde_json::from_str::<serde_json::Value>(&content).ok()
}

fn collect_opencode_provider_ids() -> Vec<String> {
    let mut provider_set = std::collections::BTreeSet::new();

    if let Ok(cli_provider_ids) = fetch_all_opencode_provider_ids() {
        provider_set.extend(cli_provider_ids);
    }

    if let Some(config) = load_opencode_config_json() {
        if let Some(model_str) = config.get("model").and_then(|value| value.as_str()) {
            if let Some((provider, _)) = model_str.split_once('/') {
                if !provider.trim().is_empty() {
                    provider_set.insert(provider.trim().to_string());
                }
            }
        }

        if let Some(providers) = config.get("provider").and_then(|value| value.as_object()) {
            provider_set.extend(
                providers
                    .keys()
                    .map(|provider| provider.trim())
                    .filter(|provider| !provider.is_empty())
                    .map(ToOwned::to_owned),
            );
        }
    }

    if let Some(auth) = load_opencode_auth_json() {
        if let Some(obj) = auth.as_object() {
            provider_set.extend(
                obj.keys()
                    .map(|provider| provider.trim())
                    .filter(|provider| !provider.is_empty())
                    .map(ToOwned::to_owned),
            );
        }
    }

    provider_set.into_iter().collect()
}

fn load_configured_opencode_models(provider: &str) -> Vec<String> {
    let Some(config) = load_opencode_config_json() else {
        return Vec::new();
    };

    let Some(provider_config) = config
        .get("provider")
        .and_then(|value| value.get(provider))
        .and_then(|value| value.as_object())
    else {
        return Vec::new();
    };

    provider_config
        .get("models")
        .and_then(|value| value.as_object())
        .map(|models| models.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default()
}

/// 聚合 opencode CLI、opencode.json 与 auth.json 中的 Provider 信息
#[tauri::command]
pub async fn read_opencode_auth_providers() -> Result<Vec<OpenCodeAuthProvider>, String> {
    let mut auth_keys_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(auth) = load_opencode_auth_json() {
        if let Some(obj) = auth.as_object() {
            for (key, val) in obj {
                if val.get("key").and_then(|v| v.as_str()).is_some() {
                    auth_keys_set.insert(key.clone());
                }
            }
        }
    }

    let mut providers = fetch_models_dev_providers()
        .await
        .unwrap_or_default();

    let existing_ids: std::collections::HashSet<String> =
        providers.iter().map(|provider| provider.id.clone()).collect();
    let fallback_ids = collect_opencode_provider_ids();

    for id in fallback_ids {
        if existing_ids.contains(&id) {
            continue;
        }
        providers.push(OpenCodeAuthProvider {
            display_name: format_provider_display_name(&id),
            id: id.clone(),
            has_key: false,
        });
    }

    for provider in &mut providers {
        provider.has_key = auth_keys_set.contains(&provider.id);
    }

    if providers.is_empty() {
        return Err("未查询到可用的 OpenCode Provider".to_string());
    }

    Ok(providers)
}

async fn fetch_models_dev_providers() -> Result<Vec<OpenCodeAuthProvider>, String> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("创建 models.dev HTTP 客户端失败: {}", e))?;

    let response = client
        .get(MODELS_DEV_PROVIDER_CATALOG_URL)
        .send()
        .await
        .map_err(|e| format!("请求 models.dev provider catalog 失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "请求 models.dev provider catalog 失败: HTTP {}",
            response.status()
        ));
    }

    let payload = response
        .json::<serde_json::Map<String, serde_json::Value>>()
        .await
        .map_err(|e| format!("解析 models.dev provider catalog 失败: {}", e))?;

    let mut providers = Vec::with_capacity(payload.len());
    for (id, value) in payload {
        let entry: ModelsDevProviderEntry = serde_json::from_value(value).unwrap_or(ModelsDevProviderEntry {
            id: None,
            name: None,
        });
        providers.push(OpenCodeAuthProvider {
            id: id.clone(),
            display_name: entry
                .name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| format_provider_display_name(&id)),
            has_key: false,
        });
    }

    providers.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    Ok(providers)
}

/// 通过 `opencode models` 命令（不带参数）获取所有可用模型的 provider 前缀，
/// 去重后返回所有 CLI 支持的 provider ID 列表。
fn fetch_all_opencode_provider_ids() -> Result<Vec<String>, String> {
    let scan_paths = crate::commands::cli::get_scan_paths_public();
    let cli_path = crate::commands::cli_support::find_cli_executable("opencode", &scan_paths)
        .ok_or_else(|| "未找到 opencode CLI".to_string())?;

    let output = crate::commands::cli_support::run_cli_command(&cli_path, &["models"])
        .map_err(|e| format!("执行 opencode models 失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("查询模型列表失败: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let mut provider_set = std::collections::BTreeSet::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(slash_pos) = trimmed.find('/') {
            let prefix = &trimmed[..slash_pos];
            if !prefix.is_empty() {
                provider_set.insert(prefix.to_string());
            }
        }
    }

    Ok(provider_set.into_iter().collect())
}

/// 从 auth.json 或 opencode.json 配置中读取指定 provider 的 API Key
#[tauri::command]
pub fn read_opencode_provider_api_key(provider: String) -> Result<Option<String>, String> {
    if let Some(config) = load_opencode_config_json() {
        if let Some(pcfg) = config
            .get("provider")
            .and_then(|p| p.get(&provider))
            .and_then(|v| v.as_object())
        {
            if let Some(api_key) = pcfg
                .get("options")
                .and_then(|o| o.get("apiKey"))
                .and_then(|v| v.as_str())
            {
                return Ok(Some(api_key.to_string()));
            }
        }
    }

    if let Some(auth) = load_opencode_auth_json() {
        if let Some(api_key) = auth
            .get(&provider)
            .and_then(|p| p.get("key"))
            .and_then(|v| v.as_str())
        {
            return Ok(Some(api_key.to_string()));
        }
    }

    Ok(None)
}

/// 通过 opencode CLI 查询指定 Provider 可用的模型列表
#[tauri::command]
pub fn list_opencode_models(provider: String) -> Result<Vec<String>, String> {
    let configured_models = load_configured_opencode_models(&provider);

    let scan_paths = crate::commands::cli::get_scan_paths_public();
    let Some(cli_path) =
        crate::commands::cli_support::find_cli_executable("opencode", &scan_paths)
    else {
        if !configured_models.is_empty() {
            return Ok(configured_models);
        }
        return Err("未找到 opencode CLI".to_string());
    };

    let output = crate::commands::cli_support::run_cli_command(&cli_path, &["models", &provider])
        .map_err(|e| format!("执行 opencode models {} 失败: {}", provider, e))?;

    if !output.status.success() {
        if !configured_models.is_empty() {
            return Ok(configured_models);
        }
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("查询模型失败: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let prefix = format!("{}/", provider);

    let models: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with(&prefix) {
                Some(trimmed[prefix.len()..].to_string())
            } else {
                None
            }
        })
        .collect();

    if models.is_empty() && !configured_models.is_empty() {
        return Ok(configured_models);
    }

    Ok(models)
}

/// 将 provider ID 格式化为人类可读的 display name
/// 不硬编码，通用算法：按连字符/点分割 → 首字母大写
/// 特殊缩写保持大写
fn format_provider_display_name(id: &str) -> String {
    let upper_words = [
        "ai", "api", "sdk", "llm", "cpu", "gpu", "db", "io", "url", "gpt",
    ];

    id.split(|c: char| c == '-' || c == '.')
        .filter(|word| *word != "plan")
        .map(|word| {
            if upper_words.contains(&word.to_lowercase().as_str()) {
                word.to_uppercase()
            } else if word == "opencode" {
                "OpenCode".to_string()
            } else if word == "coding" {
                "Coding Plan".to_string()
            } else {
                let mut c = word.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
