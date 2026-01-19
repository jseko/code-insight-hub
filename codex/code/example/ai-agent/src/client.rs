// 模型客户端实现 - 完善的流式版本

use futures::TryStreamExt;
use reqwest::Client as ReqwestClient;
use serde_json::{json, Value};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// SSE 事件类型
#[derive(Debug, Clone, PartialEq)]
pub enum SseEvent {
    TextDelta(String),
    ReasoningDelta(String),
    ToolCalls(Vec<crate::protocol::ToolCall>),
    Done,
}

/// 简化版模型客户端
pub struct ModelClient {
    api_key: String,
    model: String,
    client: ReqwestClient,
    base_url: String,
}

impl ModelClient {
    /// 创建模型客户端（自定义配置）
    pub fn new_with_config(api_key: String, model: String, base_url: String) -> Self {
        Self {
            api_key,
            model,
            client: ReqwestClient::new(),
            base_url,
        }
    }

    /// 发送消息并获取完整响应（非流式版本）
    #[allow(dead_code)]
    pub async fn chat_completion(
        &self,
        messages: Vec<Value>,
        tools: Option<Vec<Value>>,
    ) -> Result<ChatResponse, anyhow::Error> {
        // 转换消息格式以兼容智谱 API
        let formatted_messages = format_messages(messages);

        let mut request_body = json!({
            "model": self.model,
            "messages": formatted_messages,
            "stream": false
        });

        // 添加工具定义（智谱 AI 支持）
        if let Some(tools) = tools {
            if !tools.is_empty() {
                request_body["tools"] = json!(tools);
            }
        }

        // 发送请求
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(120))
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            eprintln!("⚠️  API 请求详情: {}", error_text);
            return Err(anyhow::anyhow!(
                "API 请求失败 ({}): {}",
                status,
                error_text
            ));
        }

        let response_json: Value = response.json().await?;
        self.parse_response(response_json)
    }

    /// 发送消息并获取流式响应（真正的异步流式）
    pub async fn chat_completion_stream(
        &self,
        messages: Vec<Value>,
        tools: Option<Vec<Value>>,
    ) -> Result<ResponseStream, anyhow::Error> {
        // 转换消息格式
        let formatted_messages = format_messages(messages);

        let mut request_body = json!({
            "model": self.model,
            "messages": formatted_messages,
            "stream": true,
            "tool_choice": "auto"
        });

        // 添加工具定义
        if let Some(tools) = tools {
            if !tools.is_empty() {
                request_body["tools"] = json!(tools);
            }
        }

        //println!("\n📤 请求体: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default());

        // 发送请求
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(120))
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "API 请求失败 ({}): {}",
                status,
                error_text
            ));
        }

        // 创建流式响应
        Ok(ResponseStream::new(response))
    }

    /// 解析 API 响应
    #[allow(dead_code)]
    fn parse_response(&self, response: Value) -> Result<ChatResponse, anyhow::Error> {
        let assistant = response["choices"][0]["message"].clone();

        // 检查是否有工具调用
        let tool_calls = assistant["tool_calls"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|call| {
                    let id = call["id"].as_str()?;
                    let name = call["function"]["name"].as_str()?;
                    let args = call["function"]["arguments"].clone();
                    Some(crate::protocol::ToolCall {
                        id: id.to_string(),
                        name: name.to_string(),
                        arguments: args,
                    })
                })
                .collect()
        });

        let content = assistant["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse {
            content,
            tool_calls,
            finish_reason: response["choices"][0]["finish_reason"]
                .as_str()
                .unwrap_or("stop")
                .to_string(),
        })
    }
}

/// 响应流（实现 Stream trait）
pub struct ResponseStream {
    byte_stream: Pin<Box<dyn futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    buffer: Vec<u8>,
    completed: bool,
    // 累积工具调用部分数据（用于流式工具调用解析）
    tool_call_buffer: std::collections::HashMap<String, PartialToolCall>,
}

/// 部分工具调用数据（用于累积流式工具调用）
#[derive(Debug, Default)]
struct PartialToolCall {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

impl ResponseStream {
    fn new(response: reqwest::Response) -> Self {
        // 创建字节流
        let byte_stream = Box::pin(
            response.bytes_stream()
                .map_err(|e| reqwest::Error::from(e))
        );

        Self {
            byte_stream,
            buffer: Vec::new(),
            completed: false,
            tool_call_buffer: std::collections::HashMap::new(),
        }
    }
}

impl futures::Stream for ResponseStream {
    type Item = Result<SseEvent, anyhow::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut();

        if this.completed {
            return Poll::Ready(None);
        }

        // 轮询底层字节流
        match this.byte_stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                // 处理收到的字节
                this.buffer.extend_from_slice(&bytes);

                // 逐行处理
                while let Some(pos) = this.buffer.iter().position(|&b| b == b'\n') {
                    let line_bytes = this.buffer.drain(..=pos).collect::<Vec<_>>();
                    // 只有当 buffer 不为空时才移除换行符
                    if !this.buffer.is_empty() {
                        this.buffer.remove(0);
                    }

                    // 转换为字符串
                    if let Ok(line) = String::from_utf8(line_bytes) {
                        // 处理 SSE 事件
                        if let Some(event) = this.parse_sse_line(&line) {
                            return Poll::Ready(Some(Ok(event)));
                        }
                    }
                }

                Poll::Pending
            }
            Poll::Ready(None) => {
                // 流结束，检查是否有未完成的行
                if !this.buffer.is_empty() {
                    if let Ok(line) = String::from_utf8(std::mem::take(&mut this.buffer)) {
                        if let Some(event) = this.parse_sse_line(&line) {
                            return Poll::Ready(Some(Ok(event)));
                        }
                    }
                }
                this.completed = true;
                Poll::Ready(None)
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(anyhow::anyhow!("流读取错误: {}", e))))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl ResponseStream {
    /// 解析 SSE 行
    fn parse_sse_line(&mut self, line: &str) -> Option<SseEvent> {
        // SSE 格式: data: {...}
        if !line.starts_with("data: ") {
            return None;
        }

        let json_str = &line[6..];

        // 检查结束标记
        if json_str == "[DONE]" || json_str == "DONE" {
            return Some(SseEvent::Done);
        }

        // 解析 JSON
        if let Ok(json_value) = serde_json::from_str::<Value>(json_str) {
            let choices = &json_value["choices"];
            if !choices.is_array() || choices.as_array().unwrap().is_empty() {
                return None;
            }

            let delta = &choices[0]["delta"];

            // 智谱 AI 使用 reasoning_content 字段
            if let Some(reasoning) = delta["reasoning_content"].as_str() {
                if !reasoning.is_empty() {
                    return Some(SseEvent::ReasoningDelta(reasoning.to_string()));
                }
            }

            // 检查 content 字段（兼容性）
            if let Some(content) = delta["content"].as_str() {
                if !content.is_empty() {
                    return Some(SseEvent::TextDelta(content.to_string()));
                }
            }

            // 检查工具调用 - 智谱 AI 的工具调用是流式分片的
            if let Some(calls) = delta["tool_calls"].as_array() {
                if !calls.is_empty() {
                    for call in calls {
                        // 获取工具调用索引
                        let index = call.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                        let index_str = index.to_string();

                        // 累积工具调用数据
                        let partial = self.tool_call_buffer.entry(index_str.clone()).or_default();

                        // 累积 ID
                        if let Some(id) = call.get("id").and_then(|v| v.as_str()) {
                            partial.id = Some(id.to_string());
                        }

                        // 累积函数名
                        if let Some(func) = call.get("function") {
                            if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                partial.name = Some(name.to_string());
                            }

                            // 累积参数（可能分多次到达）
                            if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                partial.arguments.push_str(args);
                            }
                        }

                        // 检查工具调用是否完成（finish_reason 为 "tool_calls"）
                        if let Some(finish_reason) = json_value["choices"][0].get("finish_reason").and_then(|v| v.as_str()) {
                            if finish_reason == "tool_calls" {
                                // 构建完整的工具调用列表
                                let mut tool_calls: Vec<crate::protocol::ToolCall> = Vec::new();

                                for (_idx, partial) in self.tool_call_buffer.drain() {
                                    if let (Some(id), Some(name)) = (partial.id, partial.name) {
                                        // 解析参数
                                        let arguments = if partial.arguments.is_empty() {
                                            serde_json::json!({})
                                        } else if let Ok(json) = serde_json::from_str::<serde_json::Value>(&partial.arguments) {
                                            json
                                        } else {
                                            serde_json::json!({"raw": partial.arguments})
                                        };

                                        tool_calls.push(crate::protocol::ToolCall {
                                            id,
                                            name,
                                            arguments,
                                        });
                                    }
                                }

                                if !tool_calls.is_empty() {
                                    println!("\n✅ 解析工具调用: {} 个", tool_calls.len());
                                    for tc in &tool_calls {
                                        println!("  - {} ({})", tc.name, tc.id);
                                    }
                                    return Some(SseEvent::ToolCalls(tool_calls));
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

/// 聊天响应
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ChatResponse {
    pub content: String,
    pub tool_calls: Option<Vec<crate::protocol::ToolCall>>,
    #[allow(dead_code)]
    pub finish_reason: String,
}

/// 格式化消息列表
fn format_messages(messages: Vec<Value>) -> Vec<Value> {
    messages.iter().filter_map(|msg| {
        // 用户消息
        if msg.get("content").is_some() && msg.get("tool_call_id").is_none() && msg.get("tool_calls").is_none() {
            Some(json!({
                "role": "user",
                "content": msg["content"]
            }))
        }
        // 助手消息（可能包含工具调用）
        else if msg.get("content").is_some() || msg.get("tool_calls").is_some() {
            let mut msg_obj = json!({
                "role": "assistant",
                "content": msg["content"].as_str().unwrap_or("")
            });
            if let Some(tool_calls) = msg.get("tool_calls") {
                let converted_tool_calls: Vec<Value> = tool_calls.as_array()
                    .map(|arr| arr.iter().map(|call| {
                        json!({
                            "id": call["id"],
                            "type": "function",
                            "function": {
                                "name": call["name"],
                                "arguments": call["arguments"]
                            }
                        })
                    }).collect())
                    .unwrap_or_default();
                msg_obj["tool_calls"] = json!(converted_tool_calls);
            }
            Some(msg_obj)
        }
        // 工具返回消息
        else if msg.get("tool_call_id").is_some() {
            Some(json!({
                "role": "tool",
                "content": msg["content"],
                "tool_call_id": msg["tool_call_id"]
            }))
        } else {
            None
        }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_client_creation() {
        let client = ModelClient::new(
            "test-key".to_string(),
            "gpt-4".to_string(),
        );

        assert_eq!(client.model, "gpt-4");
        assert_eq!(client.api_key, "test-key");
    }
}
