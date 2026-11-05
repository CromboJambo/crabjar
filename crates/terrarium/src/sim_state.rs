use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::sim_event::SimEvent;

/// Shared simulation state — protected by Arc<Mutex<T>> for tokio task sharing.
#[derive(Debug)]
pub struct SimState {
    pub tick: u64,
    pub agents: HashMap<String, serde_json::Value>,
    pub resources: HashMap<String, f64>,
    pub events: Vec<SimEvent>,
}

impl SimState {
    pub fn new() -> Self {
        let mut s = Self {
            tick: 0,
            agents: HashMap::new(),
            resources: HashMap::new(),
            events: Vec::with_capacity(128),
        };
        s.resources.insert("food".to_string(), 10.0);
        s.resources.insert("water".to_string(), 5.0);
        s.agents.insert(
            "agent-1".to_string(),
            serde_json::json!({"pos": [0,0], "energy": 100}),
        );
        s.agents.insert(
            "agent-2".to_string(),
            serde_json::json!({"pos": [5,5], "energy": 80}),
        );
        s
    }

    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "tick": self.tick,
            "agent_count": self.agents.len(),
            "resource_count": self.resources.len(),
            "event_count": self.events.len(),
        })
    }

    pub fn get_agent(&self, id: &str) -> Option<&serde_json::Value> {
        self.agents.get(id)
    }

    pub fn set_agent(&mut self, id: String, state: serde_json::Value) {
        self.agents.insert(id, state);
    }

    pub fn get_resource(&self, name: &str) -> Option<f64> {
        self.resources.get(name).copied()
    }

    pub fn set_resource(&mut self, name: String, value: f64) {
        self.resources.insert(name, value);
    }

    pub fn emit_event(&mut self, event: SimEvent) {
        self.events.push(event);
    }
}

/// Thread-safe wrapper for shared simulation state.
pub type SharedSimState = Arc<Mutex<SimState>>;
