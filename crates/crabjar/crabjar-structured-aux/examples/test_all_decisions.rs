use crabjar_plugin::Plugin;
use crabjar_structured_aux::StructuredAuxPlugin;
use std::env;

#[tokio::main]
async fn main() {
    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_default();
    let plugin = StructuredAuxPlugin::new(Some(api_key));

    // Test 1: Risk Assessment
    println!("=== Testing Risk Assessment ===");
    let risk_result = plugin
        .execute_tool(
            "structured-aux.risk",
            serde_json::json!({
                "action": "Delete all database records",
                "context": "Production environment with active users"
            }),
        )
        .await;
    println!("Risk result: {:?}", risk_result);

    // Test 2: Priority Ranking
    println!("\n=== Testing Priority Ranking ===");
    let priority_result = plugin
        .execute_tool(
            "structured-aux.priority",
            serde_json::json!({
                "tasks": [
                    {"name": "Fix login bug blocking all users"},
                    {"name": "Update documentation"},
                    {"name": "Refactor database queries"}
                ]
            }),
        )
        .await;
    println!("Priority result: {:?}", priority_result);

    // Test 3: Retry Decision
    println!("\n=== Testing Retry Decision ===");
    let retry_result = plugin
        .execute_tool(
            "structured-aux.retry",
            serde_json::json!({
                "error": "Connection timeout to database server",
                "attempts": 2
            }),
        )
        .await;
    println!("Retry result: {:?}", retry_result);

    // Test 4: Resource Allocation
    println!("\n=== Testing Resource Allocation ===");
    let resource_result = plugin
        .execute_tool(
            "structured-aux.resource",
            serde_json::json!({
                "request": "Need GPU for training a large language model",
                "resources": [
                    {"name": "cpu-workstation"},
                    {"name": "gpu-training-node"},
                    {"name": "cloud-spot-instance"}
                ]
            }),
        )
        .await;
    println!("Resource result: {:?}", resource_result);

    // Test 5: Approval (existing)
    println!("\n=== Testing Approval ===");
    let approval_result = plugin
        .execute_tool(
            "structured-aux.approval",
            serde_json::json!({
                "request": "Deploy to production",
                "policy": "All prod deploys require review"
            }),
        )
        .await;
    println!("Approval result: {:?}", approval_result);

    // Test 6: MCP Tool Selection (existing)
    println!("\n=== Testing MCP Tool Selection ===");
    let mcp_result = plugin
        .execute_tool(
            "structured-aux.mcp",
            serde_json::json!({
                "request": "What's the weather in Chicago?",
                "tools": [
                    {"name": "get_weather"},
                    {"name": "search_web"},
                    {"name": "send_email"}
                ]
            }),
        )
        .await;
    println!("MCP result: {:?}", mcp_result);

    // Test 7: Compression (existing)
    println!("\n=== Testing Compression ===");
    let compression_result = plugin
        .execute_tool(
            "structured-aux.compression",
            serde_json::json!({
                "text": "The system architecture consists of multiple microservices communicating via REST APIs. Deployment is handled by Kubernetes with automated scaling. Configuration is managed through environment variables and config maps."
            }),
        )
        .await;
    println!("Compression result: {:?}", compression_result);

    println!("\n=== All tests complete ===");
}
