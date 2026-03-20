use anyhow::Result;
use rusqlite::{types::Null, CachedStatement, Connection, Params, Row, ToSql};
use serde::{Deserialize, Serialize};

use super::support::{
    bind_value, now_rfc3339, open_db_connection, open_db_connection_with_foreign_keys,
    UpdateSqlBuilder,
};

/// 任务数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub plan_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    /// 执行智能体 ID */
    pub agent_id: Option<String>,
    /// 执行模型 ID */
    pub model_id: Option<String>,
    /// 推荐执行智能体 ID */
    pub recommended_agent_id: Option<String>,
    /// 推荐执行模型 ID */
    pub recommended_model_id: Option<String>,
    /// 推荐原因 */
    pub recommendation_reason: Option<String>,
    pub session_id: Option<String>,
    pub progress_file: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub order: i32,
    pub retry_count: i32,
    pub max_retries: i32,
    pub error_message: Option<String>,
    pub implementation_steps: Option<Vec<String>>,
    pub test_steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub block_reason: Option<String>,
    pub input_request: Option<serde_json::Value>,
    pub input_response: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// Rust 后端返回的结构（snake_case）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustTask {
    pub id: String,
    pub plan_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    /// 执行智能体 ID */
    pub agent_id: Option<String>,
    /// 执行模型 ID */
    pub model_id: Option<String>,
    pub recommended_agent_id: Option<String>,
    pub recommended_model_id: Option<String>,
    pub recommendation_reason: Option<String>,
    pub session_id: Option<String>,
    pub progress_file: Option<String>,
    pub dependencies: Option<String>, // JSON 字符串
    pub task_order: i32,
    pub retry_count: i32,
    pub max_retries: i32,
    pub error_message: Option<String>,
    pub implementation_steps: Option<String>, // JSON 字符串
    pub test_steps: Option<String>,           // JSON 字符串
    pub acceptance_criteria: Option<String>,  // JSON 字符串
    pub block_reason: Option<String>,
    pub input_request: Option<String>,  // JSON 字符串
    pub input_response: Option<String>, // JSON 字符串
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

/// 创建任务输入
#[derive(Debug, Deserialize)]
pub struct CreateTaskInput {
    pub plan_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    /// 执行智能体 ID */
    pub agent_id: Option<String>,
    /// 执行模型 ID */
    pub model_id: Option<String>,
    pub recommended_agent_id: Option<String>,
    pub recommended_model_id: Option<String>,
    pub recommendation_reason: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub order: Option<i32>,
    pub max_retries: Option<i32>,
    pub implementation_steps: Option<Vec<String>>,
    pub test_steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
}

/// 更新任务输入
#[derive(Debug, Deserialize)]
pub struct UpdateTaskInput {
    #[serde(default)]
    pub title: UpdateField<String>,
    #[serde(default)]
    pub description: UpdateField<String>,
    #[serde(default)]
    pub status: UpdateField<String>,
    #[serde(default)]
    pub priority: UpdateField<String>,
    #[serde(default)]
    pub assignee: UpdateField<String>,
    #[serde(default)]
    pub agent_id: UpdateField<String>,
    #[serde(default)]
    pub model_id: UpdateField<String>,
    #[serde(default)]
    pub recommended_agent_id: UpdateField<String>,
    #[serde(default)]
    pub recommended_model_id: UpdateField<String>,
    #[serde(default)]
    pub recommendation_reason: UpdateField<String>,
    #[serde(default)]
    pub session_id: UpdateField<String>,
    #[serde(default)]
    pub progress_file: UpdateField<String>,
    #[serde(default)]
    pub dependencies: UpdateField<Vec<String>>,
    #[serde(default)]
    pub order: UpdateField<i32>,
    #[serde(default)]
    pub retry_count: UpdateField<i32>,
    #[serde(default)]
    pub max_retries: UpdateField<i32>,
    #[serde(default)]
    pub error_message: UpdateField<String>,
    #[serde(default)]
    pub implementation_steps: UpdateField<Vec<String>>,
    #[serde(default)]
    pub test_steps: UpdateField<Vec<String>>,
    #[serde(default)]
    pub acceptance_criteria: UpdateField<Vec<String>>,
    #[serde(default)]
    pub block_reason: UpdateField<String>,
    #[serde(default)]
    pub input_request: UpdateField<serde_json::Value>,
    #[serde(default)]
    pub input_response: UpdateField<serde_json::Value>,
}

/// 批量更新任务顺序输入
#[derive(Debug, Deserialize)]
pub struct ReorderTasksInput {
    pub task_orders: Vec<TaskOrderItem>,
}

#[derive(Debug, Deserialize)]
pub struct TaskOrderItem {
    pub id: String,
    pub order: i32,
}

fn has_update<T>(field: &UpdateField<T>) -> bool {
    !matches!(field, UpdateField::Missing)
}

fn push_update<T>(builder: &mut UpdateSqlBuilder, column: &str, field: &UpdateField<T>) {
    builder.push(column, has_update(field));
}

fn bind_update_field<T: ToSql>(
    stmt: &mut CachedStatement<'_>,
    param_index: &mut usize,
    field: &UpdateField<T>,
) -> rusqlite::Result<()> {
    match field {
        UpdateField::Value(value) => stmt.raw_bind_parameter(*param_index, value)?,
        UpdateField::Null => stmt.raw_bind_parameter(*param_index, Null)?,
        UpdateField::Missing => return Ok(()),
    }

    *param_index += 1;
    Ok(())
}

fn bind_update_json<T: Serialize>(
    stmt: &mut CachedStatement<'_>,
    param_index: &mut usize,
    field: &UpdateField<T>,
    fallback: &str,
) -> rusqlite::Result<()> {
    match field {
        UpdateField::Value(value) => {
            let json = serde_json::to_string(value).unwrap_or_else(|_| fallback.to_string());
            stmt.raw_bind_parameter(*param_index, json)?;
        }
        UpdateField::Null => stmt.raw_bind_parameter(*param_index, Null)?,
        UpdateField::Missing => return Ok(()),
    }

    *param_index += 1;
    Ok(())
}

const TASK_SELECT_BY_ID_SQL: &str = r#"
    SELECT id, plan_id, parent_id, title, description, status, priority,
           assignee, agent_id, model_id, recommended_agent_id, recommended_model_id, recommendation_reason,
           session_id, progress_file, dependencies, task_order,
           retry_count, max_retries, error_message,
           implementation_steps, test_steps, acceptance_criteria,
           block_reason, input_request, input_response,
           created_at, updated_at
    FROM tasks
    WHERE id = ?1
"#;
const TASK_SELECT_BY_PLAN_SQL: &str = r#"
    SELECT id, plan_id, parent_id, title, description, status, priority,
           assignee, agent_id, model_id, recommended_agent_id, recommended_model_id, recommendation_reason,
           session_id, progress_file, dependencies, task_order,
           retry_count, max_retries, error_message,
           implementation_steps, test_steps, acceptance_criteria,
           block_reason, input_request, input_response,
           created_at, updated_at
    FROM tasks
    WHERE plan_id = ?1
    ORDER BY task_order ASC, created_at ASC
"#;
const TASK_SELECT_BY_PARENT_SQL: &str = r#"
    SELECT id, plan_id, parent_id, title, description, status, priority,
           assignee, agent_id, model_id, recommended_agent_id, recommended_model_id, recommendation_reason,
           session_id, progress_file, dependencies, task_order,
           retry_count, max_retries, error_message,
           implementation_steps, test_steps, acceptance_criteria,
           block_reason, input_request, input_response,
           created_at, updated_at
    FROM tasks
    WHERE parent_id = ?1
    ORDER BY task_order ASC, created_at ASC
"#;
const TASK_SELECT_BY_SESSION_SQL: &str = r#"
    SELECT id, plan_id, parent_id, title, description, status, priority,
           assignee, agent_id, model_id, recommended_agent_id, recommended_model_id, recommendation_reason,
           session_id, progress_file, dependencies, task_order,
           retry_count, max_retries, error_message,
           implementation_steps, test_steps, acceptance_criteria,
           block_reason, input_request, input_response,
           created_at, updated_at
    FROM tasks
    WHERE session_id = ?1
    LIMIT 1
"#;

fn map_rust_task_row(row: &Row<'_>) -> rusqlite::Result<RustTask> {
    Ok(RustTask {
        id: row.get(0)?,
        plan_id: row.get(1)?,
        parent_id: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        status: row.get(5)?,
        priority: row.get(6)?,
        assignee: row.get(7)?,
        agent_id: row.get(8)?,
        model_id: row.get(9)?,
        recommended_agent_id: row.get(10)?,
        recommended_model_id: row.get(11)?,
        recommendation_reason: row.get(12)?,
        session_id: row.get(13)?,
        progress_file: row.get(14)?,
        dependencies: row.get(15)?,
        task_order: row.get(16)?,
        retry_count: row.get(17)?,
        max_retries: row.get(18)?,
        error_message: row.get(19)?,
        implementation_steps: row.get(20)?,
        test_steps: row.get(21)?,
        acceptance_criteria: row.get(22)?,
        block_reason: row.get(23)?,
        input_request: row.get(24)?,
        input_response: row.get(25)?,
        created_at: row.get(26)?,
        updated_at: row.get(27)?,
    })
}

fn collect_tasks<P>(conn: &Connection, sql: &str, params: P) -> Result<Vec<Task>, String>
where
    P: Params,
{
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params, map_rust_task_row)
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
        .map(|tasks| tasks.into_iter().map(transform_task).collect())
}

fn fetch_task<P>(conn: &Connection, sql: &str, params: P) -> Result<Task, String>
where
    P: Params,
{
    conn.query_row(sql, params, map_rust_task_row)
        .map(transform_task)
        .map_err(|e| e.to_string())
}

fn fetch_optional_task<P>(conn: &Connection, sql: &str, params: P) -> Result<Option<Task>, String>
where
    P: Params,
{
    match conn.query_row(sql, params, map_rust_task_row) {
        Ok(task) => Ok(Some(transform_task(task))),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn serialize_json_option<T: Serialize>(value: Option<&T>, fallback: &str) -> Option<String> {
    value.map(|value| serde_json::to_string(value).unwrap_or_else(|_| fallback.to_string()))
}

/// 将 RustTask 转换为 Task
fn transform_task(rust_task: RustTask) -> Task {
    let dependencies = rust_task
        .dependencies
        .and_then(|s| serde_json::from_str(&s).ok());
    let implementation_steps = rust_task
        .implementation_steps
        .and_then(|s| serde_json::from_str(&s).ok());
    let test_steps = rust_task
        .test_steps
        .and_then(|s| serde_json::from_str(&s).ok());
    let acceptance_criteria = rust_task
        .acceptance_criteria
        .and_then(|s| serde_json::from_str(&s).ok());
    let input_request = rust_task
        .input_request
        .and_then(|s| serde_json::from_str(&s).ok());
    let input_response = rust_task
        .input_response
        .and_then(|s| serde_json::from_str(&s).ok());

    Task {
        id: rust_task.id,
        plan_id: rust_task.plan_id,
        parent_id: rust_task.parent_id,
        title: rust_task.title,
        description: rust_task.description,
        status: rust_task.status,
        priority: rust_task.priority,
        assignee: rust_task.assignee,
        agent_id: rust_task.agent_id,
        model_id: rust_task.model_id,
        recommended_agent_id: rust_task.recommended_agent_id,
        recommended_model_id: rust_task.recommended_model_id,
        recommendation_reason: rust_task.recommendation_reason,
        session_id: rust_task.session_id,
        progress_file: rust_task.progress_file,
        dependencies,
        order: rust_task.task_order,
        retry_count: rust_task.retry_count,
        max_retries: rust_task.max_retries,
        error_message: rust_task.error_message,
        implementation_steps,
        test_steps,
        acceptance_criteria,
        block_reason: rust_task.block_reason,
        input_request,
        input_response,
        created_at: rust_task.created_at,
        updated_at: rust_task.updated_at,
    }
}

/// 获取指定计划的所有任务
#[tauri::command]
pub fn list_tasks(plan_id: String) -> Result<Vec<Task>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    collect_tasks(&conn, TASK_SELECT_BY_PLAN_SQL, [&plan_id])
}

/// 获取单个任务
#[tauri::command]
pub fn get_task(id: String) -> Result<Task, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    fetch_task(&conn, TASK_SELECT_BY_ID_SQL, [&id])
}

/// 创建新任务
#[tauri::command]
pub fn create_task(input: CreateTaskInput) -> Result<Task, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let status = "pending".to_string();
    let priority = input.priority.unwrap_or_else(|| "medium".to_string());
    let dependencies_json = serialize_json_option(input.dependencies.as_ref(), "[]");
    let max_retries = input.max_retries.unwrap_or(3);
    let implementation_steps_json =
        serialize_json_option(input.implementation_steps.as_ref(), "[]");
    let test_steps_json = serialize_json_option(input.test_steps.as_ref(), "[]");
    let acceptance_criteria_json = serialize_json_option(input.acceptance_criteria.as_ref(), "[]");

    // 如果没有指定顺序，获取当前最大顺序 + 1
    let task_order = match input.order {
        Some(order) => order,
        None => {
            let max_order: i32 = conn
                .query_row(
                    "SELECT COALESCE(MAX(task_order), -1) FROM tasks WHERE plan_id = ?1",
                    [&input.plan_id],
                    |row| row.get(0),
                )
                .unwrap_or(-1);
            max_order + 1
        }
    };

    conn.execute(
        "INSERT INTO tasks (id, plan_id, parent_id, title, description, status, priority,
         assignee, agent_id, model_id, recommended_agent_id, recommended_model_id, recommendation_reason,
         session_id, progress_file, dependencies, task_order,
         retry_count, max_retries, error_message,
         implementation_steps, test_steps, acceptance_criteria,
         created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
        rusqlite::params![
            &id,
            &input.plan_id,
            &input.parent_id,
            &input.title,
            &input.description,
            &status,
            &priority,
            &input.assignee,
            &input.agent_id,
            &input.model_id,
            &input.recommended_agent_id,
            &input.recommended_model_id,
            &input.recommendation_reason,
            &None::<String>, // session_id
            &None::<String>, // progress_file
            &dependencies_json,
            &task_order,
            0, // retry_count
            &max_retries,
            &None::<String>, // error_message
            &implementation_steps_json,
            &test_steps_json,
            &acceptance_criteria_json,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    // 更新计划的 updated_at 时间
    conn.execute(
        "UPDATE plans SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &input.plan_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(Task {
        id,
        plan_id: input.plan_id,
        parent_id: input.parent_id,
        title: input.title,
        description: input.description,
        status,
        priority,
        assignee: input.assignee,
        agent_id: input.agent_id,
        model_id: input.model_id,
        recommended_agent_id: input.recommended_agent_id,
        recommended_model_id: input.recommended_model_id,
        recommendation_reason: input.recommendation_reason,
        session_id: None,
        progress_file: None,
        dependencies: input.dependencies,
        order: task_order,
        retry_count: 0,
        max_retries,
        error_message: None,
        implementation_steps: input.implementation_steps,
        test_steps: input.test_steps,
        acceptance_criteria: input.acceptance_criteria,
        block_reason: None,
        input_request: None,
        input_response: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 更新任务
#[tauri::command]
pub fn update_task(id: String, input: UpdateTaskInput) -> Result<Task, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let now = now_rfc3339();

    let mut updates = UpdateSqlBuilder::new();
    push_update(&mut updates, "title", &input.title);
    push_update(&mut updates, "description", &input.description);
    push_update(&mut updates, "status", &input.status);
    push_update(&mut updates, "priority", &input.priority);
    push_update(&mut updates, "assignee", &input.assignee);
    push_update(&mut updates, "agent_id", &input.agent_id);
    push_update(&mut updates, "model_id", &input.model_id);
    push_update(&mut updates, "recommended_agent_id", &input.recommended_agent_id);
    push_update(&mut updates, "recommended_model_id", &input.recommended_model_id);
    push_update(&mut updates, "recommendation_reason", &input.recommendation_reason);
    push_update(&mut updates, "session_id", &input.session_id);
    push_update(&mut updates, "progress_file", &input.progress_file);
    push_update(&mut updates, "dependencies", &input.dependencies);
    push_update(&mut updates, "task_order", &input.order);
    push_update(&mut updates, "retry_count", &input.retry_count);
    push_update(&mut updates, "max_retries", &input.max_retries);
    push_update(&mut updates, "error_message", &input.error_message);
    push_update(
        &mut updates,
        "implementation_steps",
        &input.implementation_steps,
    );
    push_update(&mut updates, "test_steps", &input.test_steps);
    push_update(
        &mut updates,
        "acceptance_criteria",
        &input.acceptance_criteria,
    );
    push_update(&mut updates, "block_reason", &input.block_reason);
    push_update(&mut updates, "input_request", &input.input_request);
    push_update(&mut updates, "input_response", &input.input_response);

    let sql = updates.finish("tasks", "id");

    let mut stmt = conn.prepare_cached(&sql).map_err(|e| e.to_string())?;

    let mut param_count = 1;
    bind_value(&mut stmt, &mut param_count, &now).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.title).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.description)
        .map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.status).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.priority).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.assignee).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.agent_id).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.model_id).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.recommended_agent_id)
        .map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.recommended_model_id)
        .map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.recommendation_reason)
        .map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.session_id).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.progress_file)
        .map_err(|e| e.to_string())?;
    bind_update_json(&mut stmt, &mut param_count, &input.dependencies, "[]")
        .map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.order).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.retry_count)
        .map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.max_retries)
        .map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.error_message)
        .map_err(|e| e.to_string())?;
    bind_update_json(
        &mut stmt,
        &mut param_count,
        &input.implementation_steps,
        "[]",
    )
    .map_err(|e| e.to_string())?;
    bind_update_json(&mut stmt, &mut param_count, &input.test_steps, "[]")
        .map_err(|e| e.to_string())?;
    bind_update_json(
        &mut stmt,
        &mut param_count,
        &input.acceptance_criteria,
        "[]",
    )
    .map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.block_reason)
        .map_err(|e| e.to_string())?;
    bind_update_json(&mut stmt, &mut param_count, &input.input_request, "{}")
        .map_err(|e| e.to_string())?;
    bind_update_json(&mut stmt, &mut param_count, &input.input_response, "{}")
        .map_err(|e| e.to_string())?;
    bind_value(&mut stmt, &mut param_count, &id).map_err(|e| e.to_string())?;
    stmt.raw_execute().map_err(|e| e.to_string())?;

    // 更新计划的 updated_at 时间
    let plan_id: String = conn
        .query_row("SELECT plan_id FROM tasks WHERE id = ?1", [&id], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE plans SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &plan_id],
    )
    .map_err(|e| e.to_string())?;

    // 获取更新后的任务
    fetch_task(&conn, TASK_SELECT_BY_ID_SQL, [&id])
}

/// 批量更新任务顺序
#[tauri::command]
pub fn reorder_tasks(input: ReorderTasksInput) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let now = now_rfc3339();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for item in input.task_orders {
        tx.execute(
            "UPDATE tasks SET task_order = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![item.order, &now, &item.id],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(())
}

/// 删除任务
#[tauri::command]
pub fn delete_task(id: String) -> Result<(), String> {
    let conn = open_db_connection_with_foreign_keys().map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM tasks WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取任务的子任务
#[tauri::command]
pub fn list_subtasks(parent_id: String) -> Result<Vec<Task>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    collect_tasks(&conn, TASK_SELECT_BY_PARENT_SQL, [&parent_id])
}

/// 重试任务 - 重置重试计数并恢复pending状态
#[tauri::command]
pub fn retry_task(id: String) -> Result<Task, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let now = now_rfc3339();

    conn.execute(
        "UPDATE tasks SET status = 'pending', retry_count = 0, error_message = NULL, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &id],
    )
    .map_err(|e| e.to_string())?;

    // 获取更新后的任务
    fetch_task(&conn, TASK_SELECT_BY_ID_SQL, [&id])
}

/// 批量更新任务状态
#[tauri::command]
pub fn batch_update_status(plan_id: String, status: String) -> Result<Vec<Task>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let now = now_rfc3339();

    // 只更新 pending 状态的任务
    conn.execute(
        "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE plan_id = ?3 AND status = 'pending'",
        rusqlite::params![&status, &now, &plan_id],
    )
    .map_err(|e| e.to_string())?;

    // 获取更新后的任务列表
    collect_tasks(&conn, TASK_SELECT_BY_PLAN_SQL, [&plan_id])
}

/// 停止任务执行
#[tauri::command]
pub fn stop_task(id: String) -> Result<Task, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let now = now_rfc3339();

    // 将任务状态改为 pending，保留当前重试计数
    conn.execute(
        "UPDATE tasks SET status = 'pending', updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &id],
    )
    .map_err(|e| e.to_string())?;

    // 获取更新后的任务
    fetch_task(&conn, TASK_SELECT_BY_ID_SQL, [&id])
}

/// 根据会话 ID 查找关联的任务和计划
#[tauri::command]
pub fn get_task_by_session_id(session_id: String) -> Result<Option<Task>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    fetch_optional_task(&conn, TASK_SELECT_BY_SESSION_SQL, [&session_id])
}

/// 批量创建任务（从拆分结果）
#[tauri::command]
pub fn batch_create_tasks(
    plan_id: String,
    tasks: Vec<CreateTaskInput>,
) -> Result<Vec<Task>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let now = now_rfc3339();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    // 获取当前最大顺序
    let mut max_order: i32 = tx
        .query_row(
            "SELECT COALESCE(MAX(task_order), -1) FROM tasks WHERE plan_id = ?1",
            [&plan_id],
            |row| row.get(0),
        )
        .unwrap_or(-1);

    let mut created_tasks = Vec::new();

    for task_input in tasks {
        let id = uuid::Uuid::new_v4().to_string();
        let status = "pending".to_string();
        let priority = task_input
            .priority
            .clone()
            .unwrap_or_else(|| "medium".to_string());
        let dependencies_json = serialize_json_option(task_input.dependencies.as_ref(), "[]");
        let max_retries = task_input.max_retries.unwrap_or(3);
        let implementation_steps_json =
            serialize_json_option(task_input.implementation_steps.as_ref(), "[]");
        let test_steps_json = serialize_json_option(task_input.test_steps.as_ref(), "[]");
        let acceptance_criteria_json =
            serialize_json_option(task_input.acceptance_criteria.as_ref(), "[]");

        max_order += 1;
        let task_order = task_input.order.unwrap_or(max_order);

        tx.execute(
            "INSERT INTO tasks (id, plan_id, parent_id, title, description, status, priority,
             assignee, agent_id, model_id, recommended_agent_id, recommended_model_id, recommendation_reason,
             session_id, progress_file, dependencies, task_order,
             retry_count, max_retries, error_message,
             implementation_steps, test_steps, acceptance_criteria,
             created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
            rusqlite::params![
                &id,
                &plan_id,
                &task_input.parent_id,
                &task_input.title,
                &task_input.description,
                &status,
                &priority,
                &task_input.assignee,
                &task_input.agent_id,
                &task_input.model_id,
                &task_input.recommended_agent_id,
                &task_input.recommended_model_id,
                &task_input.recommendation_reason,
                &None::<String>, // session_id
                &None::<String>, // progress_file
                &dependencies_json,
                &task_order,
                0, // retry_count
                &max_retries,
                &None::<String>, // error_message
                &implementation_steps_json,
                &test_steps_json,
                &acceptance_criteria_json,
                &now,
                &now
            ],
        )
        .map_err(|e| e.to_string())?;

        created_tasks.push(Task {
            id,
            plan_id: plan_id.clone(),
            parent_id: task_input.parent_id,
            title: task_input.title,
            description: task_input.description,
            status,
            priority,
            assignee: task_input.assignee,
            agent_id: task_input.agent_id,
            model_id: task_input.model_id,
            recommended_agent_id: task_input.recommended_agent_id,
            recommended_model_id: task_input.recommended_model_id,
            recommendation_reason: task_input.recommendation_reason,
            session_id: None,
            progress_file: None,
            dependencies: task_input.dependencies,
            order: task_order,
            retry_count: 0,
            max_retries,
            error_message: None,
            implementation_steps: task_input.implementation_steps,
            test_steps: task_input.test_steps,
            acceptance_criteria: task_input.acceptance_criteria,
            block_reason: None,
            input_request: None,
            input_response: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        });
    }

    // 更新计划的 updated_at 时间
    tx.execute(
        "UPDATE plans SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &plan_id],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(created_tasks)
}

/// 任务拆分会话结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSplitSession {
    pub id: String,
    pub plan_id: String,
    pub status: String,
    pub raw_content: Option<String>,
    pub parsed_output: Option<String>,
    pub parse_error: Option<String>,
    pub granularity: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 保存拆分会话输入
#[derive(Debug, Deserialize)]
pub struct SaveSplitSessionInput {
    pub plan_id: String,
    pub status: Option<String>,
    pub raw_content: Option<String>,
    pub parsed_output: Option<String>,
    pub parse_error: Option<String>,
    pub granularity: Option<i32>,
}

/// 保存或更新拆分会话
#[tauri::command]
pub fn save_split_session(input: SaveSplitSessionInput) -> Result<TaskSplitSession, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let now = now_rfc3339();
    let status = input.status.unwrap_or_else(|| "processing".to_string());
    let granularity = input.granularity.unwrap_or(20);

    // 检查是否已存在该 plan_id 的记录
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM task_split_sessions WHERE plan_id = ?1",
            [&input.plan_id],
            |row| row.get(0),
        )
        .ok();

    let session_id = if let Some(id) = existing {
        // 更新现有记录
        conn.execute(
            "UPDATE task_split_sessions SET status = ?1, raw_content = ?2, parsed_output = ?3,
             parse_error = ?4, granularity = ?5, updated_at = ?6 WHERE id = ?7",
            rusqlite::params![
                &status,
                &input.raw_content,
                &input.parsed_output,
                &input.parse_error,
                granularity,
                &now,
                &id
            ],
        )
        .map_err(|e| e.to_string())?;
        id
    } else {
        // 创建新记录
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO task_split_sessions (id, plan_id, status, raw_content, parsed_output,
             parse_error, granularity, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &id,
                &input.plan_id,
                &status,
                &input.raw_content,
                &input.parsed_output,
                &input.parse_error,
                granularity,
                &now,
                &now
            ],
        )
        .map_err(|e| e.to_string())?;
        id
    };

    Ok(TaskSplitSession {
        id: session_id,
        plan_id: input.plan_id,
        status,
        raw_content: input.raw_content,
        parsed_output: input.parsed_output,
        parse_error: input.parse_error,
        granularity,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 获取拆分会话
#[tauri::command]
pub fn get_split_session(plan_id: String) -> Result<Option<TaskSplitSession>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let result = conn.query_row(
        "SELECT id, plan_id, status, raw_content, parsed_output, parse_error, granularity,
         created_at, updated_at FROM task_split_sessions WHERE plan_id = ?1",
        [&plan_id],
        |row| {
            Ok(TaskSplitSession {
                id: row.get(0)?,
                plan_id: row.get(1)?,
                status: row.get(2)?,
                raw_content: row.get(3)?,
                parsed_output: row.get(4)?,
                parse_error: row.get(5)?,
                granularity: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    );

    match result {
        Ok(session) => Ok(Some(session)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// 删除拆分会话
#[tauri::command]
pub fn delete_split_session(plan_id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM task_split_sessions WHERE plan_id = ?1",
        [&plan_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
