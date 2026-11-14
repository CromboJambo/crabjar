//! Crabjar Worker Agent
//!
//! Lightweight agent that registers with crabjar-conductor, polls for tasks,
//! executes them locally, and reports results.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::env;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Worker configuration.
pub struct WorkerConfig {
    pub conductor_url: String,
    pub name: String,
    pub capabilities: Vec<String>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        let hostname = env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string());

        // Auto-detect some basic capabilities
        let mut caps = vec![];
        if let Ok(output) = Command::new("uname").arg("-s").output() {
            let os = String::from_utf8_lossy(&output.stdout).trim().to_string();
            caps.push(format!("os:{os}"));
        }

        Self {
            conductor_url: "http://localhost:8091".to_string(),
            name: hostname,
            capabilities: caps,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RegisterResponse {
    status: String,
    worker_id: Option<String>,
    auth_token: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TasksResponse {
    tasks: Vec<TaskInfo>,
}

#[derive(Debug, Deserialize)]
struct TaskInfo {
    id: String,
    goal_id: String,
    command: String,
}

#[derive(Serialize)]
struct ReportBody<'a> {
    status: &'a str,
    result: Option<&'a str>,
}

/// Shared worker state (auth token).
struct WorkerState {
    auth_token: Option<String>,
}

impl WorkerState {
    fn new() -> Self {
        Self { auth_token: None }
    }
}

/// Main worker loop.
pub async fn run(config: WorkerConfig) -> Result<()> {
    info!(name = %config.name, conductor = %config.conductor_url, "Starting crabjar-worker");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let state = Arc::new(Mutex::new(WorkerState::new()));

    // Register with conductor
    let (worker_id, auth_token) = register(&client, &config).await?;
    info!(worker_id = %worker_id, "Registered with conductor");

    {
        let mut s = state.lock().await;
        s.auth_token = Some(auth_token);
    }

    // Main poll loop
    let mut last_heartbeat = Instant::now();
    let heartbeat_interval = Duration::from_secs(30);

    loop {
        // Send periodic heartbeat
        if last_heartbeat.elapsed() >= heartbeat_interval {
            send_heartbeat(&client, &config.conductor_url, &worker_id, &state).await;
            last_heartbeat = Instant::now();
        }

        match poll_for_tasks(&client, &config.conductor_url, &worker_id, &state).await {
            Ok(Some(task)) => {
                info!(task_id = %task.id, goal_id = %task.goal_id, "Received task");
                let result = execute_task(&task.command).await;

                match report_result(
                    &client,
                    &config.conductor_url,
                    &task.id,
                    "completed",
                    result.output.as_deref(),
                    &state,
                )
                .await
                {
                    Ok(()) => info!(task_id = %task.id, "Task completed and reported"),
                    Err(e) => {
                        error!(task_id = %task.id, error = %e, "Failed to report task result");
                    }
                }
            }
            Ok(None) => {
                // No tasks available, wait before polling again
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                warn!(error = %e, "Error polling for tasks");
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

async fn register(client: &reqwest::Client, config: &WorkerConfig) -> Result<(String, String)> {
    let url = format!("{}/workers/register", config.conductor_url);
    let body = serde_json::json!({
        "name": config.name,
        "capabilities": config.capabilities
    });

    let resp = client.post(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("Registration failed: {}", resp.status()));
    }

    let reg: RegisterResponse = resp.json().await?;

    match reg.status.as_str() {
        "registered" => {}
        _ => {
            return Err(anyhow!(
                "Registration failed: {} - {}",
                reg.status,
                reg.message.unwrap_or_default()
            ));
        }
    }

    let worker_id = reg
        .worker_id
        .ok_or_else(|| anyhow!("No worker_id in registration response"))?;
    let auth_token = reg
        .auth_token
        .ok_or_else(|| anyhow!("No auth_token in registration response"))?;

    Ok((worker_id, auth_token))
}

async fn send_heartbeat(
    client: &reqwest::Client,
    conductor_url: &str,
    worker_id: &str,
    _state: &Arc<Mutex<WorkerState>>,
) {
    let url = format!("{conductor_url}/workers/heartbeat");

    // Collect local resource snapshot to include in heartbeat
    let resources = collect_resources();

    let mut body = serde_json::json!({ "worker_id": worker_id });
    if let Some(res) = resources {
        body["resources"] = res;
    }

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            debug!(worker_id, "Heartbeat sent successfully");
        }
        Ok(r) => {
            warn!(worker_id, status = r.status().as_u16(), "Heartbeat failed");
        }
        Err(e) => {
            warn!(worker_id, error = %e, "Heartbeat request failed");
        }
    }
}

/// Collect local system resources for the conductor to use in scheduling.
/// Returns None if resource detection fails entirely (fail-open).
fn collect_resources() -> Option<serde_json::Value> {
    let max_ram_mb = read_total_ram_mb()?;
    let available_ram_mb = read_available_ram_mb().unwrap_or(0);

    // GPU detection via nvidia-smi (best-effort)
    let (gpu_count, gpus_available) = detect_gpus();

    Some(serde_json::json!({
        "max_ram_mb": max_ram_mb,
        "available_ram_mb": available_ram_mb,
        "gpu_count": gpu_count,
        "gpus_available": gpus_available,
        "reported_at": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Read total RAM in MB from /proc/meminfo (Linux).
fn read_total_ram_mb() -> Option<u64> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest.trim().split_whitespace().next()?.parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

/// Read available RAM in MB from /proc/meminfo (Linux).
fn read_available_ram_mb() -> Option<u64> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kb: u64 = rest.trim().split_whitespace().next()?.parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

/// Detect NVIDIA GPUs via nvidia-smi. Returns (total_count, available_count).
/// Available = GPUs with < 90% memory used (rough heuristic for "in use").
fn detect_gpus() -> (u32, u32) {
    let output = match std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=memory.used,memory.total", "--format=csv,noheader,nounits"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return (0, 0), // No nvidia-smi or failed — no GPUs detected
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut count = 0u32;
    let mut available = 0u32;

    for line in stdout.lines() {
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() >= 2 {
            if let (Ok(used), Ok(total)) = (fields[0].parse::<u64>(), fields[1].parse::<u64>()) {
                count += 1;
                // GPU is "available" if using less than 90% of memory
                if total > 0 && used * 10 < total * 9 {
                    available += 1;
                }
            }
        }
    }

    (count, available)
}

async fn poll_for_tasks(
    client: &reqwest::Client,
    conductor_url: &str,
    worker_id: &str,
    state: &Arc<Mutex<WorkerState>>,
) -> Result<Option<TaskInfo>> {
    let url = format!("{conductor_url}/tasks/for-worker/{worker_id}");

    let s = state.lock().await;
    let token = match &s.auth_token {
        Some(t) => t.clone(),
        None => return Err(anyhow!("No auth token available")),
    };
    drop(s);

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("Poll failed: {}", resp.status()));
    }

    let tasks: TasksResponse = resp.json().await?;
    Ok(tasks.tasks.into_iter().next())
}

async fn execute_task(command: &str) -> TaskResult {
    info!(command, "Executing task");
    let start = Instant::now();

    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .expect("Failed to execute command");

    let elapsed = start.elapsed();
    info!(elapsed_ms = elapsed.as_millis(), "Task execution finished");

    TaskResult {
        exit_code: i32::from(!output.status.success()),
        output: Some(String::from_utf8_lossy(&output.stdout).to_string()),
    }
}

async fn report_result(
    client: &reqwest::Client,
    conductor_url: &str,
    task_id: &str,
    status: &str,
    result: Option<&str>,
    state: &Arc<Mutex<WorkerState>>,
) -> Result<()> {
    let url = format!("{conductor_url}/tasks/{task_id}/result");
    let body = ReportBody { status, result };

    let s = state.lock().await;
    let token = match &s.auth_token {
        Some(t) => t.clone(),
        None => return Err(anyhow!("No auth token available")),
    };
    drop(s);

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("Report failed: {}", resp.status()));
    }

    // Check for token refresh (trust level change)
    let text = resp.text().await.unwrap_or_default();
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(new_token) = parsed["auth_token"].as_str()
    {
        {
            let mut s = state.lock().await;
            s.auth_token = Some(new_token.to_string());
        }
        info!("Auth token refreshed after task completion");
    }

    Ok(())
}

struct TaskResult {
    #[allow(dead_code)]
    exit_code: i32,
    output: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let config = WorkerConfig::default();
        assert!(!config.name.is_empty());
    }

    #[test]
    fn read_total_ram_mb_returns_sane_value() {
        let mb = read_total_ram_mb().expect("should read /proc/meminfo on Linux");
        // Any real machine has at least 128MB
        assert!(mb >= 128, "Expected at least 128MB RAM, got {mb}MB");
    }

    #[test]
    fn available_ram_leq_total_ram() {
        let total = read_total_ram_mb().expect("should read total RAM");
        let available = read_available_ram_mb();
        if let Some(available) = available {
            assert!(
                available <= total,
                "Available ({available}MB) should be <= total ({total}MB)"
            );
        }
    }

    #[test]
    fn detect_gpus_returns_non_negative() {
        let (count, available) = detect_gpus();
        assert!(available <= count, "Available GPUs should not exceed total");
    }

    #[test]
    fn collect_resources_produces_valid_json() {
        let res = collect_resources();
        if let Some(json) = res {
            // Verify all required fields are present and have correct types
            assert!(json["max_ram_mb"].is_u64());
            assert!(json["available_ram_mb"].is_u64());
            assert!(json["gpu_count"].is_u64());
            assert!(json["gpus_available"].is_u64());
            assert!(json["reported_at"].is_string());

            // Verify it can be deserialized into the conductor's WorkerResources shape
            #[derive(serde::Deserialize)]
            #[allow(dead_code)]
            struct Expected {
                max_ram_mb: u64,
                available_ram_mb: u64,
                gpu_count: u32,
                gpus_available: u32,
                reported_at: String,
            }
            let parsed: Expected = serde_json::from_value(json).expect("valid WorkerResources JSON");
            assert!(parsed.max_ram_mb > 0);
        }
    }
}
