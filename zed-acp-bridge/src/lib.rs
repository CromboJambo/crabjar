use serde::{Deserialize, Serialize};
use thiserror::Error;

use agent_context::Store;
use crabjar_guard::GuardDb;
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

#[derive(Debug, Clone)]
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
    ) -> Result<zed_acp_server::GateResult, AcpBridgeError> {
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
