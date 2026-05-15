use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use crate::commands::support::{
    now_rfc3339, open_db_connection,
};

use super::constants::{
    AUTH_MODE_ALLOW_ALL, LOGIN_STATUS_CONNECTED, REPLY_STYLE_FINAL_ONLY, RUNTIME_STATUS_IDLE,
};
use super::types::{
    CreateUnattendedChannelInput, ListUnattendedEventsInput, RecordUnattendedEventInput,
    RuntimeStatusSummary, UnattendedChannel, UnattendedChannelAccount, UnattendedEventRecord,
    UnattendedThread, UpdateUnattendedChannelInput, UpdateUnattendedThreadContextInput,
    WeixinLoginStatus,
};

fn bool_to_int(value: bool) -> i32 {
    if value {
        1
    } else {
        0
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn map_channel(row: &rusqlite::Row<'_>) -> rusqlite::Result<UnattendedChannel> {
    Ok(UnattendedChannel {
        id: row.get(0)?,
        channel_type: row.get(1)?,
        name: row.get(2)?,
        enabled: row.get::<_, i32>(3)? != 0,
        default_project_id: row.get(4)?,
        default_agent_id: row.get(5)?,
        default_model_id: row.get(6)?,
        reply_style: row.get(7)?,
        allow_all_senders: row.get::<_, i32>(8)? != 0,
        future_auth_mode: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn map_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<UnattendedChannelAccount> {
    Ok(UnattendedChannelAccount {
        id: row.get(0)?,
        channel_id: row.get(1)?,
        account_id: row.get(2)?,
        user_id: row.get(3)?,
        base_url: row.get(4)?,
        bot_token: row.get(5)?,
        sync_cursor: row.get(6)?,
        login_status: row.get(7)?,
        runtime_status: row.get(8)?,
        last_connected_at: row.get(9)?,
        last_error: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn map_thread(row: &rusqlite::Row<'_>) -> rusqlite::Result<UnattendedThread> {
    Ok(UnattendedThread {
        id: row.get(0)?,
        channel_account_id: row.get(1)?,
        peer_id: row.get(2)?,
        peer_name_snapshot: row.get(3)?,
        session_id: row.get(4)?,
        active_project_id: row.get(5)?,
        active_agent_id: row.get(6)?,
        active_model_id: row.get(7)?,
        last_context_token: row.get(8)?,
        last_plan_id: row.get(9)?,
        last_task_id: row.get(10)?,
        last_message_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn map_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<UnattendedEventRecord> {
    Ok(UnattendedEventRecord {
        id: row.get(0)?,
        channel_account_id: row.get(1)?,
        thread_id: row.get(2)?,
        direction: row.get(3)?,
        event_type: row.get(4)?,
        status: row.get(5)?,
        summary: row.get(6)?,
        payload_json: row.get(7)?,
        correlation_id: row.get(8)?,
        created_at: row.get(9)?,
    })
}

/// 列出无人值守渠道配置。
pub fn list_channels() -> Result<Vec<UnattendedChannel>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, channel_type, name, enabled, default_project_id, default_agent_id,
                    default_model_id, reply_style, allow_all_senders, future_auth_mode, created_at, updated_at
             FROM unattended_channels
             ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let channels = stmt
        .query_map([], map_channel)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(channels)
}

/// 创建无人值守渠道配置。
pub fn create_channel(input: CreateUnattendedChannelInput) -> Result<UnattendedChannel, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let enabled = input.enabled.unwrap_or(true);
    let allow_all_senders = input.allow_all_senders.unwrap_or(true);
    let reply_style = input
        .reply_style
        .unwrap_or_else(|| REPLY_STYLE_FINAL_ONLY.to_string());

    conn.execute(
        "INSERT INTO unattended_channels
         (id, channel_type, name, enabled, default_project_id, default_agent_id,
          default_model_id, reply_style, allow_all_senders, future_auth_mode, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            &id,
            &input.channel_type,
            &input.name,
            bool_to_int(enabled),
            &input.default_project_id,
            &input.default_agent_id,
            &input.default_model_id,
            &reply_style,
            bool_to_int(allow_all_senders),
            AUTH_MODE_ALLOW_ALL,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    get_channel(&id)
}

/// 更新无人值守渠道配置。
pub fn update_channel(
    id: String,
    input: UpdateUnattendedChannelInput,
) -> Result<UnattendedChannel, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let current = get_channel(&id)?;
    let now = now_rfc3339();
    let next_default_project_id = if input.default_project_id.is_some() {
        normalize_optional_text(input.default_project_id)
    } else {
        current.default_project_id
    };
    let next_default_agent_id = if input.default_agent_id.is_some() {
        normalize_optional_text(input.default_agent_id)
    } else {
        current.default_agent_id
    };
    let next_default_model_id = if input.default_model_id.is_some() {
        normalize_optional_text(input.default_model_id)
    } else {
        current.default_model_id
    };

    conn.execute(
        "UPDATE unattended_channels
         SET name = ?1,
             enabled = ?2,
             default_project_id = ?3,
             default_agent_id = ?4,
             default_model_id = ?5,
             reply_style = ?6,
             allow_all_senders = ?7,
             updated_at = ?8
         WHERE id = ?9",
        params![
            input.name.unwrap_or(current.name),
            bool_to_int(input.enabled.unwrap_or(current.enabled)),
            next_default_project_id,
            next_default_agent_id,
            next_default_model_id,
            input.reply_style.unwrap_or(current.reply_style),
            bool_to_int(input.allow_all_senders.unwrap_or(current.allow_all_senders)),
            &now,
            &id
        ],
    )
    .map_err(|e| e.to_string())?;

    get_channel(&id)
}

/// 删除无人值守渠道配置。
pub fn delete_channel(id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM unattended_channels WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 获取单个无人值守渠道。
pub fn get_channel(id: &str) -> Result<UnattendedChannel, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, channel_type, name, enabled, default_project_id, default_agent_id,
                default_model_id, reply_style, allow_all_senders, future_auth_mode, created_at, updated_at
         FROM unattended_channels WHERE id = ?1",
        [id],
        map_channel,
    )
    .map_err(|e| e.to_string())
}

/// 按渠道列出账号。
pub fn list_accounts(channel_id: Option<String>) -> Result<Vec<UnattendedChannelAccount>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let sql = if channel_id.is_some() {
        "SELECT id, channel_id, account_id, user_id, base_url, bot_token, sync_cursor,
                login_status, runtime_status, last_connected_at, last_error, created_at, updated_at
         FROM unattended_channel_accounts
         WHERE channel_id = ?1
         ORDER BY updated_at DESC"
    } else {
        "SELECT id, channel_id, account_id, user_id, base_url, bot_token, sync_cursor,
                login_status, runtime_status, last_connected_at, last_error, created_at, updated_at
         FROM unattended_channel_accounts
         ORDER BY updated_at DESC"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let rows = if let Some(channel_id) = channel_id {
        stmt.query_map([channel_id], map_account)
    } else {
        stmt.query_map([], map_account)
    }
    .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// 获取单个账号。
pub fn get_account(account_row_id: &str) -> Result<UnattendedChannelAccount, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, channel_id, account_id, user_id, base_url, bot_token, sync_cursor,
                login_status, runtime_status, last_connected_at, last_error, created_at, updated_at
         FROM unattended_channel_accounts WHERE id = ?1",
        [account_row_id],
        map_account,
    )
    .map_err(|e| e.to_string())
}

/// 通过登录状态更新或创建账号。
pub fn upsert_weixin_account(
    channel_id: &str,
    login_status: &WeixinLoginStatus,
) -> Result<UnattendedChannelAccount, String> {
    let mut conn = open_db_connection().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let now = now_rfc3339();

    let account_id = login_status
        .account_id
        .clone()
        .ok_or_else(|| "缺少 account_id".to_string())?;
    let user_id = login_status.user_id.clone();
    let base_url = login_status
        .base_url
        .clone()
        .unwrap_or_else(|| super::constants::DEFAULT_WEIXIN_BASE_URL.to_string());
    let bot_token = login_status
        .bot_token
        .clone()
        .ok_or_else(|| "缺少 bot_token".to_string())?;

    let existing: Option<String> = tx
        .query_row(
            "SELECT id FROM unattended_channel_accounts WHERE channel_id = ?1 AND account_id = ?2",
            params![channel_id, &account_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let row_id = existing.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    tx.execute(
        "INSERT INTO unattended_channel_accounts
         (id, channel_id, account_id, user_id, base_url, bot_token, sync_cursor, login_status,
          runtime_status, last_connected_at, last_error, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, COALESCE((SELECT sync_cursor FROM unattended_channel_accounts WHERE id = ?1), NULL),
                 ?7, COALESCE((SELECT runtime_status FROM unattended_channel_accounts WHERE id = ?1), ?8),
                 ?9, NULL, COALESCE((SELECT created_at FROM unattended_channel_accounts WHERE id = ?1), ?10), ?11)
         ON CONFLICT(id) DO UPDATE SET
            user_id = excluded.user_id,
            base_url = excluded.base_url,
            bot_token = excluded.bot_token,
            login_status = excluded.login_status,
            last_connected_at = excluded.last_connected_at,
            last_error = NULL,
            updated_at = excluded.updated_at",
        params![
            &row_id,
            channel_id,
            &account_id,
            &user_id,
            &base_url,
            &bot_token,
            LOGIN_STATUS_CONNECTED,
            RUNTIME_STATUS_IDLE,
            &now,
            &now,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    get_account(&row_id)
}

/// 删除无人值守账号。
pub fn delete_account(account_row_id: &str) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM unattended_channel_accounts WHERE id = ?1",
        [account_row_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 更新账号运行状态。
pub fn update_account_runtime_status(
    account_row_id: &str,
    runtime_status: &str,
    last_error: Option<&str>,
) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE unattended_channel_accounts
         SET runtime_status = ?1, last_error = ?2, updated_at = ?3
         WHERE id = ?4",
        params![runtime_status, last_error, now_rfc3339(), account_row_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 更新账号同步游标。
pub fn update_account_sync_cursor(
    account_row_id: &str,
    sync_cursor: Option<&str>,
) -> Result<(), String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE unattended_channel_accounts
         SET sync_cursor = ?1, updated_at = ?2
         WHERE id = ?3",
        params![sync_cursor, now_rfc3339(), account_row_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 查询运行时状态。
pub fn list_runtime_status(
    channel_id: Option<String>,
) -> Result<Vec<RuntimeStatusSummary>, String> {
    let accounts = list_accounts(channel_id)?;
    Ok(accounts
        .into_iter()
        .map(|account| RuntimeStatusSummary {
            account_id: account.account_id,
            channel_account_id: account.id,
            runtime_status: account.runtime_status,
            last_error: account.last_error,
        })
        .collect())
}

/// 根据账号与用户获取或创建线程。
pub fn upsert_thread(
    channel_account_id: &str,
    peer_id: &str,
    peer_name_snapshot: Option<&str>,
    context_token: Option<&str>,
) -> Result<UnattendedThread, String> {
    let mut conn = open_db_connection().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let now = now_rfc3339();

    let existing = tx
        .query_row(
            "SELECT id, channel_account_id, peer_id, peer_name_snapshot, session_id, active_project_id,
                    active_agent_id, active_model_id, last_context_token, last_plan_id, last_task_id, last_message_at,
                    created_at, updated_at
             FROM unattended_threads
             WHERE channel_account_id = ?1 AND peer_id = ?2",
            params![channel_account_id, peer_id],
            map_thread,
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let thread = if let Some(existing) = existing {
        tx.execute(
            "UPDATE unattended_threads
             SET peer_name_snapshot = COALESCE(?1, peer_name_snapshot),
                 last_context_token = COALESCE(?2, last_context_token),
                 last_message_at = ?3,
                 updated_at = ?4
             WHERE id = ?5",
            params![peer_name_snapshot, context_token, &now, &now, &existing.id],
        )
        .map_err(|e| e.to_string())?;
        UnattendedThread {
            peer_name_snapshot: peer_name_snapshot
                .map(|value| value.to_string())
                .or(existing.peer_name_snapshot),
            last_context_token: context_token
                .map(|value| value.to_string())
                .or(existing.last_context_token),
            last_message_at: Some(now.clone()),
            updated_at: now.clone(),
            ..existing
        }
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO unattended_threads
             (id, channel_account_id, peer_id, peer_name_snapshot, session_id, active_project_id, active_agent_id,
              active_model_id, last_context_token, last_plan_id, last_task_id, last_message_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, NULL, ?5, NULL, NULL, ?6, ?7, ?8)",
            params![&id, channel_account_id, peer_id, peer_name_snapshot, context_token, &now, &now, &now],
        )
        .map_err(|e| e.to_string())?;
        UnattendedThread {
            id,
            channel_account_id: channel_account_id.to_string(),
            peer_id: peer_id.to_string(),
            peer_name_snapshot: peer_name_snapshot.map(|value| value.to_string()),
            session_id: None,
            active_project_id: None,
            active_agent_id: None,
            active_model_id: None,
            last_context_token: context_token.map(|value| value.to_string()),
            last_plan_id: None,
            last_task_id: None,
            last_message_at: Some(now.clone()),
            created_at: now.clone(),
            updated_at: now,
        }
    };

    tx.commit().map_err(|e| e.to_string())?;
    Ok(thread)
}

/// 列出线程。
pub fn list_threads(channel_id: Option<String>) -> Result<Vec<UnattendedThread>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let sql = if channel_id.is_some() {
        "SELECT t.id, t.channel_account_id, t.peer_id, t.peer_name_snapshot, t.session_id,
                t.active_project_id, t.active_agent_id, t.active_model_id, t.last_context_token, t.last_plan_id,
                t.last_task_id, t.last_message_at, t.created_at, t.updated_at
         FROM unattended_threads t
         INNER JOIN unattended_channel_accounts a ON a.id = t.channel_account_id
         WHERE a.channel_id = ?1
         ORDER BY t.updated_at DESC"
    } else {
        "SELECT id, channel_account_id, peer_id, peer_name_snapshot, session_id,
                active_project_id, active_agent_id, active_model_id, last_context_token, last_plan_id,
                last_task_id, last_message_at, created_at, updated_at
         FROM unattended_threads
         ORDER BY updated_at DESC"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = if let Some(channel_id) = channel_id {
        stmt.query_map([channel_id], map_thread)
    } else {
        stmt.query_map([], map_thread)
    }
    .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// 更新线程上下文。
pub fn update_thread_context(
    thread_id: &str,
    input: UpdateUnattendedThreadContextInput,
) -> Result<UnattendedThread, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE unattended_threads
         SET session_id = COALESCE(?1, session_id),
             active_project_id = COALESCE(?2, active_project_id),
             active_agent_id = COALESCE(?3, active_agent_id),
             active_model_id = COALESCE(?4, active_model_id),
             last_context_token = COALESCE(?5, last_context_token),
             last_plan_id = COALESCE(?6, last_plan_id),
             last_task_id = COALESCE(?7, last_task_id),
             updated_at = ?8
         WHERE id = ?9",
        params![
            input.session_id,
            input.active_project_id,
            input.active_agent_id,
            input.active_model_id,
            input.last_context_token,
            input.last_plan_id,
            input.last_task_id,
            now_rfc3339(),
            thread_id
        ],
    )
    .map_err(|e| e.to_string())?;

    get_thread(thread_id)
}

/// 获取单个线程。
pub fn get_thread(thread_id: &str) -> Result<UnattendedThread, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, channel_account_id, peer_id, peer_name_snapshot, session_id, active_project_id, active_agent_id,
                active_model_id, last_context_token, last_plan_id, last_task_id, last_message_at,
                created_at, updated_at
         FROM unattended_threads WHERE id = ?1",
        [thread_id],
        map_thread,
    )
    .map_err(|e| e.to_string())
}

/// 记录无人值守审计事件。
pub fn record_event(input: RecordUnattendedEventInput) -> Result<UnattendedEventRecord, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let status = input.status.unwrap_or_else(|| "success".to_string());

    conn.execute(
        "INSERT INTO unattended_events
         (id, channel_account_id, thread_id, direction, event_type, status, summary,
          payload_json, correlation_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            &id,
            input.channel_account_id,
            input.thread_id,
            &input.direction,
            &input.event_type,
            &status,
            input.summary,
            input.payload_json,
            input.correlation_id,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(UnattendedEventRecord {
        id,
        channel_account_id: input.channel_account_id,
        thread_id: input.thread_id,
        direction: input.direction,
        event_type: input.event_type,
        status,
        summary: input.summary,
        payload_json: input.payload_json,
        correlation_id: input.correlation_id,
        created_at: now,
    })
}

/// 列出审计事件。
pub fn list_events(
    input: Option<ListUnattendedEventsInput>,
) -> Result<Vec<UnattendedEventRecord>, String> {
    let conn = open_db_connection().map_err(|e| e.to_string())?;
    let filter = input.unwrap_or(ListUnattendedEventsInput {
        channel_account_id: None,
        thread_id: None,
        event_type: None,
        limit: Some(200),
    });
    let limit = filter.limit.unwrap_or(200).clamp(1, 1000) as i64;
    let mut sql = String::from(
        "SELECT id, channel_account_id, thread_id, direction, event_type, status, summary,
                payload_json, correlation_id, created_at
         FROM unattended_events
         WHERE 1 = 1",
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(channel_account_id) = filter.channel_account_id {
        sql.push_str(&format!(
            " AND channel_account_id = ?{}",
            params_vec.len() + 1
        ));
        params_vec.push(Box::new(channel_account_id));
    }
    if let Some(thread_id) = filter.thread_id {
        sql.push_str(&format!(" AND thread_id = ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(thread_id));
    }
    if let Some(event_type) = filter.event_type {
        sql.push_str(&format!(" AND event_type = ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(event_type));
    }
    sql.push_str(&format!(
        " ORDER BY created_at DESC LIMIT ?{}",
        params_vec.len() + 1
    ));
    params_vec.push(Box::new(limit));

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let refs = params_vec
        .iter()
        .map(|value| value.as_ref() as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();

    let events = stmt
        .query_map(refs.as_slice(), map_event)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(events)
}
