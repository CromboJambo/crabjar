//! Podman sandbox configuration profiles and builders.

use serde::{Deserialize, Serialize};

/// Isolation profile for agent workloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxProfile {
    /// Minimal isolation: Unix user only. Fast, least overhead.
    User,
    /// Rootless Podman container with default settings.
    Container,
    /// Rootless Podman with read-only root filesystem.
    ReadOnlyContainer,
    /// Fully isolated: separate network namespace, no host access.
    Isolated,
}

impl Default for SandboxProfile {
    fn default() -> Self {
        SandboxProfile::Container
    }
}

/// Complete sandbox configuration for an agent workload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Unique identifier for this sandbox instance.
    pub id: String,
    /// Agent name (used for container naming).
    pub agent_name: String,
    /// Isolation profile to apply.
    pub profile: SandboxProfile,
    /// Working directory inside the container.
    pub working_dir: String,
    /// Environment variables to set in the container.
    pub env_vars: Vec<(String, String)>,
    /// Host paths to mount into the container (host_path -> container_path).
    pub mounts: Vec<(String, String)>,
    /// Command to run inside the container.
    pub command: Vec<String>,
    /// Resource limits (CPU, memory).
    pub resource_limits: Option<ResourceLimits>,
    /// Network configuration.
    pub network: Option<NetworkConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_shares: Option<i32>,
    pub memory_limit_mb: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Bridge network name or "host" for host networking.
    pub network_name: String,
    /// Port mappings (container_port -> host_port).
    pub port_mappings: Vec<(i32, i32)>,
}

impl SandboxConfig {
    pub fn new(agent_name: &str, command: Vec<String>) -> Self {
        let id = format!("agent-{}", agent_name);
        SandboxConfig {
            id,
            agent_name: agent_name.to_string(),
            profile: SandboxProfile::Container,
            working_dir: "/workspace".to_string(),
            env_vars: vec![],
            mounts: vec![],
            command,
            resource_limits: None,
            network: None,
        }
    }

    pub fn with_profile(mut self, profile: SandboxProfile) -> Self {
        self.profile = profile;
        self
    }

    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.push((key.to_string(), value.to_string()));
        self
    }

    pub fn with_mount(mut self, host_path: &str, container_path: &str) -> Self {
        self.mounts.push((host_path.to_string(), container_path.to_string()));
        self
    }
}
