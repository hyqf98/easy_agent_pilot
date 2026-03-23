use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// 操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallOperationType {
    CreateFile,
    CreateDir,
    ModifyFile,
    DeleteFile,
}

/// 安装操作记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallOperation {
    pub sequence: u32,
    pub operation_type: InstallOperationType,
    pub target_path: String,
    pub backup_path: Option<String>,
    pub timestamp: String,
}

/// 安装会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSession {
    pub id: String,
    pub backup_dir: String,
    pub operations: Vec<InstallOperation>,
    pub status: String,
    pub created_at: String,
    pub error_message: Option<String>,
}

/// 安装结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub session_id: Option<String>,
    pub rollback_performed: bool,
    pub rollback_error: Option<String>,
    pub backup_location: Option<String>,
}

/// 获取备份目录路径
pub fn get_backup_base_dir() -> Result<PathBuf> {
    let persistence_dir = crate::commands::get_persistence_dir_path()?;
    Ok(persistence_dir.join("cache").join("install_backups"))
}

/// 创建安装会话
#[tauri::command]
pub fn create_install_session() -> Result<InstallSession, String> {
    let session_id = Uuid::new_v4().to_string();
    let backup_base = get_backup_base_dir().map_err(|e| e.to_string())?;

    // 确保备份基础目录存在
    fs::create_dir_all(&backup_base).map_err(|e| e.to_string())?;

    // 创建会话专属备份目录
    let backup_dir = backup_base.join(&session_id);
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let now = Utc::now().to_rfc3339();
    let session = InstallSession {
        id: session_id,
        backup_dir: backup_dir.to_string_lossy().to_string(),
        operations: Vec::new(),
        status: "active".to_string(),
        created_at: now,
        error_message: None,
    };

    // 保存会话到文件
    save_session(&session)?;

    Ok(session)
}

/// 保存会话到文件
fn save_session(session: &InstallSession) -> Result<(), String> {
    let session_file = PathBuf::from(&session.backup_dir).join("session.json");
    let content = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    fs::write(&session_file, content).map_err(|e| e.to_string())?;
    Ok(())
}

/// 加载会话
fn load_session(session_id: &str) -> Result<InstallSession, String> {
    let backup_base = get_backup_base_dir().map_err(|e| e.to_string())?;
    let session_file = backup_base.join(session_id).join("session.json");

    if !session_file.exists() {
        return Err(format!("安装会话不存在: {}", session_id));
    }

    let content = fs::read_to_string(&session_file).map_err(|e| e.to_string())?;
    let session: InstallSession = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    Ok(session)
}

/// 记录操作到日志
fn log_operation(session: &mut InstallSession, operation: InstallOperation) -> Result<(), String> {
    // 写入操作日志文件
    let log_file = PathBuf::from(&session.backup_dir).join("operations.log");
    let log_entry = format!(
        "[{}] seq={}, type={:?}, path={}, backup={:?}\n",
        operation.timestamp,
        operation.sequence,
        operation.operation_type,
        operation.target_path,
        operation.backup_path
    );

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| e.to_string())?;

    file.write_all(log_entry.as_bytes())
        .map_err(|e| e.to_string())?;

    // 添加到会话
    session.operations.push(operation);
    save_session(session)?;

    Ok(())
}

/// 备份文件
fn backup_file(session: &mut InstallSession, file_path: &Path) -> Result<Option<String>, String> {
    if !file_path.exists() {
        return Ok(None);
    }

    let backup_name = format!(
        "{}_{}",
        Uuid::new_v4(),
        file_path.file_name().unwrap_or_default().to_string_lossy()
    );
    let backup_path = PathBuf::from(&session.backup_dir).join(&backup_name);

    fs::copy(file_path, &backup_path).map_err(|e| e.to_string())?;

    Ok(Some(backup_path.to_string_lossy().to_string()))
}

/// 记录创建文件操作
#[tauri::command]
pub fn record_create_file(session_id: String, file_path: String) -> Result<InstallSession, String> {
    let mut session = load_session(&session_id)?;

    if session.status != "active" {
        return Err("安装会话已结束".to_string());
    }

    let now = Utc::now().to_rfc3339();
    let sequence = session.operations.len() as u32 + 1;

    let operation = InstallOperation {
        sequence,
        operation_type: InstallOperationType::CreateFile,
        target_path: file_path,
        backup_path: None,
        timestamp: now,
    };

    log_operation(&mut session, operation)?;

    Ok(session)
}

/// 记录创建目录操作
#[tauri::command]
pub fn record_create_dir(session_id: String, dir_path: String) -> Result<InstallSession, String> {
    let mut session = load_session(&session_id)?;

    if session.status != "active" {
        return Err("安装会话已结束".to_string());
    }

    let now = Utc::now().to_rfc3339();
    let sequence = session.operations.len() as u32 + 1;

    let operation = InstallOperation {
        sequence,
        operation_type: InstallOperationType::CreateDir,
        target_path: dir_path,
        backup_path: None,
        timestamp: now,
    };

    log_operation(&mut session, operation)?;

    Ok(session)
}

/// 记录修改文件操作（会自动备份原文件）
#[tauri::command]
pub fn record_modify_file(session_id: String, file_path: String) -> Result<InstallSession, String> {
    let mut session = load_session(&session_id)?;

    if session.status != "active" {
        return Err("安装会话已结束".to_string());
    }

    let path = PathBuf::from(&file_path);
    let backup_path = backup_file(&mut session, &path)?;

    let now = Utc::now().to_rfc3339();
    let sequence = session.operations.len() as u32 + 1;

    let operation = InstallOperation {
        sequence,
        operation_type: InstallOperationType::ModifyFile,
        target_path: file_path,
        backup_path,
        timestamp: now,
    };

    log_operation(&mut session, operation)?;

    Ok(session)
}

/// 记录删除文件操作（会自动备份原文件）
#[tauri::command]
pub fn record_delete_file(session_id: String, file_path: String) -> Result<InstallSession, String> {
    let mut session = load_session(&session_id)?;

    if session.status != "active" {
        return Err("安装会话已结束".to_string());
    }

    let path = PathBuf::from(&file_path);
    let backup_path = backup_file(&mut session, &path)?;

    let now = Utc::now().to_rfc3339();
    let sequence = session.operations.len() as u32 + 1;

    let operation = InstallOperation {
        sequence,
        operation_type: InstallOperationType::DeleteFile,
        target_path: file_path,
        backup_path,
        timestamp: now,
    };

    log_operation(&mut session, operation)?;

    Ok(session)
}

/// 执行回滚
fn execute_rollback(session: &mut InstallSession) -> Result<(), String> {
    let mut rollback_errors = Vec::new();

    // 按逆序恢复所有操作
    for operation in session.operations.iter().rev() {
        let target_path = PathBuf::from(&operation.target_path);

        match operation.operation_type {
            InstallOperationType::CreateFile | InstallOperationType::CreateDir => {
                // 删除新创建的文件或目录
                if target_path.exists() {
                    if target_path.is_dir() {
                        if let Err(e) = fs::remove_dir_all(&target_path) {
                            rollback_errors.push(format!(
                                "删除目录失败 {} -> {}: {}",
                                operation.target_path, operation.sequence, e
                            ));
                        }
                    } else if let Err(e) = fs::remove_file(&target_path) {
                        rollback_errors.push(format!(
                            "删除文件失败 {} -> {}: {}",
                            operation.target_path, operation.sequence, e
                        ));
                    }
                }
            }
            InstallOperationType::ModifyFile | InstallOperationType::DeleteFile => {
                // 从备份恢复文件
                if let Some(ref backup_path) = operation.backup_path {
                    let backup = PathBuf::from(backup_path);
                    if backup.exists() {
                        // 确保目标目录存在
                        if let Some(parent) = target_path.parent() {
                            if let Err(e) = fs::create_dir_all(parent) {
                                rollback_errors.push(format!(
                                    "创建目录失败 {} -> {}: {}",
                                    operation.target_path, operation.sequence, e
                                ));
                                continue;
                            }
                        }

                        if let Err(e) = fs::copy(&backup, &target_path) {
                            rollback_errors.push(format!(
                                "恢复文件失败 {} -> {}: {}",
                                operation.target_path, operation.sequence, e
                            ));
                        }
                    }
                } else if operation.operation_type == InstallOperationType::DeleteFile {
                    // 如果没有备份，说明文件原本不存在，确保删除
                    if target_path.exists() {
                        if let Err(e) = fs::remove_file(&target_path) {
                            rollback_errors.push(format!(
                                "删除文件失败 {} -> {}: {}",
                                operation.target_path, operation.sequence, e
                            ));
                        }
                    }
                }
            }
        }
    }

    if !rollback_errors.is_empty() {
        return Err(rollback_errors.join("; "));
    }

    Ok(())
}

/// 安装失败回滚
#[tauri::command]
pub fn rollback_install(session_id: String, error_reason: String) -> Result<InstallResult, String> {
    let mut session = load_session(&session_id)?;

    if session.status != "active" {
        return Ok(InstallResult {
            success: false,
            message: "安装会话已结束，无法回滚".to_string(),
            session_id: Some(session_id),
            rollback_performed: false,
            rollback_error: None,
            backup_location: Some(session.backup_dir.clone()),
        });
    }

    // 记录错误信息
    session.error_message = Some(error_reason.clone());
    session.status = "rolling_back".to_string();
    save_session(&session)?;

    // 执行回滚
    let rollback_result = execute_rollback(&mut session);

    match rollback_result {
        Ok(()) => {
            // 回滚成功，删除备份
            session.status = "rolled_back".to_string();
            save_session(&session)?;

            // 清理备份目录
            let _ = cleanup_backup(&session_id);

            Ok(InstallResult {
                success: false,
                message: format!("安装失败：{}，已自动回滚", error_reason),
                session_id: Some(session_id),
                rollback_performed: true,
                rollback_error: None,
                backup_location: None,
            })
        }
        Err(rollback_error) => {
            // 回滚失败
            session.status = "rollback_failed".to_string();
            save_session(&session)?;

            Ok(InstallResult {
                success: false,
                message: format!(
                    "安装失败：{}\n\n【严重警告】回滚失败：{}\n\n备份文件位置：{}\n\n建议手动恢复。",
                    error_reason, rollback_error, session.backup_dir
                ),
                session_id: Some(session_id),
                rollback_performed: true,
                rollback_error: Some(rollback_error),
                backup_location: Some(session.backup_dir.clone()),
            })
        }
    }
}

/// 完成安装（成功后清理备份）
#[tauri::command]
pub fn complete_install(session_id: String) -> Result<InstallResult, String> {
    let mut session = load_session(&session_id)?;

    if session.status != "active" {
        return Ok(InstallResult {
            success: false,
            message: "安装会话已结束".to_string(),
            session_id: Some(session_id),
            rollback_performed: false,
            rollback_error: None,
            backup_location: None,
        });
    }

    // 标记为完成
    session.status = "completed".to_string();
    save_session(&session)?;

    // 清理备份目录
    cleanup_backup(&session_id)?;

    Ok(InstallResult {
        success: true,
        message: "安装成功".to_string(),
        session_id: Some(session_id),
        rollback_performed: false,
        rollback_error: None,
        backup_location: None,
    })
}

/// 清理备份目录
fn cleanup_backup(session_id: &str) -> Result<(), String> {
    let backup_base = get_backup_base_dir().map_err(|e| e.to_string())?;
    let backup_dir = backup_base.join(session_id);

    if backup_dir.exists() {
        fs::remove_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 获取会话状态
#[tauri::command]
pub fn get_install_session_status(session_id: String) -> Result<InstallSession, String> {
    load_session(&session_id)
}

/// 取消安装会话
#[tauri::command]
pub fn cancel_install_session(session_id: String) -> Result<InstallResult, String> {
    let mut session = load_session(&session_id)?;

    if session.status != "active" {
        return Ok(InstallResult {
            success: false,
            message: "安装会话已结束".to_string(),
            session_id: Some(session_id),
            rollback_performed: false,
            rollback_error: None,
            backup_location: None,
        });
    }

    // 执行回滚
    let rollback_result = execute_rollback(&mut session);

    match rollback_result {
        Ok(()) => {
            session.status = "cancelled".to_string();
            save_session(&session)?;
            let _ = cleanup_backup(&session_id);

            Ok(InstallResult {
                success: true,
                message: "安装已取消，已自动回滚".to_string(),
                session_id: Some(session_id),
                rollback_performed: true,
                rollback_error: None,
                backup_location: None,
            })
        }
        Err(rollback_error) => {
            session.status = "cancel_rollback_failed".to_string();
            save_session(&session)?;

            Ok(InstallResult {
                success: false,
                message: format!(
                    "取消安装失败，回滚错误：{}\n备份位置：{}",
                    rollback_error, session.backup_dir
                ),
                session_id: Some(session_id),
                rollback_performed: true,
                rollback_error: Some(rollback_error),
                backup_location: Some(session.backup_dir.clone()),
            })
        }
    }
}

/// 列出所有未完成的安装会话
#[tauri::command]
pub fn list_pending_install_sessions() -> Result<Vec<InstallSession>, String> {
    let backup_base = get_backup_base_dir().map_err(|e| e.to_string())?;

    if !backup_base.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    let entries = fs::read_dir(&backup_base).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let session_file = entry.path().join("session.json");
        if session_file.exists() {
            if let Ok(content) = fs::read_to_string(&session_file) {
                if let Ok(session) = serde_json::from_str::<InstallSession>(&content) {
                    // 只返回未完成的会话
                    if session.status == "active" || session.status == "rolling_back" {
                        sessions.push(session);
                    }
                }
            }
        }
    }

    Ok(sessions)
}

/// 列出所有安装会话（包括已完成/已回滚的，用于清理）
#[tauri::command]
pub fn list_all_install_sessions() -> Result<Vec<InstallSession>, String> {
    let backup_base = get_backup_base_dir().map_err(|e| e.to_string())?;

    if !backup_base.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    let entries = fs::read_dir(&backup_base).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let session_file = entry.path().join("session.json");
        if session_file.exists() {
            if let Ok(content) = fs::read_to_string(&session_file) {
                if let Ok(session) = serde_json::from_str::<InstallSession>(&content) {
                    sessions.push(session);
                }
            }
        }
    }

    // 按创建时间倒序排列
    sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(sessions)
}

/// 手动清理安装会话的备份文件
#[tauri::command]
pub fn cleanup_install_session(session_id: String) -> Result<(), String> {
    cleanup_backup(&session_id)
}
