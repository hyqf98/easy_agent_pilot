use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, params_from_iter, OptionalExtension, ToSql};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::support::{now_rfc3339, open_db_connection, repair_memory_search_indexes};

const REFERENCED_MEMORY_BLOCK_HEADER: &str = "[用户主动引用的历史记忆]";
const CURRENT_INPUT_BLOCK_HEADER: &str = "[用户当前输入]";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemorySuggestionSourceType {
    LibraryChunk,
    RawRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySuggestion {
    pub source_type: MemorySuggestionSourceType,
    pub source_id: String,
    pub title: String,
    pub snippet: String,
    pub full_content: String,
    pub score: f64,
    pub matched_terms: Vec<String>,
    pub library_id: Option<String>,
    pub library_name: Option<String>,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMemorySuggestionsInput {
    pub session_id: String,
    pub project_id: Option<String>,
    pub draft_text: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMemorySuggestionsResult {
    pub library_suggestions: Vec<MemorySuggestion>,
    pub raw_suggestions: Vec<MemorySuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordSessionMemoryReferenceItem {
    pub source_type: MemorySuggestionSourceType,
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordSessionMemoryReferencesInput {
    pub session_id: String,
    pub message_id: String,
    pub references: Vec<RecordSessionMemoryReferenceItem>,
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

    let parsed =
        DateTime::parse_from_rfc3339(&raw).map_err(|_| format!("{} 时间格式无效", field))?;
    Ok(Some(parsed.with_timezone(&Utc).to_rfc3339()))
}

fn normalize_search_text(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .lines()
        .filter(|line| !line.trim_start().starts_with("[[memory-ref:"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn extract_raw_memory_capture_content(value: &str) -> String {
    let trimmed = value.trim();
    if !trimmed.starts_with(REFERENCED_MEMORY_BLOCK_HEADER) {
        return trimmed.to_string();
    }

    let Some(index) = trimmed.find(CURRENT_INPUT_BLOCK_HEADER) else {
        return trimmed.to_string();
    };

    trimmed[index + CURRENT_INPUT_BLOCK_HEADER.len()..]
        .trim()
        .to_string()
}

fn build_search_candidates(value: &str) -> Vec<String> {
    let normalized = normalize_search_text(value);
    if normalized.chars().count() < 4 {
        return Vec::new();
    }

    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for segment in normalized.split(|ch: char| {
        matches!(
            ch,
            '\n' | '。' | '！' | '？' | '；' | ',' | '.' | '!' | '?' | ';'
        )
    }) {
        let trimmed = segment.trim();
        let char_count = trimmed.chars().count();
        if char_count < 4 || char_count > 80 {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            unique.push(trimmed.to_string());
        }
    }

    unique.sort_by(|left, right| right.chars().count().cmp(&left.chars().count()));
    unique.truncate(3);
    if seen.insert(normalized.clone()) {
        unique.push(normalized);
    }
    unique
}

fn escape_fts_phrase(value: &str) -> String {
    value.replace('"', "\"\"")
}

fn sanitize_fts_term(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric()
                || ('\u{4e00}'..='\u{9fff}').contains(&ch)
                || ch.is_whitespace()
            {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_fts_match_query(candidates: &[String]) -> Option<String> {
    let phrases = candidates
        .iter()
        .map(|candidate| sanitize_fts_term(candidate))
        .filter(|candidate| candidate.chars().count() >= 3)
        .map(|candidate| {
            let terms = candidate
                .split_whitespace()
                .filter(|term| term.chars().count() >= 3)
                .map(|term| format!("\"{}\"", escape_fts_phrase(term)))
                .collect::<Vec<_>>();

            if terms.len() <= 1 {
                terms.into_iter().next().unwrap_or_default()
            } else {
                format!("({})", terms.join(" AND "))
            }
        })
        .filter(|candidate| !candidate.is_empty())
        .collect::<Vec<_>>();

    if phrases.is_empty() {
        None
    } else {
        Some(phrases.join(" OR "))
    }
}

fn build_matched_terms(content: &str, candidates: &[String]) -> Vec<String> {
    let normalized_content = content.to_lowercase();
    let mut matched = Vec::new();

    for candidate in candidates {
        let normalized_candidate = candidate.trim().to_lowercase();
        if normalized_candidate.chars().count() < 4 {
            continue;
        }
        if normalized_content.contains(&normalized_candidate)
            && !matched.iter().any(|entry: &String| entry == candidate)
        {
            matched.push(candidate.clone());
        }
    }

    matched
}

fn build_snippet(content: &str, candidates: &[String]) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let normalized = trimmed.to_lowercase();
    let mut best_index = None;
    let mut best_len = 0usize;

    for candidate in candidates {
        let lookup = candidate.trim().to_lowercase();
        if lookup.chars().count() < 4 {
            continue;
        }
        if let Some(index) = normalized.find(&lookup) {
            if lookup.len() > best_len {
                best_index = Some(index);
                best_len = lookup.len();
            }
        }
    }

    let char_window = 54usize;
    let chars = trimmed.chars().collect::<Vec<_>>();
    let start_char = best_index
        .map(|byte_index| trimmed[..byte_index].chars().count().saturating_sub(18))
        .unwrap_or(0);
    let end_char = (start_char + char_window).min(chars.len());
    let snippet = chars[start_char..end_char]
        .iter()
        .collect::<String>()
        .trim()
        .to_string();

    if start_char > 0 && end_char < chars.len() {
        format!("…{}…", snippet)
    } else if start_char > 0 {
        format!("…{}", snippet)
    } else if end_char < chars.len() {
        format!("{}…", snippet)
    } else {
        snippet
    }
}

fn stable_hash_text(value: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn chunk_memory_content(content: &str) -> Vec<String> {
    let normalized = content.replace("\r\n", "\n");
    let mut chunks = Vec::new();
    let mut buffer = String::new();

    for paragraph in normalized.split("\n\n") {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }

        let next = if buffer.is_empty() {
            trimmed.to_string()
        } else {
            format!("{buffer}\n\n{trimmed}")
        };

        if next.chars().count() <= 480 {
            buffer = next;
            continue;
        }

        if !buffer.is_empty() {
            chunks.push(buffer.trim().to_string());
            buffer.clear();
        }

        if trimmed.chars().count() <= 480 {
            buffer = trimmed.to_string();
            continue;
        }

        let sentences = trimmed
            .split_inclusive(|ch: char| {
                matches!(ch, '。' | '！' | '？' | '；' | '.' | '!' | '?' | ';' | '\n')
            })
            .collect::<Vec<_>>();

        if sentences.len() <= 1 {
            chunks.push(trimmed.to_string());
            continue;
        }

        let mut sentence_buffer = String::new();
        for sentence in sentences {
            let sentence = sentence.trim();
            if sentence.is_empty() {
                continue;
            }

            let next_sentence = if sentence_buffer.is_empty() {
                sentence.to_string()
            } else {
                format!("{sentence_buffer} {sentence}")
            };

            if next_sentence.chars().count() <= 320 {
                sentence_buffer = next_sentence;
                continue;
            }

            if !sentence_buffer.is_empty() {
                chunks.push(sentence_buffer.trim().to_string());
            }
            sentence_buffer = sentence.to_string();
        }

        if !sentence_buffer.trim().is_empty() {
            chunks.push(sentence_buffer.trim().to_string());
        }
    }

    if !buffer.trim().is_empty() {
        chunks.push(buffer.trim().to_string());
    }

    chunks
}

fn sync_library_chunks(
    tx: &rusqlite::Transaction<'_>,
    library_id: &str,
    content_md: &str,
    now: &str,
) -> Result<(), String> {
    tx.execute(
        "DELETE FROM memory_library_chunks WHERE library_id = ?1",
        [library_id],
    )
    .map_err(|error| error.to_string())?;

    let chunks = chunk_memory_content(content_md);
    for (index, chunk_text) in chunks.iter().enumerate() {
        tx.execute(
            r#"
            INSERT INTO memory_library_chunks (
                id,
                library_id,
                chunk_text,
                chunk_order,
                chunk_hash,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                generate_id(),
                library_id,
                chunk_text,
                index as i32,
                stable_hash_text(chunk_text),
                now,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    }

    Ok(())
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

fn list_project_memory_library_ids(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT library_id
            FROM project_memory_libraries
            WHERE project_id = ?1
            ORDER BY created_at ASC, library_id ASC
            "#,
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map([project_id], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    Ok(rows)
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

fn search_library_suggestions(
    conn: &rusqlite::Connection,
    session_id: &str,
    excluded_library_ids: &[String],
    _match_query: &str,
    candidates: &[String],
    limit: i64,
) -> Result<Vec<MemorySuggestion>, String> {
    let mut sql = String::from(
        r#"
        SELECT
            c.id,
            c.chunk_text,
            l.id,
            l.name,
            l.updated_at
        FROM memory_library_chunks c
        JOIN memory_libraries l ON l.id = c.library_id
        LEFT JOIN session_memory_reference_history h
            ON h.session_id = ?1
           AND h.source_type = 'library_chunk'
           AND h.source_id = c.id
        WHERE h.source_id IS NULL
        "#,
    );

    let search_terms = candidates
        .iter()
        .map(|candidate| candidate.trim())
        .filter(|candidate| candidate.chars().count() >= 3)
        .collect::<Vec<_>>();
    if search_terms.is_empty() {
        return Ok(Vec::new());
    }

    let mut bindings: Vec<Box<dyn ToSql>> = vec![Box::new(session_id.to_string())];
    sql.push_str(" AND (");
    for (index, term) in search_terms.iter().enumerate() {
        if index > 0 {
            sql.push_str(" OR ");
        }
        sql.push_str(&format!(
            "LOWER(c.chunk_text) LIKE LOWER(?{})",
            bindings.len() + 1
        ));
        bindings.push(Box::new(format!("%{}%", term)));
    }
    sql.push(')');

    for library_id in excluded_library_ids {
        sql.push_str(&format!(" AND l.id != ?{}", bindings.len() + 1));
        bindings.push(Box::new(library_id.clone()));
    }

    sql.push_str(
        r#"
        ORDER BY l.updated_at DESC, c.chunk_order ASC
        "#,
    );

    let params: Vec<&dyn ToSql> = bindings.iter().map(|value| value.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let mut rows = stmt
        .query_map(params_from_iter(params), |row| {
            let full_content = row.get::<_, String>(1)?;
            let library_name = row.get::<_, String>(3)?;
            let matched_terms = build_matched_terms(&full_content, candidates);
            let score = matched_terms
                .iter()
                .map(|term| term.chars().count() as f64)
                .sum::<f64>();
            Ok(MemorySuggestion {
                source_type: MemorySuggestionSourceType::LibraryChunk,
                source_id: row.get(0)?,
                title: format!("记忆库《{}》", library_name),
                snippet: build_snippet(&full_content, candidates),
                full_content,
                score,
                matched_terms,
                library_id: Some(row.get(2)?),
                library_name: Some(library_name),
                session_id: None,
                session_name: None,
                project_id: None,
                project_name: None,
                created_at: Some(row.get(4)?),
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    rows.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.created_at.cmp(&left.created_at))
    });
    rows.truncate(limit as usize);
    Ok(rows)
}

fn search_raw_memory_suggestions(
    conn: &rusqlite::Connection,
    session_id: &str,
    project_id: Option<&String>,
    draft_text: &str,
    candidates: &[String],
    limit: i64,
) -> Result<Vec<MemorySuggestion>, String> {
    let search_terms = candidates
        .iter()
        .map(|candidate| candidate.trim())
        .filter(|candidate| candidate.chars().count() >= 3)
        .collect::<Vec<_>>();
    if search_terms.is_empty() {
        return Ok(Vec::new());
    }

    let normalized_draft = normalize_search_text(draft_text);
    let mut sql = String::from(
        r#"
        SELECT
            r.id,
            r.content,
            r.session_id,
            s.name,
            r.project_id,
            p.name,
            r.created_at
        FROM raw_memory_records r
        LEFT JOIN sessions s ON s.id = r.session_id
        LEFT JOIN projects p ON p.id = r.project_id
        LEFT JOIN session_memory_reference_history h
            ON h.session_id = ?1
           AND h.source_type = 'raw_record'
           AND h.source_id = r.id
        WHERE h.source_id IS NULL
          AND TRIM(r.content) NOT LIKE '[用户主动引用的历史记忆]%'
        "#,
    );

    let mut bindings: Vec<Box<dyn ToSql>> = vec![Box::new(session_id.to_string())];
    sql.push_str(" AND (");
    for (index, term) in search_terms.iter().enumerate() {
        if index > 0 {
            sql.push_str(" OR ");
        }
        sql.push_str(&format!(
            "LOWER(r.content) LIKE LOWER(?{})",
            bindings.len() + 1
        ));
        bindings.push(Box::new(format!("%{}%", term)));
    }
    sql.push(')');
    sql.push_str(" ORDER BY r.created_at DESC");

    let params: Vec<&dyn ToSql> = bindings.iter().map(|value| value.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params_from_iter(params), |row| {
            let full_content = row.get::<_, String>(1)?;
            if normalize_search_text(&full_content) == normalized_draft {
                return Ok(None);
            }
            let matched_terms = build_matched_terms(&full_content, candidates);
            let created_at = row.get::<_, String>(6)?;
            let session_name = row.get::<_, Option<String>>(3)?;
            let row_session_id = row.get::<_, Option<String>>(2)?;
            let row_project_id = row.get::<_, Option<String>>(4)?;
            let scope_rank = if row_session_id.as_deref() == Some(session_id) {
                0.0
            } else if row_project_id.as_ref() == project_id {
                1.0
            } else {
                2.0
            };
            let match_score = matched_terms
                .iter()
                .map(|term| term.chars().count() as f64)
                .sum::<f64>();
            Ok(Some(MemorySuggestion {
                source_type: MemorySuggestionSourceType::RawRecord,
                source_id: row.get(0)?,
                title: match &session_name {
                    Some(name) => format!("原始记忆 · {}", name),
                    None => "原始记忆".to_string(),
                },
                snippet: build_snippet(&full_content, candidates),
                full_content,
                score: (10.0 - scope_rank) + match_score,
                matched_terms,
                library_id: None,
                library_name: None,
                session_id: row_session_id,
                session_name,
                project_id: row_project_id,
                project_name: row.get(5)?,
                created_at: Some(created_at),
            }))
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    let mut rows = rows.into_iter().flatten().collect::<Vec<_>>();

    rows.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.created_at.cmp(&left.created_at))
    });
    rows.truncate(limit as usize);
    Ok(rows)
}

#[tauri::command]
pub fn list_memory_libraries() -> Result<Vec<MemoryLibrary>, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
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
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    get_memory_library_by_id(&conn, &id)
}

#[tauri::command]
pub fn create_memory_library(input: CreateMemoryLibraryInput) -> Result<MemoryLibrary, String> {
    let mut conn = open_db_connection().map_err(|error| error.to_string())?;
    let now = now_rfc3339();
    let id = generate_id();
    let name = normalize_required_string(input.name, "记忆库名称")?;
    let description = normalize_optional_string(input.description);
    let content_md = input.content_md.unwrap_or_default();
    let tx = conn.transaction().map_err(|error| error.to_string())?;

    tx.execute(
        r#"
        INSERT INTO memory_libraries (id, name, description, content_md, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![&id, &name, description, &content_md, &now, &now],
    )
    .map_err(|error| error.to_string())?;
    sync_library_chunks(&tx, &id, &content_md, &now)?;
    tx.commit().map_err(|error| error.to_string())?;

    get_memory_library_by_id(&conn, &id)
}

#[tauri::command]
pub fn update_memory_library(
    id: String,
    input: UpdateMemoryLibraryInput,
) -> Result<MemoryLibrary, String> {
    let mut conn = open_db_connection().map_err(|error| error.to_string())?;
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
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    tx.execute(
        r#"
        UPDATE memory_libraries
        SET name = ?1,
            description = ?2,
            content_md = ?3,
            updated_at = ?4
        WHERE id = ?5
        "#,
        params![name, description, &content_md, &now, &id],
    )
    .map_err(|error| error.to_string())?;
    sync_library_chunks(&tx, &id, &content_md, &now)?;
    tx.commit().map_err(|error| error.to_string())?;

    get_memory_library_by_id(&conn, &id)
}

#[tauri::command]
pub fn delete_memory_library(id: String) -> Result<(), String> {
    let mut conn = open_db_connection().map_err(|error| error.to_string())?;
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    tx.execute(
        "DELETE FROM memory_library_chunks WHERE library_id = ?1",
        [&id],
    )
    .map_err(|error| error.to_string())?;
    tx.execute("DELETE FROM memory_libraries WHERE id = ?1", [&id])
        .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_raw_memory_records(
    query: ListRawMemoryRecordsQuery,
) -> Result<Vec<RawMemoryRecord>, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
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
    let conn = open_db_connection().map_err(|error| error.to_string())?;
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
    let conn = open_db_connection().map_err(|error| error.to_string())?;
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
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    repair_memory_search_indexes(&conn)
        .map_err(|error| format!("修复记忆搜索索引失败: {}", error))?;
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
        Some(order)
            if order.eq_ignore_ascii_case("latest") || order.eq_ignore_ascii_case("newest") =>
        {
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

    let conn = open_db_connection().map_err(|error| error.to_string())?;
    repair_memory_search_indexes(&conn)
        .map_err(|error| format!("修复记忆搜索索引失败: {}", error))?;
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

    let tx = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
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
    let conn = open_db_connection().map_err(|error| error.to_string())?;
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
    let content = normalize_required_string(
        extract_raw_memory_capture_content(&input.content),
        "原始记忆内容",
    )?;

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

/// 基于当前草稿实时检索可引用的记忆候选。
///
/// 用途：为主会话输入框提供轻量的本地记忆召回，先检索记忆库分块，再检索原始记忆。
/// 参数：会话 ID、项目 ID（可选）、当前草稿文本、返回上限。
/// 返回：按来源分组的记忆建议列表；已在当前会话引用过的内容会被自动排除。
#[tauri::command]
pub fn search_memory_suggestions(
    input: SearchMemorySuggestionsInput,
) -> Result<SearchMemorySuggestionsResult, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let session_id = normalize_required_string(input.session_id, "会话 ID")?;
    let candidates = build_search_candidates(&input.draft_text);
    let match_query = match build_fts_match_query(&candidates) {
        Some(query) => query,
        None => {
            return Ok(SearchMemorySuggestionsResult {
                library_suggestions: Vec::new(),
                raw_suggestions: Vec::new(),
            });
        }
    };

    let resolved_project_id = match normalize_optional_string(input.project_id) {
        Some(project_id) => Some(project_id),
        None => resolve_project_id_from_session(&conn, &session_id)?,
    };
    let excluded_library_ids = match resolved_project_id.as_ref() {
        Some(project_id) => list_project_memory_library_ids(&conn, project_id)?,
        None => Vec::new(),
    };
    let limit = input.limit.unwrap_or(6).clamp(1, 10);
    let library_limit = limit.min(4);
    let raw_limit = limit.min(6);

    let library_suggestions = search_library_suggestions(
        &conn,
        &session_id,
        &excluded_library_ids,
        &match_query,
        &candidates,
        library_limit,
    )?;
    let raw_suggestions = search_raw_memory_suggestions(
        &conn,
        &session_id,
        resolved_project_id.as_ref(),
        &input.draft_text,
        &candidates,
        raw_limit,
    )?;

    Ok(SearchMemorySuggestionsResult {
        library_suggestions,
        raw_suggestions,
    })
}

/// 记录当前会话已经实际引用并发送过的记忆，防止后续自动重复推荐。
///
/// 用途：在用户消息发送成功后持久化会话级去重历史。
/// 参数：会话 ID、消息 ID、已引用记忆列表。
/// 返回：无。
#[tauri::command]
pub fn record_session_memory_references(
    input: RecordSessionMemoryReferencesInput,
) -> Result<(), String> {
    if input.references.is_empty() {
        return Ok(());
    }

    let mut conn = open_db_connection().map_err(|error| error.to_string())?;
    let session_id = normalize_required_string(input.session_id, "会话 ID")?;
    let message_id = normalize_required_string(input.message_id, "消息 ID")?;
    let now = now_rfc3339();
    let tx = conn.transaction().map_err(|error| error.to_string())?;

    for reference in input.references {
        tx.execute(
            r#"
            INSERT INTO session_memory_reference_history (
                session_id,
                source_type,
                source_id,
                message_id,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(session_id, source_type, source_id) DO UPDATE SET
                message_id = excluded.message_id,
                created_at = excluded.created_at
            "#,
            params![
                &session_id,
                match reference.source_type {
                    MemorySuggestionSourceType::LibraryChunk => "library_chunk",
                    MemorySuggestionSourceType::RawRecord => "raw_record",
                },
                normalize_required_string(reference.source_id, "记忆来源 ID")?,
                &message_id,
                &now
            ],
        )
        .map_err(|error| error.to_string())?;
    }

    tx.commit().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_memory_merge_runs(
    query: ListMemoryMergeRunsQuery,
) -> Result<Vec<MemoryMergeRun>, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
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
    let mut conn = open_db_connection().map_err(|error| error.to_string())?;
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
    sync_library_chunks(&tx, &input.library_id, &merged_content_md, &now)?;

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

    let conn = open_db_connection().map_err(|error| error.to_string())?;
    Ok(MergeRawMemoriesIntoLibraryResult {
        library: get_memory_library_by_id(&conn, &input.library_id)?,
        merge_run: get_memory_merge_run_by_id(&conn, &merge_run_id)?,
    })
}
