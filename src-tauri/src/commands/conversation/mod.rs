//! 对话执行模块
//!
//! 该模块实现了智能体执行的策略模式，支持多种智能体类型：
//! - Claude CLI: 通过 Claude CLI 工具执行
//! - Codex CLI: 通过 Codex CLI 工具执行
//! - Claude SDK: 通过 Claude API 执行
//! - Codex SDK: 通过 OpenAI 兼容 API 执行

pub mod abort;
pub mod executor;
pub mod strategies;
pub mod strategy;
pub mod types;

// 重新导出常用类型和函数
pub use abort::set_abort_flag;
pub use executor::{get_registry, init_registry};
pub use strategy::AgentExecutionStrategy;
pub use types::{CliExecutionRequest, ExecutionRequest, SdkExecutionRequest};

use anyhow::Result;
use tauri::AppHandle;

async fn execute_compat_request(app: AppHandle, request: ExecutionRequest) -> Result<(), String> {
    let registry = get_registry().await;
    let registry = registry.read().await;
    registry
        .execute(app, request)
        .await
        .map_err(|e| e.to_string())
}

// ============== 向后兼容的命令包装器 ==============

/// 执行 Claude CLI 命令（向后兼容）
#[tauri::command]
pub async fn execute_claude_cli(
    app: AppHandle,
    request: CliExecutionRequest,
) -> Result<(), String> {
    execute_compat_request(app, ExecutionRequest::from_cli(request, "claude")).await
}

/// 执行 Codex CLI 命令（向后兼容）
#[tauri::command]
pub async fn execute_codex_cli(app: AppHandle, request: CliExecutionRequest) -> Result<(), String> {
    execute_compat_request(app, ExecutionRequest::from_cli(request, "codex")).await
}

/// 执行 Claude SDK API 调用（向后兼容）
#[tauri::command]
pub async fn execute_claude_sdk(
    app: AppHandle,
    request: SdkExecutionRequest,
) -> Result<(), String> {
    execute_compat_request(app, ExecutionRequest::from_sdk(request, "claude")).await
}
/// 执行 Codex SDK API 调用（新命令）
#[tauri::command]
pub async fn execute_codex_sdk(app: AppHandle, request: SdkExecutionRequest) -> Result<(), String> {
    execute_compat_request(app, ExecutionRequest::from_sdk(request, "codex")).await
}
/// 中断 CLI 执行
#[tauri::command]
pub async fn abort_cli_execution(session_id: String) -> Result<(), String> {
    set_abort_flag(&session_id, true).await;
    Ok(())
}

/// 中断 SDK 执行
#[tauri::command]
pub async fn abort_sdk_execution(session_id: String) -> Result<(), String> {
    set_abort_flag(&session_id, true).await;
    Ok(())
}

/// 清除中断标志（新执行开始前调用，防止残留标志影响新执行）
#[tauri::command]
pub async fn clear_session_abort_flag(session_id: String) -> Result<(), String> {
    abort::clear_abort_flag(&session_id).await;
    Ok(())
}
