use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

static ACTIVE_TIMERS: Lazy<Arc<RwLock<HashMap<String, JoinHandle<()>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// 启动后台调度器循环
pub fn start_scheduler(app_handle: AppHandle) {
    tokio::spawn(async move {
        loop {
            // 每 60 秒检查一次待执行计划
            tokio::time::sleep(Duration::from_secs(60)).await;

            if let Err(e) = check_and_trigger_scheduled_plans(&app_handle).await {
                eprintln!("Failed to check scheduled plans: {}", e);
            }
        }
    });
}

/// 恢复待执行的定时计划
pub async fn restore_scheduled_plans(app_handle: &AppHandle) {
    let db_path = match crate::commands::support::get_db_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Failed to get db path: {}", e);
            return;
        }
    };

    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to open database: {}", e);
            return;
        }
    };

    // 查询所有待执行的定时计划
    let scheduled_plans: Vec<(String, String)> = match conn
        .prepare(
            "SELECT id, scheduled_at FROM plans WHERE schedule_status = 'scheduled' AND scheduled_at IS NOT NULL",
        )
        .and_then(|mut stmt| {
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            rows.collect::<Result<Vec<_>, _>>()
        }) {
        Ok(plans) => plans,
        Err(e) => {
            eprintln!("Failed to query scheduled plans: {}", e);
            return;
        }
    };

    println!("Found {} scheduled plans to restore", scheduled_plans.len());

    let now = Utc::now();

    for (plan_id, scheduled_at_str) in scheduled_plans {
        match scheduled_at_str.parse::<DateTime<Utc>>() {
            Ok(scheduled_at) => {
                if scheduled_at <= now {
                    // 时间已过，立即触发
                    println!("Triggering overdue plan: {}", plan_id);
                    if let Err(e) = trigger_plan_execution(app_handle, &plan_id).await {
                        eprintln!("Failed to trigger plan {}: {}", plan_id, e);
                    }
                } else {
                    // 注册定时器
                    register_plan_timer(app_handle.clone(), &plan_id, scheduled_at).await;
                }
            }
            Err(e) => {
                eprintln!("Failed to parse scheduled_at for plan {}: {}", plan_id, e);
            }
        }
    }
}

/// 检查并触发到期的定时计划
async fn check_and_trigger_scheduled_plans(
    app_handle: &AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = crate::commands::support::get_db_path()?;
    let conn = rusqlite::Connection::open(&db_path)?;

    let now = Utc::now();
    let now_str = now.to_rfc3339();

    let due_plans: Vec<String> = conn
        .prepare(
            "SELECT id FROM plans WHERE schedule_status = 'scheduled' AND scheduled_at IS NOT NULL AND scheduled_at <= ?1",
        )?
        .query_map([&now_str], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    for plan_id in due_plans {
        // 检查是否已有定时器在运行
        let timers = ACTIVE_TIMERS.read().await;
        if timers.contains_key(&plan_id) {
            continue;
        }
        drop(timers);

        println!("Triggering scheduled plan: {}", plan_id);
        trigger_plan_execution(app_handle, &plan_id).await?;
    }

    Ok(())
}

/// 注册单个计划定时器
pub async fn register_plan_timer(
    app_handle: AppHandle,
    plan_id: &str,
    scheduled_at: DateTime<Utc>,
) {
    let now = Utc::now();
    let delay = scheduled_at - now;

    if delay.num_seconds() <= 0 {
        // 时间已过，立即触发
        println!("Plan {} is overdue, triggering immediately", plan_id);
        if let Err(e) = trigger_plan_execution(&app_handle, plan_id).await {
            eprintln!("Failed to trigger plan {}: {}", plan_id, e);
        }
        return;
    }

    let plan_id_owned = plan_id.to_string();
    let duration = Duration::from_secs(delay.num_seconds() as u64);

    println!(
        "Registering timer for plan {} to trigger in {} seconds",
        plan_id,
        delay.num_seconds()
    );

    let handle = tokio::spawn(async move {
        tokio::time::sleep(duration).await;

        println!("Timer triggered for plan {}", plan_id_owned);
        if let Err(e) = trigger_plan_execution(&app_handle, &plan_id_owned).await {
            eprintln!("Failed to trigger plan {}: {}", plan_id_owned, e);
        }

        // 从活动定时器中移除
        let mut timers = ACTIVE_TIMERS.write().await;
        timers.remove(&plan_id_owned);
    });

    // 存储定时器句柄
    let mut timers = ACTIVE_TIMERS.write().await;
    timers.insert(plan_id.to_string(), handle);
}

/// 触发计划执行
async fn trigger_plan_execution(
    app_handle: &AppHandle,
    plan_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = crate::commands::support::get_db_path()?;
    let conn = rusqlite::Connection::open(&db_path)?;

    let now = Utc::now().to_rfc3339();

    // 1. 更新计划状态为 executing
    conn.execute(
        "UPDATE plans SET schedule_status = 'triggered', status = 'executing', execution_status = 'running', updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, &plan_id],
    )?;

    println!("Plan {} status updated to executing", plan_id);

    // 2. 将所有 pending 状态的任务更新为 in_progress
    let updated_tasks = conn.execute(
        "UPDATE tasks SET status = 'in_progress', updated_at = ?1 WHERE plan_id = ?2 AND status = 'pending'",
        rusqlite::params![&now, &plan_id],
    )?;

    println!(
        "Updated {} tasks to in_progress for plan {}",
        updated_tasks, plan_id
    );

    // 3. 发送事件通知前端
    app_handle.emit("plan:scheduled-trigger", plan_id)?;

    println!("Emitted plan:scheduled-trigger event for plan {}", plan_id);

    Ok(())
}
