use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::RwLock;

#[cfg(target_os = "windows")]
use crate::commands::cli_support::configure_windows_std_command;

// 全局中断状态存储
lazy_static::lazy_static! {
    static ref ABORT_FLAGS: Arc<RwLock<HashMap<String, Arc<AtomicBool>>>> = Arc::new(RwLock::new(HashMap::new()));
    // 存储会话 ID 到进程 PID 的映射
    static ref SESSION_PIDS: Arc<RwLock<HashMap<String, u32>>> = Arc::new(RwLock::new(HashMap::new()));
}

fn write_abort_log(level: &str, message: &str) {
    crate::logging::write_log(level, "abort", message);
}

/// 获取或创建中断标志
pub async fn get_abort_flag(session_id: &str) -> Arc<AtomicBool> {
    let flags = ABORT_FLAGS.read().await;
    if let Some(flag) = flags.get(session_id) {
        return flag.clone();
    }
    drop(flags);

    let mut flags = ABORT_FLAGS.write().await;
    let flag = Arc::new(AtomicBool::new(false));
    flags.insert(session_id.to_string(), flag.clone());
    flag
}

/// 设置中断标志
pub async fn set_abort_flag(session_id: &str, abort: bool) {
    let flag = get_abort_flag(session_id).await;
    flag.store(abort, Ordering::SeqCst);

    if abort {
        write_abort_log(
            "INFO",
            &format!(
                "收到 CLI 中断请求: session={}, source=app_abort",
                session_id
            ),
        );
    }

    // 如果是设置中断（abort = true），同时杀死对应的进程
    if abort {
        kill_session_process(session_id).await;
    }
}

/// 检查是否应该中断
pub async fn should_abort(session_id: &str) -> bool {
    get_abort_flag(session_id).await.load(Ordering::SeqCst)
}

/// 清理中断标志
pub async fn clear_abort_flag(session_id: &str) {
    let mut flags = ABORT_FLAGS.write().await;
    flags.remove(session_id);

    // 同时清理 PID 记录
    let mut pids = SESSION_PIDS.write().await;
    pids.remove(session_id);
}

/// 注册会话的进程 PID
pub async fn register_session_pid(session_id: &str, pid: u32) {
    let mut pids = SESSION_PIDS.write().await;
    pids.insert(session_id.to_string(), pid);
    write_abort_log(
        "INFO",
        &format!("注册 CLI 进程 PID: session={}, pid={}", session_id, pid),
    );
}

/// 注销会话的进程 PID
pub async fn unregister_session_pid(session_id: &str) {
    let mut pids = SESSION_PIDS.write().await;
    pids.remove(session_id);
}

/// 杀死会话对应的进程
pub async fn kill_session_process(session_id: &str) {
    let pids = SESSION_PIDS.read().await;
    if let Some(&pid) = pids.get(session_id) {
        drop(pids); // 释放读锁

        write_abort_log(
            "INFO",
            &format!(
                "应用请求终止 CLI 进程: session={}, pid={}, source=app_abort",
                session_id, pid
            ),
        );

        // 使用系统命令杀死进程
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            // Windows 上使用 taskkill 命令强制终止进程树
            let mut command = Command::new("taskkill");
            configure_windows_std_command(&mut command);
            let output = command
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        write_abort_log("INFO", &format!("成功终止 CLI 进程: pid={}", pid));
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        write_abort_log(
                            "ERROR",
                            &format!("终止 CLI 进程失败: pid={}, error={}", pid, stderr),
                        );
                    }
                }
                Err(e) => {
                    write_abort_log("ERROR", &format!("执行 taskkill 失败: {}", e));
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            // macOS: 先杀死子进程，再杀死主进程
            // 使用 pkill 杀死所有子进程
            let _ = Command::new("pkill")
                .args(["-TERM", "-P", &pid.to_string()])
                .output();

            // 然后杀死主进程
            let output = Command::new("kill").args(["-9", &pid.to_string()]).output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        write_abort_log("INFO", &format!("成功终止 CLI 进程 (macOS): pid={}", pid));
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        write_abort_log(
                            "ERROR",
                            &format!("终止 CLI 进程失败 (macOS): pid={}, error={}", pid, stderr),
                        );
                    }
                }
                Err(e) => {
                    write_abort_log("ERROR", &format!("执行 kill 失败 (macOS): {}", e));
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            // Linux: 先杀死子进程，再杀死主进程
            // 使用 pkill 杀死所有子进程
            let _ = Command::new("pkill")
                .args(["-TERM", "-P", &pid.to_string()])
                .output();

            // 然后杀死主进程
            let output = Command::new("kill").args(["-9", &pid.to_string()]).output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        write_abort_log("INFO", &format!("成功终止 CLI 进程 (Linux): pid={}", pid));
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        write_abort_log(
                            "ERROR",
                            &format!("终止 CLI 进程失败 (Linux): pid={}, error={}", pid, stderr),
                        );
                    }
                }
                Err(e) => {
                    write_abort_log("ERROR", &format!("执行 kill 失败 (Linux): {}", e));
                }
            }
        }

        // 清理 PID 记录
        let mut pids = SESSION_PIDS.write().await;
        pids.remove(session_id);
    } else {
        drop(pids);
        write_abort_log(
            "INFO",
            &format!(
                "收到 CLI 终止请求但未找到活跃 PID: session={}, source=app_abort",
                session_id
            ),
        );
    }
}
