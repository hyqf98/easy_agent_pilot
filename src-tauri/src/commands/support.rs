use anyhow::Result;
use rusqlite::{CachedStatement, Connection, ToSql};
use std::path::PathBuf;
use std::time::Duration;

pub const RAW_MEMORY_FTS_TABLE_SQL: &str = r#"
    CREATE VIRTUAL TABLE IF NOT EXISTS raw_memory_records_fts USING fts5(
        content,
        tokenize = 'trigram',
        content = 'raw_memory_records',
        content_rowid = 'rowid'
    );
"#;

pub const MEMORY_CHUNKS_FTS_TABLE_SQL: &str = r#"
    CREATE VIRTUAL TABLE IF NOT EXISTS memory_library_chunks_fts USING fts5(
        chunk_text,
        tokenize = 'trigram',
        content = 'memory_library_chunks',
        content_rowid = 'rowid'
    );
"#;

pub const MEMORY_SEARCH_TRIGGERS_SQL: &[&str] = &[
    r#"
    CREATE TRIGGER IF NOT EXISTS raw_memory_records_ai
    AFTER INSERT ON raw_memory_records
    BEGIN
        INSERT INTO raw_memory_records_fts(rowid, content)
        VALUES (new.rowid, new.content);
    END;
    "#,
    r#"
    CREATE TRIGGER IF NOT EXISTS raw_memory_records_ad
    AFTER DELETE ON raw_memory_records
    BEGIN
        INSERT INTO raw_memory_records_fts(raw_memory_records_fts, rowid, content)
        VALUES ('delete', old.rowid, old.content);
    END;
    "#,
    r#"
    CREATE TRIGGER IF NOT EXISTS raw_memory_records_au
    AFTER UPDATE ON raw_memory_records
    BEGIN
        INSERT INTO raw_memory_records_fts(raw_memory_records_fts, rowid, content)
        VALUES ('delete', old.rowid, old.content);
        INSERT INTO raw_memory_records_fts(rowid, content)
        VALUES (new.rowid, new.content);
    END;
    "#,
    r#"
    CREATE TRIGGER IF NOT EXISTS memory_library_chunks_ai
    AFTER INSERT ON memory_library_chunks
    BEGIN
        INSERT INTO memory_library_chunks_fts(rowid, chunk_text)
        VALUES (new.rowid, new.chunk_text);
    END;
    "#,
    r#"
    CREATE TRIGGER IF NOT EXISTS memory_library_chunks_ad
    AFTER DELETE ON memory_library_chunks
    BEGIN
        INSERT INTO memory_library_chunks_fts(memory_library_chunks_fts, rowid, chunk_text)
        VALUES ('delete', old.rowid, old.chunk_text);
    END;
    "#,
    r#"
    CREATE TRIGGER IF NOT EXISTS memory_library_chunks_au
    AFTER UPDATE ON memory_library_chunks
    BEGIN
        INSERT INTO memory_library_chunks_fts(memory_library_chunks_fts, rowid, chunk_text)
        VALUES ('delete', old.rowid, old.chunk_text);
        INSERT INTO memory_library_chunks_fts(rowid, chunk_text)
        VALUES (new.rowid, new.chunk_text);
    END;
    "#,
];

pub fn get_db_path() -> Result<PathBuf> {
    let persistence_dir = super::get_persistence_dir_path()?;
    Ok(persistence_dir.join("data").join("easy-agent.db"))
}

pub fn open_db_connection() -> Result<Connection> {
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA journal_mode = WAL")?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    Ok(conn)
}

fn sqlite_object_exists(conn: &Connection, object_type: &str, name: &str) -> Result<bool> {
    let count = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = ?1 AND name = ?2",
        [object_type, name],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(count > 0)
}

/// 重建记忆模块的派生搜索结构。
///
/// `raw_memory_records_fts` 和 `memory_library_chunks_fts` 都是由源表派生出的 FTS 虚表，
/// 即便损坏也可以通过删除并重建恢复，不会丢失原始业务数据。
pub fn repair_memory_search_indexes(conn: &Connection) -> Result<()> {
    let raw_memory_exists = sqlite_object_exists(conn, "table", "raw_memory_records")?;
    let memory_chunks_exists = sqlite_object_exists(conn, "table", "memory_library_chunks")?;

    conn.execute_batch(
        r#"
        DROP TRIGGER IF EXISTS raw_memory_records_ai;
        DROP TRIGGER IF EXISTS raw_memory_records_ad;
        DROP TRIGGER IF EXISTS raw_memory_records_au;
        DROP TABLE IF EXISTS raw_memory_records_fts;

        DROP TRIGGER IF EXISTS memory_library_chunks_ai;
        DROP TRIGGER IF EXISTS memory_library_chunks_ad;
        DROP TRIGGER IF EXISTS memory_library_chunks_au;
        DROP TABLE IF EXISTS memory_library_chunks_fts;
        "#,
    )?;

    if raw_memory_exists {
        conn.execute_batch(&format!(
            "{}\n{}\n{}\n{}",
            RAW_MEMORY_FTS_TABLE_SQL,
            MEMORY_SEARCH_TRIGGERS_SQL[0],
            MEMORY_SEARCH_TRIGGERS_SQL[1],
            MEMORY_SEARCH_TRIGGERS_SQL[2],
        ))?;
        conn.execute_batch("REINDEX raw_memory_records;")?;
        conn.execute(
            "INSERT INTO raw_memory_records_fts(raw_memory_records_fts) VALUES('rebuild')",
            [],
        )?;
    }

    if memory_chunks_exists {
        conn.execute_batch(&format!(
            "{}\n{}\n{}\n{}",
            MEMORY_CHUNKS_FTS_TABLE_SQL,
            MEMORY_SEARCH_TRIGGERS_SQL[3],
            MEMORY_SEARCH_TRIGGERS_SQL[4],
            MEMORY_SEARCH_TRIGGERS_SQL[5],
        ))?;
        conn.execute_batch("REINDEX memory_library_chunks;")?;
        conn.execute(
            "INSERT INTO memory_library_chunks_fts(memory_library_chunks_fts) VALUES('rebuild')",
            [],
        )?;
    }

    Ok(())
}

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn bool_from_int(value: Option<i32>) -> Option<bool> {
    value.map(|flag| flag != 0)
}

pub struct UpdateSqlBuilder {
    updates: Vec<String>,
    next_param_index: usize,
}

impl UpdateSqlBuilder {
    pub fn new() -> Self {
        Self {
            updates: vec!["updated_at = ?1".to_string()],
            next_param_index: 2,
        }
    }

    pub fn push(&mut self, column: &str, include: bool) {
        if include {
            self.updates
                .push(format!("{} = ?{}", column, self.next_param_index));
            self.next_param_index += 1;
        }
    }

    pub fn finish(&self, table: &str, id_column: &str) -> String {
        format!(
            "UPDATE {} SET {} WHERE {} = ?{}",
            table,
            self.updates.join(", "),
            id_column,
            self.next_param_index
        )
    }
}

pub fn bind_value<T: ToSql + ?Sized>(
    stmt: &mut CachedStatement<'_>,
    param_index: &mut usize,
    value: &T,
) -> rusqlite::Result<()> {
    stmt.raw_bind_parameter(*param_index, value)?;
    *param_index += 1;
    Ok(())
}

pub fn bind_optional<T: ToSql>(
    stmt: &mut CachedStatement<'_>,
    param_index: &mut usize,
    value: &Option<T>,
) -> rusqlite::Result<()> {
    if let Some(value) = value {
        bind_value(stmt, param_index, value)?;
    }
    Ok(())
}

pub fn bind_optional_mapped<T, U, F>(
    stmt: &mut CachedStatement<'_>,
    param_index: &mut usize,
    value: &Option<T>,
    mapper: F,
) -> rusqlite::Result<()>
where
    U: ToSql,
    F: FnOnce(&T) -> U,
{
    if let Some(value) = value {
        stmt.raw_bind_parameter(*param_index, mapper(value))?;
        *param_index += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::repair_memory_search_indexes;
    use rusqlite::Connection;

    fn build_memory_test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.execute("PRAGMA foreign_keys = ON", [])
            .expect("enable foreign keys");
        conn.execute_batch(
            r#"
            CREATE TABLE projects (
                id TEXT PRIMARY KEY
            );

            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );

            CREATE TABLE raw_memory_records (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                project_id TEXT,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );

            CREATE TABLE memory_libraries (
                id TEXT PRIMARY KEY
            );

            CREATE TABLE memory_library_chunks (
                id TEXT PRIMARY KEY,
                library_id TEXT NOT NULL,
                chunk_text TEXT NOT NULL,
                chunk_order INTEGER NOT NULL DEFAULT 0,
                chunk_hash TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (library_id) REFERENCES memory_libraries(id) ON DELETE CASCADE
            );
            "#,
        )
        .expect("create test schema");
        conn
    }

    fn seed_memory_test_data(conn: &Connection) {
        conn.execute("INSERT INTO projects (id) VALUES ('project-1')", [])
            .expect("insert project");
        conn.execute(
            "INSERT INTO sessions (id, project_id) VALUES ('session-1', 'project-1')",
            [],
        )
        .expect("insert session");
        conn.execute("INSERT INTO memory_libraries (id) VALUES ('library-1')", [])
            .expect("insert memory library");
        conn.execute(
            r#"
            INSERT INTO raw_memory_records (id, session_id, project_id, content, created_at, updated_at)
            VALUES ('record-1', 'session-1', 'project-1', 'hello memory', 'now', 'now')
            "#,
            [],
        )
        .expect("insert raw memory record");
        conn.execute(
            r#"
            INSERT INTO memory_library_chunks (
                id, library_id, chunk_text, chunk_order, chunk_hash, created_at, updated_at
            ) VALUES ('chunk-1', 'library-1', 'hello chunk', 0, 'hash-1', 'now', 'now')
            "#,
            [],
        )
        .expect("insert memory chunk");
    }

    #[test]
    fn repair_memory_search_indexes_allows_raw_memory_delete() {
        let conn = build_memory_test_connection();
        repair_memory_search_indexes(&conn).expect("repair indexes");
        seed_memory_test_data(&conn);

        conn.execute("DELETE FROM raw_memory_records WHERE id = 'record-1'", [])
            .expect("delete raw memory record");

        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM raw_memory_records", [], |row| {
                row.get(0)
            })
            .expect("count remaining raw records");
        assert_eq!(remaining, 0);
    }

    #[test]
    fn repair_memory_search_indexes_allows_cascading_session_and_project_delete() {
        let session_conn = build_memory_test_connection();
        repair_memory_search_indexes(&session_conn).expect("repair indexes");
        seed_memory_test_data(&session_conn);

        session_conn
            .execute("DELETE FROM sessions WHERE id = 'session-1'", [])
            .expect("delete session");

        let session_raw_count: i64 = session_conn
            .query_row("SELECT COUNT(*) FROM raw_memory_records", [], |row| {
                row.get(0)
            })
            .expect("count raw records after session delete");
        assert_eq!(session_raw_count, 0);

        let project_conn = build_memory_test_connection();
        repair_memory_search_indexes(&project_conn).expect("repair indexes");
        seed_memory_test_data(&project_conn);

        project_conn
            .execute("DELETE FROM projects WHERE id = 'project-1'", [])
            .expect("delete project");

        let remaining_projects: i64 = project_conn
            .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
            .expect("count projects after delete");
        let remaining_sessions: i64 = project_conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .expect("count sessions after project delete");
        let remaining_raw_records: i64 = project_conn
            .query_row("SELECT COUNT(*) FROM raw_memory_records", [], |row| {
                row.get(0)
            })
            .expect("count raw records after project delete");

        assert_eq!(remaining_projects, 0);
        assert_eq!(remaining_sessions, 0);
        assert_eq!(remaining_raw_records, 0);
    }
}
