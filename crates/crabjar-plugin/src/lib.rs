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

pub mod context;
pub mod plugin_trait;
pub mod process_pool;
pub mod process_types;
pub mod registry;
pub mod types;

pub use context::PluginContext;
pub use plugin_trait::Plugin;
pub use process_pool::ProcessPool;
pub use process_types::{PluginHandle, RestartPolicy, SpawnError};
pub use registry::{PluginManifest, PluginRegistry};
pub use types::{Capability, HealthStatus, PluginError, ToolDefinition, ToolResult};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

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
            .register(
                "mock".to_string(),
                std::sync::Arc::new(MockPlugin::new("mock")),
            )
            .await
            .unwrap();

        let result = registry
            .register(
                "mock".to_string(),
                std::sync::Arc::new(MockPlugin::new("mock2")),
            )
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
            .register(
                "mock".to_string(),
                std::sync::Arc::new(MockPlugin::new("mock")),
            )
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

    #[tokio::test]
    async fn test_bind_plugins_to_registry() {
        let plugin_registry = PluginRegistry::new();
        plugin_registry
            .register(
                "test-plugin".to_string(),
                std::sync::Arc::new(MockPlugin::new("test-plugin")),
            )
            .await
            .unwrap();

        // Create tool registry in temp dir
        let tmp_dir = tempfile::tempdir().unwrap();
        let db_path = tmp_dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let tool_registry = crabjar_tool_registry::ToolRegistry::new(&conn);
        tool_registry.init().unwrap();

        // Bind plugin tools to registry
        let bound = plugin_registry
            .bind_to_registry(&tool_registry)
            .await
            .unwrap();
        assert_eq!(bound, 1); // One tool from the mock plugin

        // Verify tool is discoverable in registry
        let all_tools = tool_registry.list_all(10).unwrap();
        assert_eq!(all_tools.len(), 1);
        assert_eq!(all_tools[0].name, "test-plugin.test");
    }
}
