use anyhow::Result;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use super::mcp::McpServer;
use super::plugins_market::InstalledPlugin;
use super::skills_market::InstalledSkill;
use super::support::{bool_from_int, now_rfc3339, open_db_connection, open_db_connection_with_foreign_keys};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioAgentResourceBinding {
    pub id: String,
    pub studio_agent_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub resource_name: String,
    pub enabled: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioAgent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub specialization: Option<String>,
    pub role_type: String,
    pub runtime_agent_id: String,
    pub runtime_model_id: Option<String>,
    pub system_prompt: Option<String>,
    pub instruction_note: Option<String>,
    pub readonly_researcher: bool,
    pub planning_capability: bool,
    pub execution_capability: bool,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub resource_bindings: Vec<StudioAgentResourceBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioAgentRuntime {
    pub studio_agent_id: String,
    pub runtime_agent_id: String,
    pub runtime_model_id: Option<String>,
    pub system_prompt: Option<String>,
    pub instruction_note: Option<String>,
    pub readonly_researcher: bool,
    pub planning_capability: bool,
    pub execution_capability: bool,
    pub enabled: bool,
    pub resource_bindings: Vec<StudioAgentResourceBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResourceCatalogItem {
    pub resource_type: String,
    pub resource_id: String,
    pub name: String,
    pub description: Option<String>,
    pub source: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResourceCatalog {
    pub mcp_servers: Vec<AgentResourceCatalogItem>,
    pub skills: Vec<AgentResourceCatalogItem>,
    pub plugins: Vec<AgentResourceCatalogItem>,
}

#[derive(Debug, Deserialize)]
pub struct StudioAgentBindingInput {
    pub resource_type: String,
    pub resource_id: String,
    pub resource_name: String,
    pub enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateStudioAgentInput {
    pub name: String,
    pub description: Option<String>,
    pub specialization: Option<String>,
    pub role_type: Option<String>,
    pub runtime_agent_id: String,
    pub runtime_model_id: Option<String>,
    pub system_prompt: Option<String>,
    pub instruction_note: Option<String>,
    pub readonly_researcher: Option<bool>,
    pub planning_capability: Option<bool>,
    pub execution_capability: Option<bool>,
    pub enabled: Option<bool>,
    pub resource_bindings: Option<Vec<StudioAgentBindingInput>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStudioAgentInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub specialization: Option<String>,
    pub role_type: Option<String>,
    pub runtime_agent_id: Option<String>,
    pub runtime_model_id: Option<String>,
    pub system_prompt: Option<String>,
    pub instruction_note: Option<String>,
    pub readonly_researcher: Option<bool>,
    pub planning_capability: Option<bool>,
    pub execution_capability: Option<bool>,
    pub enabled: Option<bool>,
    pub resource_bindings: Option<Vec<StudioAgentBindingInput>>,
}

fn map_binding_row(row: &Row<'_>) -> rusqlite::Result<StudioAgentResourceBinding> {
    Ok(StudioAgentResourceBinding {
        id: row.get(0)?,
        studio_agent_id: row.get(1)?,
        resource_type: row.get(2)?,
        resource_id: row.get(3)?,
        resource_name: row.get(4)?,
        enabled: bool_from_int(row.get::<_, Option<i32>>(5)?).unwrap_or(true),
        sort_order: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn list_bindings(conn: &Connection, studio_agent_id: &str) -> Result<Vec<StudioAgentResourceBinding>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, studio_agent_id, resource_type, resource_id, resource_name, enabled, sort_order, created_at, updated_at
             FROM studio_agent_resource_bindings
             WHERE studio_agent_id = ?1
             ORDER BY resource_type ASC, sort_order ASC, created_at ASC",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map([studio_agent_id], map_binding_row)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    Ok(rows)
}

fn map_studio_agent_row(row: &Row<'_>) -> rusqlite::Result<StudioAgent> {
    Ok(StudioAgent {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        specialization: row.get(3)?,
        role_type: row.get(4)?,
        runtime_agent_id: row.get(5)?,
        runtime_model_id: row.get(6)?,
        system_prompt: row.get(7)?,
        instruction_note: row.get(8)?,
        readonly_researcher: bool_from_int(row.get::<_, Option<i32>>(9)?).unwrap_or(false),
        planning_capability: bool_from_int(row.get::<_, Option<i32>>(10)?).unwrap_or(false),
        execution_capability: bool_from_int(row.get::<_, Option<i32>>(11)?).unwrap_or(true),
        enabled: bool_from_int(row.get::<_, Option<i32>>(12)?).unwrap_or(true),
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        resource_bindings: Vec::new(),
    })
}

fn get_studio_agent_internal(conn: &Connection, id: &str) -> Result<StudioAgent, String> {
    let mut agent = conn
        .query_row(
            "SELECT id, name, description, specialization, role_type, runtime_agent_id, runtime_model_id,
                    system_prompt, instruction_note, readonly_researcher, planning_capability,
                    execution_capability, enabled, created_at, updated_at
             FROM studio_agents
             WHERE id = ?1",
            [id],
            map_studio_agent_row,
        )
        .map_err(|error| error.to_string())?;
    agent.resource_bindings = list_bindings(conn, id)?;
    Ok(agent)
}

fn replace_bindings(
    conn: &Connection,
    studio_agent_id: &str,
    bindings: &[StudioAgentBindingInput],
    now: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM studio_agent_resource_bindings WHERE studio_agent_id = ?1",
        [studio_agent_id],
    )
    .map_err(|error| error.to_string())?;

    for (index, binding) in bindings.iter().enumerate() {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO studio_agent_resource_bindings
             (id, studio_agent_id, resource_type, resource_id, resource_name, enabled, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                studio_agent_id,
                binding.resource_type.trim(),
                binding.resource_id.trim(),
                binding.resource_name.trim(),
                if binding.enabled.unwrap_or(true) { 1 } else { 0 },
                binding.sort_order.unwrap_or(index as i32),
                now,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn list_global_mcp_servers(conn: &Connection) -> Result<Vec<McpServer>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, server_type, command, args, env, url, headers, enabled, test_status, test_message, tool_count, tested_at, created_at, updated_at
             FROM mcp_servers
             ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
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
            enabled: bool_from_int(row.get::<_, Option<i32>>(8)?).unwrap_or(true),
            test_status: row.get(9)?,
            test_message: row.get(10)?,
            tool_count: row.get(11)?,
            tested_at: row.get(12)?,
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
        })
    })
    .map_err(|error| error.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| error.to_string())?;

    Ok(rows)
}

#[tauri::command]
pub fn list_studio_agents() -> Result<Vec<StudioAgent>, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, specialization, role_type, runtime_agent_id, runtime_model_id,
                    system_prompt, instruction_note, readonly_researcher, planning_capability,
                    execution_capability, enabled, created_at, updated_at
             FROM studio_agents
             ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map([], map_studio_agent_row)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    rows.into_iter()
        .map(|mut agent| {
            agent.resource_bindings = list_bindings(&conn, &agent.id)?;
            Ok(agent)
        })
        .collect::<Result<Vec<_>, String>>()
}

#[tauri::command]
pub fn get_studio_agent(id: String) -> Result<StudioAgent, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    get_studio_agent_internal(&conn, &id)
}

#[tauri::command]
pub fn create_studio_agent(input: CreateStudioAgentInput) -> Result<StudioAgent, String> {
    let conn = open_db_connection_with_foreign_keys().map_err(|error| error.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let role_type = input.role_type.unwrap_or_else(|| "custom".to_string());

    conn.execute(
        "INSERT INTO studio_agents
         (id, name, description, specialization, role_type, runtime_agent_id, runtime_model_id,
          system_prompt, instruction_note, readonly_researcher, planning_capability,
          execution_capability, enabled, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            id,
            input.name.trim(),
            input.description.as_ref().map(|value| value.trim()),
            input.specialization.as_ref().map(|value| value.trim()),
            role_type,
            input.runtime_agent_id,
            input.runtime_model_id.as_ref().map(|value| value.trim()),
            input.system_prompt.as_ref().map(|value| value.trim()),
            input.instruction_note.as_ref().map(|value| value.trim()),
            if input.readonly_researcher.unwrap_or(false) { 1 } else { 0 },
            if input.planning_capability.unwrap_or(false) { 1 } else { 0 },
            if input.execution_capability.unwrap_or(true) { 1 } else { 0 },
            if input.enabled.unwrap_or(true) { 1 } else { 0 },
            now,
            now
        ],
    )
    .map_err(|error| error.to_string())?;

    replace_bindings(
        &conn,
        &id,
        input.resource_bindings.as_deref().unwrap_or(&[]),
        &now,
    )?;

    get_studio_agent_internal(&conn, &id)
}

#[tauri::command]
pub fn update_studio_agent(id: String, input: UpdateStudioAgentInput) -> Result<StudioAgent, String> {
    let conn = open_db_connection_with_foreign_keys().map_err(|error| error.to_string())?;
    let existing = get_studio_agent_internal(&conn, &id)?;
    let now = now_rfc3339();
    let next_name = input.name.unwrap_or_else(|| existing.name.clone());
    let next_description = input.description.or_else(|| existing.description.clone());
    let next_specialization = input.specialization.or_else(|| existing.specialization.clone());
    let next_role_type = input.role_type.unwrap_or_else(|| existing.role_type.clone());
    let next_runtime_agent_id = input
        .runtime_agent_id
        .unwrap_or_else(|| existing.runtime_agent_id.clone());
    let next_runtime_model_id = input
        .runtime_model_id
        .or_else(|| existing.runtime_model_id.clone());
    let next_system_prompt = input.system_prompt.or_else(|| existing.system_prompt.clone());
    let next_instruction_note = input
        .instruction_note
        .or_else(|| existing.instruction_note.clone());
    let next_readonly_researcher = input
        .readonly_researcher
        .unwrap_or(existing.readonly_researcher);
    let next_planning_capability = input
        .planning_capability
        .unwrap_or(existing.planning_capability);
    let next_execution_capability = input
        .execution_capability
        .unwrap_or(existing.execution_capability);
    let next_enabled = input.enabled.unwrap_or(existing.enabled);

    conn.execute(
        "UPDATE studio_agents
         SET name = ?1,
             description = ?2,
             specialization = ?3,
             role_type = ?4,
             runtime_agent_id = ?5,
             runtime_model_id = ?6,
             system_prompt = ?7,
             instruction_note = ?8,
             readonly_researcher = ?9,
             planning_capability = ?10,
             execution_capability = ?11,
             enabled = ?12,
             updated_at = ?13
         WHERE id = ?14",
        params![
            next_name.trim(),
            next_description.map(|value| value.trim().to_string()),
            next_specialization.map(|value| value.trim().to_string()),
            next_role_type,
            next_runtime_agent_id,
            next_runtime_model_id.map(|value| value.trim().to_string()),
            next_system_prompt.map(|value| value.trim().to_string()),
            next_instruction_note.map(|value| value.trim().to_string()),
            if next_readonly_researcher { 1 } else { 0 },
            if next_planning_capability { 1 } else { 0 },
            if next_execution_capability { 1 } else { 0 },
            if next_enabled { 1 } else { 0 },
            now,
            id
        ],
    )
    .map_err(|error| error.to_string())?;

    if let Some(bindings) = input.resource_bindings {
        replace_bindings(&conn, &id, &bindings, &now)?;
    }

    get_studio_agent_internal(&conn, &id)
}

#[tauri::command]
pub fn delete_studio_agent(id: String) -> Result<(), String> {
    let conn = open_db_connection_with_foreign_keys().map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM studio_agents WHERE id = ?1", [id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn resolve_studio_agent_runtime(id: String) -> Result<StudioAgentRuntime, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let agent = get_studio_agent_internal(&conn, &id)?;

    Ok(StudioAgentRuntime {
        studio_agent_id: agent.id,
        runtime_agent_id: agent.runtime_agent_id,
        runtime_model_id: agent.runtime_model_id,
        system_prompt: agent.system_prompt,
        instruction_note: agent.instruction_note,
        readonly_researcher: agent.readonly_researcher,
        planning_capability: agent.planning_capability,
        execution_capability: agent.execution_capability,
        enabled: agent.enabled,
        resource_bindings: agent.resource_bindings,
    })
}

#[tauri::command]
pub async fn list_studio_agent_resource_catalog() -> Result<AgentResourceCatalog, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mcp_servers = list_global_mcp_servers(&conn)?
        .into_iter()
        .map(|server| AgentResourceCatalogItem {
            resource_type: "mcp".to_string(),
            resource_id: server.id,
            name: server.name,
            description: server.test_message,
            source: Some("global_mcp".to_string()),
            enabled: server.enabled,
        })
        .collect();

    let skills = super::skills_market::list_installed_skills()
        .await?
        .into_iter()
        .map(|skill: InstalledSkill| AgentResourceCatalogItem {
            resource_type: "skill".to_string(),
            resource_id: skill.path.clone(),
            name: skill.name,
            description: skill.description,
            source: Some(skill.source_cli),
            enabled: !skill.disabled,
        })
        .collect();

    let plugins = super::plugins_market::list_installed_plugins()?
        .into_iter()
        .map(|plugin: InstalledPlugin| AgentResourceCatalogItem {
            resource_type: "plugin".to_string(),
            resource_id: plugin.id,
            name: plugin.name,
            description: Some(format!("{} / {}", plugin.version, plugin.scope)),
            source: Some(plugin.source_market),
            enabled: plugin.enabled,
        })
        .collect();

    Ok(AgentResourceCatalog {
        mcp_servers,
        skills,
        plugins,
    })
}
