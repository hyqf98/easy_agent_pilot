use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::support::{now_rfc3339, open_db_connection};

/// SOLO 运行主记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoloRun {
    pub id: String,
    pub project_id: String,
    pub execution_path: String,
    pub name: String,
    pub requirement: String,
    pub goal: String,
    pub memory_library_ids_json: Option<String>,
    pub participant_expert_ids_json: Option<String>,
    pub coordinator_expert_id: Option<String>,
    pub coordinator_agent_id: Option<String>,
    pub coordinator_model_id: Option<String>,
    pub max_dispatch_depth: i32,
    pub current_depth: i32,
    pub current_step_id: Option<String>,
    pub status: String,
    pub execution_status: String,
    pub last_error: Option<String>,
    pub input_request_json: Option<String>,
    pub input_response_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub stopped_at: Option<String>,
}

/// SOLO 步骤记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoloStep {
    pub id: String,
    pub run_id: String,
    pub step_ref: String,
    pub parent_step_ref: Option<String>,
    pub depth: i32,
    pub title: String,
    pub description: Option<String>,
    pub execution_prompt: Option<String>,
    pub selected_expert_id: Option<String>,
    pub status: String,
    pub summary: Option<String>,
    pub result_summary: Option<String>,
    pub result_files_json: Option<String>,
    pub fail_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// SOLO 运行日志。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoloLog {
    pub id: String,
    pub run_id: String,
    pub step_id: Option<String>,
    pub scope: String,
    #[serde(rename = "type")]
    pub log_type: String,
    pub content: String,
    pub metadata: Option<String>,
    pub created_at: String,
}

/// SOLO 运行时绑定。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoloRuntimeBinding {
    pub run_id: String,
    pub runtime_key: String,
    pub external_session_id: String,
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

/// 创建 SOLO 运行输入。
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSoloRunInput {
    pub project_id: String,
    pub execution_path: String,
    pub name: String,
    pub requirement: String,
    pub goal: String,
    pub memory_library_ids_json: Option<String>,
    pub participant_expert_ids_json: Option<String>,
    pub coordinator_expert_id: Option<String>,
    pub coordinator_agent_id: Option<String>,
    pub coordinator_model_id: Option<String>,
    pub max_dispatch_depth: i32,
}

/// 更新 SOLO 运行输入。
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateSoloRunInput {
    #[serde(default)]
    pub execution_path: UpdateField<String>,
    #[serde(default)]
    pub name: UpdateField<String>,
    #[serde(default)]
    pub requirement: UpdateField<String>,
    #[serde(default)]
    pub goal: UpdateField<String>,
    #[serde(default)]
    pub memory_library_ids_json: UpdateField<String>,
    #[serde(default)]
    pub participant_expert_ids_json: UpdateField<String>,
    #[serde(default)]
    pub coordinator_expert_id: UpdateField<String>,
    #[serde(default)]
    pub coordinator_agent_id: UpdateField<String>,
    #[serde(default)]
    pub coordinator_model_id: UpdateField<String>,
    #[serde(default)]
    pub max_dispatch_depth: UpdateField<i32>,
    #[serde(default)]
    pub current_depth: UpdateField<i32>,
    #[serde(default)]
    pub current_step_id: UpdateField<String>,
    #[serde(default)]
    pub status: UpdateField<String>,
    #[serde(default)]
    pub execution_status: UpdateField<String>,
    #[serde(default)]
    pub last_error: UpdateField<String>,
    #[serde(default)]
    pub input_request_json: UpdateField<String>,
    #[serde(default)]
    pub input_response_json: UpdateField<String>,
    #[serde(default)]
    pub started_at: UpdateField<String>,
    #[serde(default)]
    pub completed_at: UpdateField<String>,
    #[serde(default)]
    pub stopped_at: UpdateField<String>,
}

/// 创建 SOLO 步骤输入。
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSoloStepInput {
    pub run_id: String,
    pub step_ref: String,
    pub parent_step_ref: Option<String>,
    pub depth: i32,
    pub title: String,
    pub description: Option<String>,
    pub execution_prompt: Option<String>,
    pub selected_expert_id: Option<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub started_at: Option<String>,
}

/// 更新 SOLO 步骤输入。
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateSoloStepInput {
    #[serde(default)]
    pub parent_step_ref: UpdateField<String>,
    #[serde(default)]
    pub depth: UpdateField<i32>,
    #[serde(default)]
    pub title: UpdateField<String>,
    #[serde(default)]
    pub description: UpdateField<String>,
    #[serde(default)]
    pub execution_prompt: UpdateField<String>,
    #[serde(default)]
    pub selected_expert_id: UpdateField<String>,
    #[serde(default)]
    pub status: UpdateField<String>,
    #[serde(default)]
    pub summary: UpdateField<String>,
    #[serde(default)]
    pub result_summary: UpdateField<String>,
    #[serde(default)]
    pub result_files_json: UpdateField<String>,
    #[serde(default)]
    pub fail_reason: UpdateField<String>,
    #[serde(default)]
    pub started_at: UpdateField<String>,
    #[serde(default)]
    pub completed_at: UpdateField<String>,
}

/// 创建 SOLO 日志输入。
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSoloLogInput {
    pub run_id: String,
    pub step_id: Option<String>,
    pub scope: String,
    pub log_type: String,
    pub content: String,
    pub metadata: Option<String>,
}

/// 写入 SOLO 运行时绑定输入。
#[derive(Debug, Clone, Deserialize)]
pub struct UpsertSoloRuntimeBindingInput {
    pub run_id: String,
    pub runtime_key: String,
    pub external_session_id: String,
}

fn map_solo_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<SoloRun> {
    Ok(SoloRun {
        id: row.get(0)?,
        project_id: row.get(1)?,
        execution_path: row.get(2)?,
        name: row.get(3)?,
        requirement: row.get(4)?,
        goal: row.get(5)?,
        memory_library_ids_json: row.get(6)?,
        participant_expert_ids_json: row.get(7)?,
        coordinator_expert_id: row.get(8)?,
        coordinator_agent_id: row.get(9)?,
        coordinator_model_id: row.get(10)?,
        max_dispatch_depth: row.get(11)?,
        current_depth: row.get(12)?,
        current_step_id: row.get(13)?,
        status: row.get(14)?,
        execution_status: row.get(15)?,
        last_error: row.get(16)?,
        input_request_json: row.get(17)?,
        input_response_json: row.get(18)?,
        created_at: row.get(19)?,
        updated_at: row.get(20)?,
        started_at: row.get(21)?,
        completed_at: row.get(22)?,
        stopped_at: row.get(23)?,
    })
}

fn parse_memory_library_ids_json(raw: Option<&String>) -> Vec<String> {
    raw.and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|library_id| library_id.trim().to_string())
        .filter(|library_id| !library_id.is_empty())
        .fold(Vec::new(), |mut acc, library_id| {
            if !acc.iter().any(|existing| existing == &library_id) {
                acc.push(library_id);
            }
            acc
        })
}

fn normalize_memory_library_ids_json(raw: Option<&String>) -> String {
    serde_json::to_string(&parse_memory_library_ids_json(raw)).unwrap_or_else(|_| "[]".to_string())
}

fn replace_solo_run_memory_libraries(
    conn: &rusqlite::Connection,
    run_id: &str,
    library_ids: &[String],
    now: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM solo_run_memory_libraries WHERE run_id = ?1",
        [run_id],
    )
    .map_err(|error| error.to_string())?;

    for library_id in library_ids {
        conn.execute(
            r#"
            INSERT INTO solo_run_memory_libraries (run_id, library_id, created_at)
            VALUES (?1, ?2, ?3)
            "#,
            rusqlite::params![run_id, library_id, now],
        )
        .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn map_solo_step(row: &rusqlite::Row<'_>) -> rusqlite::Result<SoloStep> {
    Ok(SoloStep {
        id: row.get(0)?,
        run_id: row.get(1)?,
        step_ref: row.get(2)?,
        parent_step_ref: row.get(3)?,
        depth: row.get(4)?,
        title: row.get(5)?,
        description: row.get(6)?,
        execution_prompt: row.get(7)?,
        selected_expert_id: row.get(8)?,
        status: row.get(9)?,
        summary: row.get(10)?,
        result_summary: row.get(11)?,
        result_files_json: row.get(12)?,
        fail_reason: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        started_at: row.get(16)?,
        completed_at: row.get(17)?,
    })
}

fn map_solo_log(row: &rusqlite::Row<'_>) -> rusqlite::Result<SoloLog> {
    Ok(SoloLog {
        id: row.get(0)?,
        run_id: row.get(1)?,
        step_id: row.get(2)?,
        scope: row.get(3)?,
        log_type: row.get(4)?,
        content: row.get(5)?,
        metadata: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn append_optional_text_update(
    updates: &mut Vec<String>,
    params: &mut Vec<Box<dyn rusqlite::ToSql>>,
    field: &UpdateField<String>,
    column: &str,
) {
    match field {
        UpdateField::Value(value) => {
            updates.push(format!("{} = ?", column));
            params.push(Box::new(value.clone()));
        }
        UpdateField::Null => {
            updates.push(format!("{} = NULL", column));
        }
        UpdateField::Missing => {}
    }
}

fn append_optional_i32_update(
    updates: &mut Vec<String>,
    params: &mut Vec<Box<dyn rusqlite::ToSql>>,
    field: &UpdateField<i32>,
    column: &str,
) {
    match field {
        UpdateField::Value(value) => {
            updates.push(format!("{} = ?", column));
            params.push(Box::new(*value));
        }
        UpdateField::Null => {
            updates.push(format!("{} = NULL", column));
        }
        UpdateField::Missing => {}
    }
}

/// 列出项目下的 SOLO 运行。
#[tauri::command]
pub fn list_solo_runs(project_id: String) -> Result<Vec<SoloRun>, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, project_id, execution_path, name, requirement, goal,
                   memory_library_ids_json, participant_expert_ids_json, coordinator_expert_id, coordinator_agent_id, coordinator_model_id,
                   max_dispatch_depth, current_depth, current_step_id,
                   status, execution_status, last_error, input_request_json, input_response_json,
                   created_at, updated_at, started_at, completed_at, stopped_at
            FROM solo_runs
            WHERE project_id = ?1
            ORDER BY updated_at DESC
            "#,
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map([&project_id], map_solo_run)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    Ok(rows)
}

/// 获取单个 SOLO 运行。
#[tauri::command]
pub fn get_solo_run(id: String) -> Result<SoloRun, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    conn.query_row(
        r#"
        SELECT id, project_id, execution_path, name, requirement, goal,
               memory_library_ids_json, participant_expert_ids_json, coordinator_expert_id, coordinator_agent_id, coordinator_model_id,
               max_dispatch_depth, current_depth, current_step_id,
               status, execution_status, last_error, input_request_json, input_response_json,
               created_at, updated_at, started_at, completed_at, stopped_at
        FROM solo_runs
        WHERE id = ?1
        "#,
        [&id],
        map_solo_run,
    )
    .map_err(|error| error.to_string())
}

/// 创建 SOLO 运行。
#[tauri::command]
pub fn create_solo_run(input: CreateSoloRunInput) -> Result<SoloRun, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let normalized_memory_library_ids_json =
        normalize_memory_library_ids_json(input.memory_library_ids_json.as_ref());

    conn.execute(
        r#"
        INSERT INTO solo_runs (
            id, project_id, execution_path, name, requirement, goal,
            memory_library_ids_json, participant_expert_ids_json, coordinator_expert_id, coordinator_agent_id, coordinator_model_id,
            max_dispatch_depth, current_depth, current_step_id,
            status, execution_status, last_error, input_request_json, input_response_json,
            created_at, updated_at, started_at, completed_at, stopped_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, NULL, 'draft', 'idle', NULL, NULL, NULL, ?13, ?13, NULL, NULL, NULL)
        "#,
        rusqlite::params![
            &id,
            &input.project_id,
            &input.execution_path,
            &input.name,
            &input.requirement,
            &input.goal,
            &normalized_memory_library_ids_json,
            &input.participant_expert_ids_json,
            &input.coordinator_expert_id,
            &input.coordinator_agent_id,
            &input.coordinator_model_id,
            &input.max_dispatch_depth,
            &now
        ],
    )
    .map_err(|error| error.to_string())?;

    replace_solo_run_memory_libraries(
        &conn,
        &id,
        &parse_memory_library_ids_json(Some(&normalized_memory_library_ids_json)),
        &now,
    )?;

    get_solo_run(id)
}

/// 更新 SOLO 运行状态与配置。
#[tauri::command]
pub fn update_solo_run(id: String, input: UpdateSoloRunInput) -> Result<SoloRun, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut updates = vec!["updated_at = ?".to_string()];
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now_rfc3339())];

    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.execution_path,
        "execution_path",
    );
    append_optional_text_update(&mut updates, &mut params, &input.name, "name");
    append_optional_text_update(&mut updates, &mut params, &input.requirement, "requirement");
    append_optional_text_update(&mut updates, &mut params, &input.goal, "goal");
    match &input.memory_library_ids_json {
        UpdateField::Value(value) => {
            updates.push("memory_library_ids_json = ?".to_string());
            params.push(Box::new(normalize_memory_library_ids_json(Some(value))));
        }
        UpdateField::Null => {
            updates.push("memory_library_ids_json = NULL".to_string());
        }
        UpdateField::Missing => {}
    }
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.participant_expert_ids_json,
        "participant_expert_ids_json",
    );
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.coordinator_expert_id,
        "coordinator_expert_id",
    );
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.coordinator_agent_id,
        "coordinator_agent_id",
    );
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.coordinator_model_id,
        "coordinator_model_id",
    );
    append_optional_i32_update(
        &mut updates,
        &mut params,
        &input.max_dispatch_depth,
        "max_dispatch_depth",
    );
    append_optional_i32_update(
        &mut updates,
        &mut params,
        &input.current_depth,
        "current_depth",
    );
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.current_step_id,
        "current_step_id",
    );
    append_optional_text_update(&mut updates, &mut params, &input.status, "status");
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.execution_status,
        "execution_status",
    );
    append_optional_text_update(&mut updates, &mut params, &input.last_error, "last_error");
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.input_request_json,
        "input_request_json",
    );
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.input_response_json,
        "input_response_json",
    );
    append_optional_text_update(&mut updates, &mut params, &input.started_at, "started_at");
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.completed_at,
        "completed_at",
    );
    append_optional_text_update(&mut updates, &mut params, &input.stopped_at, "stopped_at");

    params.push(Box::new(id.clone()));
    let sql = format!("UPDATE solo_runs SET {} WHERE id = ?", updates.join(", "));
    let params_ref: Vec<&dyn rusqlite::ToSql> = params
        .iter()
        .map(|value| value.as_ref() as &dyn rusqlite::ToSql)
        .collect();

    conn.execute(&sql, params_ref.as_slice())
        .map_err(|error| error.to_string())?;

    if !matches!(input.memory_library_ids_json, UpdateField::Missing) {
        let library_ids = match &input.memory_library_ids_json {
            UpdateField::Value(raw) => parse_memory_library_ids_json(Some(raw)),
            UpdateField::Null => Vec::new(),
            UpdateField::Missing => Vec::new(),
        };
        replace_solo_run_memory_libraries(&conn, &id, &library_ids, &now_rfc3339())?;
    }

    get_solo_run(id)
}

/// 删除 SOLO 运行及其关联步骤、日志与运行时绑定。
#[tauri::command]
pub fn delete_solo_run(id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM solo_runs WHERE id = ?1", [&id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// 列出 SOLO 运行下的全部步骤。
#[tauri::command]
pub fn list_solo_steps(run_id: String) -> Result<Vec<SoloStep>, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, run_id, step_ref, parent_step_ref, depth, title, description,
                   execution_prompt, selected_expert_id, status, summary, result_summary,
                   result_files_json, fail_reason, created_at, updated_at, started_at, completed_at
            FROM solo_steps
            WHERE run_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map([&run_id], map_solo_step)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    Ok(rows)
}

/// 创建 SOLO 步骤。
#[tauri::command]
pub fn create_solo_step(input: CreateSoloStepInput) -> Result<SoloStep, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let status = input.status.unwrap_or_else(|| "pending".to_string());

    conn.execute(
        r#"
        INSERT INTO solo_steps (
            id, run_id, step_ref, parent_step_ref, depth, title, description,
            execution_prompt, selected_expert_id, status, summary, result_summary,
            result_files_json, fail_reason, created_at, updated_at, started_at, completed_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, NULL, NULL, ?12, ?12, ?13, NULL)
        "#,
        rusqlite::params![
            &id,
            &input.run_id,
            &input.step_ref,
            &input.parent_step_ref,
            &input.depth,
            &input.title,
            &input.description,
            &input.execution_prompt,
            &input.selected_expert_id,
            &status,
            &input.summary,
            &now,
            &input.started_at
        ],
    )
    .map_err(|error| error.to_string())?;

    conn.execute(
        "UPDATE solo_runs SET updated_at = ?1, current_step_id = ?2, current_depth = MAX(current_depth, ?3) WHERE id = ?4",
        rusqlite::params![&now, &id, &input.depth, &input.run_id],
    )
    .map_err(|error| error.to_string())?;

    conn.query_row(
        r#"
        SELECT id, run_id, step_ref, parent_step_ref, depth, title, description,
               execution_prompt, selected_expert_id, status, summary, result_summary,
               result_files_json, fail_reason, created_at, updated_at, started_at, completed_at
        FROM solo_steps
        WHERE id = ?1
        "#,
        [&id],
        map_solo_step,
    )
    .map_err(|error| error.to_string())
}

/// 更新 SOLO 步骤。
#[tauri::command]
pub fn update_solo_step(id: String, input: UpdateSoloStepInput) -> Result<SoloStep, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut updates = vec!["updated_at = ?".to_string()];
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now_rfc3339())];

    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.parent_step_ref,
        "parent_step_ref",
    );
    append_optional_i32_update(&mut updates, &mut params, &input.depth, "depth");
    append_optional_text_update(&mut updates, &mut params, &input.title, "title");
    append_optional_text_update(&mut updates, &mut params, &input.description, "description");
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.execution_prompt,
        "execution_prompt",
    );
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.selected_expert_id,
        "selected_expert_id",
    );
    append_optional_text_update(&mut updates, &mut params, &input.status, "status");
    append_optional_text_update(&mut updates, &mut params, &input.summary, "summary");
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.result_summary,
        "result_summary",
    );
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.result_files_json,
        "result_files_json",
    );
    append_optional_text_update(&mut updates, &mut params, &input.fail_reason, "fail_reason");
    append_optional_text_update(&mut updates, &mut params, &input.started_at, "started_at");
    append_optional_text_update(
        &mut updates,
        &mut params,
        &input.completed_at,
        "completed_at",
    );

    params.push(Box::new(id.clone()));
    let sql = format!("UPDATE solo_steps SET {} WHERE id = ?", updates.join(", "));
    let params_ref: Vec<&dyn rusqlite::ToSql> = params
        .iter()
        .map(|value| value.as_ref() as &dyn rusqlite::ToSql)
        .collect();
    conn.execute(&sql, params_ref.as_slice())
        .map_err(|error| error.to_string())?;

    let step = conn
        .query_row(
            r#"
            SELECT id, run_id, step_ref, parent_step_ref, depth, title, description,
                   execution_prompt, selected_expert_id, status, summary, result_summary,
                   result_files_json, fail_reason, created_at, updated_at, started_at, completed_at
            FROM solo_steps
            WHERE id = ?1
            "#,
            [&id],
            map_solo_step,
        )
        .map_err(|error| error.to_string())?;

    conn.execute(
        "UPDATE solo_runs SET updated_at = ?1, current_step_id = ?2, current_depth = MAX(current_depth, ?3) WHERE id = ?4",
        rusqlite::params![now_rfc3339(), &step.id, &step.depth, &step.run_id],
    )
    .map_err(|error| error.to_string())?;

    Ok(step)
}

/// 记录 SOLO 日志。
#[tauri::command]
pub fn create_solo_log(input: CreateSoloLogInput) -> Result<SoloLog, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();

    conn.execute(
        r#"
        INSERT INTO solo_logs (
            id, run_id, step_id, scope, log_type, content, metadata, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        rusqlite::params![
            &id,
            &input.run_id,
            &input.step_id,
            &input.scope,
            &input.log_type,
            &input.content,
            &input.metadata,
            &now
        ],
    )
    .map_err(|error| error.to_string())?;

    conn.execute(
        "UPDATE solo_runs SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &input.run_id],
    )
    .map_err(|error| error.to_string())?;

    Ok(SoloLog {
        id,
        run_id: input.run_id,
        step_id: input.step_id,
        scope: input.scope,
        log_type: input.log_type,
        content: input.content,
        metadata: input.metadata,
        created_at: now,
    })
}

/// 更新 SOLO 运行日志内容。
#[tauri::command]
pub fn update_solo_log(
    id: String,
    content: String,
    metadata: Option<String>,
) -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;

    conn.execute(
        "UPDATE solo_logs SET content = ?1, metadata = ?2 WHERE id = ?3",
        rusqlite::params![&content, &metadata, &id],
    )
    .map_err(|error| error.to_string())?;

    Ok(())
}

/// 列出 SOLO 运行日志。
#[tauri::command]
pub fn list_solo_logs(run_id: String, step_id: Option<String>) -> Result<Vec<SoloLog>, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let sql = if step_id.is_some() {
        r#"
        SELECT id, run_id, step_id, scope, log_type, content, metadata, created_at
        FROM solo_logs
        WHERE run_id = ?1 AND step_id = ?2
        ORDER BY created_at ASC
        "#
    } else {
        r#"
        SELECT id, run_id, step_id, scope, log_type, content, metadata, created_at
        FROM solo_logs
        WHERE run_id = ?1
        ORDER BY created_at ASC
        "#
    };

    let mut stmt = conn.prepare(sql).map_err(|error| error.to_string())?;
    let mapper = |row: &rusqlite::Row<'_>| map_solo_log(row);

    let logs = if let Some(step_id) = step_id {
        stmt.query_map(rusqlite::params![&run_id, &step_id], mapper)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    } else {
        stmt.query_map([&run_id], mapper)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };

    Ok(logs)
}

/// 清理 SOLO 运行下的全部步骤与日志，并重置运行状态。
#[tauri::command]
pub fn clear_solo_run_progress(run_id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    tx.execute("DELETE FROM solo_logs WHERE run_id = ?1", [&run_id])
        .map_err(|error| error.to_string())?;
    tx.execute("DELETE FROM solo_steps WHERE run_id = ?1", [&run_id])
        .map_err(|error| error.to_string())?;
    tx.execute(
        "DELETE FROM solo_runtime_bindings WHERE run_id = ?1",
        [&run_id],
    )
    .map_err(|error| error.to_string())?;
    tx.execute(
        r#"
        UPDATE solo_runs
        SET current_depth = 0,
            current_step_id = NULL,
            status = 'draft',
            execution_status = 'idle',
            last_error = NULL,
            input_request_json = NULL,
            input_response_json = NULL,
            started_at = NULL,
            completed_at = NULL,
            stopped_at = NULL,
            updated_at = ?1
        WHERE id = ?2
        "#,
        rusqlite::params![now_rfc3339(), &run_id],
    )
    .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(())
}

/// 获取 SOLO 运行时绑定。
#[tauri::command]
pub fn get_solo_runtime_binding(
    run_id: String,
    runtime_key: String,
) -> Result<Option<SoloRuntimeBinding>, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    match conn.query_row(
        r#"
        SELECT run_id, runtime_key, external_session_id, created_at, updated_at
        FROM solo_runtime_bindings
        WHERE run_id = ?1 AND runtime_key = ?2
        "#,
        rusqlite::params![&run_id, &runtime_key],
        |row| {
            Ok(SoloRuntimeBinding {
                run_id: row.get(0)?,
                runtime_key: row.get(1)?,
                external_session_id: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    ) {
        Ok(binding) => Ok(Some(binding)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

/// 写入或更新 SOLO 运行时绑定。
#[tauri::command]
pub fn upsert_solo_runtime_binding(input: UpsertSoloRuntimeBindingInput) -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let now = now_rfc3339();
    conn.execute(
        r#"
        INSERT INTO solo_runtime_bindings (
            run_id, runtime_key, external_session_id, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?4)
        ON CONFLICT(run_id, runtime_key) DO UPDATE SET
            external_session_id = excluded.external_session_id,
            updated_at = excluded.updated_at
        "#,
        rusqlite::params![
            &input.run_id,
            &input.runtime_key,
            &input.external_session_id,
            &now
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// 删除 SOLO 运行时绑定。
#[tauri::command]
pub fn delete_solo_runtime_binding(run_id: String, runtime_key: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    conn.execute(
        "DELETE FROM solo_runtime_bindings WHERE run_id = ?1 AND runtime_key = ?2",
        rusqlite::params![&run_id, &runtime_key],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}
