// 库入口文件 - 导出公共 API

pub mod agent;
pub mod client;
pub mod protocol;
pub mod tools;
pub mod flight_tools;

// 重新导出常用类型
pub use agent::Agent;
pub use client::ModelClient;
pub use protocol::{AgentStatus, AssistantMessage, ToolCall, ToolResult, UserMessage};
pub use flight_tools::{GetFlightNumberTool, GetTicketPriceTool};

// 内部重新导出以方便内部使用
pub use flight_tools::{GetFlightNumberTool as FlightNumberTool, GetTicketPriceTool as TicketPriceTool};
