//! Product Adapter Pattern — channel-agnostic abstraction for host integrations.
//!
//! Every channel (MQTT/Home Assistant, Graph API/Teams, Discord, etc.) currently
//! requires core changes. The adapter pattern contains that blast radius by defining
//! a stable interface that channels implement and the host discovers at runtime.
//!
//! ## Design
//!
//! ```text
//!   host-core (event bus)
//!        │
//!        ▼
//!   ┌──────────────┐
//!   │ AdapterRegis- │ discovers, registers, resolves adapters
//!   │ try          │
//!   └──────┬───────┘
//!          │ resolve("mqtt") / resolve("graph")
//!          ▼
//!   ┌──────────────┐    ┌──────────────┐
//!   │ MqttAdapter  │    │ GraphAdapter │  ← channel-specific impls
//!   └──────────────┘    └──────────────┘
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! let registry = AdapterRegistry::new();
//! registry.register(Box::new(MqttAdapter::new(config))).await?;
//! registry.register(Box::new(GraphAdapter::new(config))).await?;
//!
//! // Resolve an adapter by type
//! if let Some(adapter) = registry.resolve("mqtt") {
//!     adapter.send(status_update).await?;
//! }
//! ```

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A normalized outgoing message that all adapters produce.
///
/// This is the canonical output format — adapters translate
/// channel-specific data into this representation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct OutgoingMessage {
    /// Target destination (e.g., "user-id", "team-channel-123")
    pub to: String,
    /// Message content (plain text or markdown)
    pub content: String,
    /// Message type hint: "text", "markdown", "image", "file"
    pub kind: String,
    /// Whether this is a presence/status update rather than a chat message
    pub is_status: bool,
    /// Metadata for channel-specific features (rendering hints, reactions, etc.)
    pub metadata: HashMap<String, String>,
}

impl OutgoingMessage {
    pub fn text(to: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            content: content.into(),
            kind: "text".into(),
            is_status: false,
            metadata: HashMap::new(),
        }
    }

    pub fn markdown(to: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            content: content.into(),
            kind: "markdown".into(),
            is_status: false,
            metadata: HashMap::new(),
        }
    }

    pub fn status(to: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            content: content.into(),
            kind: "text".into(),
            is_status: true,
            metadata: HashMap::new(),
        }
    }
}

/// A normalized incoming message that all adapters produce.
///
/// This is the canonical input format — adapters translate
/// channel-specific data into this representation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct IncomingMessage {
    /// Sender identifier (user ID, email, etc.)
    pub from: String,
    /// Message content
    pub content: String,
    /// Message type hint
    pub kind: String,
    /// Channel/source identifier (e.g., "mqtt", "graph")
    pub source: String,
    /// Timestamp (Unix ms)
    pub timestamp: i64,
    /// Optional thread/conversation ID for grouping
    pub thread_id: Option<String>,
    /// Metadata from the source channel
    pub metadata: HashMap<String, String>,
}

impl IncomingMessage {
    pub fn text(from: impl Into<String>, content: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            content: content.into(),
            kind: "text".into(),
            source: source.into(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            thread_id: None,
            metadata: HashMap::new(),
        }
    }
}

/// The ProductAdapter trait — every channel implements this.
///
/// New channels = new adapter, no core changes.
#[async_trait]
pub trait ProductAdapter: Send + Sync {
    /// Unique adapter type identifier (e.g., "mqtt", "graph", "discord")
    fn adapter_type(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Whether this adapter is enabled and connected
    fn is_connected(&self) -> bool;

    /// Send a message through this adapter
    async fn send(&self, msg: &OutgoingMessage) -> Result<(), AdapterError>;

    /// Handle an incoming message from the channel
    async fn handle_incoming(&self, msg: IncomingMessage) -> Result<(), AdapterError>;

    /// Health check — returns adapter status info
    async fn health(&self) -> serde_json::Value {
        serde_json::json!({
            "adapter": self.adapter_type(),
            "name": self.name(),
            "connected": self.is_connected(),
            "status": "healthy",
        })
    }
}

/// Adapter registry — discovery and lifecycle for channel adapters.
pub struct AdapterRegistry {
    adapters: RwLock<HashMap<String, Arc<dyn ProductAdapter>>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: RwLock::new(HashMap::new()),
        }
    }

    /// Register an adapter.
    pub async fn register(&self, adapter: Arc<dyn ProductAdapter>) -> Result<(), AdapterError> {
        let adapter_type = adapter.adapter_type().to_string();
        let name = adapter.name().to_string();
        let mut map = self.adapters.write().await;
        if map.contains_key(&adapter_type) {
            return Err(AdapterError::AlreadyRegistered(adapter_type));
        }
        let adapter_type_clone = adapter_type.clone();
        map.insert(adapter_type, adapter);
        tracing::info!(type = adapter_type_clone, name, "adapter registered");
        Ok(())
    }

    /// Resolve an adapter by type.
    pub async fn resolve(&self, adapter_type: &str) -> Option<Arc<dyn ProductAdapter>> {
        self.adapters.read().await.get(adapter_type).cloned()
    }

    /// List all registered adapter types.
    pub async fn list(&self) -> Vec<String> {
        self.adapters.read().await.keys().cloned().collect()
    }

    /// Unregister an adapter.
    pub async fn unregister(&self, adapter_type: &str) -> Result<(), AdapterError> {
        let mut map = self.adapters.write().await;
        map.remove(adapter_type)
            .ok_or_else(|| AdapterError::NotFound(adapter_type.to_string()))?;
        tracing::info!(adapter = adapter_type, "adapter unregistered");
        Ok(())
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter errors.
#[derive(thiserror::Error, Debug)]
pub enum AdapterError {
    #[error("adapter '{0}' already registered")]
    AlreadyRegistered(String),

    #[error("adapter '{0}' not found")]
    NotFound(String),

    #[error("adapter send error: {0}")]
    SendError(String),

    #[error("adapter receive error: {0}")]
    ReceiveError(String),

    #[error("adapter not connected")]
    NotConnected(String),

    #[error("adapter initialization failed: {0}")]
    Initialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock adapter for testing — no real network IO.
    #[derive(Clone)]
    struct TestAdapter {
        adapter_type: String,
        name: String,
        connected: bool,
    }

    #[async_trait]
    impl ProductAdapter for TestAdapter {
        fn adapter_type(&self) -> &str {
            &self.adapter_type
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn is_connected(&self) -> bool {
            self.connected
        }

        async fn send(&self, _msg: &OutgoingMessage) -> Result<(), AdapterError> {
            if !self.connected {
                return Err(AdapterError::NotConnected(self.adapter_type.clone()));
            }
            Ok(())
        }

        async fn handle_incoming(&self, _msg: IncomingMessage) -> Result<(), AdapterError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_register_and_resolve() {
        let registry = AdapterRegistry::new();
        let adapter = Arc::new(TestAdapter {
            adapter_type: "test".into(),
            name: "Test Adapter".into(),
            connected: false,
        });

        registry.register(adapter.clone()).await.unwrap();
        let resolved = registry.resolve("test").await;
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().adapter_type(), "test");
    }

    #[tokio::test]
    async fn test_resolve_missing() {
        let registry = AdapterRegistry::new();
        let resolved = registry.resolve("nonexistent").await;
        assert!(resolved.is_none());
    }

    #[tokio::test]
    async fn test_duplicate_register_fails() {
        let registry = AdapterRegistry::new();
        let adapter1 = Arc::new(TestAdapter {
            adapter_type: "test".into(),
            name: "Test 1".into(),
            connected: false,
        });
        let adapter2 = Arc::new(TestAdapter {
            adapter_type: "test".into(),
            name: "Test 2".into(),
            connected: false,
        });

        registry.register(adapter1).await.unwrap();
        assert!(registry.register(adapter2).await.is_err());
    }

    #[tokio::test]
    async fn test_send_requires_connection() {
        let registry = AdapterRegistry::new();
        let adapter = Arc::new(TestAdapter {
            adapter_type: "test".into(),
            name: "Test".into(),
            connected: false,
        });
        registry.register(adapter.clone()).await.unwrap();

        let msg = OutgoingMessage::text("user", "hello");
        assert!(adapter.send(&msg).await.is_err());

        // Simulate connecting
        // (In real adapters, this would be triggered by MQTT connect / Graph auth)
        // For this test, we just verify the is_connected gate works
        assert!(!adapter.is_connected());
    }

    #[tokio::test]
    async fn test_list_adapters() {
        let registry = AdapterRegistry::new();
        registry
            .register(Arc::new(TestAdapter {
                adapter_type: "mqtt".into(),
                name: "MQTT".into(),
                connected: false,
            }))
            .await
            .unwrap();
        registry
            .register(Arc::new(TestAdapter {
                adapter_type: "graph".into(),
                name: "Graph".into(),
                connected: false,
            }))
            .await
            .unwrap();

        let mut types = registry.list().await;
        types.sort();
        assert_eq!(types, vec!["graph", "mqtt"]);
    }

    #[test]
    fn test_outgoing_message_defaults() {
        let msg = OutgoingMessage::text("alice", "hello");
        assert_eq!(msg.to, "alice");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.kind, "text");
        assert!(!msg.is_status);
    }

    #[test]
    fn test_incoming_message_defaults() {
        let msg = IncomingMessage::text("alice", "hello", "mqtt");
        assert_eq!(msg.from, "alice");
        assert_eq!(msg.source, "mqtt");
        assert_eq!(msg.kind, "text");
    }
}
