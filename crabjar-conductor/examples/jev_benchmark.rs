//! Benchmark for crabjar-conductor JEV decision evaluation endpoint.
//! Measures latency and throughput of the embedding-based semantic matching approach.

use std::time::{Duration, Instant};
use reqwest::Client;
use serde_json::json;

const CONDUCTOR_URL: &str = "http://localhost:8091";

#[tokio::main]
async fn main() {
    println!("=== Crabjar-Conductor JEV Decision Evaluation Benchmark ===");
    println!("Conductor URL: {}", CONDUCTOR_URL);
    
    let jwt = std::env::var("CONDUCTOR_JWT")
        .expect("Set CONDUCTOR_JWT environment variable with a valid JWT token");
    
    println!();

    let client = Client::builder().timeout(Duration::from_secs(30)).build().unwrap();

    // Phase 1: Single synchronous decision evaluation
    println!("Phase 1: Single synchronous decision evaluation...");
    let start = Instant::now();
    evaluate_decision(&client, &jwt, false).await.unwrap();
    let sync_time = start.elapsed();
    println!("  Synchronous evaluation completed in {:?}", sync_time);
    println!();

    // Phase 2: Background decision evaluation (fire and forget)
    println!("Phase 2: Background decision evaluation...");
    let start = Instant::now();
    let eval_id = evaluate_decision_background(&client, &jwt).await.unwrap();
    let bg_submit_time = start.elapsed();
    println!("  Background evaluation submitted in {:?}", bg_submit_time);
    println!("  Evaluation ID: {}", eval_id);

    // Wait and poll for result
    println!("  Waiting for background evaluation to complete...");
    let start_wait = Instant::now();
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        match get_evaluation_status(&client, &jwt, &eval_id).await.unwrap() {
            Some(_) => {
                println!("  Background evaluation completed in {:?}", start_wait.elapsed());
                break;
            }
            None => continue,
        }
    }
    println!();

    // Phase 3: Throughput benchmark (multiple sequential evaluations)
    println!("Phase 3: Throughput benchmark (sequential)...");
    let iterations = 10;
    let start = Instant::now();
    for _ in 0..iterations {
        evaluate_decision(&client, &jwt, false).await.unwrap();
    }
    let throughput_time = start.elapsed();
    println!("  {} evaluations in {:?}", iterations, throughput_time);
    let avg_ns = throughput_time.as_nanos() / iterations as u128;
    println!("  Average: {}ns per evaluation", avg_ns);
    println!();

    // Summary
    println!("=== Benchmark Complete ===");
    println!("JEV uses embedding-based semantic matching via llama-server API.");
    println!("Each decision requires an HTTP call to the embedding backend.");
}

async fn evaluate_decision(client: &Client, jwt: &str, background: bool) -> Result<serde_json::Value, String> {
    let body = json!({
        "observation": "Worker ftw3 GPU utilization at 95% for 10 minutes",
        "criteria_file": "worker_health.json",
        "background": background
    });

    let response = client
        .post(format!("{}/decide/evaluate", CONDUCTOR_URL))
        .header("Authorization", format!("Bearer {}", jwt))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    let body = response.json::<serde_json::Value>().await.map_err(|e| format!("JSON parse failed: {}", e))?;

    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status, body));
    }

    Ok(body)
}

async fn evaluate_decision_background(client: &Client, jwt: &str) -> Result<String, String> {
    let result = evaluate_decision(client, jwt, true).await?;
    
    // Extract evaluation_id from response
    if let Some(eval_id) = result.get("evaluation_id") {
        Ok(eval_id.as_str().unwrap_or("").to_string())
    } else {
        Err(format!("No evaluation_id in response: {}", result))
    }
}

async fn get_evaluation_status(client: &Client, jwt: &str, eval_id: &str) -> Result<Option<serde_json::Value>, String> {
    let response = client
        .get(format!("{}/decide/evaluate/{}", CONDUCTOR_URL, eval_id))
        .header("Authorization", format!("Bearer {}", jwt))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    if status == 404 {
        return Ok(None);
    }

    let body = response.json::<serde_json::Value>().await.map_err(|e| format!("JSON parse failed: {}", e))?;
    
    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status, body));
    }

    Ok(Some(body))
}
