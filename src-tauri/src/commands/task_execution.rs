use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::support::{now_rfc3339, open_db_connection};

/// 执行日志数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    pub id: String,
    pub task_id: String,
    #[serde(rename = "type")]
    pub log_type: String,
    pub content: String,
    pub metadata: Option<String>,
    pub created_at: String,
}

/// 任务执行结果（结构化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResultRecord {
    pub id: String,
    pub task_id: String,
    pub plan_id: String,
    pub task_title_snapshot: String,
    pub task_description_snapshot: Option<String>,
    pub result_status: String, // success | failed
    pub result_summary: Option<String>,
    pub result_files: Vec<String>,
    pub fail_reason: Option<String>,
    pub created_at: String,
}

/// 保存任务执行结果输入
#[derive(Debug, Deserialize)]
pub struct SaveTaskExecutionResultInput {
    pub task_id: String,
    pub result_status: String, // success | failed
    pub result_summary: Option<String>,
    pub result_files: Option<Vec<String>>,
    pub fail_reason: Option<String>,
}

/// 计划维度任务进度项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecutionTaskProgress {
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub task_order: i32,
    pub agent_id: Option<String>,
    pub model_id: Option<String>,
    pub last_result_status: Option<String>,
    pub last_result_summary: Option<String>,
    pub last_result_files: Vec<String>,
    pub last_fail_reason: Option<String>,
    pub last_result_at: Option<String>,
    pub updated_at: String,
}

/// 计划执行进度总览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecutionProgress {
    pub plan_id: String,
    pub total_tasks: i32,
    pub pending_count: i32,
    pub in_progress_count: i32,
    pub completed_count: i32,
    pub blocked_count: i32,
    pub cancelled_count: i32,
    pub success_count: i32,
    pub failed_count: i32,
    pub tasks: Vec<PlanExecutionTaskProgress>,
}

fn parse_json_string_array(value: Option<String>) -> Vec<String> {
    match value {
        Some(raw) => serde_json::from_str::<Vec<String>>(&raw).unwrap_or_else(|error| {
            eprintln!(
                "[task_execution] Failed to parse JSON string array: {} | raw={}",
                error, raw
            );
            Vec::new()
        }),
        None => Vec::new(),
    }
}

/// 创建任务执行日志
#[tauri::command]
pub fn create_task_execution_log(
    task_id: String,
    log_type: String,
    content: String,
    metadata: Option<String>,
) -> Result<ExecutionLog, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();

    conn.execute(
        "INSERT INTO task_execution_logs (id, task_id, log_type, content, metadata, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![&id, &task_id, &log_type, &content, &metadata, &now],
    )
    .map_err(|e| e.to_string())?;

    Ok(ExecutionLog {
        id,
        task_id,
        log_type,
        content,
        metadata,
        created_at: now,
    })
}

/// 获取任务的执行日志列表
#[tauri::command]
pub fn list_task_execution_logs(task_id: String) -> Result<Vec<ExecutionLog>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, log_type, content, metadata, created_at
             FROM task_execution_logs
             WHERE task_id = ?1
             ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let logs = stmt
        .query_map([&task_id], |row| {
            Ok(ExecutionLog {
                id: row.get(0)?,
                task_id: row.get(1)?,
                log_type: row.get(2)?,
                content: row.get(3)?,
                metadata: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(logs)
}

/// 清除任务的执行日志
#[tauri::command]
pub fn clear_task_execution_logs(task_id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM task_execution_logs WHERE task_id = ?1",
        [&task_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// 保存任务执行结果（完成/失败）
#[tauri::command]
pub fn save_task_execution_result(
    input: SaveTaskExecutionResultInput,
) -> Result<TaskExecutionResultRecord, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let now = now_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();

    let (plan_id, task_title_snapshot, task_description_snapshot): (
        String,
        String,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT plan_id, title, description FROM tasks WHERE id = ?1",
            [&input.task_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| e.to_string())?;

    let result_files = input.result_files.unwrap_or_default();
    let result_files_json = if result_files.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&result_files).map_err(|e| e.to_string())?)
    };

    conn.execute(
        "INSERT INTO task_execution_results
         (id, task_id, plan_id, task_title_snapshot, task_description_snapshot,
          result_status, result_summary, result_files, fail_reason, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            &id,
            &input.task_id,
            &plan_id,
            &task_title_snapshot,
            &task_description_snapshot,
            &input.result_status,
            &input.result_summary,
            &result_files_json,
            &input.fail_reason,
            &now,
        ],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE tasks
         SET last_result_status = ?1,
             last_result_summary = ?2,
             last_result_files = ?3,
             last_fail_reason = ?4,
             last_result_at = ?5,
             updated_at = ?6
         WHERE id = ?7",
        rusqlite::params![
            &input.result_status,
            &input.result_summary,
            &result_files_json,
            &input.fail_reason,
            &now,
            &now,
            &input.task_id
        ],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE plans SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &plan_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(TaskExecutionResultRecord {
        id,
        task_id: input.task_id,
        plan_id,
        task_title_snapshot,
        task_description_snapshot,
        result_status: input.result_status,
        result_summary: input.result_summary,
        result_files,
        fail_reason: input.fail_reason,
        created_at: now,
    })
}

/// 获取计划下最近 N 条任务执行结果（用于下一个任务上下文）
#[tauri::command]
pub fn list_recent_plan_results(
    plan_id: String,
    limit: Option<i32>,
) -> Result<Vec<TaskExecutionResultRecord>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let safe_limit = limit.unwrap_or(50).clamp(1, 500);

    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, plan_id, task_title_snapshot, task_description_snapshot,
                    result_status, result_summary, result_files, fail_reason, created_at
             FROM task_execution_results
             WHERE plan_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![&plan_id, safe_limit], |row| {
            let files_raw: Option<String> = row.get(7)?;
            Ok(TaskExecutionResultRecord {
                id: row.get(0)?,
                task_id: row.get(1)?,
                plan_id: row.get(2)?,
                task_title_snapshot: row.get(3)?,
                task_description_snapshot: row.get(4)?,
                result_status: row.get(5)?,
                result_summary: row.get(6)?,
                result_files: parse_json_string_array(files_raw),
                fail_reason: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rows)
}

/// 获取计划执行进度详情（右侧面板使用）
#[tauri::command]
pub fn list_plan_execution_progress(plan_id: String) -> Result<PlanExecutionProgress, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, title, status, task_order,
                    agent_id, model_id,
                    last_result_status, last_result_summary, last_result_files,
                    last_fail_reason, last_result_at, updated_at
             FROM tasks
             WHERE plan_id = ?1
             ORDER BY task_order ASC, created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let tasks = stmt
        .query_map([&plan_id], |row| {
            let files_raw: Option<String> = row.get(8)?;
            Ok(PlanExecutionTaskProgress {
                task_id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                task_order: row.get(3)?,
                agent_id: row.get(4)?,
                model_id: row.get(5)?,
                last_result_status: row.get(6)?,
                last_result_summary: row.get(7)?,
                last_result_files: parse_json_string_array(files_raw),
                last_fail_reason: row.get(9)?,
                last_result_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut progress = PlanExecutionProgress {
        plan_id,
        total_tasks: tasks.len() as i32,
        pending_count: 0,
        in_progress_count: 0,
        completed_count: 0,
        blocked_count: 0,
        cancelled_count: 0,
        success_count: 0,
        failed_count: 0,
        tasks,
    };

    for task in &progress.tasks {
        match task.status.as_str() {
            "pending" => progress.pending_count += 1,
            "in_progress" => progress.in_progress_count += 1,
            "completed" => progress.completed_count += 1,
            "blocked" => progress.blocked_count += 1,
            "cancelled" => progress.cancelled_count += 1,
            _ => {}
        }

        match task.last_result_status.as_deref() {
            Some("success") => progress.success_count += 1,
            Some("failed") => progress.failed_count += 1,
            _ => {}
        }
    }

    Ok(progress)
}

/// 获取任务执行日志统计
#[tauri::command]
pub fn get_task_execution_log_stats(task_id: String) -> Result<ExecutionLogStats, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM task_execution_logs WHERE task_id = ?1",
            [&task_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let last_log_at: Option<String> = conn
        .query_row(
            "SELECT created_at FROM task_execution_logs WHERE task_id = ?1 ORDER BY created_at DESC LIMIT 1",
            [&task_id],
            |row| row.get(0),
        )
        .ok();

    Ok(ExecutionLogStats {
        task_id,
        log_count: count,
        last_log_at,
    })
}

/// 执行日志统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLogStats {
    pub task_id: String,
    pub log_count: i32,
    pub last_log_at: Option<String>,
}

/// 清除计划的执行结果（同时清除关联任务的日志）
#[tauri::command]
pub fn clear_plan_execution_results(plan_id: String) -> Result<i32, String> {
    let mut conn = open_db_connection().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // 获取计划下所有任务 ID
    let task_ids: Vec<String> = tx
        .query_row(
            "SELECT GROUP_CONCAT(id) FROM tasks WHERE plan_id = ?1",
            [&plan_id],
            |row| {
                let ids_str: Option<String> = row.get(0)?;
                Ok(ids_str
                    .map(|s| s.split(',').map(|s| s.to_string()).collect())
                    .unwrap_or_default())
            },
        )
        .unwrap_or_default();

    // 清除任务执行日志
    let logs_deleted = if task_ids.is_empty() {
        0
    } else {
        let placeholders = (0..task_ids.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "DELETE FROM task_execution_logs WHERE task_id IN ({})",
            placeholders
        );
        let params = rusqlite::params_from_iter(task_ids.iter());
        tx.execute(&sql, params).map_err(|e| e.to_string())?
    };

    let results_deleted = tx
        .execute(
            "DELETE FROM task_execution_results WHERE plan_id = ?1",
            [&plan_id],
        )
        .map_err(|e| e.to_string())?;

    tx.execute(
        "UPDATE tasks SET last_result_status = NULL, last_result_summary = NULL,
         last_result_files = NULL, last_fail_reason = NULL, last_result_at = NULL
         WHERE plan_id = ?1",
        [&plan_id],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(logs_deleted as i32 + results_deleted as i32)
}
