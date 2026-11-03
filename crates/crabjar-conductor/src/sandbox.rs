//! Sandbox management for ephemeral worker environments.
//!
//! This module provides the `SandboxManager` which orchestrates Podman container
//! lifecycle operations for creating, configuring, and destroying isolated
//! execution environments for agent workers.

use crabjar_sandbox::podman::{PodmanConfig, PodmanSandbox};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

/// Sandbox configuration including resource limits and network settings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxConfig {
    /// Base image to use for the sandbox container.
    pub image: String,
    /// Network mode (default, none, host).
    #[serde(default = "default_network_mode")]
    pub network_mode: String,
    /// Filesystem mounts (`host_path:container_path` pairs).
    #[serde(default)]
    pub mounts: Vec<String>,
    /// Published ports (host:container pairs).
    #[serde(default)]
    pub ports: Vec<String>,
    /// Resource limits.
    #[serde(default)]
    pub resources: SandboxResources,
}

fn default_network_mode() -> String {
    "default".to_string()
}

/// Resource limits for sandbox containers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[derive(Default)]
pub struct SandboxResources {
    /// CPU limit (e.g., "0.5" for half a core).
    #[serde(default)]
    pub cpu_limit: Option<String>,
    /// Memory limit (e.g., "512m").
    #[serde(default)]
    pub memory_limit: Option<String>,
}


/// State of a running sandbox environment.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SandboxState {
    /// Unique identifier for the sandbox (container ID).
    pub container_id: String,
    /// Configuration used to create this sandbox.
    pub config: SandboxConfig,
    /// Worker currently assigned to this sandbox (if any).
    pub worker_id: Option<String>,
    /// Unix timestamp when the sandbox was created.
    pub created_at: u64,
}

/// Sandbox manager that tracks and provisions ephemeral environments.
pub struct SandboxManager {
    /// Podman container manager for lifecycle operations.
    podman: Arc<PodmanSandbox>,
    /// Map of sandbox IDs to their state.
    sandboxes: Mutex<HashMap<String, SandboxState>>,
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SandboxManager {
    /// Create a new sandbox manager.
    #[must_use]
    pub fn new() -> Self {
        let podman_config = PodmanConfig::default();
        let podman = PodmanSandbox::new(podman_config);
        Self {
            podman: Arc::new(podman),
            sandboxes: Mutex::new(HashMap::new()),
        }
    }

    /// Provision a new ephemeral sandbox environment.
    pub fn create_sandbox(&self, config: SandboxConfig) -> Result<SandboxState, String> {
        info!(image = %config.image, "Provisioning new sandbox");

        // Generate unique container name
        let container_name = format!("crabjar-sandbox-{}", uuid::Uuid::new_v4().simple());

        // Create the container using PodmanSandbox
        let container_id = self
            .podman
            .create_container(&container_name)
            .map_err(|e| format!("Failed to create sandbox container: {e}"))?;

        // Start the container
        self.podman
            .start_container(&container_id)
            .map_err(|e| format!("Failed to start sandbox container: {e}"))?;

        let state = SandboxState {
            container_id,
            config,
            worker_id: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let sandbox_id = state.container_id.clone();
        self.sandboxes
            .lock()
            .map_err(|e| format!("Mutex poisoned: {e}"))?
            .insert(sandbox_id.clone(), state.clone());

        info!(sandbox_id = %sandbox_id, "Sandbox provisioned successfully");
        Ok(state)
    }

    /// List all active sandboxes.
    pub fn list_sandboxes(&self) -> Vec<SandboxState> {
        self.sandboxes
            .lock()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get a specific sandbox by ID.
    pub fn get_sandbox(&self, sandbox_id: &str) -> Option<SandboxState> {
        self.sandboxes
            .lock()
            .ok()
            .and_then(|m| m.get(sandbox_id).cloned())
    }

    /// Destroy a sandbox environment and clean up associated resources.
    pub fn destroy_sandbox(&self, sandbox_id: &str) -> Result<(), String> {
        let state = self
            .sandboxes
            .lock()
            .map_err(|e| format!("Mutex poisoned: {e}"))?
            .remove(sandbox_id)
            .ok_or_else(|| format!("Sandbox {sandbox_id} not found"))?;

        info!(sandbox_id = %sandbox_id, "Destroying sandbox");

        // Stop the container
        if let Err(e) = self.podman.stop_container(&state.container_id) {
            warn!(sandbox_id = %sandbox_id, error = %e, "Failed to stop container (may already be stopped)");
        }

        // Remove the container
        if let Err(e) = self.podman.destroy_container(&state.container_id) {
            warn!(sandbox_id = %sandbox_id, error = %e, "Failed to remove container (may already be removed)");
        }

        info!(sandbox_id = %sandbox_id, "Sandbox destroyed");
        Ok(())
    }

    /// Assign a worker to a sandbox.
    pub fn assign_worker(&self, sandbox_id: &str, worker_id: &str) -> Result<(), String> {
        let mut sandboxes = self.sandboxes.lock().map_err(|e| format!("Mutex poisoned: {e}"))?;
        let state = sandboxes
            .get_mut(sandbox_id)
            .ok_or_else(|| format!("Sandbox {sandbox_id} not found"))?;

        if state.worker_id.is_some() {
            return Err(format!(
                "Sandbox {} already assigned to worker {}",
                sandbox_id,
                state.worker_id.as_ref().unwrap()
            ));
        }

        state.worker_id = Some(worker_id.to_string());
        info!(sandbox_id = %sandbox_id, worker_id = %worker_id, "Worker assigned to sandbox");
        Ok(())
    }

    /// Get the Podman container manager (for direct operations).
    pub fn podman(&self) -> &Arc<PodmanSandbox> {
        &self.podman
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_manager_creation() {
        let manager = SandboxManager::new();
        assert!(manager.list_sandboxes().is_empty());
    }
}