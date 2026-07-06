//! # crabjar-plugin
//!
//! Plugin system for CrabJar — trait definitions, process pool, and lifecycle management.
//!
//! ## Three-Tier Model
//!
//! 1. **ToolServer** (subprocess) — heavy workloads, external binaries
//! 2. **Script** (in-process) — lightweight automation via Rhai/Lua
//! 3. **Reserved** (WASM) — sandboxed execution outside editor
//!
//! This crate implements Tier 1 (ToolServer). Tiers 2-3 are stubbed for future implementation.

use async_trait::async_trait;
use crabjar_guard::{ExecutionGate, GuardDb};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// ============================================================================
// Plugin Trait & Types
// ============================================================================

/// Health status of a plugin.
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded(msg) => write!(f, "degraded: {}", msg),
            HealthStatus::Unhealthy(msg) => write!(f, "unhealthy: {}", msg),
        }
    }
}

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    /// Wall-clock duration from spawn to completion (for latency budget tracking).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Error types for plugin operations.
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("plugin '{0}' not found")]
    NotFound(String),

    #[error("plugin already registered: {0}")]
    AlreadyRegistered(String),

    #[error("execution error in plugin '{name}': {reason}")]
    Execution { name: String, reason: String },

    #[error("health check failed for plugin '{name}': {status}")]
    HealthCheckFailed { name: String, status: String },

    #[error("plugin not implemented: {0}")]
    NotImplemented(String),

    #[error("guard error: {0}")]
    GuardError(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Context available to plugins during their lifecycle.
pub struct PluginContext {
    pub cwd: PathBuf,
    pub guard_db: GuardDb,
    /// Optional telemetry handle for recording plugin invocations.
    #[allow(dead_code)]
    pub telemetry_handle: Option<String>, // Simplified; would be TelemetryHandle in full impl
}

impl PluginContext {
    pub fn new(cwd: PathBuf, guard_db: GuardDb) -> Self {
        Self {
            cwd,
            guard_db,
            telemetry_handle: None,
        }
    }

    /// Create a gate for tool authorization within this context.
    pub fn create_gate(&self) -> ExecutionGate<'_> {
        ExecutionGate::new(&self.guard_db, false, self.cwd.to_string_lossy().to_string())
    }
}

/// Definition of a tool that a plugin can execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    /// Optional parameter schema (JSON Schema format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// Capabilities a plugin declares it supports.
#[derive(Debug, Clone, PartialEq)]
pub enum Capability {
    ToolExecute,
    HealthCheck,
    LifecycleHooks,
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Capability::ToolExecute => write!(f, "tool/execute"),
            Capability::HealthCheck => write!(f, "health/check"),
            Capability::LifecycleHooks => write!(f, "lifecycle/hooks"),
        }
    }
}

/// The plugin trait — every ToolServer plugin must implement this.
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Unique identifier for this plugin type (e.g., "git-tools", "docker-utils").
    fn name(&self) -> &str;

    /// Human-readable version string (semver recommended).
    fn version(&self) -> &str;

    /// Capabilities this plugin declares it supports.
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::ToolExecute] // Default: at least tool execution
    }

    /// Called when the host starts up. Plugin initializes its state.
    async fn on_start(&mut self, _ctx: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Called when the host is shutting down. Clean up resources.
    async fn on_stop(&mut self, _ctx: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Health check — returns plugin status info.
    async fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }

    /// Execute a tool call with the given arguments.
    /// Returns stdout, stderr, and exit code.
    async fn execute_tool(
        &self,
        _tool_name: &str,
        _args: serde_json::Value,
    ) -> Result<ToolResult, PluginError> {
        Err(PluginError::NotImplemented(format!(
            "execute_tool not implemented for plugin '{}'",
            self.name()
        )))
    }

    /// List all tools this plugin can execute.
    fn list_tools(&self) -> Vec<ToolDefinition>;
}

// ============================================================================
// Process Pool — manages subprocess lifecycle with startup timing and health checks
// ============================================================================

/// Policy for handling process crashes.
#[derive(Debug, Clone)]
pub enum RestartPolicy {
    /// Never restart (default).
    NoRestart,
    /// Restart up to N times with exponential backoff.
    Retry { max_retries: u32 },
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::NoRestart
    }
}

/// Errors that can occur when spawning a plugin process.
#[derive(Error, Debug)]
pub enum SpawnError {
    #[error("startup timeout after {0:?}")]
    Timeout(Duration),

    #[error("process not found: {0}")]
    ProcessNotFound(PathBuf),

    #[error("permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("spawn failed: {0}")]
    SpawnFailed(String),
}

/// Handle to a running plugin process.
#[derive(Debug, Clone)]
pub struct PluginHandle {
    pub pid: Option<u32>,
    pub name: String,
    /// Time when the process was spawned (for startup latency tracking).
    pub spawn_time: std::time::Instant,
}

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
        Self::new(8, Duration::from_millis(100), Duration::from_secs(30), RestartPolicy::NoRestart)
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

        match tokio::time::timeout(self.startup_timeout, child_stdout.read_to_end(&mut handshake_buf)).await {
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
        // TODO: Implement actual JSON-RPC communication over stdin/stdout.
        // This is a stub — the real implementation would:
        // 1. Write a JSON-RPC request to child.stdin
        // 2. Read response from child.stdout with timeout
        // 3. Parse and return ToolResult
        Err(PluginError::NotImplemented("execute_with_timeout not yet implemented".into()))
    }

    /// Check health of all running plugins.
    pub async fn check_health(&self) -> Vec<(String, HealthStatus)> {
        let instances = self.instances.read().await;
        instances
            .iter()
            .map(|h| (h.name.clone(), HealthStatus::Healthy)) // TODO: actual health check
            .collect()
    }

    /// Stop all running plugins.
    pub async fn stop_all(&self) {
        let mut instances = self.instances.write().await;
        for handle in instances.iter() {
            if let Some(pid) = handle.pid {
                // TODO: Send SIGTERM via tokio::process::Command::new("kill") or similar.
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

// ============================================================================
// Plugin Registry — discovers and loads plugins
// ============================================================================

/// Manifest for a plugin (discovered from filesystem or config).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    /// Path to the binary (for subprocess plugins) or None for in-process.
    pub path: Option<PathBuf>,
    /// Language/runtime (e.g., "rust", "python", "go") — only for stdio plugins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Tools this plugin provides.
    pub tools: Vec<ToolDefinition>,
}

/// Registry of loaded plugins (uses Arc for interior mutability).
pub struct PluginRegistry {
    plugins: RwLock<Vec<(String, std::sync::Arc<dyn Plugin>)>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(Vec::new()),
        }
    }

    /// Register a plugin by name.
    pub async fn register(&self, name: String, plugin: std::sync::Arc<dyn Plugin>) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;
        if plugins.iter().any(|(n, _)| n == &name) {
            return Err(PluginError::AlreadyRegistered(name));
        }
        plugins.push((name, plugin));
        Ok(())
    }

    /// Get a plugin by name.
    pub async fn get(&self, name: &str) -> Option<std::sync::Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins.iter().find(|(n, _)| n == name).map(|(_, p)| Arc::clone(p))
    }

    /// List all registered plugin names.
    pub async fn list(&self) -> Vec<String> {
        let plugins = self.plugins.read().await;
        plugins.iter().map(|(n, _)| n.clone()).collect()
    }

    /// List all tools across all loaded plugins.
    pub async fn list_all_tools(&self) -> Vec<ToolDefinition> {
        let plugins = self.plugins.read().await;
        plugins.iter().flat_map(|(_, p)| p.list_tools()).collect()
    }

    /// Execute a tool call on the plugin that provides it.
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<ToolResult, PluginError> {
        let plugins = self.plugins.read().await;
        for (_name, plugin) in plugins.iter() {
            let tools = plugin.list_tools();
            if tools.iter().any(|t| t.name == tool_name) {
                return plugin.execute_tool(tool_name, args).await;
            }
        }
        Err(PluginError::NotFound(format!("tool '{}' not found", tool_name)))
    }

    /// Discover plugins from a directory (search for known binary names).
    pub fn discover_plugins(&self, _dir: &Path) -> Vec<PluginManifest> {
        // TODO: Implement filesystem discovery.
        // This would scan `dir/` for binaries matching known plugin patterns
        // and read their --version output to populate manifests.
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock plugin for testing.
    #[derive(Clone)]
    struct MockPlugin {
        name: String,
        version: String,
        tools: Vec<ToolDefinition>,
    }

    impl MockPlugin {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                tools: vec![ToolDefinition {
                    name: format!("{}.test", name),
                    description: format!("Test tool for {}", name),
                    parameters: None,
                }],
            }
        }
    }

    #[async_trait]
    impl Plugin for MockPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            &self.version
        }

        fn list_tools(&self) -> Vec<ToolDefinition> {
            self.tools.clone()
        }

        async fn execute_tool(
            &self,
            tool_name: &str,
            _args: serde_json::Value,
        ) -> Result<ToolResult, PluginError> {
            Ok(ToolResult {
                stdout: format!("executed {}", tool_name),
                stderr: String::new(),
                exit_code: 0,
                duration_ms: Some(5),
            })
        }

        async fn health_check(&self) -> HealthStatus {
            HealthStatus::Healthy
        }
    }

    #[tokio::test]
    async fn test_plugin_registry_register_and_get() {
        let registry = PluginRegistry::new();
        let plugin = std::sync::Arc::new(MockPlugin::new("mock"));
        registry.register("mock".to_string(), plugin).await.unwrap();

        assert_eq!(registry.list().await, vec!["mock"]);
    }

    #[tokio::test]
    async fn test_plugin_registry_duplicate_rejection() {
        let registry = PluginRegistry::new();
        registry
            .register("mock".to_string(), std::sync::Arc::new(MockPlugin::new("mock")))
            .await
            .unwrap();

        let result = registry
            .register("mock".to_string(), std::sync::Arc::new(MockPlugin::new("mock2")))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_plugin_execute_tool() {
        let registry = PluginRegistry::new();
        registry
            .register(
                "mock".to_string(),
                std::sync::Arc::new(MockPlugin::new("mock")),
            )
            .await
            .unwrap();

        let result = registry
            .execute_tool("mock.test", serde_json::json!({}))
            .await;
        assert!(result.is_ok());
        let tool_result = result.unwrap();
        assert_eq!(tool_result.exit_code, 0);
        assert_eq!(tool_result.stdout, "executed mock.test");
    }

    #[tokio::test]
    async fn test_plugin_registry_tool_not_found() {
        let registry = PluginRegistry::new();
        registry
            .register("mock".to_string(), std::sync::Arc::new(MockPlugin::new("mock")))
            .await
            .unwrap();

        let result = registry
            .execute_tool("nonexistent.tool", serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_process_pool_creation() {
        let pool = ProcessPool::default_rust();
        assert_eq!(pool.count().await, 0);
    }

    #[tokio::test]
    async fn test_health_status_display() {
        assert_eq!(format!("{}", HealthStatus::Healthy), "healthy");
        assert_eq!(
            format!("{}", HealthStatus::Degraded("slow".into())),
            "degraded: slow"
        );
        assert_eq!(
            format!("{}", HealthStatus::Unhealthy("crashed".into())),
            "unhealthy: crashed"
        );
    }

    #[tokio::test]
    async fn test_tool_result_serialization() {
        let result = ToolResult {
            stdout: "hello".to_string(),
            stderr: "warn".to_string(),
            exit_code: 0,
            duration_ms: Some(42),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("hello"));
        assert!(json.contains("42"));

        // Deserialize back
        let deserialized: ToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.stdout, "hello");
        assert_eq!(deserialized.exit_code, 0);
    }
}
