// 智能体核心实现 - 简化版 Codex + AgentControl

use crate::client::ModelClient;
use crate::protocol::{AgentStatus, AssistantMessage, ToolCall, UserMessage};
use crate::tools::ToolRegistry;
use crate::flight_tools::{GetFlightNumberTool, GetTicketPriceTool};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 智能体状态
#[derive(Debug, Clone)]
pub struct AgentState {
    pub status: AgentStatus,
    pub conversation: Vec<Value>,
}

/// 简化版智能体（结合 Codex 和 AgentControl 的功能）
pub struct Agent {
    model_client: ModelClient,
    tool_registry: ToolRegistry,
    state: Arc<RwLock<AgentState>>,
    max_turns: usize,
    current_turn: usize,
}

impl Agent {
    #[allow(dead_code)]
    pub fn new(model_client: ModelClient) -> Self {
        let mut tool_registry = ToolRegistry::new();

        // 注册内置工具
        println!("\n🔧 初始化工具系统...");
        tool_registry.register(crate::tools::ShellTool);
        tool_registry.register(crate::tools::CurrentTimeTool);
        tool_registry.register(crate::tools::ReadFileTool);
        tool_registry.register(crate::tools::HelpTool::new(vec![
            "shell".to_string(),
            "current_time".to_string(),
            "read_file".to_string(),
            "help".to_string(),
            "get_flight_number".to_string(),
            "get_ticket_price".to_string(),
        ]));

        // 注册航班查询工具（基于 ChatGLM 教程）
        tool_registry.register(GetFlightNumberTool::new());
        tool_registry.register(GetTicketPriceTool::default());

        println!("  ✅ 工具系统初始化完成\n");


        Self {
            model_client,
            tool_registry,
            state: Arc::new(RwLock::new(AgentState {
                status: AgentStatus::Idle,
                conversation: Vec::new(),
            })),
            max_turns: 10,
            current_turn: 0,
        }
    }

    /// 处理用户消息（流式输出版本）
    #[allow(dead_code)]
    pub async fn process_message_stream<F>(&mut self, user_input: &str, mut callback: F)
    where
        F: FnMut(&str),
    {
        // 更新状态
        {
            let mut state = self.state.write().await;
            state.status = AgentStatus::Thinking;
            state.conversation.push(json!(UserMessage {
                content: user_input.to_string(),
            }));
        }

        // 运行智能体循环（流式版本）
        let _ = self.run_agent_loop_stream(user_input, &mut callback).await;
    }

    /// 处理用户消息（流式输出版本） - 返回 Result 版本
    pub async fn process_message_stream_with_result<F>(
        &mut self,
        user_input: &str,
        mut callback: F,
    ) -> Result<String, anyhow::Error>
    where
        F: FnMut(&str),
    {
        // 更新状态
        {
            let mut state = self.state.write().await;
            state.status = AgentStatus::Thinking;
            state.conversation.push(json!(UserMessage {
                content: user_input.to_string(),
            }));
        }

        // 运行智能体循环（流式版本）
        self.run_agent_loop_stream(user_input, &mut callback).await
    }

    /// 处理用户消息（类似 AgentControl::send_prompt）
    #[allow(dead_code)]
    pub async fn process_message(&mut self, user_input: &str) -> Result<String, anyhow::Error> {
        // 更新状态
        {
            let mut state = self.state.write().await;
            state.status = AgentStatus::Thinking;
            state.conversation.push(json!(UserMessage {
                content: user_input.to_string(),
            }));
        }

        // 运行智能体循环
        self.run_agent_loop(user_input).await
    }

    /// 智能体主循环（流式版本 - 真正的异步流式）
    async fn run_agent_loop_stream<F>(&mut self, _initial_input: &str, mut callback: F) -> Result<String, anyhow::Error>
    where
        F: FnMut(&str),
    {
        self.current_turn = 0;
        let mut full_response = String::new();

        loop {
            self.current_turn += 1;
            if self.current_turn > self.max_turns {
                let msg = "\n🔄 已达到最大对话轮次，建议重新开始对话。";
                callback(msg);
                return Ok(msg.to_string());
            }

            // 获取工具定义
            let tools = self.tool_registry.list_definitions();
            let tools_json: Vec<Value> = tools
                .iter()
                .map(|t| json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                }))
                .collect();

            // 添加系统提示以强制使用工具
            let mut messages = {
                let state = self.state.read().await;
                state.conversation.clone()
            };

            // 在对话开始时插入系统提示
            if messages.is_empty() || !messages[0].get("role").is_some_and(|r| r == "system") {
                let system_prompt = json!({
                    "role": "system",
                    "content": "You are a helpful AI assistant. When users ask for information that can be obtained through tools, you MUST use the available tools.\n\nAvailable tools:\n- current_time: Get current date and time\n- shell: Execute shell commands\n- read_file: Read text file contents\n- get_flight_number: Query flight number by departure, destination, and date\n- get_ticket_price: Query ticket price by flight number and date\n\nDo not guess or make up information. Always use tools when they are relevant. For flight queries, ask for missing required information if the user doesn't provide complete details."
                });
                messages.insert(0, system_prompt);
            }

            // 更新状态为思考
            {
                let mut state = self.state.write().await;
                state.status = AgentStatus::Thinking;
            }

            // 构建消息历史
            let messages = {
                let state = self.state.read().await;
                state.conversation.clone()
            };

            // 调用大模型（真流式）
            let mut stream = self
                .model_client
                .chat_completion_stream(messages, Some(tools_json))
                .await?;

            let mut turn_response = String::new();
            let mut final_tool_calls: Option<Vec<crate::protocol::ToolCall>> = None;

            // 逐个处理流式事件
            use futures::StreamExt;
            while let Some(event_result) = stream.next().await {
                let event = event_result.map_err(|e| anyhow::anyhow!("流式错误: {}", e))?;

                match event {
                    crate::client::SseEvent::TextDelta(text) => {
                        callback(&text);
                        turn_response.push_str(&text);
                        full_response.push_str(&text);
                    }
                    crate::client::SseEvent::ReasoningDelta(text) => {
                        callback(&text);
                        turn_response.push_str(&text);
                        full_response.push_str(&text);
                    }
                    crate::client::SseEvent::ToolCalls(calls) => {
                        final_tool_calls = Some(calls);
                    }
                    crate::client::SseEvent::Done => {
                        break;
                    }
                }
            }

            // 添加助手响应到对话历史
            {
                let mut state = self.state.write().await;
                state.status = AgentStatus::Idle;
                state.conversation.push(json!(AssistantMessage {
                    content: turn_response.clone(),
                    tool_calls: final_tool_calls.clone(),
                }));
            }

            // 检查是否需要执行工具
            if let Some(tool_calls) = final_tool_calls {
                if !tool_calls.is_empty() {
                    println!("\n🔧 收到工具调用: {} 个工具", tool_calls.len());
                    // 执行工具调用
                    for call in &tool_calls {
                        self.execute_tool_call(call).await?;
                    }

                    // 继续循环以获取下一个响应
                    continue;
                } else {
                    println!("\n⚠️  工具调用列表为空");
                }
            } else {
                println!("\n⚠️  没有工具调用");
            }

            // 返回最终回复
            return Ok(full_response);
        }
    }

    /// 智能体主循环（类似 CodexThread 的事件循环）
    #[allow(dead_code)]
    async fn run_agent_loop(&mut self, _initial_input: &str) -> Result<String, anyhow::Error> {
        self.current_turn = 0;

        loop {
            self.current_turn += 1;
            if self.current_turn > self.max_turns {
                return Ok("\n🔄 已达到最大对话轮次，建议重新开始对话。".to_string());
            }

            // 获取工具定义
            let tools = self.tool_registry.list_definitions();
            let tools_json: Vec<Value> = tools
                .iter()
                .map(|t| json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                }))
                .collect();

            // 添加系统提示以强制使用工具
            let mut messages = {
                let state = self.state.read().await;
                state.conversation.clone()
            };

            // 在对话开始时插入系统提示
            if messages.is_empty() || !messages[0].get("role").is_some_and(|r| r == "system") {
                let system_prompt = json!({
                    "role": "system",
                    "content": "You are a helpful AI assistant. When users ask for information that can be obtained through tools, you MUST use the available tools.\n\nAvailable tools:\n- current_time: Get current date and time\n- shell: Execute shell commands\n- read_file: Read text file contents\n- get_flight_number: Query flight number by departure, destination, and date\n- get_ticket_price: Query ticket price by flight number and date\n\nDo not guess or make up information. Always use tools when they are relevant. For flight queries, ask for missing required information if the user doesn't provide complete details."
                });
                messages.insert(0, system_prompt);
            }

            // 更新状态为思考
            {
                let mut state = self.state.write().await;
                state.status = AgentStatus::Thinking;
            }

            // 构建消息历史
            let messages = {
                let state = self.state.read().await;
                state.conversation.clone()
            };

            // 调用大模型（类似 ModelClient::stream）
            let response = self
                .model_client
                .chat_completion(messages, Some(tools_json))
                .await?;

            // 添加助手响应到对话历史
            {
                let mut state = self.state.write().await;
                state.status = AgentStatus::Idle;
                state.conversation.push(json!(AssistantMessage {
                    content: response.content.clone(),
                    tool_calls: response.tool_calls.clone(),
                }));
            }

            // 检查是否需要执行工具
            if let Some(tool_calls) = response.tool_calls {
                if !tool_calls.is_empty() {
                    // 执行工具调用
                    for call in &tool_calls {
                        self.execute_tool_call(call).await?;
                    }

                    // 继续循环以获取下一个响应
                    continue;
                }
            }

            // 返回最终回复
            return Ok(response.content);
        }
    }

    /// 执行工具调用（类似 ToolRouter::dispatch）
    #[allow(dead_code)]
    async fn execute_tool_call(&self, call: &ToolCall) -> Result<(), anyhow::Error> {
        // 更新状态为执行工具
        {
            let mut state = self.state.write().await;
            state.status = AgentStatus::ExecutingTool;
        }

        println!("\n🔧 调用工具: {} ({})", call.name, call.id);
        println!("🔧 工具参数: {}", call.arguments); // 调试输出

        // 执行工具
        let result = self.tool_registry.execute(call).await.map_err(|e| {
            anyhow::anyhow!("工具执行失败: {}", e)
        })?;

        println!("  ✅ 工具结果: {}", result.content);

        // 将工具结果添加到对话历史
        {
            let mut state = self.state.write().await;
            state.status = AgentStatus::Thinking;
            state.conversation.push(json!(result));
        }

        Ok(())
    }

    /// 获取当前状态
    #[allow(dead_code)]
    pub async fn get_status(&self) -> AgentStatus {
        let state = self.state.read().await;
        state.status
    }

    /// 重置对话
    #[allow(dead_code)]
    pub async fn reset(&mut self) {
        let mut state = self.state.write().await;
        state.status = AgentStatus::Idle;
        state.conversation.clear();
        self.current_turn = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let model_client = ModelClient::new(
            "test-key".to_string(),
            "gpt-4".to_string(),
        );
        let agent = Agent::new(model_client);

        let status = agent.get_status().await;
        assert_eq!(status, AgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_agent_reset() {
        let model_client = ModelClient::new(
            "test-key".to_string(),
            "gpt-4".to_string(),
        );
        let mut agent = Agent::new(model_client);

        // 先添加一些对话
        {
            let mut state = agent.state.write().await;
            state.conversation.push(json!(UserMessage {
                content: "hello".to_string(),
            }));
        }

        agent.reset().await;

        let state = agent.state.read().await;
        assert!(state.conversation.is_empty());
        assert_eq!(state.status, AgentStatus::Idle);
    }
}
