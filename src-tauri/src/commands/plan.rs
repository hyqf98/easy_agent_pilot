use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::support::{now_rfc3339, open_db_connection, open_db_connection_with_foreign_keys};

/// 计划数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub split_mode: String,
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
    pub split_mode: String,
    pub split_agent_id: Option<String>,
    pub split_model_id: Option<String>,
    pub status: String,
    pub agent_team: Option<String>, // JSON 字符串
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
    pub split_mode: Option<String>,
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
    pub split_mode: UpdateField<String>,
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

/// 将 RustPlan 转换为 Plan
fn transform_plan(rust_plan: RustPlan) -> Plan {
    let agent_team = rust_plan
        .agent_team
        .and_then(|s| serde_json::from_str(&s).ok());

    Plan {
        id: rust_plan.id,
        project_id: rust_plan.project_id,
        name: rust_plan.name,
        description: rust_plan.description,
        split_mode: rust_plan.split_mode,
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

fn collect_plan_task_ids(conn: &rusqlite::Connection, plan_id: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT id FROM tasks WHERE plan_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([plan_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// 获取指定项目的所有计划
#[tauri::command]
pub fn list_plans(project_id: String) -> Result<Vec<Plan>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, project_id, name, description, split_mode, status, agent_team,
                   split_agent_id, split_model_id,
                   granularity, max_retry_count, execution_status, current_task_id,
                   scheduled_at, schedule_status,
                   created_at, updated_at
            FROM plans
            WHERE project_id = ?1
            ORDER BY updated_at DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let plans = stmt
        .query_map([&project_id], |row| {
            Ok(RustPlan {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                split_mode: row.get(4)?,
                status: row.get(5)?,
                agent_team: row.get(6)?,
                split_agent_id: row.get(7)?,
                split_model_id: row.get(8)?,
                granularity: row.get(9)?,
                max_retry_count: row.get(10)?,
                execution_status: row.get(11)?,
                current_task_id: row.get(12)?,
                scheduled_at: row.get(13)?,
                schedule_status: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })
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

    let rust_plan = conn
        .query_row(
            r#"
            SELECT id, project_id, name, description, split_mode, status, agent_team,
                   split_agent_id, split_model_id,
                   granularity, max_retry_count, execution_status, current_task_id,
                   scheduled_at, schedule_status,
                   created_at, updated_at
            FROM plans
            WHERE id = ?1
            "#,
            [&id],
            |row| {
                Ok(RustPlan {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    split_mode: row.get(4)?,
                    status: row.get(5)?,
                    agent_team: row.get(6)?,
                    split_agent_id: row.get(7)?,
                    split_model_id: row.get(8)?,
                    granularity: row.get(9)?,
                    max_retry_count: row.get(10)?,
                    execution_status: row.get(11)?,
                    current_task_id: row.get(12)?,
                    scheduled_at: row.get(13)?,
                    schedule_status: row.get(14)?,
                    created_at: row.get(15)?,
                    updated_at: row.get(16)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(transform_plan(rust_plan))
}

/// 创建新计划
#[tauri::command]
pub fn create_plan(input: CreatePlanInput) -> Result<Plan, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let status = "draft".to_string();
    let execution_status = "idle".to_string();
    let split_mode = input.split_mode.unwrap_or_else(|| "ai".to_string());
    let agent_team_json = input
        .agent_team
        .as_ref()
        .map(|t| serde_json::to_string(t).unwrap_or_else(|_| "[]".to_string()));
    let granularity = input.granularity.unwrap_or(20);
    let max_retry_count = input.max_retry_count.unwrap_or(3);

    // 确定调度状态：如果有 scheduled_at 则为 scheduled，否则为 none
    let schedule_status = if input.scheduled_at.is_some() {
        Some("scheduled".to_string())
    } else {
        Some("none".to_string())
    };

    conn.execute(
        "INSERT INTO plans (id, project_id, name, description, split_mode, split_agent_id, split_model_id, status, agent_team,
         granularity, max_retry_count, execution_status, current_task_id, scheduled_at, schedule_status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        rusqlite::params![
            &id,
            &input.project_id,
            &input.name,
            &input.description,
            &split_mode,
            &input.split_agent_id,
            &input.split_model_id,
            &status,
            &agent_team_json,
            &granularity,
            &max_retry_count,
            &execution_status,
            &None::<String>, // current_task_id
            &input.scheduled_at,
            &schedule_status,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    // 更新项目的 updated_at 时间
    conn.execute(
        "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &input.project_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(Plan {
        id,
        project_id: input.project_id,
        name: input.name,
        description: input.description,
        split_mode,
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

    // 构建动态更新语句
    let mut updates: Vec<String> = vec!["updated_at = ?1".to_string()];
    let mut param_index = 2;

    if !matches!(input.name, UpdateField::Missing) {
        updates.push(format!("name = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.description, UpdateField::Missing) {
        updates.push(format!("description = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.split_mode, UpdateField::Missing) {
        updates.push(format!("split_mode = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.split_agent_id, UpdateField::Missing) {
        updates.push(format!("split_agent_id = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.split_model_id, UpdateField::Missing) {
        updates.push(format!("split_model_id = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.status, UpdateField::Missing) {
        updates.push(format!("status = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.agent_team, UpdateField::Missing) {
        updates.push(format!("agent_team = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.granularity, UpdateField::Missing) {
        updates.push(format!("granularity = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.max_retry_count, UpdateField::Missing) {
        updates.push(format!("max_retry_count = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.execution_status, UpdateField::Missing) {
        updates.push(format!("execution_status = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.current_task_id, UpdateField::Missing) {
        updates.push(format!("current_task_id = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.scheduled_at, UpdateField::Missing) {
        updates.push(format!("scheduled_at = ?{}", param_index));
        param_index += 1;
    }
    if !matches!(input.schedule_status, UpdateField::Missing) {
        updates.push(format!("schedule_status = ?{}", param_index));
        param_index += 1;
    }

    let sql = format!(
        "UPDATE plans SET {} WHERE id = ?{}",
        updates.join(", "),
        param_index
    );

    let mut stmt = conn.prepare_cached(&sql).map_err(|e| e.to_string())?;

    // 绑定参数
    let mut param_count = 1;
    stmt.raw_bind_parameter(param_count, &now)
        .map_err(|e| e.to_string())?;
    param_count += 1;

    if let UpdateField::Value(ref name) = input.name {
        stmt.raw_bind_parameter(param_count, name)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if !matches!(input.description, UpdateField::Missing) {
        match input.description {
            UpdateField::Value(ref description) => stmt
                .raw_bind_parameter(param_count, description)
                .map_err(|e| e.to_string())?,
            UpdateField::Null => stmt
                .raw_bind_parameter(param_count, rusqlite::types::Null)
                .map_err(|e| e.to_string())?,
            UpdateField::Missing => {}
        }
        param_count += 1;
    }
    if let UpdateField::Value(ref split_mode) = input.split_mode {
        stmt.raw_bind_parameter(param_count, split_mode)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if !matches!(input.split_agent_id, UpdateField::Missing) {
        match input.split_agent_id {
            UpdateField::Value(ref split_agent_id) => stmt
                .raw_bind_parameter(param_count, split_agent_id)
                .map_err(|e| e.to_string())?,
            UpdateField::Null => stmt
                .raw_bind_parameter(param_count, rusqlite::types::Null)
                .map_err(|e| e.to_string())?,
            UpdateField::Missing => {}
        }
        param_count += 1;
    }
    if !matches!(input.split_model_id, UpdateField::Missing) {
        match input.split_model_id {
            UpdateField::Value(ref split_model_id) => stmt
                .raw_bind_parameter(param_count, split_model_id)
                .map_err(|e| e.to_string())?,
            UpdateField::Null => stmt
                .raw_bind_parameter(param_count, rusqlite::types::Null)
                .map_err(|e| e.to_string())?,
            UpdateField::Missing => {}
        }
        param_count += 1;
    }
    if let UpdateField::Value(ref status) = input.status {
        stmt.raw_bind_parameter(param_count, status)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let UpdateField::Value(ref agent_team) = input.agent_team {
        let json = serde_json::to_string(agent_team).unwrap_or_else(|_| "[]".to_string());
        stmt.raw_bind_parameter(param_count, json)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let UpdateField::Value(granularity) = input.granularity {
        stmt.raw_bind_parameter(param_count, granularity)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if let UpdateField::Value(max_retry_count) = input.max_retry_count {
        stmt.raw_bind_parameter(param_count, max_retry_count)
            .map_err(|e| e.to_string())?;
        param_count += 1;
    }
    if !matches!(input.execution_status, UpdateField::Missing) {
        match input.execution_status {
            UpdateField::Value(ref execution_status) => stmt
                .raw_bind_parameter(param_count, execution_status)
                .map_err(|e| e.to_string())?,
            UpdateField::Null => stmt
                .raw_bind_parameter(param_count, rusqlite::types::Null)
                .map_err(|e| e.to_string())?,
            UpdateField::Missing => {}
        }
        param_count += 1;
    }
    if !matches!(input.current_task_id, UpdateField::Missing) {
        match input.current_task_id {
            UpdateField::Value(ref current_task_id) => stmt
                .raw_bind_parameter(param_count, current_task_id)
                .map_err(|e| e.to_string())?,
            UpdateField::Null => stmt
                .raw_bind_parameter(param_count, rusqlite::types::Null)
                .map_err(|e| e.to_string())?,
            UpdateField::Missing => {}
        }
        param_count += 1;
    }
    if !matches!(input.scheduled_at, UpdateField::Missing) {
        match input.scheduled_at {
            UpdateField::Value(ref scheduled_at) => stmt
                .raw_bind_parameter(param_count, scheduled_at)
                .map_err(|e| e.to_string())?,
            UpdateField::Null => stmt
                .raw_bind_parameter(param_count, rusqlite::types::Null)
                .map_err(|e| e.to_string())?,
            UpdateField::Missing => {}
        }
        param_count += 1;
    }
    if !matches!(input.schedule_status, UpdateField::Missing) {
        match input.schedule_status {
            UpdateField::Value(ref schedule_status) => stmt
                .raw_bind_parameter(param_count, schedule_status)
                .map_err(|e| e.to_string())?,
            UpdateField::Null => stmt
                .raw_bind_parameter(param_count, rusqlite::types::Null)
                .map_err(|e| e.to_string())?,
            UpdateField::Missing => {}
        }
        param_count += 1;
    }

    stmt.raw_bind_parameter(param_count, &id)
        .map_err(|e| e.to_string())?;
    stmt.raw_execute().map_err(|e| e.to_string())?;

    // 获取更新后的计划
    let rust_plan = conn
        .query_row(
            r#"
            SELECT id, project_id, name, description, split_mode, status, agent_team,
                   split_agent_id, split_model_id,
                   granularity, max_retry_count, execution_status, current_task_id,
                   scheduled_at, schedule_status,
                   created_at, updated_at
            FROM plans
            WHERE id = ?1
            "#,
            [&id],
            |row| {
                Ok(RustPlan {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    split_mode: row.get(4)?,
                    status: row.get(5)?,
                    agent_team: row.get(6)?,
                    split_agent_id: row.get(7)?,
                    split_model_id: row.get(8)?,
                    granularity: row.get(9)?,
                    max_retry_count: row.get(10)?,
                    execution_status: row.get(11)?,
                    current_task_id: row.get(12)?,
                    scheduled_at: row.get(13)?,
                    schedule_status: row.get(14)?,
                    created_at: row.get(15)?,
                    updated_at: row.get(16)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(transform_plan(rust_plan))
}

/// 删除计划
#[tauri::command]
pub fn delete_plan(id: String) -> Result<(), String> {
    let mut conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;
    let task_ids = collect_plan_task_ids(&conn, &id)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM plan_split_logs WHERE plan_id = ?1", [&id])
        .map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM task_split_sessions WHERE plan_id = ?1", [&id])
        .map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM task_execution_results WHERE plan_id = ?1", [&id])
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

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, project_id, name, description, split_mode, status, agent_team,
                   split_agent_id, split_model_id,
                   granularity, max_retry_count, execution_status, current_task_id,
                   scheduled_at, schedule_status,
                   created_at, updated_at
            FROM plans
            WHERE schedule_status = 'scheduled' AND scheduled_at IS NOT NULL
            ORDER BY scheduled_at ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let plans = stmt
        .query_map([], |row| {
            Ok(RustPlan {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                split_mode: row.get(4)?,
                status: row.get(5)?,
                agent_team: row.get(6)?,
                split_agent_id: row.get(7)?,
                split_model_id: row.get(8)?,
                granularity: row.get(9)?,
                max_retry_count: row.get(10)?,
                execution_status: row.get(11)?,
                current_task_id: row.get(12)?,
                scheduled_at: row.get(13)?,
                schedule_status: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })
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

    // 获取更新后的计划
    let rust_plan = conn
        .query_row(
            r#"
            SELECT id, project_id, name, description, split_mode, status, agent_team,
                   split_agent_id, split_model_id,
                   granularity, max_retry_count, execution_status, current_task_id,
                   scheduled_at, schedule_status,
                   created_at, updated_at
            FROM plans
            WHERE id = ?1
            "#,
            [&id],
            |row| {
                Ok(RustPlan {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    split_mode: row.get(4)?,
                    status: row.get(5)?,
                    agent_team: row.get(6)?,
                    split_agent_id: row.get(7)?,
                    split_model_id: row.get(8)?,
                    granularity: row.get(9)?,
                    max_retry_count: row.get(10)?,
                    execution_status: row.get(11)?,
                    current_task_id: row.get(12)?,
                    scheduled_at: row.get(13)?,
                    schedule_status: row.get(14)?,
                    created_at: row.get(15)?,
                    updated_at: row.get(16)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(transform_plan(rust_plan))
}
