// 工具系统实现

use crate::protocol::{ToolCall, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

/// 工具执行器 trait（类似 Codex 的 ToolHandler）
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    fn name(&self) -> &str;

    fn description(&self) -> &str;

    fn parameters(&self) -> serde_json::Value;

    #[allow(dead_code)]
    async fn execute(&self, arguments: serde_json::Value) -> Result<String, Box<dyn std::error::Error + Send>>;
}

/// 工具注册表（简化版 ToolRegistry）
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolExecutor>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T>(&mut self, tool: T)
    where
        T: ToolExecutor + 'static,
    {
        let name = tool.name().to_string();
        println!("  ✅ 注册工具: {}", name);
        self.tools.insert(name, Box::new(tool));
    }

    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&dyn ToolExecutor> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn list_definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters().clone(),
            })
            .collect()
    }

    #[allow(dead_code)]
    pub async fn execute(&self, call: &ToolCall) -> Result<ToolResult, String> {
        let executor = self
            .get(&call.name)
            .ok_or_else(|| format!("工具 '{}' 未找到", call.name))?;

        // 解析参数：处理智谱 API 返回的 JSON 字符串
        let parsed_args = if call.arguments.is_string() {
            serde_json::from_str::<serde_json::Value>(call.arguments.as_str().unwrap_or("{}"))
                .map_err(|e| format!("参数解析失败: {}", e))?
        } else {
            call.arguments.clone()
        };

        let result = executor
            .execute(parsed_args)
            .await
            .map_err(|e| format!("工具执行失败: {}", e))?;

        Ok(ToolResult {
            tool_call_id: call.id.clone(),
            content: result,
        })
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ========== 内置工具实现 ==========

/// Shell 命令执行工具
pub struct ShellTool;

#[async_trait]
impl ToolExecutor for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command on macOS/Linux. Use 'ifconfig' or 'ip addr' for network information, not 'hostname -I' which may not work on macOS."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute. Use 'curl ifconfig.me' to get public IP, 'ifconfig' for local network info."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, arguments: serde_json::Value) -> Result<String, Box<dyn std::error::Error + Send>> {
        println!("\n🔧 Shell 工具接收到参数: {}", arguments); // 调试输出

        let command = arguments["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少 'command' 参数"))?;

        println!("🔧 执行命令: {}", command);

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("命令执行失败: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            println!("✓ 命令执行成功");
            Ok(stdout)
        } else {
            let error = if stderr.is_empty() {
                anyhow::anyhow!("命令失败 (退出码: {:?})", output.status.code())
            } else {
                anyhow::anyhow!("{}", stderr)
            };
            println!("✗ {}", error);
            Err(error.into())
        }
    }
}

/// 当前时间工具
pub struct CurrentTimeTool;

#[async_trait]
impl ToolExecutor for CurrentTimeTool {
    fn name(&self) -> &str {
        "current_time"
    }

    fn description(&self) -> &str {
        "Get current date and time"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _arguments: serde_json::Value) -> Result<String, Box<dyn std::error::Error + Send>> {
        use chrono::Local;

        let now = Local::now();
        let time_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        Ok(time_str)
    }
}

/// 文件读取工具
pub struct ReadFileTool;

#[async_trait]
impl ToolExecutor for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read contents of a text file"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, arguments: serde_json::Value) -> Result<String, Box<dyn std::error::Error + Send>> {
        println!("\n📄 ReadFile 工具接收到参数: {}", arguments); // 调试输出

        let path = arguments["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少 'path' 参数"))?;

        println!("📄 读取文件: {}", path);

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| anyhow::anyhow!("读取文件失败: {}", e))?;

        let preview = if content.len() > 200 {
            format!("{}... (总 {} 字符)", &content[..200], content.len())
        } else {
            content.clone()
        };

        println!("✓ 文件读取成功 ({} 字符)", content.len());
        Ok(preview)
    }
}

/// 获取帮助工具
pub struct HelpTool {
    #[allow(dead_code)]
    available_tools: Vec<String>,
}

impl HelpTool {
    pub fn new(available_tools: Vec<String>) -> Self {
        Self { available_tools }
    }
}

#[async_trait]
impl ToolExecutor for HelpTool {
    fn name(&self) -> &str {
        "help"
    }

    fn description(&self) -> &str {
        "List all available tools and their descriptions"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _arguments: serde_json::Value) -> Result<String, Box<dyn std::error::Error + Send>> {
        let mut help_text = "📚 可用工具:\n".to_string();
        
        for tool_name in &self.available_tools {
            let desc = match tool_name.as_str() {
                "shell" => "执行 shell 命令",
                "current_time" => "获取当前日期和时间",
                "read_file" => "读取文本文件内容",
                "help" => "列出所有可用工具",
                _ => "未知工具",
            };
            help_text.push_str(&format!("  • {}: {}\n", tool_name, desc));
        }
        
        Ok(help_text)
    }
}
