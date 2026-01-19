// 简易版 AI 智能体实现
// 基于 Codex 架构，简化了核心功能

mod agent;
mod client;
mod tools;
mod protocol;
mod flight_tools;

use agent::Agent;
use client::ModelClient;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载 .env 文件
    dotenv::dotenv().ok();

    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🦊 灵狐 AI Agent 启动中...\n");

    // 从环境变量获取配置
    let api_key = env::var("OPENAI_API_KEY")
        .unwrap_or_else(|_| {
            eprintln!("⚠️  警告: 未设置 OPENAI_API_KEY 环境变量");
            eprintln!("⚠️  请在 .env 文件中设置或导出环境变量");
            std::process::exit(1);
        });

    let model = env::var("MODEL")
        .unwrap_or_else(|_| {
            eprintln!("⚠️  未设置 MODEL 环境变量，使用默认模型");
            "glm-4-tools".to_string()
        });

    let base_url = env::var("API_BASE_URL")
        .unwrap_or_else(|_| "https://open.bigmodel.cn/api/paas/v4/".to_string());

    // 创建模型客户端
    let model_client = ModelClient::new_with_config(
        api_key,
        model,
        base_url,
    );

    // 创建智能体
    let mut agent = Agent::new(model_client);

    println!("💡 智能体就绪，输入消息开始对话（输入 'quit' 退出）\n");
    println!("─────────────────────────────────────────────\n");

    // 主循环
    loop {
        print!("👤 You: ");
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // 退出命令（只显式 quit，空输入继续等待）
        if input.eq_ignore_ascii_case("quit") {
            println!("\n👋 再见！");
            break;
        }

        if input.is_empty() {
            continue; // 空输入跳过，不退出
        }

        print!("\n🤖 Agent: ");
        std::io::stdout().flush()?;

        // 处理用户输入（流式输出）
        match agent.process_message_stream_with_result(input, |chunk| {
            print!("{}", chunk);
            std::io::stdout().flush().ok();
        }).await {
            Ok(_) => {
                println!("\n");
                println!("─────────────────────────────────────────────\n");
            }
            Err(e) => {
                eprintln!("\n❌ 错误: {}", e);
                println!("─────────────────────────────────────────────\n");
            }
        }
    }

    Ok(())
}
