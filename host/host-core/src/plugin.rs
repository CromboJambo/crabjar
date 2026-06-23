/// Plugin API for the CrabJar host runtime.
///
/// Plugins are the "apps" that run on top of the host — Teams, Slack, etc.
/// Each plugin declares its lifecycle hooks and registers with the PluginRegistry.
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;
use tokio::sync::RwLock;
use crate::event_bus::EventBus;

/// Context available to plugins during their lifecycle.
pub struct PluginContext {
    pub plugin_id: Uuid,
    pub event_bus: Arc<EventBus>,
    pub config: HashMap<String, String>,
}

impl PluginContext {
    pub fn new(plugin_id: Uuid, event_bus: Arc<EventBus>, config: HashMap<String, String>) -> Self {
        Self {
            plugin_id,
            event_bus,
            config,
        }
    }

    /// Convenience: publish an event from this plugin.
    pub async fn emit(&self, kind: crate::event_bus::EventType) {
        let _ = self.event_bus.publish_typed(kind, self.plugin_id);
    }
}

/// The plugin trait — every app must implement this.
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Unique identifier for this plugin type.
    fn id(&self) -> &str;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Plugin version.
    fn version(&self) -> &str;

    /// Called when the host starts up. Plugin initializes its state.
    async fn on_start(&self, ctx: &PluginContext) -> Result<(), PluginError>;

    /// Called when the host is shutting down. Clean up resources.
    async fn on_stop(&self, ctx: &PluginContext) -> Result<(), PluginError>;

    /// Called when the plugin's main window/view should be shown.
    async fn on_show(&self, ctx: &PluginContext) -> Result<(), PluginError>;

    /// Called when the plugin's main window/view should be hidden.
    async fn on_hide(&self, ctx: &PluginContext) -> Result<(), PluginError>;

    /// Called when the plugin should handle a user action (e.g., tray click).
    async fn on_action(&self, ctx: &PluginContext, action: &str, data: Option<serde_json::Value>) -> Result<serde_json::Value, PluginError>;

    /// Health check — returns plugin status info.
    async fn health(&self, _ctx: &PluginContext) -> Result<serde_json::Value, PluginError> {
        Ok(serde_json::json!({
            "status": "healthy",
            "plugin": self.id(),
        }))
    }
}

/// Registry of loaded plugins.
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register a plugin.
    pub async fn register(&self, plugin: Box<dyn Plugin>) -> Result<(), PluginError> {
        let plugin_id = plugin.id().to_string();
        let mut map = self.plugins.write().await;
        if map.contains_key(&plugin_id) {
            return Err(PluginError::AlreadyRegistered(plugin_id.clone()));
        }
        map.insert(plugin_id.clone(), Arc::from(plugin));
        tracing::info!("plugin registered: {}", plugin_id);
        Ok(())
    }

    /// Get a plugin by ID.
    pub async fn get(&self, id: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.read().await.get(id).cloned()
    }

    /// List all registered plugin IDs.
    pub async fn list(&self) -> Vec<String> {
        self.plugins.read().await.keys().cloned().collect()
    }

    /// Unregister a plugin.
    pub async fn unregister(&self, id: &str) -> Result<(), PluginError> {
        let mut map = self.plugins.write().await;
        map.remove(id).ok_or_else(|| PluginError::NotFound(id.to_string()))?;
        tracing::info!("plugin unregistered: {}", id);
        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin errors.
#[derive(thiserror::Error, Debug)]
pub enum PluginError {
    #[error("plugin '{0}' already registered")]
    AlreadyRegistered(String),
    #[error("plugin '{0}' not found")]
    NotFound(String),
    #[error("plugin execution error: {0}")]
    Execution(String),
    #[error("plugin initialization failed: {0}")]
    Initialization(String),
}
