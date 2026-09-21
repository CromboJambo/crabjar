//! Podman container lifecycle executor.
//!
//! Manages rootless Podman containers for isolated agent workloads.
//! Uses the podman CLI via std::process::Command (no daemon dependency).

use crate::error::PodmanError;
use crate::sandbox_config::{SandboxConfig, SandboxProfile};
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tracing::{debug, info, warn};

/// Podman container lifecycle manager.
pub struct PodmanExecutor {
    podman_binary: String,
}

impl PodmanExecutor {
    pub fn new() -> Result<Self, PodmanError> {
        let podman_binary = Self::find_podman()?;
        Ok(Self { podman_binary })
    }

    /// Locate the podman binary on PATH.
    fn find_podman() -> Result<String, PodmanError> {
        // Try common locations first
        let candidates = ["/usr/bin/podman", "/usr/local/bin/podman", "podman"];
        for candidate in candidates {
            if let Ok(output) = Command::new(candidate).arg("--version").output() {
                if output.status.success() {
                    return Ok(candidate.to_string());
                }
            }
        }
        Err(PodmanError::BinaryNotFound("podman".to_string()))
    }

    /// Run a podman command and return its output.
    fn run(&self, args: &[String]) -> Result<Output, PodmanError> {
        let cmd_str = args.join(" ");
        debug!(command = %cmd_str, "executing podman");

        let output = Command::new(&self.podman_binary)
            .args(args)
            .output()
            .map_err(|e| PodmanError::ExecutionFailed {
                source: e,
                command: cmd_str.clone(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(PodmanError::CommandFailed {
                code: output.status.code().unwrap_or(-1),
                output: stderr,
            });
        }

        Ok(output)
    }

    /// Run a podman command that returns JSON.
    fn run_json(&self, args: &[String]) -> Result<Value, PodmanError> {
        let output = self.run(args)?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        serde_json::from_str(&stdout).map_err(|e| PodmanError::JsonParse {
            source: e,
            raw: stdout,
        })
    }

    /// Create a new container from the given config.
    pub fn create_container(&self, config: &SandboxConfig) -> Result<String, PodmanError> {
        let mut args = vec![
            "create".to_string(),
            "--name".to_string(),
            config.id.clone(),
        ];

        // Working directory
        args.push("-w".to_string());
        args.push(config.working_dir.clone());

        // Environment variables
        for (key, value) in &config.env_vars {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Mounts
        for (host_path, container_path) in &config.mounts {
            args.push("-v".to_string());
            args.push(format!("{}:{}", host_path, container_path));
        }

        // Resource limits
        if let Some(limits) = &config.resource_limits {
            if let Some(shares) = limits.cpu_shares {
                args.push("--cpu-shares".to_string());
                args.push(shares.to_string());
            }
            if let Some(mem_mb) = limits.memory_limit_mb {
                args.push("--memory".to_string());
                args.push(format!("{}m", mem_mb));
            }
        }

        // Network configuration
        if let Some(network) = &config.network {
            if network.network_name != "host" {
                args.push("--network".to_string());
                args.push(network.network_name.clone());
            }
            for (container_port, host_port) in &network.port_mappings {
                args.push("-p".to_string());
                args.push(format!("{}:{}", host_port, container_port));
            }
        }

        // Profile-specific settings
        match config.profile {
            SandboxProfile::ReadOnlyContainer => {
                args.push("--read-only".to_string());
            }
            SandboxProfile::Isolated => {
                args.push("--network".to_string());
                args.push("none".to_string());
                args.push("--pid".to_string());
                args.push("container:0".to_string()); // No PID namespace sharing
            }
            _ => {}
        }

        // Image and command
        args.push("ubuntu:latest".to_string());
        for cmd in &config.command {
            args.push(cmd.clone());
        }

        let output = self.run(&args)?;
        let container_id = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        info!(
            container_id = %container_id,
            agent_name = %config.agent_name,
            "created podman container"
        );

        Ok(container_id)
    }

    /// Start a previously created container.
    pub fn start_container(&self, container_id: &str) -> Result<(), PodmanError> {
        info!(container_id = %container_id, "starting podman container");
        self.run(&[
            "start".to_string(),
            "-a".to_string(),
            container_id.to_string(),
        ])?;
        Ok(())
    }

    /// Stop a running container.
    pub fn stop_container(&self, container_id: &str) -> Result<(), PodmanError> {
        info!(container_id = %container_id, "stopping podman container");
        self.run(&[
            "stop".to_string(),
            "-t".to_string(),
            "5".to_string(),
            container_id.to_string(),
        ])?;
        Ok(())
    }

    /// Remove a container (must be stopped first).
    pub fn remove_container(&self, container_id: &str) -> Result<(), PodmanError> {
        info!(container_id = %container_id, "removing podman container");
        self.run(&[
            "rm".to_string(),
            "-f".to_string(),
            container_id.to_string(),
        ])?;
        Ok(())
    }

    /// Execute a command inside a running container.
    pub fn exec_in_container(
        &self,
        container_id: &str,
        command: &[String],
    ) -> Result<Output, PodmanError> {
        let mut args = vec!["exec".to_string()];
        args.extend(command.iter().cloned());
        args.push(container_id.to_string());

        info!(
            container_id = %container_id,
            command = %command.join(" "),
            "executing in podman container"
        );

        self.run(&args)
    }

    /// Inspect a container and return its JSON state.
    pub fn inspect_container(&self, container_id: &str) -> Result<Value, PodmanError> {
        let output = self.run_json(&[
            "inspect".to_string(),
            container_id.to_string(),
        ])?;
        Ok(output)
    }

    /// List running containers.
    pub fn list_containers(&self) -> Result<Vec<String>, PodmanError> {
        let output = self.run(&["ps".to_string(), "-q".to_string()])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let containers: Vec<String> = stdout
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(containers)
    }

    /// Check if a container exists.
    pub fn container_exists(&self, container_id: &str) -> Result<bool, PodmanError> {
        match self.run(&[
            "inspect".to_string(),
            "--format".to_string(),
            "{{.Id}}".to_string(),
            container_id.to_string(),
        ]) {
            Ok(_) => Ok(true),
            Err(PodmanError::CommandFailed { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }
}
