//! Fleet status tracking and inference server health monitoring.
//!
//! Tracks the operational state of fleet machines and their loaded models,
//! enabling effective task handoffs and failover decisions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Health status of an inference server endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerHealth {
    Healthy { response_time_ms: u64, checked_at: u64 },
    Degraded { response_time_ms: u64, checked_at: u64 },
    Unhealthy { last_error: String, checked_at: u64 },
}

/// Status of a model loaded on a machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub name: String,
    pub endpoint: String,
    pub health: ServerHealth,
    pub last_task_completed: Option<u64>,
}

/// Operational status of a fleet machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineStatus {
    pub name: String,
    pub models: Vec<ModelStatus>,
    pub overall_health: MachineHealth,
    pub last_heartbeat: Option<u64>,
}

/// Aggregate health of a machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MachineHealth {
    Operational,
    PartiallyDegraded,
    Down,
}

/// Fleet status tracker — monitors machine and model health across the fleet.
pub struct FleetStatusTracker {
    machines: RwLock<HashMap<String, MachineStatus>>,
}

impl Default for FleetStatusTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl FleetStatusTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            machines: RwLock::new(HashMap::new()),
        }
    }

    /// Register a machine with its inference endpoints.
    pub async fn register_machine(&self, name: String, endpoints: Vec<String>) {
        let models = endpoints
            .into_iter()
            .map(|endpoint| ModelStatus {
                name: endpoint.clone(),
                endpoint,
                health: ServerHealth::Unhealthy {
                    last_error: "not yet checked".to_string(),
                    checked_at: now_epoch(),
                },
                last_task_completed: None,
            })
            .collect();

        let status = MachineStatus {
            name,
            models,
            overall_health: MachineHealth::Down,
            last_heartbeat: None,
        };

        self.machines.write().await.insert(status.name.clone(), status);
    }

    /// Update heartbeat for a machine.
    pub async fn heartbeat(&self, name: &str) {
        let mut machines = self.machines.write().await;
        if let Some(machine) = machines.get_mut(name) {
            machine.last_heartbeat = Some(now_epoch());
        }
    }

    /// Check health of all registered endpoints and update status.
    pub async fn check_all(&self) {
        let names: Vec<String> = {
            let machines = self.machines.read().await;
            machines.keys().cloned().collect()
        };

        for name in names {
            let machine = {
                let machines = self.machines.read().await;
                machines.get(&name).cloned()
            };

            if let Some(machine) = machine {
                let mut models = Vec::new();
                let mut all_healthy = true;
                let mut any_healthy = false;

                for model in &machine.models {
                    let health = self.probe_endpoint(&model.endpoint).await;
                    match &health {
                        ServerHealth::Healthy { .. } => any_healthy = true,
                        ServerHealth::Degraded { .. } | ServerHealth::Unhealthy { .. } => {
                            all_healthy = false;
                        }
                    }
                    models.push(ModelStatus {
                        name: model.name.clone(),
                        endpoint: model.endpoint.clone(),
                        health,
                        last_task_completed: model.last_task_completed,
                    });
                }

                let overall = if all_healthy {
                    MachineHealth::Operational
                } else if any_healthy {
                    MachineHealth::PartiallyDegraded
                } else {
                    MachineHealth::Down
                };

                let mut machines = self.machines.write().await;
                if let Some(m) = machines.get_mut(&name) {
                    m.models = models;
                    m.overall_health = overall;
                }
            }
        }
    }

    /// Get current fleet status snapshot.
    pub async fn get_status(&self) -> Vec<MachineStatus> {
        let machines = self.machines.read().await;
        machines.values().cloned().collect()
    }

    /// Find the best machine for a task requiring specific model capabilities.
    pub async fn find_best_machine(&self, required_model: &str) -> Option<String> {
        let machines = self.machines.read().await;
        let entries: Vec<(String, MachineStatus)> = machines.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // Prefer operational machines with the required model
        for (name, machine) in &entries {
            if matches!(machine.overall_health, MachineHealth::Operational)
                && machine.models.iter().any(|m| {
                    m.name.contains(required_model)
                        && matches!(m.health, ServerHealth::Healthy { .. })
                }) {
                    return Some(name.clone());
                }
        }

        // Fall back to partially degraded machines
        for (name, machine) in &entries {
            if matches!(machine.overall_health, MachineHealth::PartiallyDegraded)
                && machine.models.iter().any(|m| {
                    m.name.contains(required_model)
                        && matches!(m.health, ServerHealth::Healthy { .. })
                }) {
                    return Some(name.clone());
                }
        }

        None
    }

    /// Probe a single endpoint and return its health status.
    async fn probe_endpoint(&self, url: &str) -> ServerHealth {
        let start = SystemTime::now();

        match tokio::time::timeout(
            Duration::from_secs(5),
            reqwest::get(format!("{url}/v1/models")),
        )
        .await
        {
            Ok(Ok(response)) => {
                let elapsed = start.elapsed().unwrap_or_default().as_millis() as u64;
                if response.status().is_success() {
                    if elapsed < 1000 {
                        ServerHealth::Healthy {
                            response_time_ms: elapsed,
                            checked_at: now_epoch(),
                        }
                    } else {
                        ServerHealth::Degraded {
                            response_time_ms: elapsed,
                            checked_at: now_epoch(),
                        }
                    }
                } else {
                    ServerHealth::Unhealthy {
                        last_error: format!("HTTP {}", response.status()),
                        checked_at: now_epoch(),
                    }
                }
            }
            Ok(Err(e)) => ServerHealth::Unhealthy {
                last_error: e.to_string(),
                checked_at: now_epoch(),
            },
            Err(_) => ServerHealth::Unhealthy {
                last_error: "timeout".to_string(),
                checked_at: now_epoch(),
            },
        }
    }
}