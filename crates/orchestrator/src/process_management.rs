/// Process Management Tools (P1-7)
///
/// Provides Hermes-compatible `process_manage` tool for managing background
/// processes — listing, starting, stopping, and inspecting running processes.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use tokio::sync::Mutex;

// Import sandbox infrastructure
use crabjar_sandbox::podman::{PodmanConfig, PodmanSandbox};

/// Process status enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Unknown,
}

/// A managed process entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedProcess {
    pub pid: i32,
    pub command: String,
    pub status: ProcessStatus,
    pub started_at: String,
}

/// In-memory process registry (replaced by actual OS-level management in production).
#[derive(Debug, Default)]
pub struct ProcessRegistry {
    processes: HashMap<i32, ManagedProcess>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
        }
    }

    pub async fn register(&mut self, pid: i32, command: String) {
        let process = ManagedProcess {
            pid,
            command,
            status: ProcessStatus::Running,
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        self.processes.insert(pid, process);
    }

    pub async fn list(&self) -> Vec<ManagedProcess> {
        self.processes.values().cloned().collect()
    }

    pub async fn get(&self, pid: i32) -> Option<ManagedProcess> {
        self.processes.get(&pid).cloned()
    }

    pub async fn stop(&mut self, pid: i32) -> bool {
        if let Some(process) = self.processes.get_mut(&pid) {
            process.status = ProcessStatus::Stopped;
            true
        } else {
            false
        }
    }
}

/// Request payload for the process_manage tool.
#[derive(Debug, Deserialize)]
pub struct ProcessManageRequest {
    /// Action to perform: list, start, stop, inspect.
    pub action: String,
    /// Command to start (for "start" action).
    pub command: Option<String>,
    /// PID to operate on (for "stop" or "inspect" actions).
    pub pid: Option<i32>,
    /// Run in sandboxed container (Podman) for isolation.
    #[serde(default)]
    pub sandboxed: bool,
    /// Container name for sandboxed execution (optional).
    pub container_name: Option<String>,
}

/// Response payload for the process_manage tool.
#[derive(Debug, Serialize)]
pub struct ProcessManageResponse {
    pub success: bool,
    pub action: String,
    pub result: serde_json::Value,
}

/// Validate tool name format (redteam finding #16).
/// Tool names must be lowercase alphanumeric with hyphens/underscores.
fn validate_tool_name(name: &str) -> bool {
    // Must start with letter, then alphanumeric/hyphen/underscore only
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Build an error response.
fn error_response(action: &str, error: &str) -> ProcessManageResponse {
    ProcessManageResponse {
        success: false,
        action: action.to_string(),
        result: serde_json::json!({ "error": error }),
    }
}

/// Execute a process management operation.
pub async fn execute_process_management(
    request: ProcessManageRequest,
    registry: &Mutex<ProcessRegistry>,
) -> anyhow::Result<ProcessManageResponse> {
    match request.action.as_str() {
        "list" => handle_list(registry).await,
        "start" => handle_start(request, registry).await,
        "stop" => handle_stop(request, registry).await,
        "inspect" => handle_inspect(request, registry).await,
        other => Ok(error_response("unknown", &format!("Unknown action: {}", other))),
    }
}

/// List all managed processes.
async fn handle_list(
    registry: &Mutex<ProcessRegistry>,
) -> anyhow::Result<ProcessManageResponse> {
    let reg = registry.lock().await;
    let processes = reg.list().await;
    Ok(ProcessManageResponse {
        success: true,
        action: "list".to_string(),
        result: serde_json::json!({
            "processes": processes,
            "count": processes.len()
        }),
    })
}

/// Start a new process (sandboxed or unsandboxed).
async fn handle_start(
    request: ProcessManageRequest,
    registry: &Mutex<ProcessRegistry>,
) -> anyhow::Result<ProcessManageResponse> {
    let cmd = request
        .command
        .ok_or_else(|| anyhow::anyhow!("command required for start"))?;

    // Validate command format (redteam finding #14)
    if !validate_tool_name(&cmd) {
        return Ok(error_response(
            "start",
            "Invalid command format. Use lowercase alphanumeric with hyphens/underscores only.",
        ));
    }

    if request.sandboxed {
        start_sandboxed(request.container_name, &cmd).await
    } else {
        start_unsandboxed(registry, &cmd).await
    }
}

/// Start a process in a Podman sandbox container.
async fn start_sandboxed(
    container_name: Option<String>,
    cmd: &str,
) -> anyhow::Result<ProcessManageResponse> {
    let sandbox = PodmanSandbox::new(PodmanConfig::default());

    // Verify podman is available
    if let Err(e) = sandbox.verify_installation() {
        return Ok(error_response(
            "start",
            &format!("Podman not available for sandboxed execution: {}", e),
        ));
    }

    // Generate container name if not provided
    let container_name = container_name.unwrap_or_else(|| {
        format!(
            "crabjar-sandbox-{}",
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        )
    });

    // Create and run container with the command
    let container_id = match sandbox.create_container(&container_name) {
        Ok(id) => id,
        Err(e) => {
            return Ok(error_response(
                "start",
                &format!("Failed to create sandbox container: {}", e),
            ))
        }
    };

    if let Err(e) = sandbox.start_container(&container_id) {
        let _ = sandbox.destroy_container(&container_id);
        return Ok(error_response(
            "start",
            &format!("Failed to start sandbox container: {}", e),
        ));
    }

    // Execute command in container
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        let _ = sandbox.destroy_container(&container_id);
        return Ok(error_response("start", "Empty command"));
    }

    match sandbox.exec_in_container(&container_id, &parts) {
        Ok(result) => {
            let _ = sandbox.stop_container(&container_id);
            let _ = sandbox.destroy_container(&container_id);
            Ok(ProcessManageResponse {
                success: result.success(),
                action: "start".to_string(),
                result: serde_json::json!({
                    "container": container_name,
                    "command": cmd,
                    "exit_code": result.exit_code,
                    "stdout": result.stdout,
                    "stderr": result.stderr,
                    "sandboxed": true
                }),
            })
        }
        Err(e) => {
            let _ = sandbox.destroy_container(&container_id);
            Ok(error_response(
                "start",
                &format!("Failed to execute in sandbox: {}", e),
            ))
        }
    }
}

/// Start a process without sandboxing (legacy behavior).
async fn start_unsandboxed(
    registry: &Mutex<ProcessRegistry>,
    cmd: &str,
) -> anyhow::Result<ProcessManageResponse> {
    let child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn process: {}", e))?;

    let pid = child.id() as i32;
    let mut reg = registry.lock().await;
    reg.register(pid, cmd.to_string()).await;

    Ok(ProcessManageResponse {
        success: true,
        action: "start".to_string(),
        result: serde_json::json!({
            "pid": pid,
            "command": cmd,
            "status": "running",
            "sandboxed": false
        }),
    })
}

/// Stop a managed process.
async fn handle_stop(
    request: ProcessManageRequest,
    registry: &Mutex<ProcessRegistry>,
) -> anyhow::Result<ProcessManageResponse> {
    let pid = request
        .pid
        .ok_or_else(|| anyhow::anyhow!("pid required for stop"))?;

    // Try to kill the process using Unix signal
    let result = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to send signal: {}", e))?;

    if !result.status.success() {
        return Ok(ProcessManageResponse {
            success: false,
            action: "stop".to_string(),
            result: serde_json::json!({
                "pid": pid,
                "error": "Failed to terminate process"
            }),
        });
    }

    let mut reg = registry.lock().await;
    reg.stop(pid).await;

    Ok(ProcessManageResponse {
        success: true,
        action: "stop".to_string(),
        result: serde_json::json!({
            "pid": pid,
            "status": "stopped"
        }),
    })
}

/// Inspect a managed process.
async fn handle_inspect(
    request: ProcessManageRequest,
    registry: &Mutex<ProcessRegistry>,
) -> anyhow::Result<ProcessManageResponse> {
    let pid = request
        .pid
        .ok_or_else(|| anyhow::anyhow!("pid required for inspect"))?;

    let reg = registry.lock().await;
    if let Some(process) = reg.get(pid).await {
        return Ok(ProcessManageResponse {
            success: true,
            action: "inspect".to_string(),
            result: serde_json::json!(process),
        });
    }

    // Try to inspect via /proc on Linux
    let path = format!("/proc/{}", pid);
    if std::path::Path::new(&path).exists() {
        Ok(ProcessManageResponse {
            success: true,
            action: "inspect".to_string(),
            result: serde_json::json!({
                "pid": pid,
                "status": "running",
                "note": "Found in /proc but not managed by crabjar"
            }),
        })
    } else {
        Ok(ProcessManageResponse {
            success: false,
            action: "inspect".to_string(),
            result: serde_json::json!({
                "pid": pid,
                "error": "Process not found"
            }),
        })
    }
}
