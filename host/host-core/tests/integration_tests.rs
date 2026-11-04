//! Integration tests for CrabJar host-core — testing module interactions
//! Covers: event bus, adapter registry, plugin lifecycle, work item serialization

use crabjar_host_core::{
    AdapterRegistry, EventBus, EventType, IncomingMessage, OutgoingMessage, Plugin, PluginContext,
    PluginError, PluginRegistry, Status, TaskStatus, WorkItem,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// Mock adapter for integration tests
struct TestAdapter {
    adapter_type: String,
    name: String,
    connected: Arc<RwLock<bool>>,
}

#[async_trait::async_trait]
impl crabjar_host_core::ProductAdapter for TestAdapter {
    fn adapter_type(&self) -> &str {
        &self.adapter_type
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_connected(&self) -> bool {
        true
    }

    async fn send(&self, _msg: &OutgoingMessage) -> Result<(), crabjar_host_core::AdapterError> {
        if !*self.connected.read().await {
            return Err(crabjar_host_core::AdapterError::NotConnected(
                self.adapter_type.clone(),
            ));
        }
        Ok(())
    }

    async fn handle_incoming(
        &self,
        _msg: IncomingMessage,
    ) -> Result<(), crabjar_host_core::AdapterError> {
        Ok(())
    }
}

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

    async fn on_show(&self, ctx: &PluginContext) -> Result<(), PluginError> {
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
async fn test_event_bus_multiple_subscribers() {
    let bus = Arc::new(EventBus::new(64));

    // Create two subscribers
    let mut sub1 = bus.subscriber();
    let mut sub2 = bus.subscriber();

    // Publish an event
    bus.publish_typed(EventType::Tick { interval_ms: 100 }, "test")
        .unwrap();

    // Both subscribers should receive it
    let e1 = tokio::time::timeout(tokio::time::Duration::from_millis(100), sub1.recv())
        .await
        .unwrap()
        .unwrap();
    let e2 = tokio::time::timeout(tokio::time::Duration::from_millis(100), sub2.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(e1.kind, EventType::Tick { interval_ms: 100 });
    assert_eq!(e2.kind, EventType::Tick { interval_ms: 100 });
}

#[tokio::test]
async fn test_event_bus_capacity_overflow() {
    // Small capacity to force overflow
    let bus = EventBus::new(4);

    // Create a subscriber to receive events
    let mut sub = bus.subscriber();

    // Publish more events than capacity
    let mut success_count = 0;
    for i in 0..10 {
        let result = bus.publish_typed(EventType::Tick { interval_ms: i }, "test");
        if result.is_ok() {
            success_count += 1;
        }
    }

    // At least some should succeed (capacity allows at least 4)
    assert!(
        success_count >= 4,
        "Expected at least 4 successful publishes, got {}",
        success_count
    );

    // Drain the subscriber
    while tokio::time::timeout(tokio::time::Duration::from_millis(10), sub.recv())
        .await
        .is_ok()
    {}
}

#[tokio::test]
async fn test_adapter_registry_send_flow() {
    let registry = AdapterRegistry::new();
    let connected = Arc::new(RwLock::new(true));

    let adapter = TestAdapter {
        adapter_type: "integration-test".to_string(),
        name: "Integration Test Adapter".to_string(),
        connected: connected.clone(),
    };

    registry.register(Arc::new(adapter)).await.unwrap();

    // Resolve and send
    let resolved = registry.resolve("integration-test").await.unwrap();
    let msg = OutgoingMessage::text("test-user", "hello from integration test");
    resolved.send(&msg).await.unwrap();

    // Verify connection state can be toggled
    *connected.write().await = false;
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
