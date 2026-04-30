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
    pub expert_id: Option<String>,
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
    pub execution_overview: Option<String>,
    pub execution_overview_updated_at: Option<String>,
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

fn normalize_overview_text(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_overview_text(raw: &str, max_chars: usize) -> String {
    let normalized = normalize_overview_text(raw);
    let total_chars = normalized.chars().count();
    if total_chars <= max_chars {
        return normalized;
    }

    let truncated = normalized.chars().take(max_chars).collect::<String>();
    format!("{}...", truncated)
}

#[derive(Debug, Clone)]
struct OverviewFileEntry {
    path: String,
    locations: Vec<String>,
}

fn format_task_overview_item(title: &str, summary: &str) -> String {
    let compact_summary = truncate_overview_text(summary, 56);
    if compact_summary.is_empty() {
        title.to_string()
    } else {
        format!("{}（{}）", title, compact_summary)
    }
}

fn split_file_reference(raw: &str) -> (String, Option<String>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }

    if let Some(index) = trimmed.rfind("#L") {
        let path = trimmed[..index].trim().to_string();
        let location = trimmed[index + 1..].trim().to_string();
        if !path.is_empty() && !location.is_empty() {
            return (path, Some(location));
        }
    }

    if let Some(last_colon) = trimmed.rfind(':') {
        let suffix = trimmed[last_colon + 1..].trim();
        if !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()) {
            let before = &trimmed[..last_colon];
            if let Some(second_colon) = before.rfind(':') {
                let column = before[second_colon + 1..].trim();
                let path = before[..second_colon].trim();
                if !column.is_empty()
                    && column.chars().all(|ch| ch.is_ascii_digit())
                    && (path.contains('/') || path.contains('\\'))
                {
                    return (path.to_string(), Some(format!("{}:{}", column, suffix)));
                }
            }

            if before.contains('/') || before.contains('\\') {
                return (before.trim().to_string(), Some(suffix.to_string()));
            }
        }
    }

    (trimmed.to_string(), None)
}

fn format_location_label(raw: &str) -> String {
    let trimmed = raw.trim().trim_start_matches('#');
    if trimmed.is_empty() {
        return String::new();
    }

    format!("行 {}", trimmed)
}

fn push_unique_file_entry(target: &mut Vec<OverviewFileEntry>, value: &str) {
    let (path, location) = split_file_reference(value);
    if path.is_empty() {
        return;
    }

    if let Some(existing) = target.iter_mut().find(|entry| entry.path == path) {
        if let Some(location) = location {
            if !location.is_empty() && !existing.locations.iter().any(|item| item == &location) {
                existing.locations.push(location);
            }
        }
        return;
    }

    target.push(OverviewFileEntry {
        path,
        locations: location.into_iter().collect(),
    });
}

fn summarize_location_list(items: &[String], limit: usize) -> String {
    if items.is_empty() {
        return String::new();
    }

    let visible = items
        .iter()
        .take(limit)
        .map(|item| format_location_label(item))
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();

    if visible.is_empty() {
        return String::new();
    }

    if items.len() > limit {
        format!("{} 等 {} 处", visible.join("、"), items.len())
    } else {
        visible.join("、")
    }
}

fn format_file_entry(item: &OverviewFileEntry) -> String {
    if item.locations.is_empty() {
        return item.path.clone();
    }

    format!(
        "{}（{}）",
        item.path,
        summarize_location_list(&item.locations, 3)
    )
}

fn summarize_file_entries(items: &[OverviewFileEntry], limit: usize) -> String {
    if items.is_empty() {
        return String::new();
    }

    let visible = items
        .iter()
        .take(limit)
        .map(format_file_entry)
        .collect::<Vec<_>>();

    if items.len() > limit {
        format!("{} 等 {} 项", visible.join("、"), items.len())
    } else {
        visible.join("、")
    }
}

fn summarize_overview_list(items: &[String], limit: usize) -> String {
    if items.is_empty() {
        return String::new();
    }

    let visible = items.iter().take(limit).cloned().collect::<Vec<_>>();
    if items.len() > limit {
        format!("{} 等 {} 项", visible.join("、"), items.len())
    } else {
        visible.join("、")
    }
}

fn summarize_task_file_changes(
    added_files: &[OverviewFileEntry],
    modified_files: &[OverviewFileEntry],
    changed_files: &[OverviewFileEntry],
    deleted_files: &[OverviewFileEntry],
) -> String {
    let mut segments: Vec<String> = Vec::new();

    if !added_files.is_empty() {
        segments.push(format!("新增 {}", summarize_file_entries(added_files, 2)));
    }
    if !modified_files.is_empty() {
        segments.push(format!(
            "修改 {}",
            summarize_file_entries(modified_files, 2)
        ));
    }
    if !changed_files.is_empty() {
        segments.push(format!("变更 {}", summarize_file_entries(changed_files, 2)));
    }
    if !deleted_files.is_empty() {
        segments.push(format!("删除 {}", summarize_file_entries(deleted_files, 2)));
    }

    segments.join("；")
}

fn build_plan_execution_overview(
    conn: &rusqlite::Connection,
    plan_id: &str,
) -> Result<String, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT title, last_result_status, last_result_summary, last_result_files, last_fail_reason
            FROM tasks
            WHERE plan_id = ?1
              AND last_result_status IS NOT NULL
            ORDER BY COALESCE(last_result_at, updated_at) ASC, task_order ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let records = stmt
        .query_map([plan_id], |row| {
            let title: String = row.get(0)?;
            let status: Option<String> = row.get(1)?;
            let summary: Option<String> = row.get(2)?;
            let files_raw: Option<String> = row.get(3)?;
            let fail_reason: Option<String> = row.get(4)?;
            Ok((
                title,
                status,
                summary,
                parse_json_string_array(files_raw),
                fail_reason,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    if records.is_empty() {
        return Ok(String::new());
    }

    let mut success_items: Vec<String> = Vec::new();
    let mut failed_items: Vec<String> = Vec::new();

    for (title, status, summary, files, fail_reason) in records.iter() {
        let summary_text = summary.as_deref().unwrap_or_default();
        let mut added_files: Vec<OverviewFileEntry> = Vec::new();
        let mut modified_files: Vec<OverviewFileEntry> = Vec::new();
        let mut changed_files: Vec<OverviewFileEntry> = Vec::new();
        let mut deleted_files: Vec<OverviewFileEntry> = Vec::new();

        for raw_file in files {
            if let Some(path) = raw_file.strip_prefix("added:") {
                push_unique_file_entry(&mut added_files, path.trim());
                continue;
            }
            if let Some(path) = raw_file.strip_prefix("modified:") {
                push_unique_file_entry(&mut modified_files, path.trim());
                continue;
            }
            if let Some(path) = raw_file.strip_prefix("changed:") {
                push_unique_file_entry(&mut changed_files, path.trim());
                continue;
            }
            if let Some(path) = raw_file.strip_prefix("deleted:") {
                push_unique_file_entry(&mut deleted_files, path.trim());
                continue;
            }
            push_unique_file_entry(&mut changed_files, raw_file.trim());
        }

        match status.as_deref() {
            Some("success") => {
                let file_changes = summarize_task_file_changes(
                    &added_files,
                    &modified_files,
                    &changed_files,
                    &deleted_files,
                );
                if !file_changes.is_empty() {
                    success_items.push(format!("{}（{}）", title, file_changes));
                } else {
                    success_items.push(format_task_overview_item(title, summary_text));
                }
            }
            Some("failed") => {
                let reason =
                    truncate_overview_text(fail_reason.as_deref().unwrap_or(summary_text), 48);
                if reason.is_empty() {
                    failed_items.push(title.clone());
                } else {
                    failed_items.push(format!("{}（{}）", title, reason));
                }
            }
            _ => {}
        }
    }

    let executed_count = success_items.len() + failed_items.len();
    let mut segments = vec![format!(
        "成功 {} 个，失败 {} 个",
        success_items.len(),
        failed_items.len()
    )];

    if !success_items.is_empty() {
        segments.push(format!(
            "成功任务：{}",
            summarize_overview_list(&success_items, 4)
        ));
    }

    if !failed_items.is_empty() {
        segments.push(format!(
            "失败任务：{}",
            summarize_overview_list(&failed_items, 3)
        ));
    }

    if executed_count == 0 {
        return Ok(String::new());
    }

    Ok(format!("{}。", segments.join("；")))
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

/// 更新任务执行日志内容。
#[tauri::command]
pub fn update_task_execution_log(
    id: String,
    content: String,
    metadata: Option<String>,
) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE task_execution_logs SET content = ?1, metadata = ?2 WHERE id = ?3",
        rusqlite::params![&content, &metadata, &id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
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

    let execution_overview = build_plan_execution_overview(&conn, &plan_id)?;
    conn.execute(
        "UPDATE plans
         SET execution_overview = ?1,
             execution_overview_updated_at = ?2,
             updated_at = ?2
         WHERE id = ?3",
        rusqlite::params![
            if execution_overview.trim().is_empty() {
                None::<String>
            } else {
                Some(execution_overview.clone())
            },
            &now,
            &plan_id
        ],
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
                    expert_id, agent_id, model_id,
                    last_result_status, last_result_summary, last_result_files,
                    last_fail_reason, last_result_at, updated_at
             FROM tasks
             WHERE plan_id = ?1
             ORDER BY task_order ASC, created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let tasks = stmt
        .query_map([&plan_id], |row| {
            let files_raw: Option<String> = row.get(9)?;
            Ok(PlanExecutionTaskProgress {
                task_id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                task_order: row.get(3)?,
                expert_id: row.get(4)?,
                agent_id: row.get(5)?,
                model_id: row.get(6)?,
                last_result_status: row.get(7)?,
                last_result_summary: row.get(8)?,
                last_result_files: parse_json_string_array(files_raw),
                last_fail_reason: row.get(10)?,
                last_result_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let computed_execution_overview = build_plan_execution_overview(&conn, &plan_id)?;
    let computed_execution_overview = if computed_execution_overview.trim().is_empty() {
        None
    } else {
        Some(computed_execution_overview)
    };
    let computed_execution_overview_updated_at = tasks
        .iter()
        .filter_map(|task| task.last_result_at.clone())
        .max();

    let (stored_execution_overview, stored_execution_overview_updated_at): (
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT execution_overview, execution_overview_updated_at FROM plans WHERE id = ?1",
            [&plan_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    if stored_execution_overview != computed_execution_overview
        || stored_execution_overview_updated_at != computed_execution_overview_updated_at
    {
        conn.execute(
            "UPDATE plans
             SET execution_overview = ?1,
                 execution_overview_updated_at = ?2
             WHERE id = ?3",
            rusqlite::params![
                &computed_execution_overview,
                &computed_execution_overview_updated_at,
                &plan_id
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    let mut progress = PlanExecutionProgress {
        plan_id,
        execution_overview: computed_execution_overview,
        execution_overview_updated_at: computed_execution_overview_updated_at,
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
        let placeholders = (0..task_ids.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
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

    tx.execute(
        "UPDATE plans
         SET execution_overview = NULL,
             execution_overview_updated_at = NULL,
             updated_at = ?1
         WHERE id = ?2",
        rusqlite::params![now_rfc3339(), &plan_id],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(logs_deleted as i32 + results_deleted as i32)
}
