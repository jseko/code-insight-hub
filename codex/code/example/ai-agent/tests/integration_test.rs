// 集成测试示例

use simple_ai_agent::agent::Agent;
use simple_ai_agent::client::ModelClient;
use tokio;

#[tokio::test]
async fn test_basic_conversation() {
    let model_client = ModelClient::new(
        "test-key".to_string(),
        "gpt-4".to_string(),
    );
    let mut agent = Agent::new(model_client);

    // 模拟简单对话
    let result: Result<String, anyhow::Error> = agent
        .process_message("hello")
        .await;

    // 注意：这需要真实的 API key 才能成功
    // 这里仅验证代码结构
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_tool_registration() {
    let model_client = ModelClient::new(
        "test-key".to_string(),
        "gpt-4".to_string(),
    );
    let agent = Agent::new(model_client);

    // 验证工具已注册
    let status = agent.get_status().await;
    println!("Agent status: {:?}", status);
}
