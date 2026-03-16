use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, params_from_iter, OptionalExtension, ToSql};
use serde::{Deserialize, Serialize};

use super::support::{now_rfc3339, open_db_connection_with_foreign_keys};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryLibrary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub content_md: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawMemoryRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub message_id: Option<String>,
    pub content: String,
    pub source_role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMergeRun {
    pub id: String,
    pub library_id: String,
    pub source_record_ids: Vec<String>,
    pub source_record_count: i32,
    pub previous_content_md: String,
    pub merged_content_md: String,
    pub agent_id: Option<String>,
    pub model_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemoryLibraryInput {
    pub name: String,
    pub description: Option<String>,
    pub content_md: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemoryLibraryInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content_md: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRawMemoryRecordsQuery {
    pub session_id: Option<String>,
    pub project_id: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDeleteRawMemoryRecordsInput {
    pub session_id: Option<String>,
    pub project_id: Option<String>,
    pub search: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub limit: Option<i64>,
    pub delete_order: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDeleteRawMemoryRecordsResult {
    pub deleted_count: i32,
    pub deleted_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRawMemoryRecordInput {
    pub session_id: Option<String>,
    pub project_id: Option<String>,
    pub message_id: Option<String>,
    pub content: String,
    pub source_role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRawMemoryRecordInput {
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureUserMessageInput {
    pub session_id: String,
    pub message_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMemoryMergeRunsQuery {
    pub library_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeRawMemoriesIntoLibraryInput {
    pub library_id: String,
    pub source_record_ids: Vec<String>,
    pub merged_content_md: String,
    pub agent_id: Option<String>,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeRawMemoriesIntoLibraryResult {
    pub library: MemoryLibrary,
    pub merge_run: MemoryMergeRun,
}

fn get_db_connection() -> Result<rusqlite::Connection> {
    open_db_connection_with_foreign_keys()
}

fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|entry| {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_required_string(value: String, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{} 不能为空", field));
    }
    Ok(trimmed.to_string())
}

fn normalize_timestamp(value: Option<String>, field: &str) -> Result<Option<String>, String> {
    let Some(raw) = normalize_optional_string(value) else {
        return Ok(None);
    };

    let parsed = DateTime::parse_from_rfc3339(&raw)
        .map_err(|_| format!("{} 时间格式无效", field))?;
    Ok(Some(parsed.with_timezone(&Utc).to_rfc3339()))
}

fn append_raw_record_filters(
    sql: &mut String,
    bindings: &mut Vec<Box<dyn ToSql>>,
    query: &ListRawMemoryRecordsQuery,
    start_at: Option<&String>,
    end_at: Option<&String>,
) {
    if let Some(project_id) = query.project_id.clone() {
        sql.push_str(&format!(" AND r.project_id = ?{}", bindings.len() + 1));
        bindings.push(Box::new(project_id));
    }

    if let Some(session_id) = query.session_id.clone() {
        sql.push_str(&format!(" AND r.session_id = ?{}", bindings.len() + 1));
        bindings.push(Box::new(session_id));
    }

    if let Some(search) = normalize_optional_string(query.search.clone()) {
        sql.push_str(&format!(
            " AND LOWER(r.content) LIKE LOWER(?{})",
            bindings.len() + 1
        ));
        bindings.push(Box::new(format!("%{}%", search)));
    }

    if let Some(start_at) = start_at {
        sql.push_str(&format!(" AND r.created_at >= ?{}", bindings.len() + 1));
        bindings.push(Box::new(start_at.clone()));
    }

    if let Some(end_at) = end_at {
        sql.push_str(&format!(" AND r.created_at <= ?{}", bindings.len() + 1));
        bindings.push(Box::new(end_at.clone()));
    }
}

fn parse_string_array(raw: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
}

fn map_memory_library(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryLibrary> {
    Ok(MemoryLibrary {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        content_md: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn map_raw_memory_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawMemoryRecord> {
    Ok(RawMemoryRecord {
        id: row.get(0)?,
        session_id: row.get(1)?,
        session_name: row.get(2)?,
        project_id: row.get(3)?,
        project_name: row.get(4)?,
        message_id: row.get(5)?,
        content: row.get(6)?,
        source_role: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn map_memory_merge_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryMergeRun> {
    Ok(MemoryMergeRun {
        id: row.get(0)?,
        library_id: row.get(1)?,
        source_record_ids: parse_string_array(row.get::<_, String>(2)?),
        source_record_count: row.get(3)?,
        previous_content_md: row.get(4)?,
        merged_content_md: row.get(5)?,
        agent_id: row.get(6)?,
        model_id: row.get(7)?,
        created_at: row.get(8)?,
    })
}

fn get_memory_library_by_id(
    conn: &rusqlite::Connection,
    id: &str,
) -> Result<MemoryLibrary, String> {
    conn.query_row(
        r#"
        SELECT id, name, description, content_md, created_at, updated_at
        FROM memory_libraries
        WHERE id = ?1
        "#,
        [id],
        map_memory_library,
    )
    .map_err(|error| error.to_string())
}

fn get_raw_memory_record_by_id(
    conn: &rusqlite::Connection,
    id: &str,
) -> Result<RawMemoryRecord, String> {
    conn.query_row(
        r#"
        SELECT
            r.id,
            r.session_id,
            s.name,
            r.project_id,
            p.name,
            r.message_id,
            r.content,
            r.source_role,
            r.created_at,
            r.updated_at
        FROM raw_memory_records r
        LEFT JOIN sessions s ON s.id = r.session_id
        LEFT JOIN projects p ON p.id = r.project_id
        WHERE r.id = ?1
        "#,
        [id],
        map_raw_memory_record,
    )
    .map_err(|error| error.to_string())
}

fn get_memory_merge_run_by_id(
    conn: &rusqlite::Connection,
    id: &str,
) -> Result<MemoryMergeRun, String> {
    conn.query_row(
        r#"
        SELECT
            id,
            library_id,
            source_record_ids,
            source_record_count,
            previous_content_md,
            merged_content_md,
            agent_id,
            model_id,
            created_at
        FROM memory_merge_runs
        WHERE id = ?1
        "#,
        [id],
        map_memory_merge_run,
    )
    .map_err(|error| error.to_string())
}

fn resolve_project_id_from_session(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT project_id FROM sessions WHERE id = ?1",
        [session_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|error| error.to_string())
}

fn count_existing_raw_records(
    conn: &rusqlite::Connection,
    record_ids: &[String],
) -> Result<usize, String> {
    if record_ids.is_empty() {
        return Ok(0);
    }

    let placeholders = (1..=record_ids.len())
        .map(|index| format!("?{}", index))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT COUNT(*) FROM raw_memory_records WHERE id IN ({})",
        placeholders
    );
    let params = record_ids.iter().map(|id| id as &dyn ToSql);
    conn.query_row(&sql, params_from_iter(params), |row| row.get::<_, i64>(0))
        .map(|count| count as usize)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_memory_libraries() -> Result<Vec<MemoryLibrary>, String> {
    let conn = get_db_connection().map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, name, description, content_md, created_at, updated_at
            FROM memory_libraries
            ORDER BY updated_at DESC, created_at DESC
            "#,
        )
        .map_err(|error| error.to_string())?;

    let libraries = stmt
        .query_map([], map_memory_library)
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    Ok(libraries)
}

#[tauri::command]
pub fn get_memory_library(id: String) -> Result<MemoryLibrary, String> {
    let conn = get_db_connection().map_err(|error| error.to_string())?;
    get_memory_library_by_id(&conn, &id)
}

#[tauri::command]
pub fn create_memory_library(input: CreateMemoryLibraryInput) -> Result<MemoryLibrary, String> {
    let conn = get_db_connection().map_err(|error| error.to_string())?;
    let now = now_rfc3339();
    let id = generate_id();
    let name = normalize_required_string(input.name, "记忆库名称")?;

    conn.execute(
        r#"
        INSERT INTO memory_libraries (id, name, description, content_md, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            &id,
            &name,
            normalize_optional_string(input.description),
            input.content_md.unwrap_or_default(),
            &now,
            &now
        ],
    )
    .map_err(|error| error.to_string())?;

    get_memory_library_by_id(&conn, &id)
}

#[tauri::command]
pub fn update_memory_library(
    id: String,
    input: UpdateMemoryLibraryInput,
) -> Result<MemoryLibrary, String> {
    let conn = get_db_connection().map_err(|error| error.to_string())?;
    let existing = get_memory_library_by_id(&conn, &id)?;
    let now = now_rfc3339();

    let name = match input.name {
        Some(value) => normalize_required_string(value, "记忆库名称")?,
        None => existing.name,
    };
    let description = match input.description {
        Some(value) => normalize_optional_string(Some(value)),
        None => existing.description,
    };
    let content_md = input.content_md.unwrap_or(existing.content_md);

    conn.execute(
        r#"
        UPDATE memory_libraries
        SET name = ?1,
            description = ?2,
            content_md = ?3,
            updated_at = ?4
        WHERE id = ?5
        "#,
        params![name, description, content_md, &now, &id],
    )
    .map_err(|error| error.to_string())?;

    get_memory_library_by_id(&conn, &id)
}

#[tauri::command]
pub fn delete_memory_library(id: String) -> Result<(), String> {
    let conn = get_db_connection().map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM memory_libraries WHERE id = ?1", [&id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_raw_memory_records(
    query: ListRawMemoryRecordsQuery,
) -> Result<Vec<RawMemoryRecord>, String> {
    let conn = get_db_connection().map_err(|error| error.to_string())?;
    let mut sql = String::from(
        r#"
        SELECT
            r.id,
            r.session_id,
            s.name,
            r.project_id,
            p.name,
            r.message_id,
            r.content,
            r.source_role,
            r.created_at,
            r.updated_at
        FROM raw_memory_records r
        LEFT JOIN sessions s ON s.id = r.session_id
        LEFT JOIN projects p ON p.id = r.project_id
        WHERE 1 = 1
        "#,
    );

    let mut bindings: Vec<Box<dyn ToSql>> = Vec::new();
    append_raw_record_filters(&mut sql, &mut bindings, &query, None, None);

    sql.push_str(" ORDER BY r.created_at DESC");

    let params: Vec<&dyn ToSql> = bindings.iter().map(|value| value.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let records = stmt
        .query_map(params_from_iter(params), map_raw_memory_record)
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    Ok(records)
}

#[tauri::command]
pub fn create_raw_memory_record(
    input: CreateRawMemoryRecordInput,
) -> Result<RawMemoryRecord, String> {
    let conn = get_db_connection().map_err(|error| error.to_string())?;
    let now = now_rfc3339();
    let id = generate_id();
    let content = normalize_required_string(input.content, "原始记忆内容")?;
    let session_id = normalize_optional_string(input.session_id);
    let project_id = match (
        normalize_optional_string(input.project_id),
        session_id.as_ref(),
    ) {
        (Some(project_id), _) => Some(project_id),
        (None, Some(session_id)) => resolve_project_id_from_session(&conn, session_id)?,
        (None, None) => None,
    };

    conn.execute(
        r#"
        INSERT INTO raw_memory_records (
            id,
            session_id,
            project_id,
            message_id,
            content,
            source_role,
            created_at,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            &id,
            session_id,
            project_id,
            normalize_optional_string(input.message_id),
            content,
            normalize_optional_string(input.source_role).unwrap_or_else(|| "user".to_string()),
            &now,
            &now
        ],
    )
    .map_err(|error| error.to_string())?;

    get_raw_memory_record_by_id(&conn, &id)
}

#[tauri::command]
pub fn update_raw_memory_record(
    id: String,
    input: UpdateRawMemoryRecordInput,
) -> Result<RawMemoryRecord, String> {
    let conn = get_db_connection().map_err(|error| error.to_string())?;
    let existing = get_raw_memory_record_by_id(&conn, &id)?;
    let now = now_rfc3339();
    let content = match input.content {
        Some(value) => normalize_required_string(value, "原始记忆内容")?,
        None => existing.content,
    };

    conn.execute(
        r#"
        UPDATE raw_memory_records
        SET content = ?1,
            updated_at = ?2
        WHERE id = ?3
        "#,
        params![content, &now, &id],
    )
    .map_err(|error| error.to_string())?;

    get_raw_memory_record_by_id(&conn, &id)
}

#[tauri::command]
pub fn delete_raw_memory_record(id: String) -> Result<(), String> {
    let conn = get_db_connection().map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM raw_memory_records WHERE id = ?1", [&id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn batch_delete_raw_memory_records(
    input: BatchDeleteRawMemoryRecordsInput,
) -> Result<BatchDeleteRawMemoryRecordsResult, String> {
    let start_at = normalize_timestamp(input.start_at, "开始时间")?;
    let end_at = normalize_timestamp(input.end_at, "结束时间")?;
    if let (Some(start_at), Some(end_at)) = (&start_at, &end_at) {
        if start_at > end_at {
            return Err("开始时间不能晚于结束时间".to_string());
        }
    }

    let limit = input.limit.unwrap_or(0);
    if limit < 0 {
        return Err("删除条数不能小于 0".to_string());
    }
    if start_at.is_none() && end_at.is_none() && limit == 0 {
        return Err("请至少设置时间范围或删除条数".to_string());
    }

    let delete_order = match normalize_optional_string(input.delete_order) {
        Some(order) if order.eq_ignore_ascii_case("latest") || order.eq_ignore_ascii_case("newest") => {
            "DESC"
        }
        Some(order) if order.eq_ignore_ascii_case("oldest") => "ASC",
        Some(_) => return Err("删除顺序仅支持 oldest 或 latest".to_string()),
        None => "ASC",
    };

    let query = ListRawMemoryRecordsQuery {
        session_id: input.session_id,
        project_id: input.project_id,
        search: input.search,
    };

    let conn = get_db_connection().map_err(|error| error.to_string())?;
    let mut sql = String::from("SELECT r.id FROM raw_memory_records r WHERE 1 = 1");
    let mut bindings: Vec<Box<dyn ToSql>> = Vec::new();
    append_raw_record_filters(
        &mut sql,
        &mut bindings,
        &query,
        start_at.as_ref(),
        end_at.as_ref(),
    );
    sql.push_str(&format!(" ORDER BY r.created_at {}", delete_order));

    if limit > 0 {
        sql.push_str(&format!(" LIMIT ?{}", bindings.len() + 1));
        bindings.push(Box::new(limit));
    }

    let params: Vec<&dyn ToSql> = bindings.iter().map(|value| value.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let deleted_ids = stmt
        .query_map(params_from_iter(params), |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    if deleted_ids.is_empty() {
        return Ok(BatchDeleteRawMemoryRecordsResult {
            deleted_count: 0,
            deleted_ids,
        });
    }

    let tx = conn.unchecked_transaction().map_err(|error| error.to_string())?;
    let placeholders = (1..=deleted_ids.len())
        .map(|index| format!("?{}", index))
        .collect::<Vec<_>>()
        .join(", ");
    let delete_sql = format!(
        "DELETE FROM raw_memory_records WHERE id IN ({})",
        placeholders
    );
    let delete_params = deleted_ids.iter().map(|id| id as &dyn ToSql);
    tx.execute(&delete_sql, params_from_iter(delete_params))
        .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;

    Ok(BatchDeleteRawMemoryRecordsResult {
        deleted_count: deleted_ids.len() as i32,
        deleted_ids,
    })
}

#[tauri::command]
pub fn capture_user_message(input: CaptureUserMessageInput) -> Result<RawMemoryRecord, String> {
    let conn = get_db_connection().map_err(|error| error.to_string())?;
    let message_id = normalize_required_string(input.message_id, "消息 ID")?;

    let existing = conn
        .query_row(
            r#"
            SELECT
                r.id,
                r.session_id,
                s.name,
                r.project_id,
                p.name,
                r.message_id,
                r.content,
                r.source_role,
                r.created_at,
                r.updated_at
            FROM raw_memory_records r
            LEFT JOIN sessions s ON s.id = r.session_id
            LEFT JOIN projects p ON p.id = r.project_id
            WHERE r.message_id = ?1
            "#,
            [&message_id],
            map_raw_memory_record,
        )
        .optional()
        .map_err(|error| error.to_string())?;

    if let Some(record) = existing {
        return Ok(record);
    }

    let now = now_rfc3339();
    let id = generate_id();
    let project_id = resolve_project_id_from_session(&conn, &input.session_id)?;
    let content = normalize_required_string(input.content, "原始记忆内容")?;

    conn.execute(
        r#"
        INSERT INTO raw_memory_records (
            id,
            session_id,
            project_id,
            message_id,
            content,
            source_role,
            created_at,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, 'user', ?6, ?7)
        "#,
        params![
            &id,
            &input.session_id,
            project_id,
            &message_id,
            content,
            &now,
            &now
        ],
    )
    .map_err(|error| error.to_string())?;

    get_raw_memory_record_by_id(&conn, &id)
}

#[tauri::command]
pub fn list_memory_merge_runs(
    query: ListMemoryMergeRunsQuery,
) -> Result<Vec<MemoryMergeRun>, String> {
    let conn = get_db_connection().map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                id,
                library_id,
                source_record_ids,
                source_record_count,
                previous_content_md,
                merged_content_md,
                agent_id,
                model_id,
                created_at
            FROM memory_merge_runs
            WHERE library_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .map_err(|error| error.to_string())?;

    let merge_runs = stmt
        .query_map([&query.library_id], map_memory_merge_run)
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    Ok(merge_runs)
}

#[tauri::command]
pub fn merge_raw_memories_into_library(
    input: MergeRawMemoriesIntoLibraryInput,
) -> Result<MergeRawMemoriesIntoLibraryResult, String> {
    if input.source_record_ids.is_empty() {
        return Err("请先选择至少一条原始记忆".to_string());
    }

    let merged_content_md =
        normalize_required_string(input.merged_content_md, "合并后的 Markdown")?;
    let mut conn = get_db_connection().map_err(|error| error.to_string())?;
    let library = get_memory_library_by_id(&conn, &input.library_id)?;
    let existing_count = count_existing_raw_records(&conn, &input.source_record_ids)?;

    if existing_count != input.source_record_ids.len() {
        return Err("部分原始记忆不存在，无法完成压缩".to_string());
    }

    let tx = conn.transaction().map_err(|error| error.to_string())?;
    let now = now_rfc3339();
    let merge_run_id = generate_id();
    let source_record_ids_json =
        serde_json::to_string(&input.source_record_ids).map_err(|error| error.to_string())?;

    tx.execute(
        r#"
        UPDATE memory_libraries
        SET content_md = ?1,
            updated_at = ?2
        WHERE id = ?3
        "#,
        params![&merged_content_md, &now, &input.library_id],
    )
    .map_err(|error| error.to_string())?;

    tx.execute(
        r#"
        INSERT INTO memory_merge_runs (
            id,
            library_id,
            source_record_ids,
            source_record_count,
            previous_content_md,
            merged_content_md,
            agent_id,
            model_id,
            created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            &merge_run_id,
            &input.library_id,
            &source_record_ids_json,
            input.source_record_ids.len() as i32,
            &library.content_md,
            &merged_content_md,
            normalize_optional_string(input.agent_id),
            normalize_optional_string(input.model_id),
            &now
        ],
    )
    .map_err(|error| error.to_string())?;

    tx.commit().map_err(|error| error.to_string())?;

    let conn = get_db_connection().map_err(|error| error.to_string())?;
    Ok(MergeRawMemoriesIntoLibraryResult {
        library: get_memory_library_by_id(&conn, &input.library_id)?,
        merge_run: get_memory_merge_run_by_id(&conn, &merge_run_id)?,
    })
}
