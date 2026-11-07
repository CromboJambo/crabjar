//! WasmPluginAdapter — wraps a Wasm component instance as a Plugin trait implementation.
//!
//! This enables dynamic loading of .wasm files as plugins that integrate with the
//! existing PluginRegistry infrastructure.

use crate::plugin_trait::Plugin;
use crate::types::{Capability, HealthStatus, PluginError, ToolDefinition, ToolResult};
use crate::wasm::WasmEngine;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

/// Adapter that wraps a Wasm component instance as a Plugin.
pub struct WasmPluginAdapter {
    name: String,
    version: String,
    engine: Arc<WasmEngine>,
}

impl WasmPluginAdapter {
    /// Create a new WasmPluginAdapter from component bytes.
    pub fn new(
        name: &str,
        version: &str,
        engine: WasmEngine,
        _component_bytes: &[u8],
    ) -> Result<Self, PluginError> {
        // In a full implementation, this would load and instantiate the component.
        // For now, we store the engine and name for future use.
        Ok(Self {
            name: name.to_string(),
            version: version.to_string(),
            engine: Arc::new(engine),
        })
    }

    /// Create from a .wasm file path.
    pub fn from_file(
        name: &str,
        version: &str,
        engine: WasmEngine,
        path: &std::path::Path,
    ) -> Result<Self, PluginError> {
        let bytes = std::fs::read(path).map_err(|e| {
            PluginError::Internal(format!(
                "Failed to read wasm file '{}': {}",
                path.display(),
                e
            ))
        })?;
        Self::new(name, version, engine, &bytes)
    }

    /// Get the underlying WasmEngine.
    pub fn engine(&self) -> &Arc<WasmEngine> {
        &self.engine
    }
}

#[async_trait]
impl Plugin for WasmPluginAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::ToolExecute]
    }

    fn on_load(&self) {
        info!("Wasm plugin '{}' loaded", self.name);
    }

    fn on_unload(&self) {
        info!("Wasm plugin '{}' unloaded", self.name);
    }

    async fn health_check(&self) -> HealthStatus {
        // Simplified: assume healthy if the adapter exists
        HealthStatus::Healthy
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<ToolResult, PluginError> {
        // Simplified stub — actual Wasm invocation would go here
        info!("Executing Wasm tool '{}' with args", tool_name);
        Ok(ToolResult {
            stdout: format!("[wasm:{}:stub] executed with {}", self.name, args),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        // Simplified: return a static tool definition
        vec![ToolDefinition {
            name: format!("wasm_{}", self.name),
            description: format!("Wasm plugin tool for '{}'", self.name),
            parameters: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let engine = WasmEngine::new().unwrap();

        // Create a minimal Wasm module for testing (same as wasm.rs tests)
        let wasm_bytes: Vec<u8> = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // magic + version
            0x01, 0x07, 0x01, 0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, // type section
            0x03, 0x02, 0x01, 0x00, // function section
            0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00, // export section
            0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b, // code section
        ];

        let adapter = WasmPluginAdapter::new("test_wasm", "1.0.0", engine, &wasm_bytes);
        assert!(adapter.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_trait_implementation() {
        let engine = WasmEngine::new().unwrap();
        let wasm_bytes: Vec<u8> = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60, 0x02, 0x7f,
            0x7f, 0x01, 0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64,
            0x00, 0x00, 0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b,
        ];

        let adapter = WasmPluginAdapter::new("test_trait", "1.0.0", engine, &wasm_bytes).unwrap();

        // Test Plugin trait methods
        assert_eq!(adapter.name(), "test_trait");
        assert_eq!(adapter.version(), "1.0.0");
        assert!(!adapter.capabilities().is_empty());

        // Test health check
        let status = adapter.health_check().await;
        assert_eq!(status, HealthStatus::Healthy);

        // Test tool execution (stub)
        let result = adapter
            .execute_tool("test_tool", serde_json::json!({"x": 1}))
            .await;
        assert!(result.is_ok());

        // Test tool listing
        let tools = adapter.list_tools();
        assert!(!tools.is_empty());
    }
}
