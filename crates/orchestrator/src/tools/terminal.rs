//! First-class `terminal` tool matching Hermes Agent API contract.
//!
//! Runs shell commands with structured output, timeout control, and optional
//! background execution. This is the primary way agents interact with the OS.
//!
//! Arguments (JSON in args[0]):
//! - command (required): The shell command to execute
//! - background (optional bool, default false): Run as tracked background process
//! - timeout (optional u64, default 180, max 600): Max seconds for foreground commands
//! - workdir (optional string): Working directory for the command
//!
//! Returns structured JSON with output, exit_code, and execution metadata.

use serde_json::{Value, json};
use std::path::Path;
use tokio::process::Command;

/// Handle a terminal tool call.
pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("Error: terminal requires JSON arguments".to_string());
    }

    let input: Value =
        serde_json::from_str(&args[0]).map_err(|e| format!("JSON parse error: {}", e))?;

    let command = input["command"]
        .as_str()
        .ok_or("Missing required field: command")?;
    let background = input["background"].as_bool().unwrap_or(false);
    let timeout_s = input["timeout"].as_u64().unwrap_or(180).min(600);
    let workdir = input["workdir"].as_str().map(expand_path);

    if background {
        handle_background(command, workdir.as_deref()).await
    } else {
        handle_foreground(command, timeout_s, workdir.as_deref()).await
    }
}

/// Handle foreground command execution with timeout.
async fn handle_foreground(
    command: &str,
    timeout_s: u64,
    workdir: Option<&Path>,
) -> Result<String, String> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);

    if let Some(dir) = workdir {
        cmd.current_dir(dir);
    }

    let timeout = std::time::Duration::from_secs(timeout_s);
    let output = tokio::time::timeout(timeout, cmd.output())
        .await
        .map_err(|_| format!("Command timed out after {}s", timeout_s))?
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    let combined_output = if stderr.is_empty() {
        stdout.clone()
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    let result = json!({
        "output": combined_output,
        "exit_code": exit_code,
        "stdout": stdout,
        "stderr": stderr,
        "command": command,
        "workdir": workdir.map(|p| p.display().to_string()),
    });

    Ok(serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?)
}

/// Handle background process execution.
async fn handle_background(command: &str, workdir: Option<&Path>) -> Result<String, String> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    if let Some(dir) = workdir {
        cmd.current_dir(dir);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn background process: {}", e))?;
    let pid = child.id().unwrap_or(0);

    // Detach the child so it keeps running after we return
    let _ = child.wait();

    let result = json!({
        "session_id": format!("bg-{}", pid),
        "pid": pid,
        "command": command,
        "status": "started",
        "workdir": workdir.map(|p| p.display().to_string()),
    });

    Ok(serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?)
}

/// Expand `~` to home directory.
fn expand_path(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        home.join(rest)
    } else if path == "~" {
        dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
    } else {
        std::path::PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_terminal_simple_command() {
        let args = vec![r#"{"command": "echo hello"}"#.to_string()];
        let result = handle(&args).await.unwrap();
        let val: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(val["exit_code"], 0);
        assert!(val["stdout"].as_str().unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn test_terminal_exit_code() {
        let args = vec![r#"{"command": "exit 42"}"#.to_string()];
        let result = handle(&args).await.unwrap();
        let val: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(val["exit_code"], 42);
    }

    #[tokio::test]
    async fn test_terminal_stderr() {
        let args = vec![r#"{"command": "echo err >&2"}"#.to_string()];
        let result = handle(&args).await.unwrap();
        let val: Value = serde_json::from_str(&result).unwrap();
        assert!(val["stderr"].as_str().unwrap().contains("err"));
    }

    #[tokio::test]
    async fn test_terminal_workdir() {
        let args = vec![r#"{"command": "pwd", "workdir": "/tmp"}"#.to_string()];
        let result = handle(&args).await.unwrap();
        let val: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(val["exit_code"], 0);
        assert!(val["stdout"].as_str().unwrap().contains("/tmp"));
    }

    #[tokio::test]
    async fn test_terminal_timeout() {
        let args = vec![r#"{"command": "sleep 10", "timeout": 1}"#.to_string()];
        let result = handle(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timed out"));
    }

    #[tokio::test]
    async fn test_terminal_background() {
        let args = vec![r#"{"command": "sleep 1", "background": true}"#.to_string()];
        let result = handle(&args).await.unwrap();
        let val: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(val["status"], "started");
        assert!(val["session_id"].as_str().unwrap().starts_with("bg-"));
    }

    #[tokio::test]
    async fn test_terminal_missing_command() {
        let args = vec![r#"{"timeout": 5}"#.to_string()];
        let result = handle(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("command"));
    }

    #[test]
    fn test_expand_path() {
        let path = expand_path("~/projects");
        assert!(path.starts_with(dirs::home_dir().unwrap()));
        assert!(path.ends_with("projects"));
    }
}
