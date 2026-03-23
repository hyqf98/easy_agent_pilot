use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::fs;

use super::support::{
    bind_optional, bind_value, bool_from_int, now_rfc3339, open_db_connection, UpdateSqlBuilder,
};

const PROVIDER_PROFILE_SELECT_SQL: &str =
    "SELECT id, name, cli_type, is_active, api_key, base_url, provider_name, main_model, reasoning_model, haiku_model, sonnet_default, opus_default, codex_model, created_at, updated_at FROM provider_profiles";
const PROVIDER_PROFILE_SELECT_BY_ID_SQL: &str =
    "SELECT id, name, cli_type, is_active, api_key, base_url, provider_name, main_model, reasoning_model, haiku_model, sonnet_default, opus_default, codex_model, created_at, updated_at FROM provider_profiles WHERE id = ?1";
const PROVIDER_PROFILE_SELECT_ACTIVE_SQL: &str =
    "SELECT id, name, cli_type, is_active, api_key, base_url, provider_name, main_model, reasoning_model, haiku_model, sonnet_default, opus_default, codex_model, created_at, updated_at FROM provider_profiles WHERE cli_type = ?1 AND is_active = 1";

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
        created_at: now.clone(),
        updated_at: now,
    }
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
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
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
        "INSERT INTO provider_profiles (id, name, cli_type, is_active, api_key, base_url, provider_name, main_model, reasoning_model, haiku_model, sonnet_default, opus_default, codex_model, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
    bind_value(&mut stmt, &mut param_count, &id).map_err(|e| e.to_string())?;

    stmt.raw_execute().map_err(|e| e.to_string())?;

    // 获取更新后的配置
    get_provider_profile_by_id(&conn, &id)
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
        _ => {
            return Err(format!("Unknown CLI type: {}", profile.cli_type));
        }
    }

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
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
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
        _ => Err(format!("Unknown CLI type: {}", cli_type)),
    }
}

/// CLI 连接信息
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
        _ => Err(format!("Unknown CLI type: {}", cli_type)),
    }
}

/// 脱敏 API Key
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
                api_key_masked: None,
                api_key: None,
                is_valid: false,
                error_message: Some(e),
            });
        }
    }

    Ok(connections)
}
