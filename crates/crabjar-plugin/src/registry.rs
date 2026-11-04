//! Plugin registry — discovers and loads plugins.

use crate::plugin_trait::Plugin;
use crate::types::{HealthStatus, PluginError, ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

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
    pub async fn register(
        &self,
        name: String,
        plugin: std::sync::Arc<dyn Plugin>,
    ) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;
        if plugins.iter().any(|(n, _)| n == &name) {
            return Err(PluginError::AlreadyRegistered(name));
        }
        plugins.push((name, plugin));
        Ok(())
    }

    /// Call on_load hook for a registered plugin.
    pub async fn load(&self, name: &str) -> Result<(), PluginError> {
        let plugins = self.plugins.read().await;
        let (_plugin_name, plugin) = plugins
            .iter()
            .find(|(n, _)| n == name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        plugin.on_load();
        Ok(())
    }

    /// Call on_unload hook for a registered plugin.
    pub async fn unload(&self, name: &str) -> Result<(), PluginError> {
        let plugins = self.plugins.read().await;
        let (plugin_name, _) = plugins
            .iter()
            .find(|(n, _)| n == name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        let plugin = self
            .get(plugin_name)
            .await
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        plugin.on_unload();
        info!("plugin '{}' unloaded", name);
        Ok(())
    }

    /// Unregister a plugin by name (calls on_unload first).
    pub async fn unregister(&self, name: &str) -> Result<(), PluginError> {
        // Call on_unload hook before removal
        let plugins = self.plugins.read().await;
        if !plugins.iter().any(|(n, _)| n == name) {
            return Err(PluginError::NotFound(name.to_string()));
        }

        let plugin = self
            .get(name)
            .await
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        plugin.on_unload();

        // Remove from registry
        let mut plugins = self.plugins.write().await;
        plugins.retain(|(n, _)| n != name);
        info!("plugin '{}' unregistered", name);
        Ok(())
    }

    /// Get a plugin by name.
    pub async fn get(&self, name: &str) -> Option<std::sync::Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, p)| Arc::clone(p))
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
        for (_plugin_name, plugin) in plugins.iter() {
            let tools = plugin.list_tools();
            if tools.iter().any(|t| t.name == tool_name) {
                match plugin.execute_tool(tool_name, args).await {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        // Call on_error hook before propagating
                        plugin.on_error(tool_name, &e);
                        return Err(e);
                    }
                }
            }
        }
        Err(PluginError::NotFound(format!(
            "tool '{}' not found",
            tool_name
        )))
    }

    /// Discover plugins from a directory (search for known binary names).
    pub fn discover_plugins(&self, _dir: &Path) -> Vec<PluginManifest> {
        // Filesystem discovery is pending — scans `dir/` for binaries matching known plugin patterns.
        vec![]
    }

    /// Bind all plugin tools to the tool registry via the tool binder.
    ///
    /// Converts each plugin's ToolDefinitions into LoadedSkill wrappers and
    /// registers them with the tool registry for unified discovery.
    pub async fn bind_to_registry(
        &self,
        registry: &crabjar_tool_registry::ToolRegistry<'_>,
    ) -> Result<usize, crabjar_tool_binding::BindingError> {
        use crabjar_skills::{LoadedSkill, SkillManifest};
        use crabjar_tool_binding::ToolBinder;

        let plugins = self.plugins.read().await;
        let binder = ToolBinder::new(registry);

        let mut total_bound = 0;

        for (name, plugin) in plugins.iter() {
            // Convert plugin tools to skill manifest format
            let tools: Vec<crabjar_skills::ToolDefinition> = plugin
                .list_tools()
                .into_iter()
                .map(|t| crabjar_skills::ToolDefinition {
                    name: t.name,
                    description: t.description,
                    parameters: t.parameters,
                    execution_policy: "low_risk".to_string(), // Default for plugins
                    trust_layer: 3,
                })
                .collect();

            let manifest = SkillManifest {
                name: format!("plugin-{}", name),
                version: plugin.version().to_string(),
                description: format!("Plugin: {}", name),
                author: Some("crabjar-plugin".to_string()),
                license: None,
                tools,
                dependencies: Vec::new(),
                context_requirements: None,
            };

            let loaded_skill = LoadedSkill {
                manifest,
                path: PathBuf::from("/tmp/plugin-binding"), // Placeholder path
            };

            let bound = binder.bind_skill(&loaded_skill)?;
            total_bound += bound;
        }

        info!(bound = total_bound, "bound plugin tools to registry");
        Ok(total_bound)
    }
}
