use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::support::{now_rfc3339, open_db_connection};

/// 计划数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub memory_library_ids: Vec<String>,
    pub execution_overview: Option<String>,
    pub execution_overview_updated_at: Option<String>,
    pub split_mode: String,
    pub split_expert_id: Option<String>,
    pub split_agent_id: Option<String>,
    pub split_model_id: Option<String>,
    pub status: String,
    pub agent_team: Option<Vec<String>>,
    pub granularity: i32,
    pub max_retry_count: i32,
    pub execution_status: Option<String>,
    pub current_task_id: Option<String>,
    pub scheduled_at: Option<String>,
    pub schedule_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Rust 后端返回的结构（snake_case）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustPlan {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub memory_library_ids: Vec<String>,
    pub execution_overview: Option<String>,
    pub execution_overview_updated_at: Option<String>,
    pub split_mode: String,
    pub split_expert_id: Option<String>,
    pub split_agent_id: Option<String>,
    pub split_model_id: Option<String>,
    pub status: String,
    pub agent_team: Option<String>,
    pub granularity: i32,
    pub max_retry_count: i32,
    pub execution_status: Option<String>,
    pub current_task_id: Option<String>,
    pub scheduled_at: Option<String>,
    pub schedule_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(untagged)]
pub enum UpdateField<T> {
    Value(T),
    Null,
    #[default]
    Missing,
}

/// 创建计划输入
#[derive(Debug, Deserialize)]
pub struct CreatePlanInput {
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub memory_library_ids: Vec<String>,
    pub split_mode: Option<String>,
    pub split_expert_id: Option<String>,
    pub split_agent_id: Option<String>,
    pub split_model_id: Option<String>,
    pub agent_team: Option<Vec<String>>,
    pub granularity: Option<i32>,
    pub max_retry_count: Option<i32>,
    pub scheduled_at: Option<String>,
}

/// 更新计划输入
#[derive(Debug, Deserialize)]
pub struct UpdatePlanInput {
    #[serde(default)]
    pub name: UpdateField<String>,
    #[serde(default)]
    pub description: UpdateField<String>,
    #[serde(default)]
    pub memory_library_ids: UpdateField<Vec<String>>,
    #[serde(default)]
    pub execution_overview: UpdateField<String>,
    #[serde(default)]
    pub execution_overview_updated_at: UpdateField<String>,
    #[serde(default)]
    pub split_mode: UpdateField<String>,
    #[serde(default)]
    pub split_expert_id: UpdateField<String>,
    #[serde(default)]
    pub split_agent_id: UpdateField<String>,
    #[serde(default)]
    pub split_model_id: UpdateField<String>,
    #[serde(default)]
    pub status: UpdateField<String>,
    #[serde(default)]
    pub agent_team: UpdateField<Vec<String>>,
    #[serde(default)]
    pub granularity: UpdateField<i32>,
    #[serde(default)]
    pub max_retry_count: UpdateField<i32>,
    #[serde(default)]
    pub execution_status: UpdateField<String>,
    #[serde(default)]
    pub current_task_id: UpdateField<String>,
    #[serde(default)]
    pub scheduled_at: UpdateField<String>,
    #[serde(default)]
    pub schedule_status: UpdateField<String>,
}

fn normalize_memory_library_ids(library_ids: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();

    for library_id in library_ids {
        let trimmed = library_id.trim();
        if trimmed.is_empty() || normalized.iter().any(|existing| existing == trimmed) {
            continue;
        }
        normalized.push(trimmed.to_string());
    }

    normalized
}

fn list_plan_memory_library_ids(
    conn: &rusqlite::Connection,
    plan_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT library_id
            FROM plan_memory_libraries
            WHERE plan_id = ?1
            ORDER BY created_at ASC, library_id ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([plan_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn replace_plan_memory_libraries(
    conn: &rusqlite::Connection,
    plan_id: &str,
    library_ids: &[String],
    now: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM plan_memory_libraries WHERE plan_id = ?1",
        [plan_id],
    )
    .map_err(|e| e.to_string())?;

    for library_id in normalize_memory_library_ids(library_ids) {
        conn.execute(
            r#"
            INSERT INTO plan_memory_libraries (plan_id, library_id, created_at)
            VALUES (?1, ?2, ?3)
            "#,
            rusqlite::params![plan_id, library_id, now],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn transform_plan(rust_plan: RustPlan) -> Plan {
    let agent_team = rust_plan
        .agent_team
        .and_then(|s| serde_json::from_str(&s).ok());

    Plan {
        id: rust_plan.id,
        project_id: rust_plan.project_id,
        name: rust_plan.name,
        description: rust_plan.description,
        memory_library_ids: rust_plan.memory_library_ids,
        execution_overview: rust_plan.execution_overview,
        execution_overview_updated_at: rust_plan.execution_overview_updated_at,
        split_mode: rust_plan.split_mode,
        split_expert_id: rust_plan.split_expert_id,
        split_agent_id: rust_plan.split_agent_id,
        split_model_id: rust_plan.split_model_id,
        status: rust_plan.status,
        agent_team,
        granularity: rust_plan.granularity,
        max_retry_count: rust_plan.max_retry_count,
        execution_status: rust_plan.execution_status,
        current_task_id: rust_plan.current_task_id,
        scheduled_at: rust_plan.scheduled_at,
        schedule_status: rust_plan.schedule_status,
        created_at: rust_plan.created_at,
        updated_at: rust_plan.updated_at,
    }
}

fn collect_plan_task_ids(
    conn: &rusqlite::Connection,
    plan_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT id FROM tasks WHERE plan_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([plan_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn map_plan_row(
    conn: &rusqlite::Connection,
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RustPlan> {
    let plan_id: String = row.get(0)?;
    let memory_library_ids = list_plan_memory_library_ids(conn, &plan_id).map_err(|error| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            error,
        )))
    })?;

    Ok(RustPlan {
        id: plan_id,
        project_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        memory_library_ids,
        execution_overview: row.get(4)?,
        execution_overview_updated_at: row.get(5)?,
        split_mode: row.get(6)?,
        status: row.get(7)?,
        agent_team: row.get(8)?,
        split_expert_id: row.get(9)?,
        split_agent_id: row.get(10)?,
        split_model_id: row.get(11)?,
        granularity: row.get(12)?,
        max_retry_count: row.get(13)?,
        execution_status: row.get(14)?,
        current_task_id: row.get(15)?,
        scheduled_at: row.get(16)?,
        schedule_status: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

const PLAN_SELECT_SQL: &str = r#"
    SELECT id, project_id, name, description, execution_overview, execution_overview_updated_at,
           split_mode, status, agent_team,
           split_expert_id, split_agent_id, split_model_id,
           granularity, max_retry_count, execution_status, current_task_id,
           scheduled_at, schedule_status,
           created_at, updated_at
    FROM plans
"#;

/// 获取指定项目的所有计划
#[tauri::command]
pub fn list_plans(project_id: String) -> Result<Vec<Plan>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let sql = format!("{PLAN_SELECT_SQL} WHERE project_id = ?1 ORDER BY updated_at DESC");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let plans = stmt
        .query_map([&project_id], |row| map_plan_row(&conn, row))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(transform_plan)
        .collect();

    Ok(plans)
}

/// 获取单个计划
#[tauri::command]
pub fn get_plan(id: String) -> Result<Plan, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let sql = format!("{PLAN_SELECT_SQL} WHERE id = ?1");

    conn.query_row(&sql, [&id], |row| map_plan_row(&conn, row))
        .map(transform_plan)
        .map_err(|e| e.to_string())
}

/// 创建新计划
#[tauri::command]
pub fn create_plan(input: CreatePlanInput) -> Result<Plan, String> {
    let mut conn = open_db_connection().map_err(|e| e.to_string())?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let status = "draft".to_string();
    let execution_status = "idle".to_string();
    let split_mode = input.split_mode.unwrap_or_else(|| "ai".to_string());
    let memory_library_ids = normalize_memory_library_ids(&input.memory_library_ids);
    let agent_team_json = input
        .agent_team
        .as_ref()
        .map(|t| serde_json::to_string(t).unwrap_or_else(|_| "[]".to_string()));
    let granularity = input.granularity.unwrap_or(20);
    let max_retry_count = input.max_retry_count.unwrap_or(3);
    let schedule_status = if input.scheduled_at.is_some() {
        Some("scheduled".to_string())
    } else {
        Some("none".to_string())
    };

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO plans (id, project_id, name, description, split_mode, split_expert_id, split_agent_id, split_model_id, status, agent_team,
         granularity, max_retry_count, execution_status, current_task_id, execution_overview, execution_overview_updated_at, scheduled_at, schedule_status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        rusqlite::params![
            &id,
            &input.project_id,
            &input.name,
            &input.description,
            &split_mode,
            &input.split_expert_id,
            &input.split_agent_id,
            &input.split_model_id,
            &status,
            &agent_team_json,
            &granularity,
            &max_retry_count,
            &execution_status,
            &None::<String>,
            &None::<String>,
            &None::<String>,
            &input.scheduled_at,
            &schedule_status,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    replace_plan_memory_libraries(&tx, &id, &memory_library_ids, &now)?;

    tx.execute(
        "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &input.project_id],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(Plan {
        id,
        project_id: input.project_id,
        name: input.name,
        description: input.description,
        memory_library_ids,
        execution_overview: None,
        execution_overview_updated_at: None,
        split_mode,
        split_expert_id: input.split_expert_id,
        split_agent_id: input.split_agent_id,
        split_model_id: input.split_model_id,
        status,
        agent_team: input.agent_team,
        granularity,
        max_retry_count,
        execution_status: Some(execution_status),
        current_task_id: None,
        scheduled_at: input.scheduled_at,
        schedule_status,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 更新计划
#[tauri::command]
pub fn update_plan(id: String, input: UpdatePlanInput) -> Result<Plan, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let now = now_rfc3339();

    let mut updates: Vec<String> = vec!["updated_at = ?1".to_string()];
    let mut param_index = 2;

    let push_update =
        |updates: &mut Vec<String>, param_index: &mut usize, column: &str, present: bool| {
            if present {
                updates.push(format!("{column} = ?{}", *param_index));
                *param_index += 1;
            }
        };

    push_update(
        &mut updates,
        &mut param_index,
        "name",
        !matches!(input.name, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "description",
        !matches!(input.description, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "execution_overview",
        !matches!(input.execution_overview, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "execution_overview_updated_at",
        !matches!(input.execution_overview_updated_at, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "split_mode",
        !matches!(input.split_mode, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "split_expert_id",
        !matches!(input.split_expert_id, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "split_agent_id",
        !matches!(input.split_agent_id, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "split_model_id",
        !matches!(input.split_model_id, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "status",
        !matches!(input.status, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "agent_team",
        !matches!(input.agent_team, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "granularity",
        !matches!(input.granularity, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "max_retry_count",
        !matches!(input.max_retry_count, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "execution_status",
        !matches!(input.execution_status, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "current_task_id",
        !matches!(input.current_task_id, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "scheduled_at",
        !matches!(input.scheduled_at, UpdateField::Missing),
    );
    push_update(
        &mut updates,
        &mut param_index,
        "schedule_status",
        !matches!(input.schedule_status, UpdateField::Missing),
    );

    let sql = format!(
        "UPDATE plans SET {} WHERE id = ?{}",
        updates.join(", "),
        param_index
    );

    let mut stmt = conn.prepare_cached(&sql).map_err(|e| e.to_string())?;
    let mut bind_index = 1;
    stmt.raw_bind_parameter(bind_index, &now)
        .map_err(|e| e.to_string())?;
    bind_index += 1;

    macro_rules! bind_field {
        ($field:expr) => {
            if !matches!($field, UpdateField::Missing) {
                match $field {
                    UpdateField::Value(ref value) => stmt
                        .raw_bind_parameter(bind_index, value)
                        .map_err(|e| e.to_string())?,
                    UpdateField::Null => stmt
                        .raw_bind_parameter(bind_index, rusqlite::types::Null)
                        .map_err(|e| e.to_string())?,
                    UpdateField::Missing => {}
                }
                bind_index += 1;
            }
        };
    }

    bind_field!(input.name);
    bind_field!(input.description);
    bind_field!(input.execution_overview);
    bind_field!(input.execution_overview_updated_at);
    bind_field!(input.split_mode);
    bind_field!(input.split_expert_id);
    bind_field!(input.split_agent_id);
    bind_field!(input.split_model_id);
    bind_field!(input.status);

    if !matches!(input.agent_team, UpdateField::Missing) {
        match input.agent_team {
            UpdateField::Value(ref value) => {
                let json = serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string());
                stmt.raw_bind_parameter(bind_index, json)
                    .map_err(|e| e.to_string())?;
            }
            UpdateField::Null => stmt
                .raw_bind_parameter(bind_index, rusqlite::types::Null)
                .map_err(|e| e.to_string())?,
            UpdateField::Missing => {}
        }
        bind_index += 1;
    }

    if let UpdateField::Value(value) = input.granularity {
        stmt.raw_bind_parameter(bind_index, value)
            .map_err(|e| e.to_string())?;
        bind_index += 1;
    } else if matches!(input.granularity, UpdateField::Null) {
        stmt.raw_bind_parameter(bind_index, rusqlite::types::Null)
            .map_err(|e| e.to_string())?;
        bind_index += 1;
    }

    if let UpdateField::Value(value) = input.max_retry_count {
        stmt.raw_bind_parameter(bind_index, value)
            .map_err(|e| e.to_string())?;
        bind_index += 1;
    } else if matches!(input.max_retry_count, UpdateField::Null) {
        stmt.raw_bind_parameter(bind_index, rusqlite::types::Null)
            .map_err(|e| e.to_string())?;
        bind_index += 1;
    }

    bind_field!(input.execution_status);
    bind_field!(input.current_task_id);
    bind_field!(input.scheduled_at);
    bind_field!(input.schedule_status);

    stmt.raw_bind_parameter(bind_index, &id)
        .map_err(|e| e.to_string())?;
    stmt.raw_execute().map_err(|e| e.to_string())?;

    if !matches!(input.memory_library_ids, UpdateField::Missing) {
        let library_ids = match input.memory_library_ids {
            UpdateField::Value(ref value) => value.clone(),
            UpdateField::Null => Vec::new(),
            UpdateField::Missing => Vec::new(),
        };
        replace_plan_memory_libraries(&conn, &id, &library_ids, &now)?;
    }

    get_plan(id)
}

/// 删除计划
#[tauri::command]
pub fn delete_plan(id: String) -> Result<(), String> {
    let mut conn = open_db_connection().map_err(|e| e.to_string())?;
    let task_ids = collect_plan_task_ids(&conn, &id)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM plan_split_logs WHERE plan_id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM task_split_sessions WHERE plan_id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM task_execution_results WHERE plan_id = ?1",
        [&id],
    )
    .map_err(|e| e.to_string())?;

    if !task_ids.is_empty() {
        let placeholders = (0..task_ids.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");

        let delete_execution_logs_sql = format!(
            "DELETE FROM task_execution_logs WHERE task_id IN ({})",
            placeholders
        );
        tx.execute(
            &delete_execution_logs_sql,
            rusqlite::params_from_iter(task_ids.iter()),
        )
        .map_err(|e| e.to_string())?;

        let delete_usage_logs_sql = format!(
            "DELETE FROM agent_cli_usage_records WHERE task_id IN ({})",
            placeholders
        );
        tx.execute(
            &delete_usage_logs_sql,
            rusqlite::params_from_iter(task_ids.iter()),
        )
        .map_err(|e| e.to_string())?;
    }

    tx.execute("DELETE FROM plans WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取所有待执行的定时计划
#[tauri::command]
pub fn list_scheduled_plans() -> Result<Vec<Plan>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let sql = format!(
        "{PLAN_SELECT_SQL} WHERE schedule_status = 'scheduled' AND scheduled_at IS NOT NULL ORDER BY scheduled_at ASC"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let plans = stmt
        .query_map([], |row| map_plan_row(&conn, row))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(transform_plan)
        .collect();

    Ok(plans)
}

/// 取消计划定时
#[tauri::command]
pub fn cancel_plan_schedule(id: String) -> Result<Plan, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let now = now_rfc3339();

    conn.execute(
        "UPDATE plans SET schedule_status = 'cancelled', scheduled_at = NULL, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &id],
    )
    .map_err(|e| e.to_string())?;

    get_plan(id)
}
