use crabjar_plugin::Plugin;
use crabjar_structured_aux::StructuredAuxPlugin;

async fn test_approval() {
    println!("=== Testing Approval Decision ===");
    
    // Use Qwen2.5-Coder-1.5B on ftw3:8081 for approval decisions
    let plugin = StructuredAuxPlugin::with_config(
        Some("http://ftw3:8081".to_string()),
        Some("Qwen2.5-Coder-1.5B-Instruct-Q4_K_M".to_string()),
        None,
    );
    
    let args = serde_json::json!({
        "request": "Deploy application to production environment",
        "policy": "Require approval for all prod deployments"
    });
    
    match plugin.execute_tool("structured-aux.approval", args).await {
        Ok(result) => {
            println!("Approval result: {}", result.stdout);
            println!("Exit code: {}", result.exit_code);
        }
        Err(e) => {
            println!("Approval error: {}", e);
        }
    }
}

async fn test_compression() {
    println!("\n=== Testing Compression ===");
    
    // Use Kev-9B on ftw3:34159 for compression tasks
    let plugin = StructuredAuxPlugin::with_config(
        Some("http://ftw3:34159".to_string()),
        Some("/home/crombo/.cache/huggingface/hub/models--ggml-org--Kev-9B-GGUF/Kev-9B-Q4_K_M.gguf".to_string()),
        None,
    );
    
    let args = serde_json::json!({
        "text": "This is a very long context that needs to be compressed. It contains important information about the system architecture, deployment procedures, and configuration settings that should be preserved in a compact form."
    });
    
    match plugin.execute_tool("structured-aux.compression", args).await {
        Ok(result) => {
            println!("Compression result: {}", result.stdout);
            println!("Exit code: {}", result.exit_code);
        }
        Err(e) => {
            println!("Compression error: {}", e);
        }
    }
}

async fn test_mcp_selection() {
    println!("\n=== Testing MCP Tool Selection ===");
    
    // Use Qwen2.5-Coder-1.5B on ftw3:8081 for tool selection
    let plugin = StructuredAuxPlugin::with_config(
        Some("http://ftw3:8081".to_string()),
        Some("Qwen2.5-Coder-1.5B-Instruct-Q4_K_M".to_string()),
        None,
    );
    
    let args = serde_json::json!({
        "request": "Get current weather in Chicago",
        "tools": [
            {"name": "get_weather", "description": "Fetch weather data"},
            {"name": "search_web", "description": "Search the web"}
        ]
    });
    
    match plugin.execute_tool("structured-aux.mcp", args).await {
        Ok(result) => {
            println!("MCP selection result: {}", result.stdout);
            println!("Exit code: {}", result.exit_code);
        }
        Err(e) => {
            println!("MCP selection error: {}", e);
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Testing crabjar-structured-aux with local models\n");
    
    test_approval().await;
    test_compression().await;
    test_mcp_selection().await;
    test_risk_assessment().await;
    test_priority_ranking().await;
    test_retry_decision().await;
    test_resource_allocation().await;
    
    println!("\n=== All tests complete ===");
}

async fn test_risk_assessment() {
    println!("\n=== Testing Risk Assessment ===");
    
    let plugin = StructuredAuxPlugin::with_config(
        Some("http://ftw3:8081".to_string()),
        Some("Qwen2.5-Coder-1.5B-Instruct-Q4_K_M".to_string()),
        None,
    );
    
    let args = serde_json::json!({
        "action": "Delete all database records",
        "context": "Production environment with active users"
    });
    
    match plugin.execute_tool("structured-aux.risk", args).await {
        Ok(result) => {
            println!("Risk result: {}", result.stdout);
            println!("Exit code: {}", result.exit_code);
        }
        Err(e) => {
            println!("Risk error: {}", e);
        }
    }
}

async fn test_priority_ranking() {
    println!("\n=== Testing Priority Ranking ===");
    
    let plugin = StructuredAuxPlugin::with_config(
        Some("http://ftw3:8081".to_string()),
        Some("Qwen2.5-Coder-1.5B-Instruct-Q4_K_M".to_string()),
        None,
    );
    
    let args = serde_json::json!({
        "tasks": [
            {"name": "Fix login bug blocking all users"},
            {"name": "Update documentation"},
            {"name": "Refactor database queries"}
        ]
    });
    
    match plugin.execute_tool("structured-aux.priority", args).await {
        Ok(result) => {
            println!("Priority result: {}", result.stdout);
            println!("Exit code: {}", result.exit_code);
        }
        Err(e) => {
            println!("Priority error: {}", e);
        }
    }
}

async fn test_retry_decision() {
    println!("\n=== Testing Retry Decision ===");
    
    let plugin = StructuredAuxPlugin::with_config(
        Some("http://ftw3:8081".to_string()),
        Some("Qwen2.5-Coder-1.5B-Instruct-Q4_K_M".to_string()),
        None,
    );
    
    let args = serde_json::json!({
        "error": "Connection timeout to database server",
        "attempts": 2
    });
    
    match plugin.execute_tool("structured-aux.retry", args).await {
        Ok(result) => {
            println!("Retry result: {}", result.stdout);
            println!("Exit code: {}", result.exit_code);
        }
        Err(e) => {
            println!("Retry error: {}", e);
        }
    }
}

async fn test_resource_allocation() {
    println!("\n=== Testing Resource Allocation ===");
    
    let plugin = StructuredAuxPlugin::with_config(
        Some("http://ftw3:8081".to_string()),
        Some("Qwen2.5-Coder-1.5B-Instruct-Q4_K_M".to_string()),
        None,
    );
    
    let args = serde_json::json!({
        "request": "Need GPU for training a large language model",
        "resources": [
            {"name": "cpu-workstation"},
            {"name": "gpu-training-node"},
            {"name": "cloud-spot-instance"}
        ]
    });
    
    match plugin.execute_tool("structured-aux.resource", args).await {
        Ok(result) => {
            println!("Resource result: {}", result.stdout);
            println!("Exit code: {}", result.exit_code);
        }
        Err(e) => {
            println!("Resource error: {}", e);
        }
    }
}