use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
// MCP SDK (官方 Rust 实现)
use rmcp::{
    model::CallToolRequestParams,
    transport::{StreamableHttpClientTransport, TokioChildProcess},
    ServiceExt,
};

// 导入内置 MCP 服务器
use super::builtin_mcp::{
    call_builtin_tool, get_builtin_tools, BUILTIN_SERVER_ID, BUILTIN_SERVER_NAME,
};
use super::mcp_shared::{
    build_stdio_command, ensure_mcp_servers_object, format_call_tool_result, parse_args_string,
    read_json_config_or_default, write_json_config_pretty,
};
use super::support::open_db_connection;

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub server_type: String,
    pub command: String,
    pub args: Option<String>,
    pub env: Option<String>,
    pub url: Option<String>,
    pub headers: Option<String>,
    pub enabled: bool,
    pub test_status: Option<String>,
    pub test_message: Option<String>,
    pub tool_count: Option<i32>,
    pub tested_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// MCP 连接测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTestResult {
    pub success: bool,
    pub message: String,
    pub tool_count: Option<i32>,
}

type StoredServerRuntime = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// 创建 MCP 服务器的输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMcpServerInput {
    pub name: String,
    pub server_type: String,
    pub command: String,
    pub args: Option<String>,
    pub env: Option<String>,
    pub url: Option<String>,
    pub headers: Option<String>,
}

/// 更新 MCP 服务器的输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMcpServerInput {
    pub id: String,
    pub name: String,
    pub server_type: String,
    pub command: String,
    pub args: Option<String>,
    pub env: Option<String>,
    pub url: Option<String>,
    pub headers: Option<String>,
    pub enabled: bool,
}

/// 获取 MCP 配置文件路径
/// stdio 类型写入 ~/.claude.json
/// http 类型写入 ~/.config/easy-agent/mcp-http.json
fn get_mcp_config_path(server_type: &str) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;

    match server_type {
        "stdio" => Ok(home.join(".claude.json")),
        "http" => {
            let config_dir = home.join(".config").join("easy-agent");
            if !config_dir.exists() {
                let _ = fs::create_dir_all(&config_dir);
            }
            Ok(config_dir.join("mcp-http.json"))
        }
        _ => Err(format!("Unknown server type: {}", server_type)),
    }
}

/// 将 MCP 服务器配置写入配置文件
fn write_mcp_to_config_file(server: &McpServer) -> Result<(), String> {
    let config_path = get_mcp_config_path(&server.server_type)?;

    let mut config = read_json_config_or_default(&config_path, json!({"mcpServers": {}}))?;

    // 根据服务器类型构建配置
    let mcp_config = match server.server_type.as_str() {
        "stdio" => {
            let mut cfg = json!({
                "command": server.command,
            });
            if let Some(args) = &server.args {
                if !args.trim().is_empty() {
                    cfg["args"] = json!(parse_args_string(Some(args)));
                }
            }
            if let Some(env) = &server.env {
                if !env.trim().is_empty() {
                    if let Ok(env_map) = serde_json::from_str::<Value>(env) {
                        cfg["env"] = env_map;
                    }
                }
            }
            cfg
        }
        "http" => {
            let mut cfg = json!({
                "type": "http",
            });
            if let Some(url) = &server.url {
                cfg["url"] = json!(url);
            }
            if let Some(headers) = &server.headers {
                if !headers.trim().is_empty() {
                    if let Ok(headers_map) = serde_json::from_str::<Value>(headers) {
                        cfg["headers"] = headers_map;
                    }
                }
            }
            cfg
        }
        _ => json!({}),
    };

    let mcp_servers = ensure_mcp_servers_object(&mut config)?;
    mcp_servers.insert(server.name.clone(), mcp_config);

    write_json_config_pretty(&config_path, &config)?;

    Ok(())
}

/// 从配置文件中删除 MCP 服务器
fn remove_mcp_from_config_file(server_type: &str, server_name: &str) -> Result<(), String> {
    let config_path = get_mcp_config_path(server_type)?;

    if !config_path.exists() {
        return Ok(());
    }

    let mut config = read_json_config_or_default(&config_path, json!({"mcpServers": {}}))?;
    let mcp_servers = ensure_mcp_servers_object(&mut config)?;
    mcp_servers.remove(server_name);
    write_json_config_pretty(&config_path, &config)?;

    Ok(())
}

/// 获取所有 MCP 服务器配置 (Tauri 命令)
#[tauri::command]
pub fn list_mcp_servers() -> Result<Vec<McpServer>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, server_type, command, args, env, url, headers, enabled, test_status, test_message, tool_count, tested_at, created_at, updated_at FROM mcp_servers ORDER BY created_at DESC"
        )
        .map_err(|e| e.to_string())?;

    let mut servers = stmt
        .query_map([], |row| {
            Ok(McpServer {
                id: row.get(0)?,
                name: row.get(1)?,
                server_type: row.get(2)?,
                command: row.get(3)?,
                args: row.get(4)?,
                env: row.get(5)?,
                url: row.get(6)?,
                headers: row.get(7)?,
                enabled: row.get::<_, i32>(8)? != 0,
                test_status: row.get(9)?,
                test_message: row.get(10)?,
                tool_count: row.get(11)?,
                tested_at: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 添加内置服务器到列表开头
    let builtin_server = McpServer {
        id: BUILTIN_SERVER_ID.to_string(),
        name: BUILTIN_SERVER_NAME.to_string(),
        server_type: "builtin".to_string(),
        command: String::new(),
        args: None,
        env: None,
        url: None,
        headers: None,
        enabled: true,
        test_status: Some("success".to_string()),
        test_message: Some("内置服务器始终可用".to_string()),
        tool_count: Some(get_builtin_tools().len() as i32),
        tested_at: Some(Utc::now().to_rfc3339()),
        created_at: String::new(),
        updated_at: String::new(),
    };

    servers.insert(0, builtin_server);

    Ok(servers)
}

/// 检查名称是否重复
fn check_name_duplicate(
    conn: &Connection,
    name: &str,
    exclude_id: Option<&str>,
) -> Result<bool, String> {
    let count = match exclude_id {
        Some(id) => {
            let mut stmt = conn
                .prepare("SELECT COUNT(*) FROM mcp_servers WHERE name = ?1 AND id != ?2")
                .map_err(|e| e.to_string())?;
            stmt.query_row(params![name, id], |row| row.get::<_, i32>(0))
                .map_err(|e| e.to_string())?
        }
        None => {
            let mut stmt = conn
                .prepare("SELECT COUNT(*) FROM mcp_servers WHERE name = ?1")
                .map_err(|e| e.to_string())?;
            stmt.query_row(params![name], |row| row.get::<_, i32>(0))
                .map_err(|e| e.to_string())?
        }
    };

    Ok(count > 0)
}

fn validate_server_input(
    name: &str,
    server_type: &str,
    command: &str,
    url: Option<&str>,
) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("名称不能为空".to_string());
    }

    let normalized_type = server_type.trim().to_string();
    if normalized_type.is_empty() {
        return Err("服务器类型不能为空".to_string());
    }

    if normalized_type == "stdio" && command.trim().is_empty() {
        return Err("命令不能为空".to_string());
    }

    if normalized_type == "http" && url.unwrap_or("").trim().is_empty() {
        return Err("URL不能为空".to_string());
    }

    Ok(normalized_type)
}

fn load_runtime_server_config(id: &str) -> Result<StoredServerRuntime, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT name, server_type, command, args, env, url, headers FROM mcp_servers WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;

    stmt.query_row(params![id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
        ))
    })
    .map_err(|e| format!("Server not found: {}", e))
}

fn serialize_optional_map(
    map: Option<&std::collections::HashMap<String, String>>,
) -> Option<String> {
    map.map(|value| serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()))
}

fn normalize_runtime_config(config: McpConfigInput) -> StoredServerRuntime {
    (
        config.name,
        config.transport_type,
        config.command.unwrap_or_default(),
        config.args.map(|args| args.join(" ")),
        serialize_optional_map(config.env.as_ref()),
        config.url,
        serialize_optional_map(config.headers.as_ref()),
    )
}

fn map_rmcp_tools(tools: Vec<rmcp::model::Tool>) -> Vec<McpTool> {
    tools
        .into_iter()
        .map(|tool| {
            let input_schema =
                serde_json::to_value(&tool.input_schema).unwrap_or_else(|_| json!({}));
            McpTool {
                name: tool.name.to_string(),
                description: tool.description.unwrap_or_default().to_string(),
                input_schema,
            }
        })
        .collect()
}

/// 添加 MCP 服务器配置 (Tauri 命令)
#[tauri::command]
pub fn add_mcp_server(input: CreateMcpServerInput) -> Result<McpServer, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let server_type = validate_server_input(
        &input.name,
        &input.server_type,
        &input.command,
        input.url.as_deref(),
    )?;

    // 检查名称是否重复
    if check_name_duplicate(&conn, &input.name, None)? {
        return Err("名称已存在，请使用其他名称".to_string());
    }

    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO mcp_servers (id, name, server_type, command, args, env, url, headers, enabled, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![id, input.name, server_type, input.command, input.args, input.env, input.url, input.headers, 1, now, now],
    )
    .map_err(|e| e.to_string())?;

    let server = McpServer {
        id,
        name: input.name,
        server_type: server_type.clone(),
        command: input.command,
        args: input.args,
        env: input.env,
        url: input.url,
        headers: input.headers,
        enabled: true,
        test_status: None,
        test_message: None,
        tool_count: None,
        tested_at: None,
        created_at: now.clone(),
        updated_at: now,
    };

    // 写入配置文件
    if let Err(e) = write_mcp_to_config_file(&server) {
        eprintln!("警告: 写入 MCP 配置文件失败: {}", e);
        // 继续返回成功，因为数据库写入已成功
    }

    Ok(server)
}

/// 更新 MCP 服务器配置 (Tauri 命令)
#[tauri::command]
pub fn update_mcp_server(input: UpdateMcpServerInput) -> Result<McpServer, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let server_type = validate_server_input(
        &input.name,
        &input.server_type,
        &input.command,
        input.url.as_deref(),
    )?;

    // 检查名称是否重复
    if check_name_duplicate(&conn, &input.name, Some(&input.id))? {
        return Err("名称已存在，请使用其他名称".to_string());
    }

    // 获取旧的服务器信息（用于配置文件更新）
    let old_server: Option<(String, String)> = conn
        .query_row(
            "SELECT server_type, name FROM mcp_servers WHERE id = ?1",
            params![input.id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .ok();

    let now = Utc::now().to_rfc3339();
    let enabled_int = if input.enabled { 1 } else { 0 };

    conn.execute(
        "UPDATE mcp_servers SET name = ?1, server_type = ?2, command = ?3, args = ?4, env = ?5, url = ?6, headers = ?7, enabled = ?8, updated_at = ?9 WHERE id = ?10",
        params![input.name, server_type, input.command, input.args, input.env, input.url, input.headers, enabled_int, now, input.id],
    )
    .map_err(|e| e.to_string())?;

    // 构建更新后的服务器对象
    let updated_server = McpServer {
        id: input.id.clone(),
        name: input.name.clone(),
        server_type: server_type.clone(),
        command: input.command,
        args: input.args,
        env: input.env,
        url: input.url,
        headers: input.headers,
        enabled: input.enabled,
        test_status: None,
        test_message: None,
        tool_count: None,
        tested_at: None,
        created_at: String::new(),
        updated_at: now,
    };

    // 更新配置文件
    if let Some((old_type, old_name)) = old_server {
        // 如果类型或名称发生变化，从旧配置文件中删除
        if old_type != server_type || old_name != input.name {
            let _ = remove_mcp_from_config_file(&old_type, &old_name);
        }
    }
    // 写入新配置
    let _ = write_mcp_to_config_file(&updated_server);

    Ok(updated_server)
}

/// 删除 MCP 服务器配置 (Tauri 命令)
#[tauri::command]
pub fn delete_mcp_server(id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    // 先获取服务器信息，以便删除配置文件
    let server: Option<(String, String)> = conn
        .query_row(
            "SELECT server_type, name FROM mcp_servers WHERE id = ?1",
            params![id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .ok();

    // 从数据库删除
    conn.execute("DELETE FROM mcp_servers WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    // 从配置文件删除
    if let Some((server_type, name)) = server {
        let _ = remove_mcp_from_config_file(&server_type, &name);
    }

    Ok(())
}

/// 切换 MCP 服务器启用状态 (Tauri 命令)
#[tauri::command]
pub fn toggle_mcp_server(id: String, enabled: bool) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let enabled_int = if enabled { 1 } else { 0 };

    conn.execute(
        "UPDATE mcp_servers SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
        params![enabled_int, now, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// 测试 MCP 服务器连接 (Tauri 命令)
#[tauri::command]
pub async fn test_mcp_connection(id: String) -> Result<McpTestResult, String> {
    let (name, server_type, command, args, env, url, headers) = load_runtime_server_config(&id)?;

    // 根据服务器类型执行不同的测试逻辑
    let test_result = match server_type.as_str() {
        "http" => test_http_mcp(&name, url, headers).await,
        _ => test_stdio_mcp(&name, &command, args, env).await,
    };

    // 保存测试结果到数据库
    let now = Utc::now().to_rfc3339();
    let status = if test_result.success {
        "success"
    } else {
        "failed"
    };

    // 重新获取连接来更新数据库
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE mcp_servers SET test_status = ?1, test_message = ?2, tool_count = ?3, tested_at = ?4 WHERE id = ?5",
        params![status, test_result.message.clone(), test_result.tool_count, now, id],
    )
    .map_err(|e| format!("Failed to save test result: {}", e))?;

    Ok(test_result)
}

/// 测试 HTTP 类型 MCP 服务器
async fn test_http_mcp(name: &str, url: Option<String>, _headers: Option<String>) -> McpTestResult {
    let url = match url {
        Some(u) if !u.trim().is_empty() => u,
        _ => {
            return McpTestResult {
                success: false,
                message: "HTTP URL不能为空".to_string(),
                tool_count: None,
            };
        }
    };

    eprintln!(
        "[MCP] Using rmcp HTTP transport to test HTTP MCP server: {}",
        url
    );

    // 使用 rmcp 的 StreamableHttpClientTransport 连接到 HTTP MCP 服务器
    let transport = StreamableHttpClientTransport::from_uri(url.as_str());

    // 使用 rmcp 连接到服务器
    let service = match ().serve(transport).await {
        Ok(s) => s,
        Err(e) => {
            return McpTestResult {
                success: false,
                message: format!("连接 MCP 服务器失败: {}", e),
                tool_count: None,
            };
        }
    };

    // 获取工具列表以验证连接
    let tools_result = match service.list_tools(Default::default()).await {
        Ok(result) => result,
        Err(e) => {
            let _ = service.cancel().await;
            return McpTestResult {
                success: false,
                message: format!("获取工具列表失败: {}", e),
                tool_count: None,
            };
        }
    };

    let tool_count = Some(tools_result.tools.len() as i32);

    // 关闭连接
    let _ = service.cancel().await;

    McpTestResult {
        success: true,
        message: format!("连接成功，服务器「{}」可用", name),
        tool_count,
    }
}

/// 测试 stdio 类型 MCP 服务器（使用官方 rmcp SDK）
async fn test_stdio_mcp(
    name: &str,
    command: &str,
    args: Option<String>,
    env: Option<String>,
) -> McpTestResult {
    eprintln!("[MCP] Using rmcp SDK to test stdio MCP server: {}", name);
    let cmd = build_stdio_command(command, args.as_deref(), env.as_deref());

    // 使用 rmcp 的 TokioChildProcess 启动 MCP 服务器
    let transport = match TokioChildProcess::new(cmd) {
        Ok(t) => t,
        Err(e) => {
            return McpTestResult {
                success: false,
                message: format!("启动 MCP 进程失败: {} (命令: {})", e, command),
                tool_count: None,
            };
        }
    };

    // 使用 rmcp 连接到服务器
    let service = match ().serve(transport).await {
        Ok(s) => s,
        Err(e) => {
            return McpTestResult {
                success: false,
                message: format!("连接 MCP 服务器失败: {}", e),
                tool_count: None,
            };
        }
    };

    // 获取工具列表以验证连接
    let tools_result = match service.list_tools(Default::default()).await {
        Ok(result) => result,
        Err(e) => {
            let _ = service.cancel().await;
            return McpTestResult {
                success: false,
                message: format!("获取工具列表失败: {}", e),
                tool_count: None,
            };
        }
    };

    let tool_count = Some(tools_result.tools.len() as i32);

    // 关闭连接
    let _ = service.cancel().await;

    McpTestResult {
        success: true,
        message: format!("连接成功，服务器「{}」可用", name),
        tool_count,
    }
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// MCP Tools List Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolsListResult {
    pub success: bool,
    pub message: String,
    pub tools: Vec<McpTool>,
}

/// MCP Tool Call Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallResult {
    pub success: bool,
    pub message: String,
    pub result: Value,
    pub error: Option<String>,
}

/// 列出 MCP 服务器的工具列表
#[tauri::command]
pub async fn list_mcp_tools(server_id: String) -> Result<McpToolsListResult, String> {
    // 检查是否是内置服务器
    if server_id == BUILTIN_SERVER_ID {
        let tools = get_builtin_tools();
        return Ok(McpToolsListResult {
            success: true,
            message: format!("成功获取 {} 个内置工具", tools.len()),
            tools: tools
                .into_iter()
                .map(|t| McpTool {
                    name: t.name,
                    description: t.description,
                    input_schema: t.input_schema,
                })
                .collect(),
        });
    }

    let (name, server_type, command, args, env, url, headers) =
        load_runtime_server_config(&server_id)?;

    // 根据服务器类型执行不同的获取工具列表逻辑
    match server_type.as_str() {
        "http" => list_http_mcp_tools(&name, url, headers).await,
        _ => list_stdio_mcp_tools(&name, &command, args, env).await,
    }
}

/// 列出 HTTP 类型 MCP 服务器的工具
async fn list_http_mcp_tools(
    _name: &str,
    url: Option<String>,
    _headers: Option<String>,
) -> Result<McpToolsListResult, String> {
    let url = match url {
        Some(u) if !u.trim().is_empty() => u,
        _ => {
            return Ok(McpToolsListResult {
                success: false,
                message: "HTTP URL 不能为空".to_string(),
                tools: vec![],
            });
        }
    };

    eprintln!(
        "[MCP] Using rmcp HTTP transport to connect to HTTP MCP server: {}",
        url
    );

    // 使用 rmcp 的 StreamableHttpClientTransport 连接到 HTTP MCP 服务器
    let transport = StreamableHttpClientTransport::from_uri(url.as_str());

    // 使用 rmcp 连接到服务器
    eprintln!("[MCP] Connecting to HTTP MCP server...");
    let service = match ().serve(transport).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(McpToolsListResult {
                success: false,
                message: format!("连接 MCP 服务器失败: {}", e),
                tools: vec![],
            });
        }
    };

    eprintln!("[MCP] Connected, listing tools...");

    // 获取工具列表
    let tools_result = match service.list_tools(Default::default()).await {
        Ok(result) => result,
        Err(e) => {
            let _ = service.cancel().await;
            return Ok(McpToolsListResult {
                success: false,
                message: format!("获取工具列表失败: {}", e),
                tools: vec![],
            });
        }
    };

    let tools = map_rmcp_tools(tools_result.tools);

    eprintln!("[MCP] Found {} tools", tools.len());

    // 关闭连接
    let _ = service.cancel().await;

    Ok(McpToolsListResult {
        success: true,
        message: format!("成功获取 {} 个工具", tools.len()),
        tools,
    })
}

/// 列出 stdio 类型 MCP 服务器的工具（使用官方 rmcp SDK）
async fn list_stdio_mcp_tools(
    _name: &str,
    command: &str,
    args: Option<String>,
    env: Option<String>,
) -> Result<McpToolsListResult, String> {
    eprintln!("[MCP] Using rmcp SDK to connect to stdio MCP server");

    let cmd = build_stdio_command(command, args.as_deref(), env.as_deref());

    // 使用 rmcp 的 TokioChildProcess 启动 MCP 服务器
    let transport = match TokioChildProcess::new(cmd) {
        Ok(t) => t,
        Err(e) => {
            return Ok(McpToolsListResult {
                success: false,
                message: format!("启动 MCP 进程失败: {} (命令: {})", e, command),
                tools: vec![],
            });
        }
    };

    // 使用 rmcp 连接到服务器
    eprintln!("[MCP] Connecting to MCP server...");
    let service = match ().serve(transport).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(McpToolsListResult {
                success: false,
                message: format!("连接 MCP 服务器失败: {}", e),
                tools: vec![],
            });
        }
    };

    eprintln!("[MCP] Connected, listing tools...");

    // 获取工具列表
    let tools_result = match service.list_tools(Default::default()).await {
        Ok(result) => result,
        Err(e) => {
            let _ = service.cancel().await;
            return Ok(McpToolsListResult {
                success: false,
                message: format!("获取工具列表失败: {}", e),
                tools: vec![],
            });
        }
    };

    let tools = map_rmcp_tools(tools_result.tools);

    eprintln!("[MCP] Found {} tools", tools.len());

    // 关闭连接
    let _ = service.cancel().await;

    Ok(McpToolsListResult {
        success: true,
        message: format!("成功获取 {} 个工具", tools.len()),
        tools,
    })
}

/// 调用 MCP 工具
#[tauri::command]
pub async fn call_mcp_tool(
    server_id: String,
    tool_name: String,
    params: Value,
) -> Result<McpToolCallResult, String> {
    // 检查是否是内置服务器
    if server_id == BUILTIN_SERVER_ID {
        let result = call_builtin_tool(&tool_name, params).await?;
        return Ok(McpToolCallResult {
            success: true,
            message: "内置工具调用成功".to_string(),
            result,
            error: None,
        });
    }

    let (name, server_type, command, args, env, url, headers) =
        load_runtime_server_config(&server_id)?;

    // 根据服务器类型执行不同的调用逻辑
    match server_type.as_str() {
        "http" => call_http_mcp_tool(&name, url, headers, &tool_name, params).await,
        _ => call_stdio_mcp_tool(&name, &command, args, env, &tool_name, params).await,
    }
}

/// 调用 HTTP 类型 MCP 工具
async fn call_http_mcp_tool(
    _name: &str,
    url: Option<String>,
    _headers: Option<String>,
    tool_name: &str,
    params: Value,
) -> Result<McpToolCallResult, String> {
    let url = match url {
        Some(u) if !u.trim().is_empty() => u,
        _ => {
            return Ok(McpToolCallResult {
                success: false,
                message: "HTTP URL 不能为空".to_string(),
                result: Value::Null,
                error: Some("URL is empty".to_string()),
            });
        }
    };

    eprintln!(
        "[MCP] Using rmcp HTTP transport to call tool: {}",
        tool_name
    );

    // 使用 rmcp 的 StreamableHttpClientTransport 连接到 HTTP MCP 服务器
    let transport = StreamableHttpClientTransport::from_uri(url.as_str());

    // 使用 rmcp 连接到服务器
    let service = match ().serve(transport).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(McpToolCallResult {
                success: false,
                message: format!("连接 MCP 服务器失败: {}", e),
                result: Value::Null,
                error: Some(format!("{}", e)),
            });
        }
    };

    eprintln!("[MCP] Calling tool: {}", tool_name);

    // 构建调用参数
    let call_params = CallToolRequestParams {
        meta: None,
        name: tool_name.to_string().into(),
        arguments: params.as_object().cloned(),
        task: None,
    };

    // 调用工具
    let call_result = match service.call_tool(call_params).await {
        Ok(result) => result,
        Err(e) => {
            let _ = service.cancel().await;
            return Ok(McpToolCallResult {
                success: false,
                message: format!("调用工具失败: {}", e),
                result: Value::Null,
                error: Some(format!("{}", e)),
            });
        }
    };

    // 转换结果并进行 JSON 格式化
    let result_value = format_call_tool_result(&call_result);

    // 关闭连接
    let _ = service.cancel().await;

    Ok(McpToolCallResult {
        success: true,
        message: "工具调用成功".to_string(),
        result: result_value,
        error: None,
    })
}

/// 调用 stdio 类型 MCP 工具（使用官方 rmcp SDK）
async fn call_stdio_mcp_tool(
    _name: &str,
    command: &str,
    args: Option<String>,
    env: Option<String>,
    tool_name: &str,
    params: Value,
) -> Result<McpToolCallResult, String> {
    eprintln!("[MCP] Using rmcp SDK to call tool: {}", tool_name);

    let cmd = build_stdio_command(command, args.as_deref(), env.as_deref());

    // 使用 rmcp 的 TokioChildProcess 启动 MCP 服务器
    let transport = match TokioChildProcess::new(cmd) {
        Ok(t) => t,
        Err(e) => {
            return Ok(McpToolCallResult {
                success: false,
                message: format!("启动 MCP 进程失败: {}", e),
                result: Value::Null,
                error: Some(format!("{}", e)),
            });
        }
    };

    // 使用 rmcp 连接到服务器
    eprintln!("[MCP] Connecting to MCP server...");
    let service = match ().serve(transport).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(McpToolCallResult {
                success: false,
                message: format!("连接 MCP 服务器失败: {}", e),
                result: Value::Null,
                error: Some(format!("{}", e)),
            });
        }
    };

    eprintln!("[MCP] Calling tool: {}", tool_name);

    // 构建调用参数
    let call_params = CallToolRequestParams {
        meta: None,
        name: tool_name.to_string().into(),
        arguments: params.as_object().cloned(),
        task: None,
    };

    // 调用工具
    let call_result = match service.call_tool(call_params).await {
        Ok(result) => result,
        Err(e) => {
            let _ = service.cancel().await;
            return Ok(McpToolCallResult {
                success: false,
                message: format!("调用工具失败: {}", e),
                result: Value::Null,
                error: Some(format!("{}", e)),
            });
        }
    };

    // 转换结果并进行 JSON 格式化
    let result_value = format_call_tool_result(&call_result);

    // 关闭连接
    let _ = service.cancel().await;

    Ok(McpToolCallResult {
        success: true,
        message: "工具调用成功".to_string(),
        result: result_value,
        error: None,
    })
}

// ============================================================================
// 基于配置参数的 MCP 测试命令（不依赖数据库）
// ============================================================================

/// MCP 配置输入参数（用于测试，不依赖数据库）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfigInput {
    pub name: String,
    pub transport_type: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub url: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// 根据配置参数获取 MCP 工具列表
#[tauri::command]
pub async fn list_mcp_tools_by_config(
    config: McpConfigInput,
) -> Result<McpToolsListResult, String> {
    let (name, server_type, command, args, env, url, headers) = normalize_runtime_config(config);

    // 根据服务器类型执行不同的获取工具列表逻辑
    match server_type.as_str() {
        "http" | "sse" => list_http_mcp_tools(&name, url, headers).await,
        _ => list_stdio_mcp_tools(&name, &command, args, env).await,
    }
}

/// 根据配置参数调用 MCP 工具
#[tauri::command]
pub async fn call_mcp_tool_by_config(
    config: McpConfigInput,
    tool_name: String,
    params: Value,
) -> Result<McpToolCallResult, String> {
    let (name, server_type, command, args, env, url, headers) = normalize_runtime_config(config);

    // 根据服务器类型执行不同的调用逻辑
    match server_type.as_str() {
        "http" | "sse" => call_http_mcp_tool(&name, url, headers, &tool_name, params).await,
        _ => call_stdio_mcp_tool(&name, &command, args, env, &tool_name, params).await,
    }
}
