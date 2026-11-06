//! Plugin types: errors, results, health status, tool definitions.

use serde::{Deserialize, Serialize};
use thiserror::Error;

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

    #[error("internal plugin error: {0}")]
    Internal(String),
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
