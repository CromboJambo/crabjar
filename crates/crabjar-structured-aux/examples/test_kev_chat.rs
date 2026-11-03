use reqwest::Client;
use serde_json::json;

async fn test_kev_chat_completion(client: &Client) {
    println!("=== Test 1: Kev-9B Chat Completion ===");
    
    let response = client
        .post("http://ftw3:34159/v1/chat/completions")
        .header("Content-Type", "application/json")
        .body(json!({
            "model": "/home/crombo/.cache/huggingface/hub/models--ggml-org--Kev-9B-GGUF/Kev-9B-Q4_K_M.gguf",
            "messages": [
                {"role": "user", "content": "What is the capital of France?"}
            ],
            "temperature": 0.1,
            "max_tokens": 50
        })
        .to_string())
        .send()
        .await;
        
    if response.is_ok() {
        let resp = response.unwrap();
        let status = resp.status();
        let body = resp.text().await.unwrap();
        println!("Status: {}", status);
        println!("Response: {}", body);
    } else {
        println!("✗ Request failed: {:?}", response.err());
    }
}

async fn test_kev_structured_prompt(client: &Client) {
    println!("\n=== Test 2: Kev-9B with Structured Prompt ===");
    
    let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
    
    let response = client
        .post("http://ftw3:34159/v1/chat/completions")
        .header("Content-Type", "application/json")
        .body(json!({
            "model": "/home/crombo/.cache/huggingface/hub/models--ggml-org--Kev-9B-GGUF/Kev-9B-Q4_K_M.gguf",
            "messages": [
                {"role": "user", "content": format!("Analyze this code: {}", code)}
            ],
            "temperature": 0.1,
            "max_tokens": 50
        })
        .to_string())
        .send()
        .await;
        
    if response.is_ok() {
        let resp = response.unwrap();
        let status = resp.status();
        let body = resp.text().await.unwrap();
        println!("Status: {}", status);
        println!("Response: {}", body);
    } else {
        println!("✗ Request failed: {:?}", response.err());
    }
}

#[tokio::main]
async fn main() {
    println!("Testing Kev-9B chat completion capabilities\n");
    
    let client = Client::new();
    
    test_kev_chat_completion(&client).await;
    test_kev_structured_prompt(&client).await;
    
    println!("\n=== All tests complete ===");
}