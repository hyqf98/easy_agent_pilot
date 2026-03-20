use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

use super::conversation::types::McpServerConfig;
use super::support::{bool_from_int, now_rfc3339, open_db_connection};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub agent_id: String,
    pub system_prompt: Option<String>,
    pub role_definition: Option<String>,
    pub working_style: Option<String>,
    pub output_style: Option<String>,
    pub tool_usage_policy: Option<String>,
    pub domain_tags: Vec<String>,
    pub capability_tags: Vec<String>,
    pub readonly_researcher: bool,
    pub planning_capability: bool,
    pub execution_capability: bool,
    pub default_execution_mode: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionProfile {
    pub agent_id: String,
    pub system_prompt: Option<String>,
    pub role_definition: Option<String>,
    pub working_style: Option<String>,
    pub output_style: Option<String>,
    pub tool_usage_policy: Option<String>,
    pub domain_tags: Vec<String>,
    pub capability_tags: Vec<String>,
    pub readonly_researcher: bool,
    pub planning_capability: bool,
    pub execution_capability: bool,
    pub default_execution_mode: Option<String>,
    pub mcp_servers: Vec<McpServerConfig>,
    pub skill_names: Vec<String>,
    pub plugin_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertAgentProfileInput {
    pub agent_id: String,
    pub system_prompt: Option<String>,
    pub role_definition: Option<String>,
    pub working_style: Option<String>,
    pub output_style: Option<String>,
    pub tool_usage_policy: Option<String>,
    pub domain_tags: Option<Vec<String>>,
    pub capability_tags: Option<Vec<String>>,
    pub readonly_researcher: Option<bool>,
    pub planning_capability: Option<bool>,
    pub execution_capability: Option<bool>,
    pub default_execution_mode: Option<String>,
}

#[derive(Debug)]
struct AgentExecutionMeta {
    agent_type: String,
    provider: Option<String>,
    cli_path: Option<String>,
}

fn parse_string_array(raw: Option<String>) -> Vec<String> {
    raw.and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn serialize_string_array(values: &[String]) -> Option<String> {
    let normalized = values
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        None
    } else {
        serde_json::to_string(&normalized).ok()
    }
}

fn default_profile(agent_id: &str) -> AgentProfile {
    let now = now_rfc3339();
    AgentProfile {
        agent_id: agent_id.to_string(),
        system_prompt: None,
        role_definition: None,
        working_style: None,
        output_style: None,
        tool_usage_policy: None,
        domain_tags: Vec::new(),
        capability_tags: Vec::new(),
        readonly_researcher: false,
        planning_capability: false,
        execution_capability: true,
        default_execution_mode: None,
        created_at: now.clone(),
        updated_at: now,
    }
}

fn map_agent_profile_row(row: &Row<'_>) -> rusqlite::Result<AgentProfile> {
    Ok(AgentProfile {
        agent_id: row.get(0)?,
        system_prompt: row.get(1)?,
        role_definition: row.get(2)?,
        working_style: row.get(3)?,
        output_style: row.get(4)?,
        tool_usage_policy: row.get(5)?,
        domain_tags: parse_string_array(row.get(6)?),
        capability_tags: parse_string_array(row.get(7)?),
        readonly_researcher: bool_from_int(row.get::<_, Option<i32>>(8)?).unwrap_or(false),
        planning_capability: bool_from_int(row.get::<_, Option<i32>>(9)?).unwrap_or(false),
        execution_capability: bool_from_int(row.get::<_, Option<i32>>(10)?).unwrap_or(true),
        default_execution_mode: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn get_agent_execution_meta(conn: &Connection, agent_id: &str) -> Result<AgentExecutionMeta, String> {
    conn.query_row(
        "SELECT type, provider, cli_path FROM agents WHERE id = ?1",
        [agent_id],
        |row| {
            Ok(AgentExecutionMeta {
                agent_type: row.get(0)?,
                provider: row.get(1)?,
                cli_path: row.get(2)?,
            })
        },
    )
    .map_err(|error| error.to_string())
}

fn list_sdk_mcp_servers(conn: &Connection, agent_id: &str) -> Result<Vec<McpServerConfig>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, transport_type, command, args, env, url, headers
             FROM agent_mcp_configs
             WHERE agent_id = ?1 AND enabled = 1
             ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map([agent_id], |row| {
        Ok(McpServerConfig {
            id: row.get(0)?,
            name: row.get(1)?,
            transport_type: row.get(2)?,
            command: row.get(3)?,
            args: row.get(4)?,
            env: row.get(5)?,
            url: row.get(6)?,
            headers: row.get(7)?,
        })
    })
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn list_sdk_resource_names(
    conn: &Connection,
    table: &str,
    agent_id: &str,
) -> Result<Vec<String>, String> {
    let sql = format!(
        "SELECT name FROM {} WHERE agent_id = ?1 AND enabled = 1 ORDER BY updated_at DESC",
        table
    );
    let mut stmt = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map([agent_id], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn map_cli_mcp_servers(
    config: super::cli_config::ClaudeCliConfig,
) -> Vec<McpServerConfig> {
    config
        .mcpServers
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, server)| !server.disabled)
        .map(|(name, server)| McpServerConfig {
            id: format!("cli-{name}"),
            name,
            transport_type: if server.url.as_deref().is_some() {
                if server
                    .url
                    .as_deref()
                    .map(|url| url.contains("/sse"))
                    .unwrap_or(false)
                {
                    "sse".to_string()
                } else {
                    "http".to_string()
                }
            } else {
                "stdio".to_string()
            },
            command: server.command,
            args: server.args.map(|args| args.join(" ")),
            env: server
                .env
                .and_then(|env| serde_json::to_string(&env).ok()),
            url: server.url,
            headers: server
                .headers
                .and_then(|headers| serde_json::to_string(&headers).ok()),
        })
        .collect()
}

#[tauri::command]
pub fn list_agent_profiles() -> Result<Vec<AgentProfile>, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT agent_id, system_prompt, role_definition, working_style, output_style, tool_usage_policy,
                    domain_tags, capability_tags, readonly_researcher, planning_capability,
                    execution_capability, default_execution_mode, created_at, updated_at
             FROM agent_profiles
             ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map([], map_agent_profile_row)
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_agent_profile(agent_id: String) -> Result<AgentProfile, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let profile = conn.query_row(
        "SELECT agent_id, system_prompt, role_definition, working_style, output_style, tool_usage_policy,
                domain_tags, capability_tags, readonly_researcher, planning_capability,
                execution_capability, default_execution_mode, created_at, updated_at
         FROM agent_profiles
         WHERE agent_id = ?1",
        [&agent_id],
        map_agent_profile_row,
    );

    match profile {
        Ok(profile) => Ok(profile),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(default_profile(&agent_id)),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
pub fn upsert_agent_profile(input: UpsertAgentProfileInput) -> Result<AgentProfile, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let existing = get_agent_profile(input.agent_id.clone())?;
    let now = now_rfc3339();
    let created_at = if existing.created_at.is_empty() {
        now.clone()
    } else {
        existing.created_at.clone()
    };

    let merged = AgentProfile {
        agent_id: input.agent_id.clone(),
        system_prompt: input.system_prompt.or(existing.system_prompt),
        role_definition: input.role_definition.or(existing.role_definition),
        working_style: input.working_style.or(existing.working_style),
        output_style: input.output_style.or(existing.output_style),
        tool_usage_policy: input.tool_usage_policy.or(existing.tool_usage_policy),
        domain_tags: input.domain_tags.unwrap_or(existing.domain_tags),
        capability_tags: input.capability_tags.unwrap_or(existing.capability_tags),
        readonly_researcher: input
            .readonly_researcher
            .unwrap_or(existing.readonly_researcher),
        planning_capability: input
            .planning_capability
            .unwrap_or(existing.planning_capability),
        execution_capability: input
            .execution_capability
            .unwrap_or(existing.execution_capability),
        default_execution_mode: input
            .default_execution_mode
            .or(existing.default_execution_mode),
        created_at: created_at.clone(),
        updated_at: now.clone(),
    };

    conn.execute(
        "INSERT INTO agent_profiles (
            agent_id, system_prompt, role_definition, working_style, output_style, tool_usage_policy,
            domain_tags, capability_tags, readonly_researcher, planning_capability,
            execution_capability, default_execution_mode, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(agent_id) DO UPDATE SET
            system_prompt = excluded.system_prompt,
            role_definition = excluded.role_definition,
            working_style = excluded.working_style,
            output_style = excluded.output_style,
            tool_usage_policy = excluded.tool_usage_policy,
            domain_tags = excluded.domain_tags,
            capability_tags = excluded.capability_tags,
            readonly_researcher = excluded.readonly_researcher,
            planning_capability = excluded.planning_capability,
            execution_capability = excluded.execution_capability,
            default_execution_mode = excluded.default_execution_mode,
            updated_at = excluded.updated_at",
        rusqlite::params![
            &merged.agent_id,
            &merged.system_prompt,
            &merged.role_definition,
            &merged.working_style,
            &merged.output_style,
            &merged.tool_usage_policy,
            serialize_string_array(&merged.domain_tags),
            serialize_string_array(&merged.capability_tags),
            if merged.readonly_researcher { 1 } else { 0 },
            if merged.planning_capability { 1 } else { 0 },
            if merged.execution_capability { 1 } else { 0 },
            &merged.default_execution_mode,
            &created_at,
            &merged.updated_at,
        ],
    )
    .map_err(|error| error.to_string())?;

    get_agent_profile(input.agent_id)
}

#[tauri::command]
pub fn get_agent_execution_profile(agent_id: String) -> Result<AgentExecutionProfile, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let meta = get_agent_execution_meta(&conn, &agent_id)?;
    let profile = get_agent_profile(agent_id.clone())?;

    let (mcp_servers, skill_names, plugin_names) = if meta.agent_type == "cli" {
        let config = super::cli_config::read_cli_config(
            meta.cli_path.clone().unwrap_or_default(),
            meta.provider.clone(),
        )?;
        let scan = super::scan::scan_cli_config(meta.cli_path.clone(), meta.provider.clone())?;
        (
            map_cli_mcp_servers(config),
            scan.skills.into_iter().map(|skill| skill.name).collect(),
            scan.plugins
                .into_iter()
                .filter(|plugin| plugin.enabled)
                .map(|plugin| plugin.name)
                .collect(),
        )
    } else {
        (
            list_sdk_mcp_servers(&conn, &agent_id)?,
            list_sdk_resource_names(&conn, "agent_skills_configs", &agent_id)?,
            list_sdk_resource_names(&conn, "agent_plugins_configs", &agent_id)?,
        )
    };

    Ok(AgentExecutionProfile {
        agent_id,
        system_prompt: profile.system_prompt,
        role_definition: profile.role_definition,
        working_style: profile.working_style,
        output_style: profile.output_style,
        tool_usage_policy: profile.tool_usage_policy,
        domain_tags: profile.domain_tags,
        capability_tags: profile.capability_tags,
        readonly_researcher: profile.readonly_researcher,
        planning_capability: profile.planning_capability,
        execution_capability: profile.execution_capability,
        default_execution_mode: profile.default_execution_mode,
        mcp_servers,
        skill_names,
        plugin_names,
    })
}
