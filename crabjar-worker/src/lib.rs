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
            caps.push(format!("os:{}", os));
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
                        error!(task_id = %task.id, error = %e, "Failed to report task result")
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
        _ => return Err(anyhow!("Registration failed: {} - {}", reg.status, reg.message.unwrap_or_default())),
    }

    let worker_id = reg.worker_id.ok_or_else(|| anyhow!("No worker_id in registration response"))?;
    let auth_token = reg.auth_token.ok_or_else(|| anyhow!("No auth_token in registration response"))?;

    Ok((worker_id, auth_token))
}

async fn send_heartbeat(
    client: &reqwest::Client,
    conductor_url: &str,
    worker_id: &str,
    _state: &Arc<Mutex<WorkerState>>,
) {
    let url = format!("{}/workers/heartbeat", conductor_url);

    let body = serde_json::json!({ "worker_id": worker_id });

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

async fn poll_for_tasks(
    client: &reqwest::Client,
    conductor_url: &str,
    worker_id: &str,
    state: &Arc<Mutex<WorkerState>>,
) -> Result<Option<TaskInfo>> {
    let url = format!("{}/tasks/for-worker/{}", conductor_url, worker_id);

    let s = state.lock().await;
    let token = match &s.auth_token {
        Some(t) => t.clone(),
        None => return Err(anyhow!("No auth token available")),
    };
    drop(s);

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
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
        exit_code: if output.status.success() { 0 } else { 1 },
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
    let url = format!("{}/tasks/{}/result", conductor_url, task_id);
    let body = ReportBody { status, result };

    let s = state.lock().await;
    let token = match &s.auth_token {
        Some(t) => t.clone(),
        None => return Err(anyhow!("No auth token available")),
    };
    drop(s);

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("Report failed: {}", resp.status()));
    }

    // Check for token refresh (trust level change)
    let text = resp.text().await.unwrap_or_default();
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(new_token) = parsed["auth_token"].as_str() {
            {
                let mut s = state.lock().await;
                s.auth_token = Some(new_token.to_string());
            }
            info!("Auth token refreshed after task completion");
        }
    }

    Ok(())
}

struct TaskResult {
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
}
