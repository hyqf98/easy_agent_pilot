use std::sync::Arc;

use anyhow::Result;
use tauri::AppHandle;

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

        strategy.execute(app, request).await
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
}

/// 初始化策略注册表
pub async fn init_registry() {
    use super::strategies::{
        ClaudeCliStrategy, ClaudeSdkStrategy, CodexCliStrategy, CodexSdkStrategy,
    };

    let mut registry = REGISTRY.write().await;
    registry.register(Arc::new(ClaudeCliStrategy));
    registry.register(Arc::new(CodexCliStrategy));
    registry.register(Arc::new(ClaudeSdkStrategy));
    registry.register(Arc::new(CodexSdkStrategy));
}

/// 获取策略注册表
pub async fn get_registry() -> Arc<tokio::sync::RwLock<StrategyRegistry>> {
    REGISTRY.clone()
}

/// 统一执行命令
#[tauri::command]
pub async fn execute_agent(app: AppHandle, request: ExecutionRequest) -> Result<(), String> {
    let registry = get_registry().await;
    let registry = registry.read().await;

    registry
        .execute(app, request)
        .await
        .map_err(|e| e.to_string())
}
