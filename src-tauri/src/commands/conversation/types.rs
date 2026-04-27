use serde::{Deserialize, Serialize};

use crate::commands::message::MessageAttachment;

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    /// MCP 配置 ID
    pub id: String,
    /// MCP 名称
    pub name: String,
    /// 传输类型
    pub transport_type: String,
    /// 命令 (stdio 类型)
    pub command: Option<String>,
    /// 参数 (stdio 类型)
    pub args: Option<String>,
    /// 环境变量 (stdio 类型)
    pub env: Option<String>,
    /// URL (sse/http 类型)
    pub url: Option<String>,
    /// 请求头 (sse/http 类型)
    pub headers: Option<String>,
}

/// CLI 执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliExecutionRequest {
    /// 会话 ID
    pub session_id: String,
    /// CLI 路径
    pub cli_path: String,
    /// 模型 ID
    pub model_id: Option<String>,
    /// 消息历史
    pub messages: Vec<MessageInput>,
    /// 工作目录
    pub working_directory: Option<String>,
    /// 允许的工具列表
    pub allowed_tools: Option<Vec<String>>,
    /// CLI 输出格式 (text/json/stream-json)
    pub cli_output_format: Option<String>,
    /// JSON Schema（当输出格式为 json 时可选）
    pub json_schema: Option<String>,
    /// 额外 CLI 参数
    pub extra_cli_args: Option<Vec<String>>,
    /// MCP 服务器配置列表
    pub mcp_servers: Option<Vec<McpServerConfig>>,
    /// 恢复已存在的 CLI 会话 ID
    pub resume_session_id: Option<String>,
}

/// SDK 执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkExecutionRequest {
    /// 会话 ID
    pub session_id: String,
    /// API 密钥
    pub api_key: String,
    /// API 端点
    pub base_url: Option<String>,
    /// 模型 ID
    pub model_id: String,
    /// 消息历史
    pub messages: Vec<MessageInput>,
    /// 系统提示
    pub system_prompt: Option<String>,
    /// 最大令牌数
    pub max_tokens: Option<u32>,
    /// 工具定义
    pub tools: Option<Vec<ToolDefinition>>,
    /// MCP 服务器配置列表
    pub mcp_servers: Option<Vec<McpServerConfig>>,
}

/// 统一执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRequest {
    /// 会话 ID
    pub session_id: String,
    /// 计划 ID（任务拆分模式使用）
    pub plan_id: Option<String>,
    /// 智能体类型 (cli/sdk)
    pub agent_type: String,
    /// 提供者 (claude/codex)
    pub provider: String,
    /// CLI 路径 (仅 CLI 类型需要)
    pub cli_path: Option<String>,
    /// API 密钥 (仅 SDK 类型需要)
    pub api_key: Option<String>,
    /// API 端点
    pub base_url: Option<String>,
    /// 模型 ID
    pub model_id: Option<String>,
    /// 消息历史
    pub messages: Vec<MessageInput>,
    /// 工作目录
    pub working_directory: Option<String>,
    /// 允许的工具列表
    pub allowed_tools: Option<Vec<String>>,
    /// 系统提示
    pub system_prompt: Option<String>,
    /// 最大令牌数
    pub max_tokens: Option<u32>,
    /// 工具定义
    pub tools: Option<Vec<ToolDefinition>>,
    /// CLI 输出格式 (text/json/stream-json)
    pub cli_output_format: Option<String>,
    /// JSON Schema（当输出格式为 json 时可选）
    pub json_schema: Option<String>,
    /// 额外 CLI 参数
    pub extra_cli_args: Option<Vec<String>>,
    /// MCP 服务器配置列表
    pub mcp_servers: Option<Vec<McpServerConfig>>,
    /// 执行模式（chat/task_split/task_execution/solo_execution）
    pub execution_mode: Option<String>,
    /// 响应模式（stream_text/json_once）
    pub response_mode: Option<String>,
    /// 恢复已存在的 CLI 会话 ID
    pub resume_session_id: Option<String>,
}

impl ExecutionRequest {
    pub fn from_cli(request: CliExecutionRequest, provider: &str) -> Self {
        Self {
            session_id: request.session_id,
            plan_id: None,
            agent_type: "cli".to_string(),
            provider: provider.to_string(),
            cli_path: Some(request.cli_path),
            api_key: None,
            base_url: None,
            model_id: request.model_id,
            messages: request.messages,
            working_directory: request.working_directory,
            allowed_tools: request.allowed_tools,
            system_prompt: None,
            max_tokens: None,
            tools: None,
            cli_output_format: request.cli_output_format,
            json_schema: request.json_schema,
            extra_cli_args: request.extra_cli_args,
            mcp_servers: request.mcp_servers,
            execution_mode: None,
            response_mode: None,
            resume_session_id: request.resume_session_id,
        }
    }

    pub fn from_sdk(request: SdkExecutionRequest, provider: &str) -> Self {
        Self {
            session_id: request.session_id,
            plan_id: None,
            agent_type: "sdk".to_string(),
            provider: provider.to_string(),
            cli_path: None,
            api_key: Some(request.api_key),
            base_url: request.base_url,
            model_id: Some(request.model_id),
            messages: request.messages,
            working_directory: None,
            allowed_tools: None,
            system_prompt: request.system_prompt,
            max_tokens: request.max_tokens,
            tools: request.tools,
            cli_output_format: None,
            json_schema: None,
            extra_cli_args: None,
            mcp_servers: request.mcp_servers,
            execution_mode: None,
            response_mode: None,
            resume_session_id: None,
        }
    }
}

/// 消息输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageInput {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub attachments: Option<Vec<MessageAttachment>>,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// 流式事件（CLI/SDK 共用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamEvent {
    /// 事件类型
    #[serde(rename = "type")]
    pub event_type: String,
    /// 会话 ID
    pub session_id: String,
    /// 内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// 工具名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// 工具调用 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// 工具输入
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<String>,
    /// 工具结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<String>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 输入 token 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    /// 输出 token 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    /// 原始输入 token 数量（provider 原始 JSON）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_input_tokens: Option<u32>,
    /// 原始输出 token 数量（provider 原始 JSON）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_output_tokens: Option<u32>,
    /// 命中缓存读取的输入 token 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
    /// 新建缓存写入的输入 token 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    /// 模型名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 外部 CLI 会话 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_session_id: Option<String>,
}

pub type CliStreamEvent = StreamEvent;
pub type SdkStreamEvent = StreamEvent;
