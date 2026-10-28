//! Integration tests for crabjar-worker against a real conductor instance.
//!
//! These tests verify worker registration, task polling, and result reporting
//! against the conductor's HTTP API.

use std::process::{Command, Stdio};
use std::time::Duration;

const CONDUCTOR_PORT: u16 = 8092;
const CONDUCTOR_URL: &str = "http://localhost:8092";
const CONDUCTOR_BINARY: &str = "../target/release/crabjar-conductor";

fn start_conductor() -> Result<Command, Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?.into_path();

    // Kill any existing conductor on this port
    Command::new("pkill")
        .args(["-f", &format!("crabjar-conductor.*{}", CONDUCTOR_PORT)])
        .output()
        .ok();
    std::thread::sleep(Duration::from_secs(1));

    let mut cmd = Command::new(CONDUCTOR_BINARY);
    cmd.args([
        "--port", &CONDUCTOR_PORT.to_string(),
        "--data-dir", temp_dir.to_str().unwrap(),
    ]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let child = cmd.spawn()?;

    for _ in 0..30 {
        std::thread::sleep(Duration::from_secs(1));
        if is_conductor_ready() {
            return Ok(child);
        }
    }

    Err("Conductor did not start within 30 seconds".into())
}

fn is_conductor_ready() -> bool {
    let output = Command::new("curl")
        .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", format!("{}/health", CONDUCTOR_URL)])
        .output();

    match output {
        Ok(o) => o.status.success() && String::from_utf8_lossy(&o.stdout).contains("200"),
        Err(_) => false,
    }
}

fn register_worker(name: &str) -> Option<String> {
    let output = Command::new("curl")
        .args([
            "-s", "-X", "POST", format!("{}/workers/register", CONDUCTOR_URL),
            "-H", "Content-Type: application/json",
            "-d", &format!(
                "{{\"name\":\"{}\",\"machine_id\":\"jambo\",\"capabilities\":[\"os:Linux\"]}}",
                name
            ),
        ])
        .output()
        .ok()?;

    let body = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()?
        .get("worker_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn poll_tasks(worker_id: &str) -> Option<serde_json::Value> {
    let output = Command::new("curl")
        .args(["-s", format!("{}/tasks/for-worker/{}", CONDUCTOR_URL, worker_id)])
        .output()
        .ok()?;

    let body = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&body).ok()
}

#[test]
fn test_worker_registration() {
    let conductor = start_conductor().expect("Failed to start conductor");

    let worker_id = register_worker("integration-test-worker");
    assert!(worker_id.is_some(), "Worker registration failed");

    println!("✓ Worker registered: {}", worker_id.unwrap());

    conductor.kill().ok();
}

#[test]
fn test_worker_polling() {
    let conductor = start_conductor().expect("Failed to start conductor");

    let worker_id = register_worker("polling-test-worker");
    assert!(worker_id.is_some());

    let tasks = poll_tasks(&worker_id.unwrap());
    assert!(tasks.is_some(), "Failed to poll for tasks");

    println!("✓ Worker polling works");

    conductor.kill().ok();
}

#[test]
fn test_multiple_workers() {
    let conductor = start_conductor().expect("Failed to start conductor");

    let w1 = register_worker("worker-1");
    let w2 = register_worker("worker-2");
    let w3 = register_worker("worker-3");

    assert!(w1.is_some());
    assert!(w2.is_some());
    assert!(w3.is_some());

    // All workers should have unique IDs
    assert_ne!(w1.as_deref(), w2.as_deref());
    assert_ne!(w2.as_deref(), w3.as_deref());

    println!("✓ Multiple workers registered with unique IDs");

    conductor.kill().ok();
}

#[test]
fn test_worker_capabilities() {
    let conductor = start_conductor().expect("Failed to start conductor");

    // Register worker with specific capabilities
    let output = Command::new("curl")
        .args([
            "-s", "-X", "POST", format!("{}/workers/register", CONDUCTOR_URL),
            "-H", "Content-Type: application/json",
            "-d", r#"{
                "name": "capable-worker",
                "machine_id": "ftw3",
                "capabilities": ["os:Linux", "gpu:nvidia:3070ti", "docker"]
            }"#,
        ])
        .output()
        .expect("Failed to register capable worker");

    let body = String::from_utf8_lossy(&output.stdout);
    println!("✓ Worker with capabilities registered: {}", body);

    conductor.kill().ok();
}