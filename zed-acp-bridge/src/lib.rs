use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use agent_context::Store;
use crabjar_guard::{GateResult, GuardDb};
use zed_acp_server::AcpSession;

#[derive(Error, Debug)]
pub enum AcpBridgeError {
    #[error("bridge error: {0}")]
    BridgeError(String),
    #[error("store error: {0}")]
    StoreError(String),
    #[error("guard error: {0}")]
    GuardError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub function_name: String,
    pub arguments: serde_json::Value,
}

impl ToolSchema {
    pub fn from_function_call(name: &str, args: &str) -> Result<Self, AcpBridgeError> {
        let parsed =
            serde_json::from_str(args).map_err(|e| AcpBridgeError::BridgeError(e.to_string()))?;
        Ok(Self {
            function_name: name.to_string(),
            arguments: parsed,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryEvent {
    pub event_type: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

pub struct AcpBridge {
    pub session: Option<AcpSession>,
    pub events: Vec<TrajectoryEvent>,
    pub knowledge_store: Option<Store>,
    pub guard_db: Option<GuardDb>,
}

impl AcpBridge {
    pub fn new() -> Self {
        Self {
            session: None,
            events: Vec::new(),
            knowledge_store: None,
            guard_db: None,
        }
    }

    pub fn with_session(mut self, session: AcpSession) -> Self {
        self.session = Some(session);
        self
    }

    pub fn with_knowledge_store(mut self, store: Store) -> Self {
        self.knowledge_store = Some(store);
        self
    }

    pub fn with_guard_db(mut self, guard_db: GuardDb) -> Self {
        self.guard_db = Some(guard_db);
        self
    }

    pub fn record_event(&mut self, event_type: &str, data: serde_json::Value) {
        let timestamp = chrono::Utc::now().to_rfc3339();
        self.events.push(TrajectoryEvent {
            event_type: event_type.to_string(),
            timestamp,
            data,
        });
    }

    pub fn query_knowledge(
        &self,
        tags: &[&str],
        limit: usize,
    ) -> Result<Vec<serde_json::Value>, AcpBridgeError> {
        let store = self
            .knowledge_store
            .as_ref()
            .ok_or_else(|| AcpBridgeError::StoreError("no knowledge store".into()))?;
        let rows = store
            .query(tags, limit, "", "", "")
            .map_err(|e| AcpBridgeError::StoreError(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "content": row.content,
                    "tags": row.tags,
                    "metadata": row.metadata,
                    "active": row.active,
                })
            })
            .collect())
    }

    pub fn check_guard(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<GateResult, AcpBridgeError> {
        let guard_db = self
            .guard_db
            .as_ref()
            .ok_or_else(|| AcpBridgeError::GuardError("no guard db".into()))?;
        let gate = crabjar_guard::ExecutionGate::new(guard_db, false, ".");
        let result = gate
            .check(crabjar_guard::GateContext {
                action_type: "bridge_tool",
                command,
                args: args.to_vec(),
                trust_layer: 3,
                confidence: crabjar_guard::TrustScore::new(0.9),
                source_event_id: None,
                can_interrupt: true,
                pid: None,
                scope: None,
                target_scope: None,
                domains: vec![], // zed-acp-bridge: no known domains at this layer
                context_budget: None,
                context_fragment_tokens: None,
            })
            .map_err(|e| AcpBridgeError::GuardError(e.to_string()))?;
        Ok(result)
    }

    pub fn trajectory(&self) -> &[TrajectoryEvent] {
        &self.events
    }
}

impl Default for AcpBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn new_creates_empty_bridge() {
        let bridge = AcpBridge::new();
        assert!(bridge.session.is_none());
        assert!(bridge.events.is_empty());
        assert!(bridge.knowledge_store.is_none());
        assert!(bridge.guard_db.is_none());
    }

    #[test]
    fn default_creates_empty_bridge() {
        let bridge: AcpBridge = Default::default();
        assert!(bridge.events.is_empty());
    }

    #[test]
    fn with_session_sets_session() {
        let session = AcpSession::new("/test".to_string());
        let bridge = AcpBridge::new().with_session(session);
        assert!(bridge.session.is_some());
    }

    #[test]
    fn with_knowledge_store_sets_store() {
        let db = agent_context::Store::open(":memory:").unwrap();
        let bridge = AcpBridge::new().with_knowledge_store(db);
        assert!(bridge.knowledge_store.is_some());
    }

    #[test]
    fn with_guard_db_sets_db() {
        let dir = tempdir().unwrap();
        let guard_db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let bridge = AcpBridge::new().with_guard_db(guard_db);
        assert!(bridge.guard_db.is_some());
    }

    #[test]
    fn record_event_adds_to_trajectory() {
        let mut bridge = AcpBridge::new();
        bridge.record_event("test_event", serde_json::json!({"key": "value"}));
        assert_eq!(bridge.events.len(), 1);
        assert_eq!(bridge.events[0].event_type, "test_event");
        assert_eq!(bridge.events[0].data["key"], "value");
    }

    #[test]
    fn record_event_sets_timestamp() {
        let mut bridge = AcpBridge::new();
        bridge.record_event("event", serde_json::json!({}));
        assert!(!bridge.events[0].timestamp.is_empty());
        assert!(bridge.events[0].timestamp.contains('-'));
    }

    #[test]
    fn trajectory_returns_reference() {
        let mut bridge = AcpBridge::new();
        bridge.record_event("e1", serde_json::json!({}));
        let traj = bridge.trajectory();
        assert_eq!(traj.len(), 1);
    }

    #[test]
    fn trajectory_empty_returns_empty_slice() {
        let bridge = AcpBridge::new();
        assert!(bridge.trajectory().is_empty());
    }

    #[test]
    fn tool_schema_from_function_call_valid() {
        let schema = ToolSchema::from_function_call("echo", r#"{"msg": "hello"}"#).unwrap();
        assert_eq!(schema.function_name, "echo");
        assert_eq!(schema.arguments["msg"], "hello");
    }

    #[test]
    fn tool_schema_from_function_call_invalid_json_fails() {
        let result = ToolSchema::from_function_call("echo", "not json");
        assert!(result.is_err());
    }

    #[test]
    fn query_knowledge_without_store_returns_error() {
        let bridge = AcpBridge::new();
        let result = bridge.query_knowledge(&["test"], 10);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("no knowledge store")
        );
    }

    #[test]
    fn check_guard_without_guard_db_returns_error() {
        let bridge = AcpBridge::new();
        let result = bridge.check_guard("echo", &["hello".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no guard db"));
    }

    #[test]
    fn multiple_event_types_preserved() {
        let mut bridge = AcpBridge::new();
        bridge.record_event("start", serde_json::json!({"step": 1}));
        bridge.record_event("middle", serde_json::json!({"step": 2}));
        bridge.record_event("end", serde_json::json!({"step": 3}));
        assert_eq!(bridge.trajectory().len(), 3);
        assert_eq!(bridge.trajectory()[0].event_type, "start");
        assert_eq!(bridge.trajectory()[1].event_type, "middle");
        assert_eq!(bridge.trajectory()[2].event_type, "end");
    }

    #[test]
    fn tool_schema_arguments_can_be_nested() {
        let json = r#"{"nested": {"key": "value"}, "list": [1, 2, 3]}"#;
        let schema = ToolSchema::from_function_call("complex", json).unwrap();
        assert_eq!(schema.arguments["nested"]["key"], "value");
        assert_eq!(schema.arguments["list"][0], 1);
    }

    #[test]
    fn trajectory_event_clone_works() {
        let mut bridge = AcpBridge::new();
        bridge.record_event("test", serde_json::json!({"data": 42}));
        let event = bridge.events[0].clone();
        assert_eq!(event.event_type, "test");
        assert_eq!(event.data["data"], 42);
    }
}
