//! Core orchestration logic — goal decomposition and task lifecycle management.

use crate::audit::{AuditContext, AuditEventType, ConductorAuditLogger};
use crate::decision::{BackendConfig, CacheStats, DecisionGate, DecisionPolicy};
use crate::fleet_status::FleetStatusTracker;
use crate::promotion::PromotionManager;
use crate::sandbox::SandboxManager;
use crate::state_store::{Goal, GoalStatus, StateStore, Task, Worker};
use anyhow::Result;
use rusqlite::Connection;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub mod decisions;
pub mod workers;

#[cfg(test)]
mod tests;

/// Configuration for the conductor service.
pub struct ConductorConfig {
    pub listen_addr: String,
    pub listen_port: u16,
    pub pesti_server_url: String,
    /// Optional path to mirror-log database for attention-based decision filtering.
    pub mirror_log_db_path: Option<String>,
}

impl Default for ConductorConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port: 8091,
            pesti_server_url: "http://localhost:8000".to_string(),
            mirror_log_db_path: None,
        }
    }
}

/// The conductor — receives goals, decomposes them into tasks, manages lifecycle.
pub struct Conductor {
    pub(crate) store: StateStore,
    pub(crate) config: ConductorConfig,
    pub(crate) decision_gate: Option<DecisionGate>,
    pub(crate) hybrid_layer: Option<crate::attention::HybridDecisionLayer>,
    promotion_manager: PromotionManager,
    fleet_status: FleetStatusTracker,
    sandbox_manager: SandboxManager,
    audit_logger: Option<Arc<Mutex<ConductorAuditLogger>>>,
}

impl Conductor {
    /// Create a new conductor with an in-memory database (for testing).
    pub fn new(config: ConductorConfig) -> Result<Self> {
        let store = StateStore::open(":memory:")?;
        let db_conn = Connection::open(":memory:")?;
        let promotion_manager = PromotionManager::new(db_conn)?;
        Ok(Self {
            store,
            config,
            decision_gate: None,
            hybrid_layer: None,
            promotion_manager,
            fleet_status: FleetStatusTracker::default(),
            sandbox_manager: SandboxManager::default(),
            audit_logger: None,
        })
    }

    /// Create a conductor with a specific database path.
    pub fn with_db(config: ConductorConfig, db_path: &str) -> Result<Self> {
        let store = StateStore::open(db_path)?;
        let db_conn = Connection::open(db_path)?;
        let promotion_manager = PromotionManager::new(db_conn)?;

        // Initialize audit logger with the same database
        let audit_logger = match ConductorAuditLogger::new(db_path) {
            Ok(logger) => Some(Arc::new(Mutex::new(logger))),
            Err(e) => {
                warn!(error = %e, "Failed to initialize audit logger");
                None
            }
        };

        Ok(Self {
            store,
            config,
            decision_gate: None,
            hybrid_layer: None,
            promotion_manager,
            fleet_status: FleetStatusTracker::default(),
            sandbox_manager: SandboxManager::default(),
            audit_logger,
        })
    }

    /// Get a summary of the current fleet state for JEV evaluation context.
    fn get_fleet_state_summary(&self) -> String {
        let workers = self.list_workers();
        let online_workers: Vec<&Worker> = workers
            .iter()
            .filter(|w| w.status == crate::state_store::WorkerStatus::Online)
            .collect();

        let ready_tasks = match self.store.get_ready_tasks() {
            Ok(tasks) => tasks.len(),
            Err(_) => 0,
        };

        format!(
            "{} workers online ({} total), {} ready tasks in queue",
            online_workers.len(),
            workers.len(),
            ready_tasks
        )
    }

    /// Submit a new goal for decomposition and execution.
    pub fn submit_goal(&self, description: String) -> Result<Goal> {
        info!(description = %description, "Submitting new goal");

        // Pre-execution readiness check via JEV decision gate
        if let Some(gate) = &self.decision_gate {
            let fleet_state = self.get_fleet_state_summary();
            match tokio::runtime::Handle::current().block_on(gate.evaluate_pre_execution(
                &description,
                &fleet_state,
                None,
                Some(&self.store),
            )) {
                Ok(result) => {
                    info!(
                        goal_description = %description,
                        decision = %result.result.option,
                        confidence = result.result.confidence,
                        "JEV pre-execution readiness check"
                    );

                    if result.result.option == "abort" {
                        return Err(anyhow::anyhow!(
                            "JEV decision gate: fleet not ready for goal '{}'. Rationale: {}",
                            description,
                            result.result.rationale
                        ));
                    } else if result.result.option == "hold" {
                        warn!(
                            goal_description = %description,
                            "JEV decision gate recommends holding goal due to fleet state"
                        );
                    }
                }
                Err(e) => {
                    warn!("JEV pre-execution check failed: {}", e);
                    // Continue without JEV if evaluation fails
                }
            }
        }

        // Create the goal record
        let goal = self.store.create_goal(description.clone())?;
        info!(goal_id = %goal.id, "Goal created");

        // Audit: log goal submission
        if let Some(logger) = &self.audit_logger {
            let logger = logger.clone();
            let goal_id = goal.id.clone();
            tokio::spawn(async move {
                let logger = logger.lock().await;
                let ctx = AuditContext {
                    task_id: Some(goal_id),
                    worker_id: None,
                    outcome: "goal_submitted".to_string(),
                    duration_ms: None,
                    metadata: json!({}),
                };
                let _ = logger.log_event(AuditEventType::GoalDecomposed, ctx).await;
            });
        }

        // Decompose into tasks using pesti-server (LLM inference)
        let tasks = self.decompose(goal.id.clone(), &description)?;
        info!(goal_id = %goal.id, task_count = tasks.len(), "Goal decomposed into tasks");

        // Mark as running once decomposition is complete
        self.store
            .update_goal_status(&goal.id, GoalStatus::Running)?;

        Ok(goal)
    }

    /// Decompose a goal into executable tasks using LLM inference.
    fn decompose(&self, goal_id: String, description: &str) -> Result<Vec<Task>> {
        info!(goal_id = %goal_id, pesti_url = %self.config.pesti_server_url, "Decomposing goal via pesti-server");

        // Call pesti-server to break the goal into steps via LLM inference.
        let prompt = format!(
            "You are a task planner that breaks goals into executable shell commands.\n\
             \nGoal: {description}\n\n\
             Return ONLY a JSON array (no markdown, no explanation) with this exact format:\n\
             [{{\"command\": \"shell command here\", \"description\": \"what it does\"}}]\n\
             \nEach object must have exactly two fields: 'command' and 'description'.\n\
             Do not include depends_on. Just the command and description.\n"
        );

        // Configure HTTP client with explicit timeout and DNS resolution
        let url = format!("{}/v1/chat/completions", self.config.pesti_server_url);
        info!(goal_id = %goal_id, url = %url, "Connecting to pesti-server");

        // Debug: resolve the hostname ourselves to see what IP we get
        let parsed = reqwest::Url::parse(&url).map_err(|e| anyhow::anyhow!("invalid URL: {e}"))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("no host in URL"))?;
        info!(goal_id = %goal_id, host = %host, "Resolved hostname");

        // Try resolving manually to see what we get
        match std::net::ToSocketAddrs::to_socket_addrs(&format!("{host}:8081")) {
            Ok(addrs) => {
                for addr in addrs {
                    info!(goal_id = %goal_id, addr = ?addr, "DNS resolution result");
                }
            }
            Err(e) => {
                error!(goal_id = %goal_id, err = %e, host = %host, "Manual DNS resolution failed");
            }
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_mins(2))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build HTTP client: {e}"))?;

        let body = serde_json::json!({
            "model": "orchestrator_merged",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a task planner that outputs ONLY JSON arrays. No prose, no markdown formatting."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.1,
            "max_tokens": 2048
        });

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| anyhow::anyhow!("pesti-server request failed: {e}"))?;

        let status = resp.status();
        info!(goal_id = %goal_id, status = status.as_u16(), "Received response from pesti-server");

        let text = resp
            .text()
            .map_err(|e| anyhow::anyhow!("failed to read response: {e}"))?;

        if !status.is_success() {
            return Err(anyhow::anyhow!("pesti-server error {status}: {text}"));
        }

        // Parse the LLM response
        let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            anyhow::anyhow!(
                "failed to parse pesti response as JSON: {} - {}",
                e,
                &text[..std::cmp::min(200, text.len())]
            )
        })?;

        // Extract content from chat completion format
        let content = parsed["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("no content in response"))?
            .trim();

        // Extract JSON array from LLM response. The LLM may wrap it in prose,
        // markdown code fences, or [RESPONSE] tags. Find the actual JSON array.
        let json_to_parse: String = if let Some(fence_start) = content.find("```") {
            // Has code fences - extract content between them
            let after_open = &content[fence_start + 3..];
            // Skip optional language tag on first line (e.g., "json\n")
            let actual_content = if let Some(newline) = after_open.find('\n') {
                &after_open[newline + 1..]
            } else {
                after_open
            };
            if let Some(fence_end) = actual_content.rfind("```") {
                actual_content[..fence_end].trim().to_string()
            } else {
                actual_content.trim().to_string()
            }
        } else {
            // No code fences - find the JSON array directly
            // Look for first '[' that starts an object array (skip "[RESPONSE]")
            let mut found = None;
            let chars: Vec<char> = content.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if chars[i] == '[' {
                    // Check if this looks like a JSON array (next non-whitespace is '{' or '[')
                    let rest: String = chars[i..].iter().collect();
                    let trimmed = rest.trim_start();
                    if trimmed.starts_with('[') || trimmed.starts_with('{') {
                        found = Some(i);
                        break;
                    }
                }
                i += 1;
            }
            match found {
                Some(pos) => content[pos..].trim().to_string(),
                None => content.to_string(),
            }
        };

        // Try to parse the content as JSON array of tasks
        let task_specs: Vec<serde_json::Value> =
            serde_json::from_str(&json_to_parse).map_err(|e| {
                anyhow::anyhow!(
                    "failed to parse task specs from LLM: {} - {}",
                    e,
                    &json_to_parse[..std::cmp::min(200, json_to_parse.len())]
                )
            })?;

        // Create tasks in the store
        let mut tasks = Vec::new();
        for spec in task_specs {
            let command = spec["command"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("task spec missing 'command'"))?
                .to_string();
            let description = spec["description"].as_str().unwrap_or(&command).to_string();

            debug!(goal_id = %goal_id, command = %command, "Creating task from LLM spec");
            let task = self
                .store
                .create_task(&goal_id, format!("[{description}] {command}"))?;
            tasks.push(task);
        }

        info!(goal_id = %goal_id, task_count = tasks.len(), "Created tasks from decomposition");

        Ok(tasks)
    }

    /// Get status of a goal and its tasks.
    pub fn goal_status(&self, goal_id: &str) -> Result<Option<GoalStatus>> {
        let goal = self.store.get_goal(goal_id)?;
        Ok(goal.map(|g| g.status))
    }

    /// Get full goal details.
    pub fn get_goal(&self, goal_id: &str) -> Result<Option<Goal>> {
        self.store
            .get_goal(goal_id)
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// Get the state store (for testing and API layer).
    pub fn store(&self) -> &StateStore {
        &self.store
    }

    /// Get the promotion manager (for evidence-based worker promotions).
    pub fn promotion_manager(&self) -> &PromotionManager {
        &self.promotion_manager
    }

    /// Get the audit logger for querying audit trails.
    pub fn audit_logger(&self) -> Option<&Arc<Mutex<ConductorAuditLogger>>> {
        self.audit_logger.as_ref()
    }

    /// Get the sandbox manager (for ephemeral worker environment provisioning).
    pub fn sandbox_manager(&self) -> &SandboxManager {
        &self.sandbox_manager
    }

    /// Get the fleet status tracker.
    pub fn fleet_status(&self) -> &FleetStatusTracker {
        &self.fleet_status
    }
}
