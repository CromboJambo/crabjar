//! Integration tests for plugin lifecycle — start/stop with event bus interaction.

use crabjar_host_core::{EventBus, EventType, Plugin, PluginContext, PluginError, PluginRegistry};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// Mock plugin for integration tests
struct TestPlugin {
    id: String,
    name: String,
    version: String,
    started: Arc<RwLock<bool>>,
    events_received: Arc<RwLock<Vec<String>>>,
}

#[async_trait::async_trait]
impl Plugin for TestPlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn on_start(&self, _ctx: &PluginContext) -> Result<(), PluginError> {
        *self.started.write().await = true;
        Ok(())
    }

    async fn on_stop(&self, _ctx: &PluginContext) -> Result<(), PluginError> {
        *self.started.write().await = false;
        Ok(())
    }

    async fn on_show(&self, _ctx: &PluginContext) -> Result<(), PluginError> {
        self.events_received.write().await.push("shown".to_string());
        Ok(())
    }

    async fn on_hide(&self, _ctx: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn on_action(
        &self,
        ctx: &PluginContext,
        action: &str,
        data: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, PluginError> {
        self.events_received
            .write()
            .await
            .push(format!("action:{}:{}", action, data.is_some()));
        Ok(serde_json::json!({"handled": true, "action": action}))
    }
}

#[tokio::test]
async fn test_plugin_lifecycle_with_event_bus() {
    let bus = Arc::new(EventBus::new(128));
    let registry = PluginRegistry::new();
    let started = Arc::new(RwLock::new(false));
    let events_received = Arc::new(RwLock::new(Vec::new()));

    let plugin = TestPlugin {
        id: "integration-plugin".to_string(),
        name: "Integration Plugin".to_string(),
        version: "1.0.0".to_string(),
        started: started.clone(),
        events_received: events_received.clone(),
    };

    registry.register(Box::new(plugin)).await.unwrap();

    // Get plugin and start it
    let resolved = registry.get("integration-plugin").await.unwrap();
    let ctx = PluginContext::new(Uuid::new_v4(), bus.clone(), HashMap::new());
    resolved.on_start(&ctx).await.unwrap();

    assert!(*started.read().await);

    // Publish an event through the plugin context
    ctx.emit(EventType::Tick { interval_ms: 50 }).await;

    // Verify plugin can receive events from bus
    let _sub = bus.subscriber();
    bus.publish_typed(
        EventType::Agent {
            stage: "testing".into(),
            work_item_id: "123".into(),
        },
        "test",
    )
    .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Stop the plugin
    resolved.on_stop(&ctx).await.unwrap();
    assert!(!*started.read().await);
}
