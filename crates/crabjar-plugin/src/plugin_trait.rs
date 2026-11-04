//! Plugin trait definition.

use crate::types::{Capability, HealthStatus, PluginError, ToolDefinition, ToolResult};
use async_trait::async_trait;
use tracing::{info, warn};

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

    /// Called when the plugin is loaded/registered. Initializes state.
    fn on_load(&self) {
        info!("plugin '{}' loaded", self.name());
    }

    /// Called when the plugin is unloaded/unregistered. Clean up resources.
    fn on_unload(&self) {
        info!("plugin '{}' unloaded", self.name());
    }

    /// Called when a tool execution fails with an error.
    fn on_error(&self, tool_name: &str, error: &PluginError) {
        warn!(
            plugin = self.name(),
            tool = tool_name,
            error = %error,
            "plugin tool execution error"
        );
    }

    /// Legacy alias for on_load — called when the host starts up.
    fn on_start(&self) {
        self.on_load();
    }

    /// Legacy alias for on_unload — called when the host is shutting down.
    fn on_stop(&self) {
        self.on_unload();
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
