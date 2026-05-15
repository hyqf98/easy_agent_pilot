use anyhow::Result;
use rusqlite::{types::Null, CachedStatement, Connection, Params, Row, ToSql};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::support::{
    bind_value, now_rfc3339, open_db_connection,
    UpdateSqlBuilder,
};

/// 任务数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub project_id: Option<String>,
    pub plan_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    pub expert_id: Option<String>,
    /// 执行智能体 ID */
    pub agent_id: Option<String>,
    /// 执行模型 ID */
    pub model_id: Option<String>,
    pub session_id: Option<String>,
    pub cli_session_provider: Option<String>,
    pub progress_file: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub order: i32,
    pub retry_count: i32,
    pub max_retries: i32,
    pub error_message: Option<String>,
    pub implementation_steps: Option<Vec<String>>,
    pub test_steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub memory_library_ids: Option<Vec<String>>,
    pub block_reason: Option<String>,
    pub input_request: Option<serde_json::Value>,
    pub input_response: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// 任务运行时绑定。
/// 任务在不同 CLI 运行时下会产生各自独立的外部恢复游标，必须按 runtime_key 隔离存储。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRuntimeBinding {
    pub task_id: String,
    pub runtime_key: String,
    pub external_session_id: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Rust 后端返回的结构（snake_case）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustTask {
    pub id: String,
    pub project_id: Option<String>,
    pub plan_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    pub expert_id: Option<String>,
    /// 执行智能体 ID */
    pub agent_id: Option<String>,
    /// 执行模型 ID */
    pub model_id: Option<String>,
    pub session_id: Option<String>,
    pub cli_session_provider: Option<String>,
    pub progress_file: Option<String>,
    pub dependencies: Option<String>, // JSON 字符串
    pub task_order: i32,
    pub retry_count: i32,
    pub max_retries: i32,
    pub error_message: Option<String>,
    pub implementation_steps: Option<String>, // JSON 字符串
    pub test_steps: Option<String>,           // JSON 字符串
    pub acceptance_criteria: Option<String>,  // JSON 字符串
    pub memory_library_ids: Option<String>,   // JSON 字符串
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
    pub expert_id: Option<String>,
    /// 执行智能体 ID */
    pub agent_id: Option<String>,
    /// 执行模型 ID */
    pub model_id: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub order: Option<i32>,
    pub max_retries: Option<i32>,
    pub implementation_steps: Option<Vec<String>>,
    pub test_steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub memory_library_ids: Option<Vec<String>>,
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
    pub expert_id: UpdateField<String>,
    #[serde(default)]
    pub agent_id: UpdateField<String>,
    #[serde(default)]
    pub model_id: UpdateField<String>,
    #[serde(default)]
    pub session_id: UpdateField<String>,
    #[serde(default)]
    pub cli_session_provider: UpdateField<String>,
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
    pub memory_library_ids: UpdateField<Vec<String>>,
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

fn replace_task_memory_libraries(
    conn: &rusqlite::Connection,
    task_id: &str,
    library_ids: &[String],
    now: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM task_memory_libraries WHERE task_id = ?1",
        [task_id],
    )
    .map_err(|e| e.to_string())?;

    for library_id in normalize_memory_library_ids(library_ids) {
        conn.execute(
            r#"
            INSERT INTO task_memory_libraries (task_id, library_id, created_at)
            VALUES (?1, ?2, ?3)
            "#,
            rusqlite::params![task_id, library_id, now],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn list_task_memory_library_ids(conn: &Connection, task_id: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT library_id
            FROM task_memory_libraries
            WHERE task_id = ?1
            ORDER BY created_at ASC, library_id ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([task_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

const TASK_SELECT_BY_ID_SQL: &str = r#"
    SELECT id, project_id, plan_id, parent_id, title, description, status, priority,
           assignee, expert_id, agent_id, model_id, session_id, cli_session_provider, progress_file, dependencies, task_order,
           retry_count, max_retries, error_message,
           implementation_steps, test_steps, acceptance_criteria, memory_library_ids,
           block_reason, input_request, input_response,
           created_at, updated_at
    FROM tasks
    WHERE id = ?1
"#;
const TASK_SELECT_BY_PLAN_SQL: &str = r#"
    SELECT id, project_id, plan_id, parent_id, title, description, status, priority,
           assignee, expert_id, agent_id, model_id, session_id, cli_session_provider, progress_file, dependencies, task_order,
           retry_count, max_retries, error_message,
           implementation_steps, test_steps, acceptance_criteria, memory_library_ids,
           block_reason, input_request, input_response,
           created_at, updated_at
    FROM tasks
    WHERE plan_id = ?1
    ORDER BY task_order ASC, created_at ASC
"#;
const TASK_SELECT_BY_PROJECT_UNPLANNED_SQL: &str = r#"
    SELECT id, project_id, plan_id, parent_id, title, description, status, priority,
           assignee, expert_id, agent_id, model_id, session_id, cli_session_provider, progress_file, dependencies, task_order,
           retry_count, max_retries, error_message,
           implementation_steps, test_steps, acceptance_criteria, memory_library_ids,
           block_reason, input_request, input_response,
           created_at, updated_at
    FROM tasks
    WHERE project_id = ?1
      AND (
        trim(COALESCE(plan_id, '')) = ''
        OR NOT EXISTS (
          SELECT 1
          FROM plans
          WHERE plans.id = tasks.plan_id
        )
      )
    ORDER BY task_order ASC, created_at ASC
"#;
const TASK_SELECT_BY_PARENT_SQL: &str = r#"
    SELECT id, project_id, plan_id, parent_id, title, description, status, priority,
           assignee, expert_id, agent_id, model_id, session_id, cli_session_provider, progress_file, dependencies, task_order,
           retry_count, max_retries, error_message,
           implementation_steps, test_steps, acceptance_criteria, memory_library_ids,
           block_reason, input_request, input_response,
           created_at, updated_at
    FROM tasks
    WHERE parent_id = ?1
    ORDER BY task_order ASC, created_at ASC
"#;
const TASK_SELECT_BY_SESSION_SQL: &str = r#"
    SELECT id, project_id, plan_id, parent_id, title, description, status, priority,
           assignee, expert_id, agent_id, model_id, session_id, cli_session_provider, progress_file, dependencies, task_order,
           retry_count, max_retries, error_message,
           implementation_steps, test_steps, acceptance_criteria, memory_library_ids,
           block_reason, input_request, input_response,
           created_at, updated_at
    FROM tasks
    WHERE session_id = ?1
    LIMIT 1
"#;

fn map_rust_task_row(row: &Row<'_>) -> rusqlite::Result<RustTask> {
    Ok(RustTask {
        id: row.get(0)?,
        project_id: row.get(1)?,
        plan_id: row.get(2)?,
        parent_id: row.get(3)?,
        title: row.get(4)?,
        description: row.get(5)?,
        status: row.get(6)?,
        priority: row.get(7)?,
        assignee: row.get(8)?,
        expert_id: row.get(9)?,
        agent_id: row.get(10)?,
        model_id: row.get(11)?,
        session_id: row.get(12)?,
        cli_session_provider: row.get(13)?,
        progress_file: row.get(14)?,
        dependencies: row.get(15)?,
        task_order: row.get(16)?,
        retry_count: row.get(17)?,
        max_retries: row.get(18)?,
        error_message: row.get(19)?,
        implementation_steps: row.get(20)?,
        test_steps: row.get(21)?,
        acceptance_criteria: row.get(22)?,
        memory_library_ids: row.get(23)?,
        block_reason: row.get(24)?,
        input_request: row.get(25)?,
        input_response: row.get(26)?,
        created_at: row.get(27)?,
        updated_at: row.get(28)?,
    })
}

fn map_rust_task_with_conn(conn: &Connection, row: &Row<'_>) -> rusqlite::Result<RustTask> {
    let task_id: String = row.get(0)?;
    let memory_library_ids = list_task_memory_library_ids(conn, &task_id).map_err(|error| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            error,
        )))
    })?;

    let mut task = map_rust_task_row(row)?;
    task.memory_library_ids =
        Some(serde_json::to_string(&memory_library_ids).unwrap_or_else(|_| "[]".to_string()));
    Ok(task)
}

fn collect_task_subtree_ids(conn: &Connection, task_id: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            WITH RECURSIVE descendants(id) AS (
                SELECT id FROM tasks WHERE id = ?1
                UNION ALL
                SELECT tasks.id
                FROM tasks
                INNER JOIN descendants ON tasks.parent_id = descendants.id
            )
            SELECT id FROM descendants
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([task_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn cleanup_deleted_task_references(
    tx: &rusqlite::Transaction<'_>,
    plan_id: &str,
    deleted_task_ids: &[String],
) -> Result<(), String> {
    if deleted_task_ids.is_empty() {
        return Ok(());
    }

    let deleted_task_id_set = deleted_task_ids
        .iter()
        .cloned()
        .collect::<HashSet<String>>();
    let now = now_rfc3339();

    let placeholders = (0..deleted_task_ids.len())
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let delete_usage_sql = format!(
        "DELETE FROM agent_cli_usage_records WHERE task_id IN ({})",
        placeholders
    );
    tx.execute(
        &delete_usage_sql,
        rusqlite::params_from_iter(deleted_task_ids.iter()),
    )
    .map_err(|e| e.to_string())?;

    let clear_current_task_sql = format!(
        "UPDATE plans SET current_task_id = NULL, updated_at = ?1 WHERE current_task_id IN ({})",
        placeholders
    );
    let mut clear_current_task_params: Vec<&dyn ToSql> =
        Vec::with_capacity(deleted_task_ids.len() + 1);
    clear_current_task_params.push(&now);
    for task_id in deleted_task_ids {
        clear_current_task_params.push(task_id);
    }
    tx.execute(
        &clear_current_task_sql,
        rusqlite::params_from_iter(clear_current_task_params),
    )
    .map_err(|e| e.to_string())?;

    let dependency_rows = {
        let mut stmt = tx
            .prepare(
                r#"
                SELECT id, dependencies
                FROM tasks
                WHERE plan_id = ?1
                  AND dependencies IS NOT NULL
                  AND dependencies != ''
                "#,
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([plan_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    for (task_id, dependencies_json) in dependency_rows {
        if deleted_task_id_set.contains(&task_id) {
            continue;
        }

        let Some(dependencies_json) = dependencies_json else {
            continue;
        };

        let dependencies: Vec<String> =
            serde_json::from_str(&dependencies_json).unwrap_or_default();
        if dependencies.is_empty() {
            continue;
        }

        let filtered_dependencies = dependencies
            .into_iter()
            .filter(|dependency_id| !deleted_task_id_set.contains(dependency_id))
            .collect::<Vec<_>>();

        let filtered_dependencies_json =
            serde_json::to_string(&filtered_dependencies).map_err(|e| e.to_string())?;

        if filtered_dependencies_json == dependencies_json {
            continue;
        }

        tx.execute(
            "UPDATE tasks SET dependencies = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![filtered_dependencies_json, &now, &task_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn collect_tasks<P>(conn: &Connection, sql: &str, params: P) -> Result<Vec<Task>, String>
where
    P: Params,
{
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params, |row| map_rust_task_with_conn(conn, row))
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
        .map(|tasks| tasks.into_iter().map(transform_task).collect())
}

fn fetch_task<P>(conn: &Connection, sql: &str, params: P) -> Result<Task, String>
where
    P: Params,
{
    conn.query_row(sql, params, |row| map_rust_task_with_conn(conn, row))
        .map(transform_task)
        .map_err(|e| e.to_string())
}

fn fetch_optional_task<P>(conn: &Connection, sql: &str, params: P) -> Result<Option<Task>, String>
where
    P: Params,
{
    match conn.query_row(sql, params, |row| map_rust_task_with_conn(conn, row)) {
        Ok(task) => Ok(Some(transform_task(task))),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn serialize_json_option<T: Serialize>(value: Option<&T>, fallback: &str) -> Option<String> {
    value.map(|value| serde_json::to_string(value).unwrap_or_else(|_| fallback.to_string()))
}

fn resolve_task_project_id(conn: &Connection, plan_id: &str) -> Result<String, String> {
    conn.query_row(
        "SELECT project_id FROM plans WHERE id = ?1",
        [plan_id],
        |row| row.get::<_, String>(0),
    )
    .map_err(|_| format!("Plan not found for task: {}", plan_id))
}

fn map_task_runtime_binding_row(row: &Row<'_>) -> rusqlite::Result<TaskRuntimeBinding> {
    Ok(TaskRuntimeBinding {
        task_id: row.get(0)?,
        runtime_key: row.get(1)?,
        external_session_id: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn get_task_runtime_binding_internal(
    conn: &Connection,
    task_id: &str,
    runtime_key: &str,
) -> Result<Option<TaskRuntimeBinding>, String> {
    match conn.query_row(
        r#"
        SELECT task_id, runtime_key, external_session_id, created_at, updated_at
        FROM task_runtime_bindings
        WHERE task_id = ?1 AND runtime_key = ?2
        "#,
        rusqlite::params![task_id, runtime_key],
        map_task_runtime_binding_row,
    ) {
        Ok(binding) => Ok(Some(binding)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
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
    let memory_library_ids = rust_task
        .memory_library_ids
        .and_then(|s| serde_json::from_str(&s).ok());
    let input_request = rust_task
        .input_request
        .and_then(|s| serde_json::from_str(&s).ok());
    let input_response = rust_task
        .input_response
        .and_then(|s| serde_json::from_str(&s).ok());

    Task {
        id: rust_task.id,
        project_id: rust_task.project_id,
        plan_id: rust_task.plan_id,
        parent_id: rust_task.parent_id,
        title: rust_task.title,
        description: rust_task.description,
        status: rust_task.status,
        priority: rust_task.priority,
        assignee: rust_task.assignee,
        expert_id: rust_task.expert_id,
        agent_id: rust_task.agent_id,
        model_id: rust_task.model_id,
        session_id: rust_task.session_id,
        cli_session_provider: rust_task.cli_session_provider,
        progress_file: rust_task.progress_file,
        dependencies,
        order: rust_task.task_order,
        retry_count: rust_task.retry_count,
        max_retries: rust_task.max_retries,
        error_message: rust_task.error_message,
        implementation_steps,
        test_steps,
        acceptance_criteria,
        memory_library_ids,
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

/// 获取项目下未挂载到有效计划的任务。
/// 用途：支持计划页在“未选择计划”时展示项目级直接待办任务或历史脏数据任务。
#[tauri::command]
pub fn list_project_unplanned_tasks(project_id: String) -> Result<Vec<Task>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    collect_tasks(&conn, TASK_SELECT_BY_PROJECT_UNPLANNED_SQL, [&project_id])
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
    let project_id = resolve_task_project_id(&conn, &input.plan_id)?;

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
    let memory_library_ids_json = serialize_json_option(input.memory_library_ids.as_ref(), "[]");

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
        "INSERT INTO tasks (id, project_id, plan_id, parent_id, title, description, status, priority,
         assignee, expert_id, agent_id, model_id, session_id, cli_session_provider, progress_file, dependencies, task_order,
         retry_count, max_retries, error_message,
         implementation_steps, test_steps, acceptance_criteria, memory_library_ids,
         created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
        rusqlite::params![
            &id,
            &project_id,
            &input.plan_id,
            &input.parent_id,
            &input.title,
            &input.description,
            &status,
            &priority,
            &input.assignee,
            &input.expert_id,
            &input.agent_id,
            &input.model_id,
            &None::<String>, // session_id
            &None::<String>, // cli_session_provider
            &None::<String>, // progress_file
            &dependencies_json,
            &task_order,
            0, // retry_count
            &max_retries,
            &None::<String>, // error_message
            &implementation_steps_json,
            &test_steps_json,
            &acceptance_criteria_json,
            &memory_library_ids_json,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    replace_task_memory_libraries(
        &conn,
        &id,
        input.memory_library_ids.as_deref().unwrap_or(&[]),
        &now,
    )?;

    // 更新计划的 updated_at 时间
    conn.execute(
        "UPDATE plans SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &input.plan_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(Task {
        id,
        project_id: Some(project_id),
        plan_id: input.plan_id,
        parent_id: input.parent_id,
        title: input.title,
        description: input.description,
        status,
        priority,
        assignee: input.assignee,
        expert_id: input.expert_id,
        agent_id: input.agent_id,
        model_id: input.model_id,
        session_id: None,
        cli_session_provider: None,
        progress_file: None,
        dependencies: input.dependencies,
        order: task_order,
        retry_count: 0,
        max_retries,
        error_message: None,
        implementation_steps: input.implementation_steps,
        test_steps: input.test_steps,
        acceptance_criteria: input.acceptance_criteria,
        memory_library_ids: Some(normalize_memory_library_ids(
            input.memory_library_ids.as_deref().unwrap_or(&[]),
        )),
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
    push_update(&mut updates, "expert_id", &input.expert_id);
    push_update(&mut updates, "agent_id", &input.agent_id);
    push_update(&mut updates, "model_id", &input.model_id);
    push_update(&mut updates, "session_id", &input.session_id);
    push_update(
        &mut updates,
        "cli_session_provider",
        &input.cli_session_provider,
    );
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
    push_update(
        &mut updates,
        "memory_library_ids",
        &input.memory_library_ids,
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
    bind_update_field(&mut stmt, &mut param_count, &input.expert_id).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.agent_id).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.model_id).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.session_id).map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.cli_session_provider)
        .map_err(|e| e.to_string())?;
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
    bind_update_json(&mut stmt, &mut param_count, &input.memory_library_ids, "[]")
        .map_err(|e| e.to_string())?;
    bind_update_field(&mut stmt, &mut param_count, &input.block_reason)
        .map_err(|e| e.to_string())?;
    bind_update_json(&mut stmt, &mut param_count, &input.input_request, "{}")
        .map_err(|e| e.to_string())?;
    bind_update_json(&mut stmt, &mut param_count, &input.input_response, "{}")
        .map_err(|e| e.to_string())?;
    bind_value(&mut stmt, &mut param_count, &id).map_err(|e| e.to_string())?;
    stmt.raw_execute().map_err(|e| e.to_string())?;

    if !matches!(input.memory_library_ids, UpdateField::Missing) {
        let library_ids = match input.memory_library_ids {
            UpdateField::Value(ref value) => value.clone(),
            UpdateField::Null => Vec::new(),
            UpdateField::Missing => Vec::new(),
        };
        replace_task_memory_libraries(&conn, &id, &library_ids, &now)?;
    }

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
    let mut conn = open_db_connection().map_err(|e| e.to_string())?;
    let deleted_task_ids = collect_task_subtree_ids(&conn, &id)?;
    if deleted_task_ids.is_empty() {
        return Ok(());
    }

    let plan_id: String = conn
        .query_row("SELECT plan_id FROM tasks WHERE id = ?1", [&id], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    cleanup_deleted_task_references(&tx, &plan_id, &deleted_task_ids)?;

    tx.execute("DELETE FROM tasks WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

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

/// 获取指定任务在某个运行时下的恢复绑定。
#[tauri::command]
pub fn get_task_runtime_binding(
    task_id: String,
    runtime_key: String,
) -> Result<Option<TaskRuntimeBinding>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    get_task_runtime_binding_internal(&conn, &task_id, &runtime_key)
}

/// 创建或更新任务的运行时恢复绑定。
#[tauri::command]
pub fn upsert_task_runtime_binding(
    task_id: String,
    runtime_key: String,
    external_session_id: String,
) -> Result<TaskRuntimeBinding, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let now = now_rfc3339();

    conn.execute(
        r#"
        INSERT INTO task_runtime_bindings (
            task_id,
            runtime_key,
            external_session_id,
            created_at,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(task_id, runtime_key) DO UPDATE SET
            external_session_id = excluded.external_session_id,
            updated_at = excluded.updated_at
        "#,
        rusqlite::params![&task_id, &runtime_key, &external_session_id, &now, &now],
    )
    .map_err(|e| e.to_string())?;

    get_task_runtime_binding_internal(&conn, &task_id, &runtime_key)?
        .ok_or_else(|| "任务运行时绑定写入后读取失败".to_string())
}

/// 删除任务在某个运行时下的恢复绑定。
#[tauri::command]
pub fn delete_task_runtime_binding(task_id: String, runtime_key: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM task_runtime_bindings WHERE task_id = ?1 AND runtime_key = ?2",
        rusqlite::params![&task_id, &runtime_key],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 批量创建任务（从拆分结果）
#[tauri::command]
pub fn batch_create_tasks(
    plan_id: String,
    tasks: Vec<CreateTaskInput>,
) -> Result<Vec<Task>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let project_id = resolve_task_project_id(&conn, &plan_id)?;

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
        let memory_library_ids_json =
            serialize_json_option(task_input.memory_library_ids.as_ref(), "[]");

        max_order += 1;
        let task_order = task_input.order.unwrap_or(max_order);

        tx.execute(
            "INSERT INTO tasks (id, project_id, plan_id, parent_id, title, description, status, priority,
             assignee, expert_id, agent_id, model_id, session_id, cli_session_provider, progress_file, dependencies, task_order,
             retry_count, max_retries, error_message,
             implementation_steps, test_steps, acceptance_criteria, memory_library_ids,
             created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
            rusqlite::params![
                &id,
                &project_id,
                &plan_id,
                &task_input.parent_id,
                &task_input.title,
                &task_input.description,
                &status,
                &priority,
                &task_input.assignee,
                &task_input.expert_id,
                &task_input.agent_id,
                &task_input.model_id,
                &None::<String>, // session_id
                &None::<String>, // cli_session_provider
                &None::<String>, // progress_file
                &dependencies_json,
                &task_order,
                0, // retry_count
                &max_retries,
                &None::<String>, // error_message
                &implementation_steps_json,
                &test_steps_json,
                &acceptance_criteria_json,
                &memory_library_ids_json,
                &now,
                &now
            ],
        )
        .map_err(|e| e.to_string())?;

        replace_task_memory_libraries(
            &tx,
            &id,
            task_input.memory_library_ids.as_deref().unwrap_or(&[]),
            &now,
        )?;

        created_tasks.push(Task {
            id,
            project_id: Some(project_id.clone()),
            plan_id: plan_id.clone(),
            parent_id: task_input.parent_id,
            title: task_input.title,
            description: task_input.description,
            status,
            priority,
            assignee: task_input.assignee,
            expert_id: task_input.expert_id,
            agent_id: task_input.agent_id,
            model_id: task_input.model_id,
            session_id: None,
            cli_session_provider: None,
            progress_file: None,
            dependencies: task_input.dependencies,
            order: task_order,
            retry_count: 0,
            max_retries,
            error_message: None,
            implementation_steps: task_input.implementation_steps,
            test_steps: task_input.test_steps,
            acceptance_criteria: task_input.acceptance_criteria,
            memory_library_ids: Some(normalize_memory_library_ids(
                task_input.memory_library_ids.as_deref().unwrap_or(&[]),
            )),
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
