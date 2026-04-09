use anyhow::Result;
use async_trait::async_trait;
use tauri::AppHandle;

use super::types::ExecutionRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentRuntimeKind {
    ClaudeCli,
    CodexCli,
    OpenCodeCli,
    ClaudeSdk,
    CodexSdk,
}

impl AgentRuntimeKind {
    pub fn from_request(agent_type: &str, provider: &str) -> Option<Self> {
        match (agent_type, provider) {
            ("cli", "claude") => Some(Self::ClaudeCli),
            ("cli", "codex") => Some(Self::CodexCli),
            ("cli", "opencode") => Some(Self::OpenCodeCli),
            ("sdk", "claude") => Some(Self::ClaudeSdk),
            ("sdk", "codex") => Some(Self::CodexSdk),
            _ => None,
        }
    }

    pub fn event_name(&self, session_id: &str) -> String {
        match self {
            Self::ClaudeCli => format!("claude-stream-{}", session_id),
            Self::CodexCli => format!("codex-stream-{}", session_id),
            Self::OpenCodeCli => format!("opencode-stream-{}", session_id),
            Self::ClaudeSdk => format!("sdk-stream-{}", session_id),
            Self::CodexSdk => format!("codex-sdk-stream-{}", session_id),
        }
    }
}

/// 智能体执行策略 Trait
#[async_trait]
pub trait AgentExecutionStrategy: Send + Sync {
    /// 返回当前策略对应的运行时类型
    fn kind(&self) -> AgentRuntimeKind;

    /// 执行智能体调用
    async fn execute(&self, app: AppHandle, request: ExecutionRequest) -> Result<()>;
}
