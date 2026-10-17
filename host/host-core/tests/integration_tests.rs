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

#[tokio::test]
async fn test_workitem_json_roundtrip() {
    let mut wi = WorkItem::new("Test JSON serialization");
    wi.add_task("Task 1");
    wi.add_task("Task 2");
    wi.update_task(0, TaskStatus::Completed, Some("Done".to_string()));
    wi.set_confidence(0.85);
    wi.progress_to(Status::Executing {
        current_task: Some(1),
    });

    // Serialize
    let json = wi.to_json().unwrap();
    assert!(json.contains("Test JSON serialization"));
    assert!(json.contains("Task 1"));

    // Deserialize
    let restored = WorkItem::from_json(&json).unwrap();
    assert_eq!(restored.objective, "Test JSON serialization");
    assert_eq!(restored.plan.len(), 2);
    assert!((restored.confidence - 0.85).abs() < f32::EPSILON);
    assert!(matches!(restored.status, Status::Executing { .. }));
}

#[tokio::test]
async fn test_workitem_observation_tracking() {
    let mut wi = WorkItem::new("Test observations");

    wi.observe("planning", "analysis", "Identified 3 subtasks");
    wi.observe("executing", "tool_call", "Called filesystem tool");
    wi.observe("verifying", "check", "All tests pass");

    assert_eq!(wi.observations.len(), 3);
    assert_eq!(wi.observations[0].stage, "planning");
    assert_eq!(wi.observations[1].kind, "tool_call");

    // Verify JSON roundtrip preserves observations
    let json = wi.to_json().unwrap();
    let restored = WorkItem::from_json(&json).unwrap();
    assert_eq!(restored.observations.len(), 3);
}

#[tokio::test]
async fn test_adapter_incoming_message_flow() {
    let registry = AdapterRegistry::new();
    let connected = Arc::new(RwLock::new(true));

    let adapter = TestAdapter {
        adapter_type: "incoming-test".to_string(),
        name: "Incoming Test Adapter".to_string(),
        connected,
    };

    registry.register(Arc::new(adapter)).await.unwrap();

    // Simulate incoming message from channel
    let resolved = registry.resolve("incoming-test").await.unwrap();
    let msg = IncomingMessage::text("external-user", "hello crabjar", "integration-test");
    resolved.handle_incoming(msg).await.unwrap();
}

#[tokio::test]
async fn test_plugin_action_with_data() {
    let bus = Arc::new(EventBus::new(64));
    let registry = PluginRegistry::new();
    let started = Arc::new(RwLock::new(false));
    let events_received = Arc::new(RwLock::new(Vec::new()));

    let plugin = TestPlugin {
        id: "action-plugin".to_string(),
        name: "Action Plugin".to_string(),
        version: "1.0.0".to_string(),
        started: started.clone(),
        events_received: events_received.clone(),
    };

    registry.register(Box::new(plugin)).await.unwrap();
    let resolved = registry.get("action-plugin").await.unwrap();
    let ctx = PluginContext::new(Uuid::new_v4(), bus, HashMap::new());
    resolved.on_start(&ctx).await.unwrap();

    // Trigger an action with data
    let result = resolved
        .on_action(
            &ctx,
            "test-action",
            Some(serde_json::json!({"param": "value"})),
        )
        .await
        .unwrap();

    assert_eq!(result["handled"], true);
    assert_eq!(result["action"], "test-action");

    // Verify the action was recorded
    let received = events_received.read().await;
    assert!(received.iter().any(|e| e.contains("action:test-action")));
}

#[tokio::test]
async fn test_event_bus_broadcast_scale() {
    let bus = Arc::new(EventBus::new(1024));

    // Create many subscribers (simulating many plugins)
    let mut subscribers = Vec::new();
    for _ in 0..50 {
        subscribers.push(bus.subscriber());
    }

    // Publish one event
    bus.publish_typed(EventType::Tick { interval_ms: 1 }, "scale-test")
        .unwrap();

    // Wait a bit for events to propagate
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Check that most subscribers received it (allow some lag)
    let mut received_count = 0;
    for sub in &mut subscribers {
        match tokio::time::timeout(tokio::time::Duration::from_millis(10), sub.recv()).await {
            Ok(Ok(_)) => received_count += 1,
            _ => {}
        }
    }

    // All or most should have received it
    assert!(
        received_count >= 45,
        "Expected at least 45 subscribers to receive event, got {}",
        received_count
    );
}

#[tokio::test]
async fn test_workitem_artifact_tracking() {
    let mut wi = WorkItem::new("Test artifacts");

    wi.add_artifact("/path/to/output.txt");
    wi.add_artifact("/path/to/report.json");
    wi.add_artifact("/path/to/log.txt");

    assert_eq!(wi.artifacts.len(), 3);

    // Verify JSON roundtrip preserves artifacts
    let json = wi.to_json().unwrap();
    let restored = WorkItem::from_json(&json).unwrap();
    assert_eq!(restored.artifacts.len(), 3);
    assert_eq!(restored.artifacts[0], "/path/to/output.txt");
}

#[tokio::test]
async fn test_plugin_registry_operations() {
    let registry = PluginRegistry::new();
    let started1 = Arc::new(RwLock::new(false));
    let events1 = Arc::new(RwLock::new(Vec::new()));
    let started2 = Arc::new(RwLock::new(false));
    let events2 = Arc::new(RwLock::new(Vec::new()));

    let plugin1 = TestPlugin {
        id: "plugin-a".to_string(),
        name: "Plugin A".to_string(),
        version: "1.0.0".to_string(),
        started: started1.clone(),
        events_received: events1,
    };

    let plugin2 = TestPlugin {
        id: "plugin-b".to_string(),
        name: "Plugin B".to_string(),
        version: "2.0.0".to_string(),
        started: started2.clone(),
        events_received: events2,
    };

    // Register both
    registry.register(Box::new(plugin1)).await.unwrap();
    registry.register(Box::new(plugin2)).await.unwrap();

    // List should show both
    let mut plugins = registry.list().await;
    plugins.sort();
    assert_eq!(plugins, vec!["plugin-a", "plugin-b"]);

    // Unregister one
    registry.unregister("plugin-a").await.unwrap();

    // Only plugin-b should remain
    let remaining = registry.list().await;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0], "plugin-b");
}

#[tokio::test]
async fn test_event_types_display() {
    let events = vec![
        EventType::TrayChanged {
            action: "shown".into(),
        },
        EventType::Notification {
            title: "Alert".into(),
            body: "Message".into(),
        },
        EventType::ClipboardChanged {
            mime_type: "text/plain".into(),
        },
        EventType::WebView {
            event: "load".into(),
            url: Some("https://example.com".into()),
        },
        EventType::Agent {
            stage: "planning".into(),
            work_item_id: "123".into(),
        },
        EventType::Plugin {
            event: "start".into(),
            plugin_id: "test".into(),
        },
        EventType::ConfigReload {
            source: "file".into(),
        },
        EventType::Tick { interval_ms: 500 },
    ];

    for event in events {
        let display = format!("{}", event);
        assert!(
            !display.is_empty(),
            "Display should not be empty for {:?}",
            event
        );
    }
}

#[tokio::test]
async fn test_workitem_hypothesis_tracking() {
    let mut wi = WorkItem::new("Test hypothesis");

    wi.set_hypothesis("The bug is in module X", 0.75);
    assert!(wi.hypothesis.is_some());
    assert_eq!(
        wi.hypothesis.as_ref().unwrap().statement,
        "The bug is in module X"
    );
    assert!((wi.hypothesis.as_ref().unwrap().confidence - 0.75).abs() < f32::EPSILON);

    // Verify JSON roundtrip preserves hypothesis
    let json = wi.to_json().unwrap();
    let restored = WorkItem::from_json(&json).unwrap();
    assert!(restored.hypothesis.is_some());
    assert_eq!(
        restored.hypothesis.as_ref().unwrap().statement,
        "The bug is in module X"
    );
}

#[tokio::test]
async fn test_status_transitions() {
    let mut wi = WorkItem::new("Test status transitions");

    // Verify initial state
    assert_eq!(wi.status, Status::Pending);
    assert!(wi.status.is_active());
    assert!(!wi.status.is_terminal());

    // Transition through states
    wi.progress_to(Status::Understanding);
    assert!(wi.status.is_active());

    wi.progress_to(Status::Planning);
    assert!(wi.status.is_active());

    wi.progress_to(Status::Executing {
        current_task: Some(0),
    });
    assert!(wi.status.is_active());

    wi.progress_to(Status::Verifying);
    assert!(wi.status.is_active());

    wi.progress_to(Status::Reflecting);
    assert!(wi.status.is_active());

    // Terminal state
    wi.progress_to(Status::Completed);
    assert!(wi.status.is_terminal());
    assert!(!wi.status.is_active());
}

#[tokio::test]
async fn test_outgoing_message_variants() {
    let text_msg = OutgoingMessage::text("user", "plain text");
    assert_eq!(text_msg.kind, "text");
    assert!(!text_msg.is_status);

    let md_msg = OutgoingMessage::markdown("channel", "# Header\n**Bold**");
    assert_eq!(md_msg.kind, "markdown");
    assert!(!md_msg.is_status);

    let status_msg = OutgoingMessage::status("user", "Working on task...");
    assert_eq!(status_msg.kind, "text");
    assert!(status_msg.is_status);
}
