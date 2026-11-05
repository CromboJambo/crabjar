//! CrabjarSurfaceAdapter: Translates crabjar-conductor HTTP API responses
//! into herdr-protocol wire types for GPUI rendering.
//!
//! This adapter enables the crabjar-gpui dashboard to display multi-agent
//! state using the same surface patch protocol as herdr-gpui, providing
//! GPU-accelerated rendering of agent status across split panes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent lifecycle status mapped from crabjar conductor worker states
/// to herdr-protocol AgentStatus enum values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Working,
    Blocked,
    Done,
    Unknown,
}

impl From<&str> for AgentStatus {
    fn from(status: &str) -> Self {
        match status {
            "idle" | "available" => AgentStatus::Idle,
            "working" | "busy" | "running" => AgentStatus::Working,
            "blocked" | "waiting" | "paused" => AgentStatus::Blocked,
            "done" | "completed" | "finished" => AgentStatus::Done,
            _ => AgentStatus::Unknown,
        }
    }
}

impl AgentStatus {
    /// Color code for status indicators (RGB values)
    pub fn color(&self) -> [u8; 3] {
        match self {
            AgentStatus::Idle => [100, 149, 237],    // CornflowerBlue
            AgentStatus::Working => [50, 205, 50],   // LimeGreen
            AgentStatus::Blocked => [255, 165, 0],   // Orange
            AgentStatus::Done => [70, 130, 180],     // SteelBlue
            AgentStatus::Unknown => [128, 128, 128], // Gray
        }
    }

    pub fn label(&self) -> &str {
        match self {
            AgentStatus::Idle => "IDLE",
            AgentStatus::Working => "WORKING",
            AgentStatus::Blocked => "BLOCKED",
            AgentStatus::Done => "DONE",
            AgentStatus::Unknown => "UNKNOWN",
        }
    }
}

/// Worker state translated from crabjar-conductor API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrabjarAgent {
    pub id: String,
    pub name: String,
    pub status: AgentStatus,
    pub capabilities: Vec<String>,
    pub current_task: Option<String>,
    pub trust_level: u8,
    pub last_heartbeat: Option<u64>,
}

impl CrabjarAgent {
    /// Translate conductor worker JSON into crabjar agent state.
    pub fn from_conductor_response(worker: &serde_json::Value) -> Self {
        let id = worker["id"].as_str().unwrap_or("unknown").to_string();
        let name = worker["name"].as_str().unwrap_or("unnamed").to_string();
        let status_str = worker["status"].as_str().unwrap_or("unknown");
        let capabilities: Vec<String> = worker["capabilities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        CrabjarAgent {
            id,
            name,
            status: AgentStatus::from(status_str),
            capabilities,
            current_task: None,
            trust_level: 0,
            last_heartbeat: None,
        }
    }
}

/// Goal state from crabjar-conductor API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrabjarGoal {
    pub id: String,
    pub description: String,
    pub status: String,
    pub created_at: String,
}

impl CrabjarGoal {
    pub fn from_conductor_response(value: &serde_json::Value) -> Self {
        CrabjarGoal {
            id: value["goal_id"].as_str().unwrap_or("unknown").to_string(),
            description: value["description"].as_str().unwrap_or("").to_string(),
            status: value["status"].as_str().unwrap_or("unknown").to_string(),
            created_at: value["created_at"].as_str().unwrap_or("").to_string(),
        }
    }
}

/// The CrabjarSurfaceAdapter translates conductor API state into herdr protocol
/// surface patches for GPUI rendering.
pub struct CrabjarSurfaceAdapter {
    boot_id: String,
    projection_revision: u64,
    surface_revision: u64,
    agents: HashMap<String, CrabjarAgent>,
}

impl CrabjarSurfaceAdapter {
    pub fn new(boot_id: &str) -> Self {
        CrabjarSurfaceAdapter {
            boot_id: boot_id.to_string(),
            projection_revision: 0,
            surface_revision: 0,
            agents: HashMap::new(),
        }
    }

    /// Update agent state from conductor API response.
    pub fn update_agents(&mut self, workers: &[serde_json::Value]) {
        for worker in workers {
            let agent = CrabjarAgent::from_conductor_response(worker);
            self.agents.insert(agent.id.clone(), agent);
        }
        self.projection_revision += 1;
        self.surface_revision += 1;
    }

    /// Get all agents for rendering.
    pub fn agents(&self) -> &HashMap<String, CrabjarAgent> {
        &self.agents
    }

    /// Generate a surface patch describing the current agent state.
    /// This would be serialized and sent to the GPUI renderer.
    pub fn generate_patch(&mut self) -> serde_json::Value {
        let agents_vec: Vec<serde_json::Value> = self
            .agents
            .values()
            .map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "name": a.name,
                    "status": match a.status {
                        AgentStatus::Idle => "idle",
                        AgentStatus::Working => "working",
                        AgentStatus::Blocked => "blocked",
                        AgentStatus::Done => "done",
                        AgentStatus::Unknown => "unknown",
                    },
                    "capabilities": a.capabilities,
                    "trust_level": a.trust_level,
                })
            })
            .collect();

        serde_json::json!({
            "boot_id": self.boot_id,
            "projection_revision": self.projection_revision,
            "surface_revision": self.surface_revision,
            "agents": agents_vec,
        })
    }
}
