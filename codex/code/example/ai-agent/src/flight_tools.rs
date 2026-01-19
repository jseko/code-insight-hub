// 航班查询工具示例 - 基于 ChatGLM 函数调用教程

use crate::tools::ToolExecutor;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;

/// 查询航班号工具
/// 对应 ChatGLM 教程中的: get_flight_number(departure: str, destination: str, date: str)
pub struct GetFlightNumberTool {
    // 模拟的航班数据库
    flights: HashMap<String, HashMap<String, String>>,
}

impl GetFlightNumberTool {
    pub fn new() -> Self {
        let mut flights: HashMap<String, HashMap<String, String>> = HashMap::new();

        // 北京出发
        let mut beijing = HashMap::new();
        beijing.insert("上海".to_string(), "1234".to_string());
        beijing.insert("广州".to_string(), "8321".to_string());
        beijing.insert("深圳".to_string(), "5678".to_string());
        flights.insert("北京".to_string(), beijing);

        // 上海出发
        let mut shanghai = HashMap::new();
        shanghai.insert("北京".to_string(), "1233".to_string());
        shanghai.insert("广州".to_string(), "8123".to_string());
        shanghai.insert("深圳".to_string(), "5432".to_string());
        flights.insert("上海".to_string(), shanghai);

        // 广州出发
        let mut guangzhou = HashMap::new();
        guangzhou.insert("北京".to_string(), "8322".to_string());
        guangzhou.insert("上海".to_string(), "8124".to_string());
        guangzhou.insert("深圳".to_string(), "3456".to_string());
        flights.insert("广州".to_string(), guangzhou);

        Self { flights }
    }
}

impl Default for GetFlightNumberTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolExecutor for GetFlightNumberTool {
    fn name(&self) -> &str {
        "get_flight_number"
    }

    fn description(&self) -> &str {
        "根据始发地、目的地和日期，查询对应日期的航班号"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "departure": {
                    "description": "出发地",
                    "type": "string"
                },
                "destination": {
                    "description": "目的地",
                    "type": "string"
                },
                "date": {
                    "description": "日期（格式：YYYY-MM-DD）",
                    "type": "string"
                }
            },
            "required": ["departure", "destination", "date"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String, Box<dyn std::error::Error + Send>> {
        println!("\n✈️  查询航班号工具接收到参数: {}", arguments);

        let departure = arguments["departure"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少 'departure' 参数"))?;

        let destination = arguments["destination"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少 'destination' 参数"))?;

        let _date = arguments["date"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少 'date' 参数"))?;

        // 从数据库查询航班号
        if let Some(destinations) = self.flights.get(departure) {
            if let Some(flight_number) = destinations.get(destination) {
                let result = json!({
                    "flight_number": flight_number,
                    "departure": departure,
                    "destination": destination
                });

                println!("✓ 查询成功: 航班号 {}", flight_number);
                return Ok(serde_json::to_string(&result)
                    .map_err(|e| anyhow::anyhow!("JSON 序列化失败: {}", e))?);
            }
        }

        let error = format!("未找到从 {} 到 {} 的航班", departure, destination);
        println!("✗ {}", error);
        Err(anyhow::anyhow!(error).into())
    }
}

/// 查询航班票价工具
/// 对应 ChatGLM 教程中的: get_ticket_price(flight_number: str, date: str)
pub struct GetTicketPriceTool;

impl Default for GetTicketPriceTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl ToolExecutor for GetTicketPriceTool {
    fn name(&self) -> &str {
        "get_ticket_price"
    }

    fn description(&self) -> &str {
        "查询某航班在某日的票价"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "flight_number": {
                    "description": "航班号",
                    "type": "string"
                },
                "date": {
                    "description": "日期（格式：YYYY-MM-DD）",
                    "type": "string"
                }
            },
            "required": ["flight_number", "date"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String, Box<dyn std::error::Error + Send>> {
        println!("\n💰 查询票价工具接收到参数: {}", arguments);

        let flight_number = arguments["flight_number"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少 'flight_number' 参数"))?;

        // 模拟票价查询（实际应用中应该查询数据库或 API）
        let price = match flight_number {
            "1234" | "1233" => 1500,
            "8321" | "8322" => 1200,
            "8123" | "8124" => 1300,
            "5678" | "5432" => 1100,
            "3456" => 1000,
            _ => 800,
        };

        let result = json!({
            "ticket_price": price,
            "flight_number": flight_number,
            "currency": "CNY"
        });

        println!("✓ 查询成功: 票价 {} 元", price);
        Ok(serde_json::to_string(&result)
            .map_err(|e| anyhow::anyhow!("JSON 序列化失败: {}", e))?)
    }
}

/// 注册所有航班工具的辅助函数
pub fn register_flight_tools(registry: &mut crate::tools::ToolRegistry) {
    println!("\n🛫 注册航班查询工具...");
    registry.register(GetFlightNumberTool::new());
    registry.register(GetTicketPriceTool::default());
    println!("  ✅ 航班查询工具注册完成\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_flight_number() {
        let tool = GetFlightNumberTool::new();

        let args = json!({
            "departure": "北京",
            "destination": "上海",
            "date": "2024-01-20"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("1234"));
    }

    #[tokio::test]
    async fn test_get_ticket_price() {
        let tool = GetTicketPriceTool;

        let args = json!({
            "flight_number": "1234",
            "date": "2024-01-20"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("1500"));
    }
}
