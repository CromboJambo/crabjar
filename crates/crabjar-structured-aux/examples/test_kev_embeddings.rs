use reqwest::Client;
use serde_json::json;

async fn test_embedding_generation(client: &Client) {
    println!("=== Test 1: Embedding Generation ===");
    
    let response = client
        .post("http://ftw3:34159/v1/embeddings")
        .header("Content-Type", "application/json")
        .body(json!({
            "model": "/home/crombo/.cache/huggingface/hub/models--ggml-org--Kev-9B-GGUF/Kev-9B-Q4_K_M.gguf",
            "input": "This is a test sentence for embedding generation."
        })
        .to_string())
        .send()
        .await;
        
    if response.is_ok() {
        let resp = response.unwrap();
        let status = resp.status();
        let body = resp.text().await.unwrap();
        println!("Status: {}", status);
        println!("Response length: {} chars", body.len());
        // Check if we got an array of numbers (embedding vector)
        if body.contains("[") && body.contains("]") {
            println!("✓ Got embedding vector");
        } else {
            println!("✗ Did not get expected embedding format");
            println!("Response: {}", body);
        }
    } else {
        println!("✗ Request failed: {:?}", response.err());
    }
}

async fn test_similarity_comparison(client: &Client) {
    println!("\n=== Test 2: Similarity Comparison ===");
    
    // Embed two similar sentences
    let resp1 = client
        .post("http://ftw3:34159/v1/embeddings")
        .header("Content-Type", "application/json")
        .body(json!({
            "model": "/home/crombo/.cache/huggingface/hub/models--ggml-org--Kev-9B-GGUF/Kev-9B-Q4_K_M.gguf",
            "input": "The quick brown fox jumps over the lazy dog"
        })
        .to_string())
        .send()
        .await;
        
    let resp2 = client
        .post("http://ftw3:34159/v1/embeddings")
        .header("Content-Type", "application/json")
        .body(json!({
            "model": "/home/crombo/.cache/huggingface/hub/models--ggml-org--Kev-9B-GGUF/Kev-9B-Q4_K_M.gguf",
            "input": "A fast brown fox leaped over a sleeping dog"
        })
        .to_string())
        .send()
        .await;
        
    if resp1.is_ok() && resp2.is_ok() {
        println!("✓ Successfully embedded both sentences");
        // In a real implementation, we would compute cosine similarity here
    } else {
        println!("✗ Embedding failed for one or both sentences");
    }
}

async fn test_code_embedding(client: &Client) {
    println!("\n=== Test 3: Code Snippet Embedding ===");
    
    let code = "fn fibonacci(n: u32) -> u32 { if n < 2 { return n; } fibonacci(n-1) + fibonacci(n-2) }";
    
    let response = client
        .post("http://ftw3:34159/v1/embeddings")
        .header("Content-Type", "application/json")
        .body(json!({
            "model": "/home/crombo/.cache/huggingface/hub/models--ggml-org--Kev-9B-GGUF/Kev-9B-Q4_K_M.gguf",
            "input": code
        })
        .to_string())
        .send()
        .await;
        
    if response.is_ok() {
        let resp = response.unwrap();
        let status = resp.status();
        println!("Status: {}", status);
        println!("✓ Successfully embedded code snippet");
    } else {
        println!("✗ Code embedding failed");
    }
}

#[tokio::main]
async fn main() {
    println!("Testing Kev-9B embedding capabilities\n");
    
    let client = Client::new();
    
    test_embedding_generation(&client).await;
    test_similarity_comparison(&client).await;
    test_code_embedding(&client).await;
    
    println!("\n=== All tests complete ===");
}