use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use tauri::AppHandle;
use tokio::sync::RwLock;

use super::strategy::{AgentExecutionStrategy, AgentRuntimeKind};
use super::types::ExecutionRequest;

/// 策略注册表
pub struct StrategyRegistry {
    strategies: Vec<Arc<dyn AgentExecutionStrategy>>,
}

impl StrategyRegistry {
    /// 创建新的策略注册表
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }

    /// 注册策略
    pub fn register(&mut self, strategy: Arc<dyn AgentExecutionStrategy>) {
        self.strategies.push(strategy);
    }

    /// 获取支持的策略
    pub fn get_strategy(&self, kind: AgentRuntimeKind) -> Option<Arc<dyn AgentExecutionStrategy>> {
        self.strategies
            .iter()
            .find(|strategy| strategy.kind() == kind)
            .cloned()
    }

    /// 执行智能体调用
    pub async fn execute(&self, app: AppHandle, request: ExecutionRequest) -> Result<()> {
        let kind = AgentRuntimeKind::from_request(&request.agent_type, &request.provider)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "不支持的智能体类型: {} ({})",
                    request.agent_type,
                    request.provider
                )
            })?;

        let strategy = self.get_strategy(kind).ok_or_else(|| {
            anyhow::anyhow!(
                "未注册智能体运行时策略: {} ({})",
                request.agent_type,
                request.provider
            )
        })?;

        let session_id = request.session_id.clone();
        mark_execution_session_active(&session_id).await;
        let result = strategy.execute(app, request).await;
        mark_execution_session_inactive(&session_id).await;
        result
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// 全局策略注册表
lazy_static::lazy_static! {
    static ref REGISTRY: Arc<tokio::sync::RwLock<StrategyRegistry>> =
        Arc::new(tokio::sync::RwLock::new(StrategyRegistry::new()));
    static ref ACTIVE_EXECUTION_SESSIONS: Arc<RwLock<HashSet<String>>> =
        Arc::new(RwLock::new(HashSet::new()));
}

/// 初始化策略注册表
pub async fn init_registry() {
    use super::strategies::{
        ClaudeCliStrategy, ClaudeSdkStrategy, CodexCliStrategy, CodexSdkStrategy,
        OpenCodeCliStrategy,
    };

    let mut registry = REGISTRY.write().await;
    registry.register(Arc::new(ClaudeCliStrategy));
    registry.register(Arc::new(CodexCliStrategy));
    registry.register(Arc::new(OpenCodeCliStrategy));
    registry.register(Arc::new(ClaudeSdkStrategy));
    registry.register(Arc::new(CodexSdkStrategy));
}

/// 获取策略注册表
pub async fn get_registry() -> Arc<tokio::sync::RwLock<StrategyRegistry>> {
    REGISTRY.clone()
}

/// 标记执行会话为活跃状态。
pub async fn mark_execution_session_active(session_id: &str) {
    let mut sessions = ACTIVE_EXECUTION_SESSIONS.write().await;
    sessions.insert(session_id.to_string());
}

/// 清理执行会话的活跃状态。
pub async fn mark_execution_session_inactive(session_id: &str) {
    let mut sessions = ACTIVE_EXECUTION_SESSIONS.write().await;
    sessions.remove(session_id);
}

/// 判断执行会话当前是否仍在本进程内活跃。
pub async fn is_execution_session_active_internal(session_id: &str) -> bool {
    let sessions = ACTIVE_EXECUTION_SESSIONS.read().await;
    sessions.contains(session_id)
}

/// 查询执行会话当前是否仍在运行。
#[tauri::command]
pub async fn is_execution_session_active(session_id: String) -> Result<bool, String> {
    Ok(is_execution_session_active_internal(&session_id).await)
}

/// 统一执行命令
#[tauri::command]
pub async fn execute_agent(app: AppHandle, request: ExecutionRequest) -> Result<(), String> {
    let registry = get_registry().await;
    let registry = registry.read().await;

    let session_id = request.session_id.clone();
    let provider = request.provider.clone();
    let agent_type = request.agent_type.clone();

    registry.execute(app, request).await.map_err(|error| {
        let message = error.to_string();
        crate::logging::write_log(
            "ERROR",
            "conversation-executor",
            &format!(
                "execute_agent failed | session_id={} | provider={} | agent_type={} | {}",
                session_id, provider, agent_type, message
            ),
        );
        message
    })
}
