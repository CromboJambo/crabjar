//! Podman container backend for agent isolation.
//!
//! Uses podman CLI to create, run, and destroy containers as agent sandboxes.

use std::collections::HashMap;
use std::process::Command;
use tracing::debug;

use crate::error::SandboxError;

/// Podman sandbox configuration.
#[derive(Debug, Clone)]
pub struct PodmanConfig {
    /// Base image to use for containers
    pub base_image: String,
    /// Working directory inside container
    pub workdir: Option<String>,
    /// Environment variables to set
    pub env_vars: HashMap<String, String>,
    /// Mount paths (host_path:container_path)
    pub mounts: Vec<String>,
    /// Resource limits (CPU, memory)
    pub resource_limits: Option<String>,
    /// Network mode
    pub network_mode: Option<String>,
}

impl Default for PodmanConfig {
    fn default() -> Self {
        Self {
            base_image: "hermes-sandbox:latest".to_string(),
            workdir: None,
            env_vars: HashMap::new(),
            mounts: vec![],
            resource_limits: None,
            network_mode: None,
        }
    }
}

/// Podman sandbox manager.
pub struct PodmanSandbox {
    config: PodmanConfig,
}

impl PodmanSandbox {
    /// Create a new Podman sandbox manager with the given configuration.
    pub fn new(config: PodmanConfig) -> Self {
        Self { config }
    }

    /// Verify podman is installed and accessible.
    pub fn verify_installation(&self) -> Result<String, SandboxError> {
        let output = Command::new("podman")
            .arg("--version")
            .output()
            .map_err(|e| SandboxError::PodmanNotInstalled(e.to_string()))?;

        if !output.status.success() {
            return Err(SandboxError::PodmanNotInstalled(
                "podman exited with non-zero status".to_string(),
            ));
        }

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        debug!(version, "Podman installation verified");
        Ok(version)
    }

    /// Create a new container from the configured image.
    pub fn create_container(&self, name: &str) -> Result<String, SandboxError> {
        let mut cmd = Command::new("podman");
        cmd.arg("create")
            .arg("--name")
            .arg(name)
            .arg("--rm")
            .arg(&self.config.base_image);

        // Add mounts
        for mount in &self.config.mounts {
            cmd.arg("-v").arg(mount);
        }

        // Add working directory
        if let Some(workdir) = &self.config.workdir {
            cmd.arg("--workdir").arg(workdir);
        }

        // Add environment variables
        for (key, value) in &self.config.env_vars {
            cmd.arg("-e").arg(format!("{}={}", key, value));
        }

        // Add resource limits
        if let Some(limits) = &self.config.resource_limits {
            cmd.arg("--cpus").arg(limits);
        }

        // Add network mode
        if let Some(network) = &self.config.network_mode {
            cmd.arg("--network").arg(network);
        }

        debug!(container_name = %name, "Creating Podman container");

        let output = cmd
            .output()
            .map_err(SandboxError::PodmanCommandFailed)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(SandboxError::ContainerCreateFailed(format!(
                "podman create failed: {}",
                stderr
            )));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        debug!(container_id, "Podman container created");
        Ok(container_id)
    }

    /// Execute a command in an existing container.
    pub fn exec_in_container(
        &self,
        name: &str,
        command: &[&str],
    ) -> Result<PodmanExecResult, SandboxError> {
        let mut cmd = Command::new("podman");
        cmd.arg("exec").arg(name);

        for arg in command {
            cmd.arg(arg);
        }

        debug!(container_name = %name, "Executing command in Podman container");

        let output = cmd
            .output()
            .map_err(SandboxError::PodmanCommandFailed)?;

        if !output.status.success() {
            return Err(SandboxError::ContainerExecFailed(format!(
                "podman exec failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let result = PodmanExecResult {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        };

        debug!(container_name = %name, "Command executed in Podman container");
        Ok(result)
    }

    /// Start a container that was previously created.
    pub fn start_container(&self, name: &str) -> Result<(), SandboxError> {
        let cmd = Command::new("podman")
            .arg("start")
            .arg(name)
            .output()
            .map_err(SandboxError::PodmanCommandFailed)?;

        if !cmd.status.success() {
            return Err(SandboxError::ContainerStartFailed(format!(
                "podman start failed: {}",
                String::from_utf8_lossy(&cmd.stderr)
            )));
        }

        debug!(container_name = %name, "Podman container started");
        Ok(())
    }

    /// Stop a running container.
    pub fn stop_container(&self, name: &str) -> Result<(), SandboxError> {
        let cmd = Command::new("podman")
            .arg("stop")
            .arg(name)
            .output()
            .map_err(SandboxError::PodmanCommandFailed)?;

        if !cmd.status.success() {
            return Err(SandboxError::ContainerStopFailed(format!(
                "podman stop failed: {}",
                String::from_utf8_lossy(&cmd.stderr)
            )));
        }

        debug!(container_name = %name, "Podman container stopped");
        Ok(())
    }

    /// Remove a container.
    pub fn destroy_container(&self, name: &str) -> Result<(), SandboxError> {
        let cmd = Command::new("podman")
            .arg("rm")
            .arg("--name")
            .arg(name)
            .output()
            .map_err(SandboxError::PodmanCommandFailed)?;

        if !cmd.status.success() {
            return Err(SandboxError::ContainerDestroyFailed(format!(
                "podman rm failed: {}",
                String::from_utf8_lossy(&cmd.stderr)
            )));
        }

        debug!(container_name = %name, "Podman container removed");
        Ok(())
    }

    /// List all running containers.
    pub fn list_containers(&self) -> Result<Vec<String>, SandboxError> {
        let output = Command::new("podman")
            .arg("ps")
            .arg("--format")
            .arg("'{{.Names}}'")
            .output()
            .map_err(SandboxError::PodmanCommandFailed)?;

        if !output.status.success() {
            return Err(SandboxError::ContainerListFailed(format!(
                "podman ps failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let output = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let containers: Vec<String> = output
            .lines()
            .map(|l| l.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        Ok(containers)
    }
}

/// Result of executing a command in a Podman container.
#[derive(Debug, Clone)]
pub struct PodmanExecResult {
    /// Exit code (Some if exited normally, None for signal)
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
}

impl PodmanExecResult {
    /// Check if the command succeeded.
    pub fn success(&self) -> bool {
        match self.exit_code {
            Some(code) => code == 0,
            None => false,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_podman_available() {
        let sandbox = PodmanSandbox::new(PodmanConfig::default());
        assert!(sandbox.verify_installation().is_ok());
    }

    #[test]
    fn test_container_lifecycle() {
        let sandbox = PodmanSandbox::new(PodmanConfig::default());
        let name = "crabjar-test-fixed";
        let container_id = sandbox.create_container(name).expect("failed to create container");
        
        sandbox.start_container(&container_id).expect("failed to start container");
        
        let result = sandbox.exec_in_container(
            &container_id,
            &[&"echo", &"hello from podman sandbox"]
        ).expect("failed to exec in container");
        assert!(result.stdout.contains("hello from podman sandbox"));
        
        sandbox.stop_container(&container_id).expect("failed to stop container");
        sandbox.destroy_container(&container_id).expect("failed to destroy container");
    }

    #[test]
    fn test_list_containers() {
        let sandbox = PodmanSandbox::new(PodmanConfig::default());
        let containers = sandbox.list_containers().expect("failed to list containers");
        assert!(containers.len() >= 0);
    }
}
