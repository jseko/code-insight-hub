// 协议定义 - 消息和事件类型

use serde::{Deserialize, Serialize};

/// 用户消息类型
#[derive(Debug, Clone, Serialize)]
pub struct UserMessage {
    pub content: String,
}

/// AI 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: String,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
}

/// 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Thinking,
    #[allow(dead_code)]
    ExecutingTool,
    #[allow(dead_code)]
    Error,
}

/// 工具定义
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}
