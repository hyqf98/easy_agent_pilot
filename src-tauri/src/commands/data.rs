use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 导出的数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,
    pub exported_at: String,
    pub projects: Vec<ProjectExport>,
    pub sessions: Vec<SessionExport>,
    pub messages: Vec<MessageExport>,
    pub agents: Vec<AgentExport>,
    pub mcp_servers: Vec<McpServerExport>,
    pub cli_paths: Vec<CliPathExport>,
    pub market_sources: Vec<MarketSourceExport>,
    pub app_settings: Vec<AppSettingExport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectExport {
    pub id: String,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionExport {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub agent_type: String,
    pub status: String,
    pub pinned: bool,
    pub last_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageExport {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub attachments: Option<String>,
    pub status: String,
    pub tokens: Option<i32>,
    pub error_message: Option<String>,
    pub tool_calls: Option<String>,
    pub thinking: Option<String>,
    pub edit_traces: Option<String>,
    pub runtime_notices: Option<String>,
    pub compression_metadata: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentExport {
    pub id: String,
    pub name: String,
    pub type_: String,
    pub mode: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub cli_path: Option<String>,
    pub status: Option<String>,
    pub test_message: Option<String>,
    pub tested_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerExport {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Option<String>,
    pub env: Option<String>,
    pub enabled: bool,
    pub test_status: Option<String>,
    pub test_message: Option<String>,
    pub tool_count: Option<i32>,
    pub tested_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CliPathExport {
    pub id: String,
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketSourceExport {
    pub id: String,
    pub name: String,
    pub type_: String,
    pub url_or_path: String,
    pub status: String,
    pub enabled: bool,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettingExport {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

/// 获取数据库路径
fn get_db_path() -> Result<PathBuf> {
    let persistence_dir = super::get_persistence_dir_path()?;
    Ok(persistence_dir.join("data").join("easy-agent.db"))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataManagementStats {
    pub storage_path: String,
    pub database_path: String,
    pub total_size_bytes: u64,
    pub session_data_size_bytes: u64,
    pub message_data_size_bytes: u64,
    pub log_data_size_bytes: u64,
    pub config_data_size_bytes: u64,
    pub project_count: i64,
    pub session_count: i64,
    pub message_count: i64,
    pub log_count: i64,
}

fn file_size(path: &PathBuf) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn total_database_size(db_path: &PathBuf) -> u64 {
    let mut total = file_size(db_path);

    if let Some(file_name) = db_path.file_name().and_then(|name| name.to_str()) {
        for suffix in ["-wal", "-shm"] {
            let sibling = db_path.with_file_name(format!("{}{}", file_name, suffix));
            total += file_size(&sibling);
        }
    }

    total
}

fn query_count(conn: &Connection, sql: &str) -> Result<i64, String> {
    conn.query_row(sql, [], |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())
}

fn query_size(conn: &Connection, sql: &str) -> Result<u64, String> {
    let value = conn
        .query_row(sql, [], |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())?;
    Ok(value.max(0) as u64)
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, String> {
    let count = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(count > 0)
}

#[tauri::command]
pub fn get_data_management_stats() -> Result<DataManagementStats, String> {
    let persistence_dir = super::get_persistence_dir_path().map_err(|e| e.to_string())?;
    let db_path = get_db_path().map_err(|e| e.to_string())?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    let project_count = query_count(&conn, "SELECT COUNT(*) FROM projects")?;
    let session_count = query_count(&conn, "SELECT COUNT(*) FROM sessions")?;
    let message_count = query_count(&conn, "SELECT COUNT(*) FROM messages")?;
    let log_count = query_count(&conn, "SELECT COUNT(*) FROM task_execution_logs")?
        + query_count(&conn, "SELECT COUNT(*) FROM task_execution_results")?
        + query_count(&conn, "SELECT COUNT(*) FROM task_split_sessions")?
        + query_count(&conn, "SELECT COUNT(*) FROM plan_split_logs")?;

    let session_data_size_bytes = query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(name, '')) +
            LENGTH(COALESCE(path, '')) +
            LENGTH(COALESCE(description, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM projects",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(project_id, '')) +
            LENGTH(COALESCE(name, '')) +
            LENGTH(COALESCE(agent_type, '')) +
            LENGTH(COALESCE(status, '')) +
            LENGTH(COALESCE(last_message, '')) +
            LENGTH(COALESCE(error_message, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM sessions",
    )?;

    let message_data_size_bytes = query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(session_id, '')) +
            LENGTH(COALESCE(role, '')) +
            LENGTH(COALESCE(content, '')) +
            LENGTH(COALESCE(attachments, '')) +
            LENGTH(COALESCE(status, '')) +
            LENGTH(COALESCE(error_message, '')) +
            LENGTH(COALESCE(tool_calls, '')) +
            LENGTH(COALESCE(thinking, '')) +
            LENGTH(COALESCE(edit_traces, '')) +
            LENGTH(COALESCE(runtime_notices, '')) +
            LENGTH(COALESCE(compression_metadata, '')) +
            LENGTH(COALESCE(created_at, ''))
        ), 0) FROM messages",
    )?;

    let log_data_size_bytes = query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(project_id, '')) +
            LENGTH(COALESCE(name, '')) +
            LENGTH(COALESCE(description, '')) +
            LENGTH(COALESCE(split_agent_id, '')) +
            LENGTH(COALESCE(split_model_id, '')) +
            LENGTH(COALESCE(status, '')) +
            LENGTH(COALESCE(agent_team, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM plans",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(plan_id, '')) +
            LENGTH(COALESCE(parent_id, '')) +
            LENGTH(COALESCE(title, '')) +
            LENGTH(COALESCE(description, '')) +
            LENGTH(COALESCE(status, '')) +
            LENGTH(COALESCE(priority, '')) +
            LENGTH(COALESCE(assignee, '')) +
            LENGTH(COALESCE(session_id, '')) +
            LENGTH(COALESCE(progress_file, '')) +
            LENGTH(COALESCE(dependencies, '')) +
            LENGTH(COALESCE(last_result_status, '')) +
            LENGTH(COALESCE(last_result_summary, '')) +
            LENGTH(COALESCE(last_result_files, '')) +
            LENGTH(COALESCE(last_fail_reason, '')) +
            LENGTH(COALESCE(last_result_at, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM tasks",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(plan_id, '')) +
            LENGTH(COALESCE(status, '')) +
            LENGTH(COALESCE(raw_content, '')) +
            LENGTH(COALESCE(parsed_output, '')) +
            LENGTH(COALESCE(parse_error, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, '')) +
            LENGTH(COALESCE(execution_session_id, '')) +
            LENGTH(COALESCE(execution_request_json, '')) +
            LENGTH(COALESCE(llm_messages_json, '')) +
            LENGTH(COALESCE(messages_json, '')) +
            LENGTH(COALESCE(form_queue_json, '')) +
            LENGTH(COALESCE(error_message, '')) +
            LENGTH(COALESCE(started_at, '')) +
            LENGTH(COALESCE(completed_at, '')) +
            LENGTH(COALESCE(stopped_at, ''))
        ), 0) FROM task_split_sessions",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(plan_id, '')) +
            LENGTH(COALESCE(session_id, '')) +
            LENGTH(COALESCE(log_type, '')) +
            LENGTH(COALESCE(content, '')) +
            LENGTH(COALESCE(metadata, '')) +
            LENGTH(COALESCE(created_at, ''))
        ), 0) FROM plan_split_logs",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(task_id, '')) +
            LENGTH(COALESCE(log_type, '')) +
            LENGTH(COALESCE(content, '')) +
            LENGTH(COALESCE(metadata, '')) +
            LENGTH(COALESCE(created_at, ''))
        ), 0) FROM task_execution_logs",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(task_id, '')) +
            LENGTH(COALESCE(plan_id, '')) +
            LENGTH(COALESCE(task_title_snapshot, '')) +
            LENGTH(COALESCE(task_description_snapshot, '')) +
            LENGTH(COALESCE(result_status, '')) +
            LENGTH(COALESCE(result_summary, '')) +
            LENGTH(COALESCE(result_files, '')) +
            LENGTH(COALESCE(fail_reason, '')) +
            LENGTH(COALESCE(created_at, ''))
        ), 0) FROM task_execution_results",
    )?;

    let config_data_size_bytes = query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(name, '')) +
            LENGTH(COALESCE(type, '')) +
            LENGTH(COALESCE(mode, '')) +
            LENGTH(COALESCE(api_key, '')) +
            LENGTH(COALESCE(base_url, '')) +
            LENGTH(COALESCE(model, '')) +
            LENGTH(COALESCE(cli_path, '')) +
            LENGTH(COALESCE(status, '')) +
            LENGTH(COALESCE(test_message, '')) +
            LENGTH(COALESCE(tested_at, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM agents",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(name, '')) +
            LENGTH(COALESCE(server_type, '')) +
            LENGTH(COALESCE(command, '')) +
            LENGTH(COALESCE(args, '')) +
            LENGTH(COALESCE(env, '')) +
            LENGTH(COALESCE(url, '')) +
            LENGTH(COALESCE(headers, '')) +
            LENGTH(COALESCE(test_status, '')) +
            LENGTH(COALESCE(test_message, '')) +
            LENGTH(COALESCE(tested_at, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM mcp_servers",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(name, '')) +
            LENGTH(COALESCE(path, '')) +
            LENGTH(COALESCE(version, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM cli_paths",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(name, '')) +
            LENGTH(COALESCE(type, '')) +
            LENGTH(COALESCE(url_or_path, '')) +
            LENGTH(COALESCE(status, '')) +
            LENGTH(COALESCE(last_synced_at, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM market_sources",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(key, '')) +
            LENGTH(COALESCE(value, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM app_settings",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(agent_id, '')) +
            LENGTH(COALESCE(model_id, '')) +
            LENGTH(COALESCE(display_name, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM agent_models",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(agent_id, '')) +
            LENGTH(COALESCE(name, '')) +
            LENGTH(COALESCE(transport_type, '')) +
            LENGTH(COALESCE(command, '')) +
            LENGTH(COALESCE(args, '')) +
            LENGTH(COALESCE(env, '')) +
            LENGTH(COALESCE(url, '')) +
            LENGTH(COALESCE(headers, '')) +
            LENGTH(COALESCE(scope, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM agent_mcp_configs",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(agent_id, '')) +
            LENGTH(COALESCE(name, '')) +
            LENGTH(COALESCE(description, '')) +
            LENGTH(COALESCE(skill_path, '')) +
            LENGTH(COALESCE(scripts_path, '')) +
            LENGTH(COALESCE(references_path, '')) +
            LENGTH(COALESCE(assets_path, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM agent_skills_configs",
    )? + query_size(
        &conn,
        "SELECT COALESCE(SUM(
            LENGTH(COALESCE(id, '')) +
            LENGTH(COALESCE(agent_id, '')) +
            LENGTH(COALESCE(name, '')) +
            LENGTH(COALESCE(version, '')) +
            LENGTH(COALESCE(description, '')) +
            LENGTH(COALESCE(plugin_path, '')) +
            LENGTH(COALESCE(created_at, '')) +
            LENGTH(COALESCE(updated_at, ''))
        ), 0) FROM agent_plugins_configs",
    )?;

    Ok(DataManagementStats {
        storage_path: persistence_dir.to_string_lossy().to_string(),
        database_path: db_path.to_string_lossy().to_string(),
        total_size_bytes: total_database_size(&db_path),
        session_data_size_bytes,
        message_data_size_bytes,
        log_data_size_bytes,
        config_data_size_bytes,
        project_count,
        session_count,
        message_count,
        log_count,
    })
}

/// 导出所有数据
#[tauri::command]
pub fn export_all_data() -> Result<ExportData, String> {
    let db_path = get_db_path().map_err(|e| e.to_string())?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    // 导出项目
    let projects = export_projects(&conn)?;
    // 导出会话
    let sessions = export_sessions(&conn)?;
    // 导出消息
    let messages = export_messages(&conn)?;
    // 导出智能体配置
    let agents = export_agents(&conn)?;
    // 导出 MCP 服务器配置
    let mcp_servers = export_mcp_servers(&conn)?;
    // 导出 CLI 路径配置
    let cli_paths = export_cli_paths(&conn)?;
    // 导出市场源配置
    let market_sources = export_market_sources(&conn)?;
    // 导出应用设置
    let app_settings = export_app_settings(&conn)?;

    Ok(ExportData {
        version: "1.1.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        projects,
        sessions,
        messages,
        agents,
        mcp_servers,
        cli_paths,
        market_sources,
        app_settings,
    })
}

/// 将数据导出到文件
#[tauri::command]
pub fn export_data_to_file(file_path: String) -> Result<String, String> {
    let data = export_all_data()?;
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;

    let path = PathBuf::from(&file_path);

    // 确保父目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    fs::write(&path, json).map_err(|e| e.to_string())?;

    Ok(file_path)
}

fn export_projects(conn: &Connection) -> Result<Vec<ProjectExport>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, path, description, created_at, updated_at FROM projects ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;

    let projects = stmt
        .query_map([], |row| {
            Ok(ProjectExport {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(projects)
}

fn export_sessions(conn: &Connection) -> Result<Vec<SessionExport>, String> {
    let mut stmt = conn
        .prepare("SELECT id, project_id, name, agent_type, status, pinned, last_message, created_at, updated_at FROM sessions ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;

    let sessions = stmt
        .query_map([], |row| {
            Ok(SessionExport {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                agent_type: row.get(3)?,
                status: row.get(4)?,
                pinned: row.get::<_, i32>(5)? != 0,
                last_message: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(sessions)
}

fn export_messages(conn: &Connection) -> Result<Vec<MessageExport>, String> {
    let mut stmt = conn
        .prepare("SELECT id, session_id, role, content, attachments, status, tokens, error_message, tool_calls, thinking, edit_traces, runtime_notices, compression_metadata, created_at FROM messages ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;

    let messages = stmt
        .query_map([], |row| {
            Ok(MessageExport {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                attachments: row.get(4)?,
                status: row.get(5)?,
                tokens: row.get(6)?,
                error_message: row.get(7)?,
                tool_calls: row.get(8)?,
                thinking: row.get(9)?,
                edit_traces: row.get(10)?,
                runtime_notices: row.get(11)?,
                compression_metadata: row.get(12)?,
                created_at: row.get(13)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(messages)
}

fn export_agents(conn: &Connection) -> Result<Vec<AgentExport>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, type, mode, api_key, base_url, model, cli_path, status, test_message, tested_at, created_at, updated_at FROM agents ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;

    let agents = stmt
        .query_map([], |row| {
            Ok(AgentExport {
                id: row.get(0)?,
                name: row.get(1)?,
                type_: row.get(2)?,
                mode: row.get(3)?,
                api_key: row.get(4)?,
                base_url: row.get(5)?,
                model: row.get(6)?,
                cli_path: row.get(7)?,
                status: row.get(8)?,
                test_message: row.get(9)?,
                tested_at: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(agents)
}

fn export_mcp_servers(conn: &Connection) -> Result<Vec<McpServerExport>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, command, args, env, enabled, test_status, test_message, tool_count, tested_at, created_at, updated_at FROM mcp_servers ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;

    let mcp_servers = stmt
        .query_map([], |row| {
            Ok(McpServerExport {
                id: row.get(0)?,
                name: row.get(1)?,
                command: row.get(2)?,
                args: row.get(3)?,
                env: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                test_status: row.get(6)?,
                test_message: row.get(7)?,
                tool_count: row.get(8)?,
                tested_at: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(mcp_servers)
}

fn export_cli_paths(conn: &Connection) -> Result<Vec<CliPathExport>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, path, version, created_at, updated_at FROM cli_paths ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;

    let cli_paths = stmt
        .query_map([], |row| {
            Ok(CliPathExport {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                version: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(cli_paths)
}

fn export_market_sources(conn: &Connection) -> Result<Vec<MarketSourceExport>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, type, url_or_path, status, enabled, last_synced_at, created_at, updated_at FROM market_sources ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;

    let market_sources = stmt
        .query_map([], |row| {
            Ok(MarketSourceExport {
                id: row.get(0)?,
                name: row.get(1)?,
                type_: row.get(2)?,
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

    Ok(market_sources)
}

fn export_app_settings(conn: &Connection) -> Result<Vec<AppSettingExport>, String> {
    let mut stmt = conn
        .prepare("SELECT key, value, updated_at FROM app_settings ORDER BY key ASC")
        .map_err(|e| e.to_string())?;

    let app_settings = stmt
        .query_map([], |row| {
            Ok(AppSettingExport {
                key: row.get(0)?,
                value: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(app_settings)
}

/// 导入结果统计
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub projects_imported: usize,
    pub sessions_imported: usize,
    pub messages_imported: usize,
    pub agents_imported: usize,
    pub mcp_servers_imported: usize,
    pub cli_paths_imported: usize,
    pub market_sources_imported: usize,
    pub app_settings_imported: usize,
}

/// 验证导入数据格式
#[tauri::command]
pub fn validate_import_data(file_path: String) -> Result<ExportData, String> {
    let content = fs::read_to_string(&file_path).map_err(|e| format!("无法读取文件: {}", e))?;

    let data: ExportData =
        serde_json::from_str(&content).map_err(|e| format!("无效的数据格式: {}", e))?;

    // 验证版本
    if data.version.is_empty() {
        return Err("缺少版本信息".to_string());
    }

    Ok(data)
}

/// 清除所有数据
#[tauri::command]
pub fn clear_all_data() -> Result<(), String> {
    let db_path = get_db_path().map_err(|e| e.to_string())?;
    let mut conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    // 开启事务
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // 按照外键依赖顺序删除（子表先删除）
    let delete_statements = [
        ("session_mcp", "DELETE FROM session_mcp"),
        ("messages", "DELETE FROM messages"),
        ("task_execution_logs", "DELETE FROM task_execution_logs"),
        (
            "task_execution_results",
            "DELETE FROM task_execution_results",
        ),
        ("plan_split_logs", "DELETE FROM plan_split_logs"),
        ("task_split_sessions", "DELETE FROM task_split_sessions"),
        ("window_session_locks", "DELETE FROM window_session_locks"),
        ("tasks", "DELETE FROM tasks"),
        ("plans", "DELETE FROM plans"),
        ("project_access_log", "DELETE FROM project_access_log"),
        ("memory_compressions", "DELETE FROM memory_compressions"),
        ("user_memories", "DELETE FROM user_memories"),
        ("raw_memory_records", "DELETE FROM raw_memory_records"),
        ("memory_merge_runs", "DELETE FROM memory_merge_runs"),
        (
            "project_memory_libraries",
            "DELETE FROM project_memory_libraries",
        ),
        ("memory_libraries", "DELETE FROM memory_libraries"),
        ("memory_categories", "DELETE FROM memory_categories"),
        (
            "installed_mcp_test_results",
            "DELETE FROM installed_mcp_test_results",
        ),
        ("mcp_install_history", "DELETE FROM mcp_install_history"),
        ("agent_mcp_configs", "DELETE FROM agent_mcp_configs"),
        ("agent_skills_configs", "DELETE FROM agent_skills_configs"),
        ("agent_plugins_configs", "DELETE FROM agent_plugins_configs"),
        ("agent_models", "DELETE FROM agent_models"),
        ("provider_profiles", "DELETE FROM provider_profiles"),
        ("employees", "DELETE FROM employees"),
        ("departments", "DELETE FROM departments"),
        ("sessions", "DELETE FROM sessions"),
        ("projects", "DELETE FROM projects"),
        ("agents", "DELETE FROM agents"),
        ("mcp_servers", "DELETE FROM mcp_servers"),
        ("cli_paths", "DELETE FROM cli_paths"),
        ("market_sources", "DELETE FROM market_sources"),
        ("skills", "DELETE FROM skills"),
        ("themes", "DELETE FROM themes"),
        ("app_state", "DELETE FROM app_state"),
        ("app_settings", "DELETE FROM app_settings"),
    ];

    for (table, sql) in delete_statements {
        if table_exists(&tx, table)? {
            tx.execute(sql, [])
                .map_err(|e| format!("删除 {} 失败: {}", table, e))?;
        }
    }

    // 提交事务
    tx.commit().map_err(|e| e.to_string())?;

    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")
        .map_err(|e| format!("压缩数据库失败: {}", e))?;

    Ok(())
}

/// 导出数据类型选项
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportOptions {
    pub include_projects: bool,
    pub include_sessions: bool,
    pub include_messages: bool,
    pub include_agents: bool,
    pub include_mcp_servers: bool,
    pub include_cli_paths: bool,
    pub include_market_sources: bool,
    pub include_app_settings: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            include_projects: true,
            include_sessions: true,
            include_messages: true,
            include_agents: true,
            include_mcp_servers: true,
            include_cli_paths: true,
            include_market_sources: true,
            include_app_settings: true,
        }
    }
}

/// 导出选定的数据类型
#[tauri::command]
pub fn export_selected_data(options: ExportOptions) -> Result<ExportData, String> {
    let db_path = get_db_path().map_err(|e| e.to_string())?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    let projects = if options.include_projects {
        export_projects(&conn)?
    } else {
        vec![]
    };

    let sessions = if options.include_sessions {
        export_sessions(&conn)?
    } else {
        vec![]
    };

    let messages = if options.include_messages {
        export_messages(&conn)?
    } else {
        vec![]
    };

    let agents = if options.include_agents {
        export_agents(&conn)?
    } else {
        vec![]
    };

    let mcp_servers = if options.include_mcp_servers {
        export_mcp_servers(&conn)?
    } else {
        vec![]
    };

    let cli_paths = if options.include_cli_paths {
        export_cli_paths(&conn)?
    } else {
        vec![]
    };

    let market_sources = if options.include_market_sources {
        export_market_sources(&conn)?
    } else {
        vec![]
    };

    let app_settings = if options.include_app_settings {
        export_app_settings(&conn)?
    } else {
        vec![]
    };

    Ok(ExportData {
        version: "1.1.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        projects,
        sessions,
        messages,
        agents,
        mcp_servers,
        cli_paths,
        market_sources,
        app_settings,
    })
}

/// 将选定的数据导出到文件
#[tauri::command]
pub fn export_selected_to_file(
    file_path: String,
    options: ExportOptions,
) -> Result<String, String> {
    let data = export_selected_data(options)?;
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;

    let path = PathBuf::from(&file_path);

    // 确保父目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    fs::write(&path, json).map_err(|e| e.to_string())?;

    Ok(file_path)
}

/// 从文件导入数据
#[tauri::command]
pub fn import_data_from_file(file_path: String) -> Result<ImportResult, String> {
    // 先验证数据格式
    let data = validate_import_data(file_path)?;

    // 获取数据库路径并打开连接
    let db_path = get_db_path().map_err(|e| e.to_string())?;
    let mut conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    // 开启事务
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut result = ImportResult {
        projects_imported: 0,
        sessions_imported: 0,
        messages_imported: 0,
        agents_imported: 0,
        mcp_servers_imported: 0,
        cli_paths_imported: 0,
        market_sources_imported: 0,
        app_settings_imported: 0,
    };

    // 导入项目
    for project in &data.projects {
        let res = tx.execute(
            "INSERT OR REPLACE INTO projects (id, name, path, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [
                &project.id,
                &project.name,
                &project.path,
                &project.description.clone().unwrap_or_default(),
                &project.created_at,
                &project.updated_at,
            ],
        );
        if res.is_ok() {
            result.projects_imported += 1;
        }
    }

    // 导入会话
    for session in &data.sessions {
        let pinned = if session.pinned { 1 } else { 0 };
        let res = tx.execute(
            "INSERT OR REPLACE INTO sessions (id, project_id, name, agent_type, status, pinned, last_message, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            [
                &session.id,
                &session.project_id,
                &session.name,
                &session.agent_type,
                &session.status,
                &pinned.to_string(),
                &session.last_message.clone().unwrap_or_default(),
                &session.created_at,
                &session.updated_at,
            ],
        );
        if res.is_ok() {
            result.sessions_imported += 1;
        }
    }

    // 导入消息
    for message in &data.messages {
        let tokens = message.tokens.map(|t| t.to_string()).unwrap_or_default();
        let res = tx.execute(
            "INSERT OR REPLACE INTO messages (id, session_id, role, content, attachments, status, tokens, error_message, tool_calls, thinking, edit_traces, runtime_notices, compression_metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            [
                &message.id,
                &message.session_id,
                &message.role,
                &message.content,
                &message.attachments.clone().unwrap_or_default(),
                &message.status,
                &tokens,
                &message.error_message.clone().unwrap_or_default(),
                &message.tool_calls.clone().unwrap_or_default(),
                &message.thinking.clone().unwrap_or_default(),
                &message.edit_traces.clone().unwrap_or_default(),
                &message.runtime_notices.clone().unwrap_or_default(),
                &message.compression_metadata.clone().unwrap_or_default(),
                &message.created_at,
            ],
        );
        if res.is_ok() {
            result.messages_imported += 1;
        }
    }

    // 导入智能体配置
    for agent in &data.agents {
        let res = tx.execute(
            "INSERT OR REPLACE INTO agents (id, name, type, mode, api_key, base_url, model, cli_path, status, test_message, tested_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            [
                &agent.id,
                &agent.name,
                &agent.type_,
                &agent.mode,
                &agent.api_key.clone().unwrap_or_default(),
                &agent.base_url.clone().unwrap_or_default(),
                &agent.model.clone().unwrap_or_default(),
                &agent.cli_path.clone().unwrap_or_default(),
                &agent.status.clone().unwrap_or_default(),
                &agent.test_message.clone().unwrap_or_default(),
                &agent.tested_at.clone().unwrap_or_default(),
                &agent.created_at,
                &agent.updated_at,
            ],
        );
        if res.is_ok() {
            result.agents_imported += 1;
        }
    }

    // 导入 MCP 服务器配置
    for server in &data.mcp_servers {
        let enabled = if server.enabled { 1 } else { 0 };
        let tool_count = server.tool_count.map(|t| t.to_string()).unwrap_or_default();
        let res = tx.execute(
            "INSERT OR REPLACE INTO mcp_servers (id, name, command, args, env, enabled, test_status, test_message, tool_count, tested_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            [
                &server.id,
                &server.name,
                &server.command,
                &server.args.clone().unwrap_or_default(),
                &server.env.clone().unwrap_or_default(),
                &enabled.to_string(),
                &server.test_status.clone().unwrap_or_default(),
                &server.test_message.clone().unwrap_or_default(),
                &tool_count,
                &server.tested_at.clone().unwrap_or_default(),
                &server.created_at,
                &server.updated_at,
            ],
        );
        if res.is_ok() {
            result.mcp_servers_imported += 1;
        }
    }

    // 导入 CLI 路径配置
    for cli in &data.cli_paths {
        let res = tx.execute(
            "INSERT OR REPLACE INTO cli_paths (id, name, path, version, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [
                &cli.id,
                &cli.name,
                &cli.path,
                &cli.version.clone().unwrap_or_default(),
                &cli.created_at,
                &cli.updated_at,
            ],
        );
        if res.is_ok() {
            result.cli_paths_imported += 1;
        }
    }

    // 导入市场源配置
    for source in &data.market_sources {
        let enabled = if source.enabled { 1 } else { 0 };
        let res = tx.execute(
            "INSERT OR REPLACE INTO market_sources (id, name, type, url_or_path, status, enabled, last_synced_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            [
                &source.id,
                &source.name,
                &source.type_,
                &source.url_or_path,
                &source.status,
                &enabled.to_string(),
                &source.last_synced_at.clone().unwrap_or_default(),
                &source.created_at,
                &source.updated_at,
            ],
        );
        if res.is_ok() {
            result.market_sources_imported += 1;
        }
    }

    // 导入应用设置
    for setting in &data.app_settings {
        let res = tx.execute(
            "INSERT OR REPLACE INTO app_settings (key, value, updated_at)
             VALUES (?1, ?2, ?3)",
            [&setting.key, &setting.value, &setting.updated_at],
        );
        if res.is_ok() {
            result.app_settings_imported += 1;
        }
    }

    // 提交事务
    tx.commit().map_err(|e| e.to_string())?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands, database};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct HomeGuard {
        original_home: Option<String>,
    }

    impl HomeGuard {
        fn set(temp_home: &PathBuf) -> Self {
            let original_home = std::env::var("HOME").ok();
            std::env::set_var("HOME", temp_home);
            Self { original_home }
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            if let Some(original_home) = &self.original_home {
                std::env::set_var("HOME", original_home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    fn unique_temp_home() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!(
            "easy-agent-data-test-{}-{}",
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn data_management_roundtrip_works_in_isolated_home() {
        let temp_home = unique_temp_home();
        fs::create_dir_all(&temp_home).unwrap();
        let _home_guard = HomeGuard::set(&temp_home);

        commands::init_persistence_dirs().unwrap();
        database::init_database().unwrap();

        let db_path = get_db_path().unwrap();
        let conn = Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO projects (id, name, path, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [
                "project-1",
                "Demo",
                "/tmp/demo",
                "demo project",
                "2026-03-18T00:00:00Z",
                "2026-03-18T00:00:00Z",
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO sessions (id, project_id, name, agent_type, status, pinned, last_message, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            [
                "session-1",
                "project-1",
                "Session",
                "codex",
                "idle",
                "0",
                "hello",
                "2026-03-18T00:00:01Z",
                "2026-03-18T00:00:01Z",
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO messages (id, session_id, role, content, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [
                "message-1",
                "session-1",
                "user",
                "hello world",
                "completed",
                "2026-03-18T00:00:02Z",
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO plans (id, project_id, name, description, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            [
                "plan-1",
                "project-1",
                "Plan",
                "plan description",
                "draft",
                "2026-03-18T00:00:03Z",
                "2026-03-18T00:00:03Z",
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, plan_id, title, description, status, priority, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            [
                "task-1",
                "plan-1",
                "Task",
                "task description",
                "pending",
                "high",
                "2026-03-18T00:00:04Z",
                "2026-03-18T00:00:04Z",
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO task_execution_logs (id, task_id, log_type, content, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [
                "log-1",
                "task-1",
                "info",
                "log content",
                "{\"source\":\"test\"}",
                "2026-03-18T00:00:05Z",
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO task_execution_results (id, task_id, plan_id, task_title_snapshot, task_description_snapshot, result_status, result_summary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            [
                "result-1",
                "task-1",
                "plan-1",
                "Task",
                "task description",
                "completed",
                "done",
                "2026-03-18T00:00:06Z",
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            ["language", "zh-CN", "2026-03-18T00:00:07Z"],
        )
        .unwrap();

        let stats_before = get_data_management_stats().unwrap();
        assert_eq!(stats_before.project_count, 1);
        assert_eq!(stats_before.session_count, 1);
        assert_eq!(stats_before.message_count, 1);
        assert_eq!(stats_before.log_count, 2);
        assert!(stats_before.total_size_bytes > 0);
        assert!(stats_before.message_data_size_bytes > 0);
        assert!(stats_before.log_data_size_bytes > 0);

        let backup_path = temp_home.join("backup.json");
        export_selected_to_file(
            backup_path.to_string_lossy().to_string(),
            ExportOptions::default(),
        )
        .unwrap();
        validate_import_data(backup_path.to_string_lossy().to_string()).unwrap();

        clear_all_data().unwrap();

        let stats_cleared = get_data_management_stats().unwrap();
        assert_eq!(stats_cleared.project_count, 0);
        assert_eq!(stats_cleared.session_count, 0);
        assert_eq!(stats_cleared.message_count, 0);
        assert_eq!(stats_cleared.log_count, 0);

        let import_result =
            import_data_from_file(backup_path.to_string_lossy().to_string()).unwrap();
        assert_eq!(import_result.projects_imported, 1);
        assert_eq!(import_result.sessions_imported, 1);
        assert_eq!(import_result.messages_imported, 1);
        assert_eq!(import_result.app_settings_imported, 1);

        let stats_after = get_data_management_stats().unwrap();
        assert_eq!(stats_after.project_count, 1);
        assert_eq!(stats_after.session_count, 1);
        assert_eq!(stats_after.message_count, 1);
        assert_eq!(stats_after.log_count, 0);

        fs::remove_dir_all(&temp_home).unwrap();
    }
}
