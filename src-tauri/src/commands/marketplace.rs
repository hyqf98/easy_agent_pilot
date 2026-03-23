use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Market source type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum MarketSourceType {
    Github,
    RemoteJson,
    LocalDir,
}

impl From<&str> for MarketSourceType {
    fn from(s: &str) -> Self {
        match s {
            "github" => MarketSourceType::Github,
            "remote_json" => MarketSourceType::RemoteJson,
            "local_dir" => MarketSourceType::LocalDir,
            _ => MarketSourceType::Github,
        }
    }
}

impl std::fmt::Display for MarketSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketSourceType::Github => write!(f, "github"),
            MarketSourceType::RemoteJson => write!(f, "remote_json"),
            MarketSourceType::LocalDir => write!(f, "local_dir"),
        }
    }
}

/// Market source status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum MarketSourceStatus {
    Active,
    Inactive,
    Error,
}

impl From<&str> for MarketSourceStatus {
    fn from(s: &str) -> Self {
        match s {
            "active" => MarketSourceStatus::Active,
            "inactive" => MarketSourceStatus::Inactive,
            "error" => MarketSourceStatus::Error,
            _ => MarketSourceStatus::Active,
        }
    }
}

impl std::fmt::Display for MarketSourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketSourceStatus::Active => write!(f, "active"),
            MarketSourceStatus::Inactive => write!(f, "inactive"),
            MarketSourceStatus::Error => write!(f, "error"),
        }
    }
}

/// Market source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSource {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub url_or_path: String,
    pub status: String,
    pub enabled: bool,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Create market source input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMarketSourceInput {
    pub name: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub url_or_path: String,
}

/// Update market source input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMarketSourceInput {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub url_or_path: String,
    pub enabled: bool,
}

/// Test connection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
}

/// Get database connection
fn get_db_connection() -> Result<Connection, String> {
    let persistence_dir = crate::commands::get_persistence_dir_path().map_err(|e| e.to_string())?;
    let db_path = persistence_dir.join("data").join("easy-agent.db");
    Connection::open(&db_path).map_err(|e| e.to_string())
}

/// List all market sources (Tauri command)
#[tauri::command]
pub fn list_market_sources() -> Result<Vec<MarketSource>, String> {
    let conn = get_db_connection()?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, type, url_or_path, status, enabled, last_synced_at, created_at, updated_at FROM market_sources ORDER BY created_at DESC"
        )
        .map_err(|e| e.to_string())?;

    let sources = stmt
        .query_map([], |row| {
            Ok(MarketSource {
                id: row.get(0)?,
                name: row.get(1)?,
                source_type: row.get(2)?,
                url_or_path: row.get(3)?,
                status: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                last_synced_at: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(sources)
}

/// Check if name is duplicate
fn check_name_duplicate(
    conn: &Connection,
    name: &str,
    exclude_id: Option<&str>,
) -> Result<bool, String> {
    let count = match exclude_id {
        Some(id) => {
            let mut stmt = conn
                .prepare("SELECT COUNT(*) FROM market_sources WHERE name = ?1 AND id != ?2")
                .map_err(|e| e.to_string())?;
            stmt.query_row(params![name, id], |row| row.get::<_, i32>(0))
                .map_err(|e| e.to_string())?
        }
        None => {
            let mut stmt = conn
                .prepare("SELECT COUNT(*) FROM market_sources WHERE name = ?1")
                .map_err(|e| e.to_string())?;
            stmt.query_row(params![name], |row| row.get::<_, i32>(0))
                .map_err(|e| e.to_string())?
        }
    };

    Ok(count > 0)
}

/// Add market source (Tauri command)
#[tauri::command]
pub fn add_market_source(input: CreateMarketSourceInput) -> Result<MarketSource, String> {
    let conn = get_db_connection()?;

    // Validate name
    if input.name.trim().is_empty() {
        return Err("名称不能为空".to_string());
    }

    // Validate URL/path
    if input.url_or_path.trim().is_empty() {
        return Err("URL/路径不能为空".to_string());
    }

    // Validate source type
    let source_type = input.source_type.trim();
    if !matches!(source_type, "github" | "remote_json" | "local_dir") {
        return Err("无效的市场源类型".to_string());
    }

    // Check for duplicate name
    if check_name_duplicate(&conn, &input.name, None)? {
        return Err("名称已存在，请使用其他名称".to_string());
    }

    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO market_sources (id, name, type, url_or_path, status, enabled, last_synced_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![id, input.name, source_type, input.url_or_path, "active", 1, None::<String>, now, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(MarketSource {
        id,
        name: input.name,
        source_type: source_type.to_string(),
        url_or_path: input.url_or_path,
        status: "active".to_string(),
        enabled: true,
        last_synced_at: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Update market source (Tauri command)
#[tauri::command]
pub fn update_market_source(input: UpdateMarketSourceInput) -> Result<MarketSource, String> {
    let conn = get_db_connection()?;

    // Validate name
    if input.name.trim().is_empty() {
        return Err("名称不能为空".to_string());
    }

    // Validate URL/path
    if input.url_or_path.trim().is_empty() {
        return Err("URL/路径不能为空".to_string());
    }

    // Validate source type
    let source_type = input.source_type.trim();
    if !matches!(source_type, "github" | "remote_json" | "local_dir") {
        return Err("无效的市场源类型".to_string());
    }

    // Check for duplicate name
    if check_name_duplicate(&conn, &input.name, Some(&input.id))? {
        return Err("名称已存在，请使用其他名称".to_string());
    }

    let now = Utc::now().to_rfc3339();
    let enabled_int = if input.enabled { 1 } else { 0 };

    conn.execute(
        "UPDATE market_sources SET name = ?1, type = ?2, url_or_path = ?3, enabled = ?4, updated_at = ?5 WHERE id = ?6",
        params![input.name, source_type, input.url_or_path, enabled_int, now, input.id],
    )
    .map_err(|e| e.to_string())?;

    Ok(MarketSource {
        id: input.id,
        name: input.name,
        source_type: source_type.to_string(),
        url_or_path: input.url_or_path,
        status: "active".to_string(),
        enabled: input.enabled,
        last_synced_at: None,
        created_at: String::new(),
        updated_at: now,
    })
}

/// Delete market source (Tauri command)
#[tauri::command]
pub fn delete_market_source(id: String) -> Result<(), String> {
    let conn = get_db_connection()?;

    conn.execute("DELETE FROM market_sources WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Toggle market source enabled status (Tauri command)
#[tauri::command]
pub fn toggle_market_source(id: String, enabled: bool) -> Result<(), String> {
    let conn = get_db_connection()?;
    let now = Utc::now().to_rfc3339();
    let enabled_int = if enabled { 1 } else { 0 };

    conn.execute(
        "UPDATE market_sources SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
        params![enabled_int, now, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Update market source status in database
fn update_market_source_status(conn: &Connection, id: &str, status: &str) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE market_sources SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, now, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Test and update market source status (Tauri command)
#[tauri::command]
pub async fn test_and_update_market_source(id: String) -> Result<TestConnectionResult, String> {
    // Get the market source info first (synchronously)
    let (source_type, url_or_path) = {
        let conn = get_db_connection()?;
        let mut stmt = conn
            .prepare("SELECT type, url_or_path FROM market_sources WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        stmt.query_row(params![&id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
    }; // conn and stmt are dropped here

    // Test connection (async)
    let result = test_market_source_connection_impl(source_type, url_or_path).await;

    // Update status based on result (synchronously)
    let status = if result.success { "active" } else { "error" };
    let conn = get_db_connection()?;
    update_market_source_status(&conn, &id, status)?;

    Ok(result)
}

/// Test market source connection implementation
async fn test_market_source_connection_impl(
    source_type: String,
    url_or_path: String,
) -> TestConnectionResult {
    match source_type.as_str() {
        "github" => {
            // Validate GitHub format: owner/repo
            let parts: Vec<&str> = url_or_path.split('/').collect();
            if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
                return TestConnectionResult {
                    success: false,
                    message: "GitHub 格式应为 owner/repo".to_string(),
                };
            }

            // Try to fetch marketplace.json from GitHub
            let url = format!(
                "https://raw.githubusercontent.com/{}/{}/main/marketplace.json",
                parts[0], parts[1]
            );

            match reqwest::Client::new()
                .get(&url)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        TestConnectionResult {
                            success: true,
                            message: "成功连接到 GitHub 仓库".to_string(),
                        }
                    } else {
                        TestConnectionResult {
                            success: false,
                            message: format!(
                                "无法访问 marketplace.json: HTTP {}",
                                response.status()
                            ),
                        }
                    }
                }
                Err(e) => TestConnectionResult {
                    success: false,
                    message: format!("连接失败: {}", e),
                },
            }
        }
        "remote_json" => {
            // Try to fetch remote JSON
            match reqwest::Client::new()
                .get(&url_or_path)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        // Validate JSON format
                        match response.text().await {
                            Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                                Ok(_) => TestConnectionResult {
                                    success: true,
                                    message: "成功获取并验证 JSON 配置".to_string(),
                                },
                                Err(e) => TestConnectionResult {
                                    success: false,
                                    message: format!("JSON 解析失败: {}", e),
                                },
                            },
                            Err(e) => TestConnectionResult {
                                success: false,
                                message: format!("读取响应失败: {}", e),
                            },
                        }
                    } else {
                        TestConnectionResult {
                            success: false,
                            message: format!("HTTP 错误: {}", response.status()),
                        }
                    }
                }
                Err(e) => TestConnectionResult {
                    success: false,
                    message: format!("连接失败: {}", e),
                },
            }
        }
        "local_dir" => {
            // Check if directory exists and contains marketplace.json
            let path = std::path::Path::new(&url_or_path);
            if !path.exists() {
                return TestConnectionResult {
                    success: false,
                    message: "目录不存在".to_string(),
                };
            }

            if !path.is_dir() {
                return TestConnectionResult {
                    success: false,
                    message: "路径不是目录".to_string(),
                };
            }

            let marketplace_path = path.join("marketplace.json");
            if !marketplace_path.exists() {
                return TestConnectionResult {
                    success: false,
                    message: "目录中未找到 marketplace.json".to_string(),
                };
            }

            // Try to read and validate JSON
            match std::fs::read_to_string(&marketplace_path) {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(_) => TestConnectionResult {
                        success: true,
                        message: "成功读取并验证 marketplace.json".to_string(),
                    },
                    Err(e) => TestConnectionResult {
                        success: false,
                        message: format!("JSON 解析失败: {}", e),
                    },
                },
                Err(e) => TestConnectionResult {
                    success: false,
                    message: format!("读取文件失败: {}", e),
                },
            }
        }
        _ => TestConnectionResult {
            success: false,
            message: "未知的市场源类型".to_string(),
        },
    }
}

/// Test market source connection (Tauri command)
#[tauri::command]
pub async fn test_market_source_connection(
    source_type: String,
    url_or_path: String,
) -> Result<TestConnectionResult, String> {
    Ok(test_market_source_connection_impl(source_type, url_or_path).await)
}
