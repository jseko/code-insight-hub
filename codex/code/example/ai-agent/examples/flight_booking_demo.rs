// 航班查询演示 - 基于 ChatGLM 函数调用教程
//
// 这个示例展示了如何使用 ChatGLM 的函数调用功能来实现航班查询系统
// 对应教程中的完整流程：
// 1. 定义工具 (get_flight_number, get_ticket_price)
// 2. 与模型交互，触发工具调用
// 3. 使用模型生成的参数调用外部函数
// 4. 将结果返回给模型，生成自然语言回复

use simple_ai_agent::{Agent, ModelClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载 .env 文件
    dotenv::dotenv().ok();

    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🛫 航班查询系统启动...\n");
    println!("═════════════════════════════════════════════\n");

    // 从环境变量获取配置
    let api_key = std::env::var("OPENAI_API_KEY")
        .unwrap_or_else(|_| {
            eprintln!("⚠️  警告: 未设置 OPENAI_API_KEY 环境变量");
            std::process::exit(1);
        });

    let model = std::env::var("MODEL").unwrap_or_else(|_| "glm-4-tools".to_string());
    let base_url = std::env::var("API_BASE_URL")
        .unwrap_or_else(|_| "https://open.bigmodel.cn/api/paas/v4/".to_string());

    // 创建模型客户端
    let model_client = ModelClient::new_with_config(api_key, model, base_url);

    // 创建智能体
    let mut agent = Agent::new(model_client);

    println!("💡 智能体就绪，可以开始查询航班信息\n");
    println!("═════════════════════════════════════════════\n");
    println!("📚 可用功能：");
    println!("  • 查询航班号：从某地到某地的航班");
    println!("  • 查询票价：某航班在某日的价格");
    println!("  • 获取当前时间");
    println!("  • 执行 shell 命令");
    println!("  • 读取文件内容");
    println!("═════════════════════════════════════════════\n");

    // 演示对话流程
    let demo_queries = vec![
        "帮我查询2024年1月20日从北京前往上海的航班",
        "这趟航班的价格是多少？",
    ];

    println!("🎯 自动演示模式：\n");

    for (i, query) in demo_queries.iter().enumerate() {
        println!("👤 用户查询 {}: {}", i + 1, query);
        print!("🤖 智能体回复: ");
        std::io::stdout().flush()?;

        match agent.process_message_stream_with_result(query, |chunk| {
            print!("{}", chunk);
            std::io::stdout().flush().ok();
        }).await {
            Ok(_) => {
                println!("\n");
            }
            Err(e) => {
                eprintln!("\n❌ 错误: {}", e);
            }
        }

        println!("─────────────────────────────────────────────\n");

        // 在演示之间稍作暂停
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    println!("═════════════════════════════════════════════");
    println!("✨ 演示完成！");
    println!("═════════════════════════════════════════════\n");

    // 进入交互模式
    println!("🎮 现在可以输入自己的查询（输入 'quit' 退出）：\n");

    loop {
        print!("👤 You: ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("quit") {
            println!("\n👋 再见！");
            break;
        }

        if input.is_empty() {
            continue;
        }

        print!("\n🤖 Agent: ");
        std::io::stdout().flush()?;

        match agent.process_message_stream_with_result(input, |chunk| {
            print!("{}", chunk);
            std::io::stdout().flush().ok();
        }).await {
            Ok(_) => {
                println!("\n");
            }
            Err(e) => {
                eprintln!("\n❌ 错误: {}", e);
            }
        }

        println!("─────────────────────────────────────────────\n");
    }

    Ok(())
}
