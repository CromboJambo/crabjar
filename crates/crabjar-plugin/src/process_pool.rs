//! Process pool — manages subprocess lifecycle with startup timing and health checks.

use crate::process_types::{PluginHandle, RestartPolicy, SpawnError};
use crate::types::{HealthStatus, PluginError, ToolResult};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Manages a pool of plugin subprocesses with lifecycle management.
pub struct ProcessPool {
    /// Maximum number of concurrent plugin instances.
    max_instances: usize,
    /// Target startup latency budget (~100ms for Rust plugins).
    startup_timeout: Duration,
    /// Interval between health checks (used by future background monitor).
    #[allow(dead_code)]
    health_check_interval: Duration,
    /// Policy for handling crashed processes.
    #[allow(dead_code)]
    restart_policy: RestartPolicy,
    /// Currently running plugin handles (protected by RwLock).
    instances: RwLock<Vec<PluginHandle>>,
}

impl ProcessPool {
    pub fn new(
        max_instances: usize,
        startup_timeout: Duration,
        health_check_interval: Duration,
        restart_policy: RestartPolicy,
    ) -> Self {
        Self {
            max_instances,
            startup_timeout,
            health_check_interval,
            restart_policy,
            instances: RwLock::new(Vec::new()),
        }
    }

    /// Default configuration for Rust plugins (100ms startup budget).
    pub fn default_rust() -> Self {
        Self::new(
            8,
            Duration::from_millis(100),
            Duration::from_secs(30),
            RestartPolicy::NoRestart,
        )
    }

    /// Spawn a plugin subprocess and return a handle.
    pub async fn spawn(&self, binary_path: &Path, name: &str) -> Result<PluginHandle, SpawnError> {
        // Check capacity
        let instances = self.instances.read().await;
        if instances.len() >= self.max_instances {
            drop(instances);
            return Err(SpawnError::SpawnFailed(format!(
                "pool full (max {} instances)",
                self.max_instances
            )));
        }
        drop(instances);

        // Check binary exists and is executable
        if !binary_path.exists() {
            return Err(SpawnError::ProcessNotFound(binary_path.to_path_buf()));
        }

        let start = std::time::Instant::now();

        // Spawn the process with stdin/stdout piped for JSON-RPC communication.
        // stderr is inherited so errors are visible in logs.
        let mut child = match Command::new(binary_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
        {
            Ok(child) => child,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                return Err(SpawnError::PermissionDenied(binary_path.to_path_buf()));
            }
            Err(e) => {
                return Err(SpawnError::SpawnFailed(format!(
                    "failed to spawn '{}': {}",
                    binary_path.display(),
                    e
                )));
            }
        };

        // Wait for the process to start (or timeout).
        // For JSON-RPC plugins, we expect a handshake message on stdout.
        let mut child_stdout = child.stdout.take().expect("stdout should be piped");
        use tokio::io::AsyncReadExt;
        let mut handshake_buf = Vec::new();

        match tokio::time::timeout(
            self.startup_timeout,
            child_stdout.read_to_end(&mut handshake_buf),
        )
        .await
        {
            Ok(Ok(_)) => {
                // Handshake received — plugin started successfully.
                debug!(
                    "plugin '{}' spawned in {:?} (handshake: {} bytes)",
                    name,
                    start.elapsed(),
                    handshake_buf.len()
                );
            }
            Ok(Err(e)) => {
                warn!("plugin '{}' failed during startup: {}", name, e);
            }
            Err(_) => {
                // Timeout — kill the process.
                let _ = child.kill().await;
                return Err(SpawnError::Timeout(self.startup_timeout));
            }
        }

        let pid = child.id();
        let handle = PluginHandle {
            pid,
            name: name.to_string(),
            spawn_time: start,
        };

        // Register the handle.
        let mut instances = self.instances.write().await;
        instances.push(handle.clone());

        info!(
            "plugin '{}' registered (pid={:?}, startup={:?})",
            name,
            pid,
            start.elapsed()
        );

        Ok(handle)
    }

    /// Execute a tool call on a plugin handle with timeout.
    pub async fn execute_with_timeout(
        &self,
        _handle: &PluginHandle,
        _tool_name: &str,
        _args: serde_json::Value,
        _timeout: Duration,
    ) -> Result<ToolResult, PluginError> {
        // JSON-RPC communication over stdin/stdout is pending.
        Err(PluginError::NotImplemented(
            "execute_with_timeout not yet implemented".to_string(),
        ))
    }

    /// Check health of all running plugins.
    pub async fn check_health(&self) -> Vec<(String, HealthStatus)> {
        let instances = self.instances.read().await;
        // Active processes are assumed healthy until a real health probe is implemented.
        instances
            .iter()
            .map(|h| (h.name.clone(), HealthStatus::Healthy))
            .collect()
    }

    /// Stop all running plugins.
    pub async fn stop_all(&self) {
        let mut instances = self.instances.write().await;
        for handle in instances.iter() {
            if let Some(pid) = handle.pid {
                // Process cleanup via SIGTERM is pending — currently just clears the registry.
                debug!("stopping plugin '{}' (pid={})", handle.name, pid);
            }
        }
        instances.clear();
    }

    /// Get the number of running instances.
    pub async fn count(&self) -> usize {
        self.instances.read().await.len()
    }
}
