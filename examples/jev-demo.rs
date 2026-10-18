// JEV demo: test model routing decision via conductor HTTP API
use std::process::Command;

fn main() {
    println!("=== JEV Model Routing Decision Demo ===\n");

    // Scenario: Complex legal document translation
    let task = "Translate a complex legal document from French to English. The document contains specialized legal terminology, cross-references to statutes, and requires high accuracy.";

    println!("Task: {}", task);
    println!("\nEvaluating with JEV model routing criteria...\n");

    // Call the conductor's evaluate endpoint via HTTP
    let body = format!(
        r#"{{"task": "{}", "task_complexity": "high", "latency_requirement_ms": 5000, "cost_budget_usd": 0.10}}"#,
        task
    );

    let output = Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &body,
            "http://localhost:8090/decide/evaluate",
        ])
        .output()
        .expect("Failed to execute curl");

    let response = String::from_utf8_lossy(&output.stdout);
    println!("JEV Decision: {}", response);
}
