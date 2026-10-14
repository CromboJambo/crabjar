//! CrabJar SSH Bastion — execute commands on remote machines via SSH.
//!
//! Phase 1 MVP: single command execution with hardcoded target config.
//! Uses system ssh binary for simplicity and reliability.

/// Result of a remote command execution.
#[derive(Debug, Clone)]
pub struct RemoteOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Target configuration for SSH connections.
struct TargetConfig {
    host: &'static str,
    user: &'static str,
    key_path: &'static str,
}

/// Get target configuration by name.
fn get_target_config(name: &str) -> Option<TargetConfig> {
    match name {
        "jambo" => Some(TargetConfig {
            host: "127.0.0.1",
            user: "crombo",
            key_path: "~/.ssh/id_lab",
        }),
        _ => None,
    }
}

/// Expand ~ to home directory.
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/crombo".to_string());
        format!("{}{}", home, &path[1..])
    } else {
        path.to_string()
    }
}

/// Execute a command on a remote machine via SSH.
///
/// # Arguments
/// * `target` - Target name (e.g., "jambo")
/// * `command` - Command to execute
/// * `args` - Command arguments
/// * `key_path` - Path to SSH private key (None uses target default)
///
/// # Returns
/// Remote command output or error
pub async fn execute_remote(
    target: &str,
    command: &str,
    args: &[String],
    _key_path: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    // Phase 1 MVP: hardcoded target configuration
    let config = get_target_config(target)
        .ok_or_else(|| format!("unknown target: {}", target))?;

    // Build full command with args
    let mut full_cmd = command.to_string();
    if !args.is_empty() {
        for arg in args {
            full_cmd.push_str(" ");
            full_cmd.push_str(arg);
        }
    }

    // Execute via system ssh binary
    let key_path = expand_tilde(config.key_path);
    
    let output = tokio::process::Command::new("ssh")
        .arg("-i")
        .arg(&key_path)
        .arg("-o")
        .arg("StrictHostKeyChecking=accept-new")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg(format!("{}@{}", config.user, config.host))
        .arg(&full_cmd)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("remote command failed (exit {}): {}", 
            output.status.code().unwrap_or(-1), stderr.trim()).into());
    }

    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        std::env::set_var("HOME", "/home/testuser");
        assert_eq!(expand_tilde("~/file"), "/home/testuser/file");
        assert_eq!(expand_tilde("/absolute/path"), "/absolute/path");
    }

    #[test]
    fn test_target_config() {
        let config = get_target_config("jambo").unwrap();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.user, "crombo");
    }

    #[test]
    fn test_unknown_target() {
        assert!(get_target_config("unknown").is_none());
    }
}
