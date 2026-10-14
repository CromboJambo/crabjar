//! CrabJar SSH Bastion — execute commands on remote machines via SSH.
//!
//! Uses lab topology definitions for target resolution, enabling clean separation
//! of infrastructure code (public) from environment-specific configuration (private).
//! Actual IPs are resolved at runtime via environment variables or private config.

use crabjar_lab_topology::{default_topology, TopologyError};
use std::env;

/// Result of a remote command execution.
#[derive(Debug, Clone)]
pub struct RemoteOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Resolve target host IP from topology and environment overrides.
fn resolve_target_host(topology_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let topo = default_topology();
    
    // Get machine definition from topology
    let machine = topo.get_machine(topology_name)?;
    
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
    let topo = default_topology();
    
    // Resolve SSH config from topology
    let ssh_config = topo.get_ssh_config(target)?;
    
    // Resolve actual host IP (with environment override support)
    let host = resolve_target_host(target)?;
    
    // Build full command with args
    let mut full_cmd = command.to_string();
    if !args.is_empty() {
        for arg in args {
            full_cmd.push_str(" ");
            full_cmd.push_str(arg);
        }
    }
    
    // Execute via system ssh binary
    let key_path = expand_tilde(&ssh_config.key_path);
    
    let mut ssh_cmd = tokio::process::Command::new("ssh");
    ssh_cmd
        .arg("-i")
        .arg(&key_path)
        .arg("-o")
        .arg("StrictHostKeyChecking=accept-new")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("BatchMode=yes");
    
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

/// List all available targets from topology.
pub fn list_targets() -> Vec<String> {
    let topo = default_topology();
    topo.machine_names()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        env::set_var("HOME", "/home/testuser");
        assert_eq!(expand_tilde("~/file"), "/home/testuser/file");
        assert_eq!(expand_tilde("/absolute/path"), "/absolute/path");
    }

    #[test]
    fn test_list_targets() {
        let targets = list_targets();
        assert!(targets.contains(&"jambo".to_string()));
        assert!(targets.contains(&"ftw3".to_string()));
    }

    #[test]
    fn test_resolve_target_with_env_override() {
        env::set_var("FTW3_TAILSCALE_IP", "100.113.106.95");
        let host = resolve_target_host("ftw3").unwrap();
        assert_eq!(host, "100.113.106.95");
        env::remove_var("FTW3_TAILSCALE_IP");
    }
}