use anyhow::Result;
use rusqlite::{CachedStatement, Connection, ToSql};
use std::path::PathBuf;

pub fn get_db_path() -> Result<PathBuf> {
    let persistence_dir = super::get_persistence_dir_path()?;
    Ok(persistence_dir.join("data").join("easy-agent.db"))
}

pub fn open_db_connection() -> Result<Connection> {
    let db_path = get_db_path()?;
    Ok(Connection::open(&db_path)?)
}

pub fn open_db_connection_with_foreign_keys() -> Result<Connection> {
    let conn = open_db_connection()?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    Ok(conn)
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
