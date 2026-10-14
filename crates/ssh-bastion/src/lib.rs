//! CrabJar SSH Bastion — execute commands on remote machines via SSH.
//!
//! Uses lab topology definitions for target resolution, enabling clean separation
//! of infrastructure code (public) from environment-specific configuration (private).
//! Actual IPs are resolved at runtime via environment variables or private config.

use crabjar_lab_topology::{LabTopology, TopologyError};
use std::env;
use std::sync::Arc;

/// Result of a remote command execution.
#[derive(Debug, Clone)]
pub struct RemoteOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Load topology from file or environment variable.
fn load_topology() -> Result<LabTopology, Box<dyn std::error::Error>> {
    // Check for explicit topology file path via environment
    if let Ok(path) = env::var("LAB_TOPOLOGY_FILE") {
        return LabTopology::from_file(&path).map_err(|e| e.to_string().into());
    }

    // Fall back to default location
    let home = env::var("HOME").unwrap_or_else(|_| "/home/crombo".to_string());
    let default_path = format!("{}/.crabjar/topology.toml", home);
    if std::path::Path::new(&default_path).exists() {
        return LabTopology::from_file(&default_path).map_err(|e| e.to_string().into());
    }

    Err("no topology file found (set LAB_TOPOLOGY_FILE or place at ~/.crabjar/topology.toml)".into())
}

/// Resolve target host IP from topology and environment overrides.
fn resolve_target_host(
    topology: &LabTopology,
    topology_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Get machine definition from topology
    let machine = topology
        .get_machine(topology_name)
        .map_err(|e| e.to_string())?;

    // Prefer Tailscale IP if available and overridden via environment
    if let Some(ts_ip_ref) = &machine.network.tailscale_ip {
        // Check for environment override (runtime secret injection)
        let env_var = format!(
            "{}_TAILSCALE_IP",
            topology_name.to_uppercase().replace('-', "_")
        );
        if let Ok(env_ip) = env::var(&env_var) {
            return Ok(env_ip);
        }
        // Fall back to topology placeholder (for testing with known IPs)
        if ts_ip_ref.starts_with("${") {
            // This is a placeholder — try the LAN IP instead
        } else {
            return Ok(ts_ip_ref.clone());
        }
    }

    // Try LAN IP from first interface
    if let Some(iface) = machine.network.interfaces.first() {
        let env_var = format!(
            "{}_LAN_IP",
            topology_name.to_uppercase().replace('-', "_")
        );
        if let Ok(env_ip) = env::var(&env_var) {
            return Ok(env_ip);
        }
        // Fall back to topology value (may be placeholder or actual IP)
        if !iface.ip.starts_with("${") {
            return Ok(iface.ip.clone());
        }
    }

    Err(format!("no resolvable IP for target '{}'", topology_name).into())
}

/// Expand ~ to home directory.
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        let home = env::var("HOME").unwrap_or_else(|_| "/home/crombo".to_string());
        format!("{}{}", home, &path[1..])
    } else {
        path.to_string()
    }
}

/// Execute a command on a remote machine via SSH.
///
/// # Arguments
/// * `target` - Target name from lab topology (e.g., "jambo", "ftw3")
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
    let topo = Arc::new(load_topology()?);

    // Resolve SSH config from topology
    let ssh_config = topo
        .get_ssh_config(target)
        .map_err(|e| e.to_string())?;

    // Resolve actual host IP (with environment override support)
    let host = resolve_target_host(&topo, target)?;

    // Build full command with args
    let mut full_cmd = command.to_string();
    if !args.is_empty() {
        for arg in args {
            full_cmd.push_str(" ");
            full_cmd.push_str(arg);
        }
    }

    // Build SSH options common to both auth methods
    let mut ssh_options = vec![
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
    ];

    if ssh_config.password_auth {
        // Password auth — use sshpass for non-interactive password input
        let env_var = format!(
            "{}_SSH_PASSWORD",
            target.to_uppercase().replace('-', "_")
        );

        // Try environment variable first, then fall back to file
        let password = match env::var(&env_var) {
            Ok(pw) => pw,
            Err(_) => {
                // Try reading from ~/.crabjar/<target>_password file
                let home = env::var("HOME").unwrap_or_else(|_| "/home/crombo".to_string());
                let pw_file = format!("{}/.crabjar/{}_password", home, target);
                match std::fs::read_to_string(&pw_file) {
                    Ok(content) => content.trim().to_string(),
                    Err(_) => {
                        return Err(format!(
                            "password auth required but {} not set and {} not found",
                            env_var, pw_file
                        )
                        .into())
                    }
                }
            }
        };

        // Build sshpass command string
        let port_flag = if ssh_config.port != 22 {
            format!("-p {}", ssh_config.port)
        } else {
            String::new()
        };

        let ssh_cmd_str = format!(
            "sshpass -p '{}' ssh {} {} {}@{} {}",
            password,
            port_flag,
            ssh_options.join(" "),
            ssh_config.user,
            host,
            full_cmd
        );

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&ssh_cmd_str)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(format!(
                "remote command failed (exit {}): {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )
            .into());
        }

        Ok(stdout)
    } else {
        // Key-based auth
        let key_path = match &ssh_config.key_path {
            Some(path) => expand_tilde(path),
            None => expand_tilde("~/.ssh/id_lab"), // default lab key
        };

        ssh_options.push("-i".to_string());
        ssh_options.push(key_path);
        ssh_options.push("-o".to_string());
        ssh_options.push("BatchMode=yes".to_string());

        let mut ssh_cmd = tokio::process::Command::new("ssh");
        for opt in &ssh_options {
            ssh_cmd.arg(opt);
        }

        // Add port flag if non-standard
        if ssh_config.port != 22 {
            ssh_cmd.arg("-p").arg(ssh_config.port.to_string());
        }

        let output = ssh_cmd
            .arg(format!("{}@{}", ssh_config.user, host))
            .arg(&full_cmd)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(format!(
                "remote command failed (exit {}): {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )
            .into());
        }

        Ok(stdout)
    }
}

/// List all available targets from topology.
pub fn list_targets() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let topo = load_topology()?;
    Ok(topo.machine_names())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        unsafe { env::set_var("HOME", "/home/testuser"); }
        assert_eq!(expand_tilde("~/file"), "/home/testuser/file");
        assert_eq!(expand_tilde("/absolute/path"), "/absolute/path");
    }
}
