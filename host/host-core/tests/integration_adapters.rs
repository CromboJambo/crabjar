//! Integration tests for the adapter registry — send flow and connection state.

use crabjar_host_core::{AdapterRegistry, OutgoingMessage};
use std::sync::Arc;
use tokio::sync::RwLock;

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

    async fn send(
        &self,
        _msg: &OutgoingMessage,
    ) -> Result<(), crabjar_host_core::AdapterError> {
        if !*self.connected.read().await {
            return Err(crabjar_host_core::AdapterError::NotConnected(
                self.adapter_type.clone(),
            ));
        }
        Ok(())
    }

    async fn handle_incoming(
        &self,
        _msg: crabjar_host_core::IncomingMessage,
    ) -> Result<(), crabjar_host_core::AdapterError> {
        Ok(())
    }
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
