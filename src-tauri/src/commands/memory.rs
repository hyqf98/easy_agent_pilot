use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::support::{now_rfc3339, open_db_connection_with_foreign_keys};

// ==================== 类型定义 ====================

/// 记忆分类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCategory {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub order_index: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 用户记忆
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMemory {
    pub id: String,
    pub session_id: Option<String>,
    pub category_id: Option<String>,
    pub title: String,
    pub content: String,
    pub compressed_content: Option<String>,
    pub is_compressed: bool,
    pub source_type: String,
    pub source_message_ids: Option<String>,
    pub tags: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 记忆压缩历史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCompression {
    pub id: String,
    pub memory_id: String,
    pub original_content: String,
    pub compressed_content: String,
    pub compression_ratio: Option<f64>,
    pub model_id: Option<String>,
    pub created_at: String,
}

// ==================== 输入类型 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMemoryCategoryInput {
    pub parent_id: Option<String>,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub order_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMemoryCategoryInput {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub order_index: Option<i32>,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserMemoryInput {
    pub session_id: Option<String>,
    pub category_id: Option<String>,
    pub title: String,
    pub content: String,
    pub source_type: Option<String>,
    pub source_message_ids: Option<String>,
    pub tags: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserMemoryInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub compressed_content: Option<String>,
    pub is_compressed: Option<bool>,
    pub category_id: Option<String>,
    pub tags: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMemoryCompressionInput {
    pub memory_id: String,
    pub original_content: String,
    pub compressed_content: String,
    pub compression_ratio: Option<f64>,
    pub model_id: Option<String>,
}

// ==================== 查询参数类型 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMemoriesQuery {
    pub category_id: Option<String>,
    pub session_id: Option<String>,
    pub is_compressed: Option<bool>,
    pub source_type: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total: i32,
    pub compressed: i32,
    pub uncompressed: i32,
    pub by_category: Vec<(Option<String>, i32)>,
}

// ==================== 数据库辅助函数 ====================

/// 获取数据库连接
fn get_db_connection() -> Result<rusqlite::Connection> {
    open_db_connection_with_foreign_keys()
}

fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ==================== 记忆分类命令 ====================

/// 列出所有记忆分类
#[tauri::command]
pub fn list_memory_categories() -> Result<Vec<MemoryCategory>, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, parent_id, name, icon, color, description, order_index, created_at, updated_at
             FROM memory_categories
             ORDER BY order_index ASC, created_at ASC"
        )
        .map_err(|e| e.to_string())?;

    let categories = stmt
        .query_map([], |row| {
            Ok(MemoryCategory {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                name: row.get(2)?,
                icon: row.get(3)?,
                color: row.get(4)?,
                description: row.get(5)?,
                order_index: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(categories)
}

/// 获取单个记忆分类
#[tauri::command]
pub fn get_memory_category(id: String) -> Result<MemoryCategory, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;

    let category = conn
        .query_row(
            "SELECT id, parent_id, name, icon, color, description, order_index, created_at, updated_at
             FROM memory_categories WHERE id = ?",
            [&id],
            |row| {
                Ok(MemoryCategory {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    name: row.get(2)?,
                    icon: row.get(3)?,
                    color: row.get(4)?,
                    description: row.get(5)?,
                    order_index: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            }
        )
        .map_err(|e| e.to_string())?;

    Ok(category)
}

/// 创建记忆分类
#[tauri::command]
pub fn create_memory_category(input: CreateMemoryCategoryInput) -> Result<MemoryCategory, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;
    let now = now_rfc3339();
    let id = generate_id();

    conn.execute(
        "INSERT INTO memory_categories (id, parent_id, name, icon, color, description, order_index, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &id,
            &input.parent_id,
            &input.name,
            &input.icon,
            &input.color,
            &input.description,
            &input.order_index.unwrap_or(0),
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(MemoryCategory {
        id,
        parent_id: input.parent_id,
        name: input.name,
        icon: input.icon,
        color: input.color,
        description: input.description,
        order_index: input.order_index.unwrap_or(0),
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 更新记忆分类
#[tauri::command]
pub fn update_memory_category(
    id: String,
    input: UpdateMemoryCategoryInput,
) -> Result<MemoryCategory, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;
    let now = now_rfc3339();

    // 先获取现有记录
    let existing = conn
        .query_row(
            "SELECT id, parent_id, name, icon, color, description, order_index, created_at, updated_at
             FROM memory_categories WHERE id = ?",
            [&id],
            |row| {
                Ok(MemoryCategory {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    name: row.get(2)?,
                    icon: row.get(3)?,
                    color: row.get(4)?,
                    description: row.get(5)?,
                    order_index: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            }
        )
        .map_err(|e| e.to_string())?;

    let updated = MemoryCategory {
        id: existing.id,
        parent_id: input.parent_id.or(existing.parent_id),
        name: input.name.unwrap_or(existing.name),
        icon: input.icon.or(existing.icon),
        color: input.color.or(existing.color),
        description: input.description.or(existing.description),
        order_index: input.order_index.unwrap_or(existing.order_index),
        created_at: existing.created_at,
        updated_at: now,
    };

    conn.execute(
        "UPDATE memory_categories
         SET parent_id = ?1, name = ?2, icon = ?3, color = ?4, description = ?5, order_index = ?6, updated_at = ?7
         WHERE id = ?8",
        rusqlite::params![
            &updated.parent_id,
            &updated.name,
            &updated.icon,
            &updated.color,
            &updated.description,
            &updated.order_index,
            &updated.updated_at,
            &updated.id
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(updated)
}

/// 删除记忆分类
#[tauri::command]
pub fn delete_memory_category(id: String) -> Result<(), String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM memory_categories WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ==================== 用户记忆命令 ====================

/// 列出用户记忆（支持筛选）
#[tauri::command]
pub fn list_memories(query: ListMemoriesQuery) -> Result<Vec<UserMemory>, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;

    let mut sql = String::from(
        "SELECT id, session_id, category_id, title, content, compressed_content, is_compressed, source_type, source_message_ids, tags, metadata, created_at, updated_at
         FROM user_memories WHERE 1=1"
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(category_id) = &query.category_id {
        sql.push_str(" AND category_id = ?");
        params.push(Box::new(category_id.clone()));
    }

    if let Some(session_id) = &query.session_id {
        sql.push_str(" AND session_id = ?");
        params.push(Box::new(session_id.clone()));
    }

    if let Some(is_compressed) = query.is_compressed {
        sql.push_str(" AND is_compressed = ?");
        params.push(Box::new(if is_compressed { 1 } else { 0 }));
    }

    if let Some(source_type) = &query.source_type {
        sql.push_str(" AND source_type = ?");
        params.push(Box::new(source_type.clone()));
    }

    if let Some(search) = &query.search {
        sql.push_str(" AND (title LIKE ? OR content LIKE ?)");
        let search_pattern = format!("%{}%", search);
        params.push(Box::new(search_pattern.clone()));
        params.push(Box::new(search_pattern));
    }

    sql.push_str(" ORDER BY created_at DESC");

    if let Some(limit) = query.limit {
        sql.push_str(&format!(" LIMIT {}", limit));
        if let Some(offset) = query.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }
    }

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let memories = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(UserMemory {
                id: row.get(0)?,
                session_id: row.get(1)?,
                category_id: row.get(2)?,
                title: row.get(3)?,
                content: row.get(4)?,
                compressed_content: row.get(5)?,
                is_compressed: row.get::<_, i32>(6)? != 0,
                source_type: row.get(7)?,
                source_message_ids: row.get(8)?,
                tags: row.get(9)?,
                metadata: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(memories)
}

/// 获取单个用户记忆
#[tauri::command]
pub fn get_memory(id: String) -> Result<UserMemory, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;

    let memory = conn
        .query_row(
            "SELECT id, session_id, category_id, title, content, compressed_content, is_compressed, source_type, source_message_ids, tags, metadata, created_at, updated_at
             FROM user_memories WHERE id = ?",
            [&id],
            |row| {
                Ok(UserMemory {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    category_id: row.get(2)?,
                    title: row.get(3)?,
                    content: row.get(4)?,
                    compressed_content: row.get(5)?,
                    is_compressed: row.get::<_, i32>(6)? != 0,
                    source_type: row.get(7)?,
                    source_message_ids: row.get(8)?,
                    tags: row.get(9)?,
                    metadata: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            }
        )
        .map_err(|e| e.to_string())?;

    Ok(memory)
}

/// 创建用户记忆
#[tauri::command]
pub fn create_memory(input: CreateUserMemoryInput) -> Result<UserMemory, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;
    let now = now_rfc3339();
    let id = generate_id();
    let source_type = input.source_type.unwrap_or_else(|| "auto".to_string());
    let compressed_content: Option<String> = None;
    let is_compressed = 0;

    conn.execute(
        "INSERT INTO user_memories (id, session_id, category_id, title, content, compressed_content, is_compressed, source_type, source_message_ids, tags, metadata, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            &id,
            &input.session_id,
            &input.category_id,
            &input.title,
            &input.content,
            &compressed_content,
            &is_compressed,
            &source_type,
            &input.source_message_ids,
            &input.tags,
            &input.metadata,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(UserMemory {
        id,
        session_id: input.session_id,
        category_id: input.category_id,
        title: input.title,
        content: input.content,
        compressed_content,
        is_compressed: false,
        source_type,
        source_message_ids: input.source_message_ids,
        tags: input.tags,
        metadata: input.metadata,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 更新用户记忆
#[tauri::command]
pub fn update_memory(id: String, input: UpdateUserMemoryInput) -> Result<UserMemory, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;
    let now = now_rfc3339();

    // 先获取现有记录
    let existing = conn
        .query_row(
            "SELECT id, session_id, category_id, title, content, compressed_content, is_compressed, source_type, source_message_ids, tags, metadata, created_at, updated_at
             FROM user_memories WHERE id = ?",
            [&id],
            |row| {
                Ok(UserMemory {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    category_id: row.get(2)?,
                    title: row.get(3)?,
                    content: row.get(4)?,
                    compressed_content: row.get(5)?,
                    is_compressed: row.get::<_, i32>(6)? != 0,
                    source_type: row.get(7)?,
                    source_message_ids: row.get(8)?,
                    tags: row.get(9)?,
                    metadata: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            }
        )
        .map_err(|e| e.to_string())?;

    let is_compressed_int = if input.is_compressed.unwrap_or(existing.is_compressed) {
        1
    } else {
        0
    };

    let updated = UserMemory {
        id: existing.id,
        session_id: existing.session_id,
        category_id: input.category_id.or(existing.category_id),
        title: input.title.unwrap_or(existing.title),
        content: input.content.unwrap_or(existing.content),
        compressed_content: input.compressed_content.or(existing.compressed_content),
        is_compressed: input.is_compressed.unwrap_or(existing.is_compressed),
        source_type: existing.source_type,
        source_message_ids: existing.source_message_ids,
        tags: input.tags.or(existing.tags),
        metadata: input.metadata.or(existing.metadata),
        created_at: existing.created_at,
        updated_at: now,
    };

    conn.execute(
        "UPDATE user_memories
         SET title = ?1, content = ?2, compressed_content = ?3, is_compressed = ?4, category_id = ?5, tags = ?6, metadata = ?7, updated_at = ?8
         WHERE id = ?9",
        rusqlite::params![
            &updated.title,
            &updated.content,
            &updated.compressed_content,
            &is_compressed_int,
            &updated.category_id,
            &updated.tags,
            &updated.metadata,
            &updated.updated_at,
            &updated.id
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(updated)
}

/// 删除用户记忆
#[tauri::command]
pub fn delete_memory(id: String) -> Result<(), String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM user_memories WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 批量删除用户记忆
#[tauri::command]
pub fn batch_delete_memories(ids: Vec<String>) -> Result<(), String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;

    for id in ids {
        conn.execute("DELETE FROM user_memories WHERE id = ?1", [&id])
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 采集用户消息（自动从会话消息创建记忆)
#[tauri::command]
pub fn capture_user_message(
    session_id: String,
    message_id: String,
    title: String,
    content: String,
) -> Result<UserMemory, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;
    let now = now_rfc3339();
    let id = generate_id();
    let source_type = "auto";
    let source_message_ids =
        serde_json::to_string(&vec![message_id]).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO user_memories (id, session_id, category_id, title, content, compressed_content, is_compressed, source_type, source_message_ids, tags, metadata, created_at, updated_at)
         VALUES (?1, ?2, NULL, ?3, ?4, NULL, 0, ?5, ?6, NULL, NULL, ?7, ?8)",
        rusqlite::params![
            &id,
            &session_id,
            &title,
            &content,
            &source_type,
            &source_message_ids,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(UserMemory {
        id,
        session_id: Some(session_id),
        category_id: None,
        title,
        content,
        compressed_content: None,
        is_compressed: false,
        source_type: source_type.to_string(),
        source_message_ids: Some(source_message_ids),
        tags: None,
        metadata: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 获取记忆统计信息
#[tauri::command]
pub fn get_memory_stats() -> Result<MemoryStats, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;

    let total: i32 = conn
        .query_row("SELECT COUNT(*) FROM user_memories", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let compressed: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM user_memories WHERE is_compressed = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT category_id, COUNT(*) FROM user_memories GROUP BY category_id")
        .map_err(|e| e.to_string())?;

    let by_category = stmt
        .query_map([], |row| {
            let category_id: Option<String> = row.get(0)?;
            let count: i32 = row.get(1)?;
            Ok((category_id, count))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(MemoryStats {
        total,
        compressed,
        uncompressed: total - compressed,
        by_category,
    })
}

// ==================== 记忆压缩命令 ====================

/// 创建记忆压缩记录
#[tauri::command]
pub fn create_memory_compression(
    input: CreateMemoryCompressionInput,
) -> Result<MemoryCompression, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;
    let now = now_rfc3339();
    let id = generate_id();

    conn.execute(
        "INSERT INTO memory_compressions (id, memory_id, original_content, compressed_content, compression_ratio, model_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            &id,
            &input.memory_id,
            &input.original_content,
            &input.compressed_content,
            &input.compression_ratio,
            &input.model_id,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    // 更新记忆的压缩状态
    conn.execute(
        "UPDATE user_memories SET compressed_content = ?1, is_compressed = 1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![&input.compressed_content, &now, &input.memory_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(MemoryCompression {
        id,
        memory_id: input.memory_id,
        original_content: input.original_content,
        compressed_content: input.compressed_content,
        compression_ratio: input.compression_ratio,
        model_id: input.model_id,
        created_at: now,
    })
}

/// 获取记忆的压缩历史
#[tauri::command]
pub fn list_memory_compressions(memory_id: String) -> Result<Vec<MemoryCompression>, String> {
    let conn = get_db_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, memory_id, original_content, compressed_content, compression_ratio, model_id, created_at
             FROM memory_compressions WHERE memory_id = ?
             ORDER BY created_at DESC"
        )
        .map_err(|e| e.to_string())?;

    let compressions = stmt
        .query_map([&memory_id], |row| {
            Ok(MemoryCompression {
                id: row.get(0)?,
                memory_id: row.get(1)?,
                original_content: row.get(2)?,
                compressed_content: row.get(3)?,
                compression_ratio: row.get(4)?,
                model_id: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(compressions)
}
