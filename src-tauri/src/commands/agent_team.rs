use anyhow::Result;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use super::support::{now_rfc3339, open_db_connection_with_foreign_keys};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeam {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub mode: String,
    pub coordinator_agent_id: Option<String>,
    pub coordinator_model_id: Option<String>,
    pub subagent_max_concurrent: i32,
    pub max_children_per_task: i32,
    pub members: Vec<AgentTeamMember>,
    pub relations: Vec<AgentTeamRelation>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamMember {
    pub id: String,
    pub team_id: String,
    pub agent_id: String,
    pub model_id: Option<String>,
    pub role: String,
    pub capability_preset: String,
    pub display_name: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamRelation {
    pub id: String,
    pub team_id: String,
    pub parent_member_id: String,
    pub child_member_id: String,
    pub relation_type: String,
    pub auto_dispatch_allowed: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentTeamInput {
    pub name: String,
    pub description: Option<String>,
    pub mode: Option<String>,
    pub coordinator_agent_id: Option<String>,
    pub coordinator_model_id: Option<String>,
    pub subagent_max_concurrent: Option<i32>,
    pub max_children_per_task: Option<i32>,
    pub members: Vec<CreateAgentTeamMemberInput>,
    pub relations: Vec<CreateAgentTeamRelationInput>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentTeamInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub mode: Option<String>,
    pub coordinator_agent_id: Option<String>,
    pub coordinator_model_id: Option<String>,
    pub subagent_max_concurrent: Option<i32>,
    pub max_children_per_task: Option<i32>,
    pub members: Option<Vec<CreateAgentTeamMemberInput>>,
    pub relations: Option<Vec<CreateAgentTeamRelationInput>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAgentTeamMemberInput {
    pub id: Option<String>,
    pub agent_id: String,
    pub model_id: Option<String>,
    pub role: String,
    pub capability_preset: Option<String>,
    pub display_name: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAgentTeamRelationInput {
    pub id: Option<String>,
    pub parent_member_id: String,
    pub child_member_id: String,
    pub relation_type: Option<String>,
    pub auto_dispatch_allowed: Option<bool>,
}

fn map_team_row(row: &Row<'_>) -> rusqlite::Result<AgentTeam> {
    Ok(AgentTeam {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        mode: row.get(3)?,
        coordinator_agent_id: row.get(4)?,
        coordinator_model_id: row.get(5)?,
        subagent_max_concurrent: row.get(6)?,
        max_children_per_task: row.get(7)?,
        members: Vec::new(),
        relations: Vec::new(),
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn map_member_row(row: &Row<'_>) -> rusqlite::Result<AgentTeamMember> {
    Ok(AgentTeamMember {
        id: row.get(0)?,
        team_id: row.get(1)?,
        agent_id: row.get(2)?,
        model_id: row.get(3)?,
        role: row.get(4)?,
        capability_preset: row.get(5)?,
        display_name: row.get(6)?,
        sort_order: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn map_relation_row(row: &Row<'_>) -> rusqlite::Result<AgentTeamRelation> {
    Ok(AgentTeamRelation {
        id: row.get(0)?,
        team_id: row.get(1)?,
        parent_member_id: row.get(2)?,
        child_member_id: row.get(3)?,
        relation_type: row.get(4)?,
        auto_dispatch_allowed: row.get::<_, i32>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn list_members(conn: &Connection, team_id: &str) -> Result<Vec<AgentTeamMember>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, team_id, agent_id, model_id, role, capability_preset, display_name, sort_order, created_at, updated_at
             FROM studio_agent_team_members WHERE team_id = ?1 ORDER BY sort_order ASC, created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([team_id], map_member_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn list_relations(conn: &Connection, team_id: &str) -> Result<Vec<AgentTeamRelation>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, team_id, parent_member_id, child_member_id, relation_type, auto_dispatch_allowed, created_at, updated_at
             FROM studio_agent_team_relations WHERE team_id = ?1 ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([team_id], map_relation_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn load_team(conn: &Connection, team_id: &str) -> Result<AgentTeam, String> {
    let mut team = conn
        .query_row(
            "SELECT id, name, description, mode, coordinator_agent_id, coordinator_model_id,
                    subagent_max_concurrent, max_children_per_task, created_at, updated_at
             FROM studio_agent_teams WHERE id = ?1",
            [team_id],
            map_team_row,
        )
        .map_err(|e| e.to_string())?;

    team.members = list_members(conn, team_id)?;
    team.relations = list_relations(conn, team_id)?;
    Ok(team)
}

fn replace_members_and_relations(
    conn: &Connection,
    team_id: &str,
    members: &[CreateAgentTeamMemberInput],
    relations: &[CreateAgentTeamRelationInput],
    now: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM studio_agent_team_relations WHERE team_id = ?1",
        [team_id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM studio_agent_team_members WHERE team_id = ?1", [team_id])
        .map_err(|e| e.to_string())?;

    for (index, member) in members.iter().enumerate() {
        let id = member
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        conn.execute(
            "INSERT INTO studio_agent_team_members
             (id, team_id, agent_id, model_id, role, capability_preset, display_name, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                team_id,
                member.agent_id,
                member.model_id,
                member.role,
                member
                    .capability_preset
                    .clone()
                    .unwrap_or_else(|| "readonly_research".to_string()),
                member.display_name,
                member.sort_order.unwrap_or(index as i32),
                now,
                now
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for relation in relations {
        let id = relation
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        conn.execute(
            "INSERT INTO studio_agent_team_relations
             (id, team_id, parent_member_id, child_member_id, relation_type, auto_dispatch_allowed, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                team_id,
                relation.parent_member_id,
                relation.child_member_id,
                relation
                    .relation_type
                    .clone()
                    .unwrap_or_else(|| "delegates_to".to_string()),
                if relation.auto_dispatch_allowed.unwrap_or(true) {
                    1
                } else {
                    0
                },
                now,
                now
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn list_agent_teams() -> Result<Vec<AgentTeam>, String> {
    let conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, mode, coordinator_agent_id, coordinator_model_id,
                    subagent_max_concurrent, max_children_per_task, created_at, updated_at
             FROM studio_agent_teams ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let teams = stmt
        .query_map([], map_team_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut hydrated = Vec::with_capacity(teams.len());
    for mut team in teams {
        team.members = list_members(&conn, &team.id)?;
        team.relations = list_relations(&conn, &team.id)?;
        hydrated.push(team);
    }

    Ok(hydrated)
}

#[tauri::command]
pub fn create_agent_team(input: CreateAgentTeamInput) -> Result<AgentTeam, String> {
    let mut conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();

    tx.execute(
        "INSERT INTO studio_agent_teams
         (id, name, description, mode, coordinator_agent_id, coordinator_model_id, subagent_max_concurrent,
          max_children_per_task, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            &id,
            &input.name,
            &input.description,
            input.mode.unwrap_or_else(|| "coordinator_subagents".to_string()),
            &input.coordinator_agent_id,
            &input.coordinator_model_id,
            input.subagent_max_concurrent.unwrap_or(3),
            input.max_children_per_task.unwrap_or(5),
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    replace_members_and_relations(&tx, &id, &input.members, &input.relations, &now)?;
    tx.commit().map_err(|e| e.to_string())?;

    load_team(&conn, &id)
}

#[tauri::command]
pub fn update_agent_team(id: String, input: UpdateAgentTeamInput) -> Result<AgentTeam, String> {
    let mut conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let existing = load_team(&tx, &id)?;
    let now = now_rfc3339();

    tx.execute(
        "UPDATE studio_agent_teams
         SET name = ?1,
             description = ?2,
             mode = ?3,
             coordinator_agent_id = ?4,
             coordinator_model_id = ?5,
             subagent_max_concurrent = ?6,
             max_children_per_task = ?7,
             updated_at = ?8
         WHERE id = ?9",
        params![
            input.name.unwrap_or(existing.name),
            input.description.or(existing.description),
            input.mode.unwrap_or(existing.mode),
            input.coordinator_agent_id.or(existing.coordinator_agent_id),
            input.coordinator_model_id.or(existing.coordinator_model_id),
            input
                .subagent_max_concurrent
                .unwrap_or(existing.subagent_max_concurrent),
            input
                .max_children_per_task
                .unwrap_or(existing.max_children_per_task),
            &now,
            &id
        ],
    )
    .map_err(|e| e.to_string())?;

    if let Some(members) = input.members {
        let relations = input.relations.unwrap_or(existing.relations.into_iter().map(|relation| {
            CreateAgentTeamRelationInput {
                id: Some(relation.id),
                parent_member_id: relation.parent_member_id,
                child_member_id: relation.child_member_id,
                relation_type: Some(relation.relation_type),
                auto_dispatch_allowed: Some(relation.auto_dispatch_allowed),
            }
        }).collect());

        replace_members_and_relations(&tx, &id, &members, &relations, &now)?;
    } else if let Some(relations) = input.relations {
        let members = existing
            .members
            .into_iter()
            .map(|member| CreateAgentTeamMemberInput {
                id: Some(member.id),
                agent_id: member.agent_id,
                model_id: member.model_id,
                role: member.role,
                capability_preset: Some(member.capability_preset),
                display_name: member.display_name,
                sort_order: Some(member.sort_order),
            })
            .collect::<Vec<_>>();
        replace_members_and_relations(&tx, &id, &members, &relations, &now)?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    load_team(&conn, &id)
}

#[tauri::command]
pub fn delete_agent_team(id: String) -> Result<(), String> {
    let conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM studio_agent_teams WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
