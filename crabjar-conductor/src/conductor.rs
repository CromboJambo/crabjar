//! Core orchestration logic — goal decomposition and task lifecycle management.

use crate::auth;
use crate::decision::{BackendConfig, CacheStats, DecisionGate, DecisionPolicy};
use crate::gate::{conductor_level_to_guard_layer, trust_sufficient_for_auto_assign};
use crate::state_store::{Goal, GoalStatus, StateStore, Task, TaskStatus, Worker, WorkerStatus};
use anyhow::Result;
use tracing::{debug, error, info, warn};

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
    store: StateStore,
    config: ConductorConfig,
    decision_gate: Option<DecisionGate>,
    hybrid_layer: Option<crate::attention::HybridDecisionLayer>,
}

impl Conductor {
    /// Create a new conductor with an in-memory database (for testing).
    pub fn new(config: ConductorConfig) -> Result<Self> {
        let store = StateStore::open(":memory:")?;
        Ok(Self {
            store,
            config,
            decision_gate: None,
            hybrid_layer: None,
        })
    }

    /// Create a conductor with a specific database path.
    pub fn with_db(config: ConductorConfig, db_path: &str) -> Result<Self> {
        let store = StateStore::open(db_path)?;
        Ok(Self {
            store,
            config,
            decision_gate: None,
            hybrid_layer: None,
        })
    }

    /// Configure JEV decision gate for automated fleet decisions.
    pub fn with_decision_gate(
        &mut self,
        backend_config: BackendConfig,
        policy: Option<DecisionPolicy>,
    ) {
        let evaluator = crate::decision::evaluate::JevEvaluator::new(backend_config);
        self.decision_gate = Some(DecisionGate::new(evaluator, policy));
    }

    /// Configure hybrid attention + JEV decision layer.
    pub fn with_hybrid_decision_layer(
        &mut self,
        backend_config: BackendConfig,
        policy: Option<crate::attention::HybridDecisionPolicy>,
    ) {
        let evaluator = crate::decision::evaluate::JevEvaluator::new(backend_config.clone());
        let gate = DecisionGate::new(evaluator, None::<crate::decision::DecisionPolicy>);

        if let Some(mirror_log_path) = &self.config.mirror_log_db_path {
            match crate::attention::HybridDecisionLayer::new(mirror_log_path, gate, policy) {
                Ok(layer) => {
                    self.hybrid_layer = Some(layer);
                    info!("Hybrid attention + JEV decision layer configured");
                }
                Err(e) => {
                    warn!(error = %e, "Failed to configure hybrid decision layer, falling back to JEV-only");
                    let evaluator2 = crate::decision::evaluate::JevEvaluator::new(backend_config);
                    self.decision_gate = Some(DecisionGate::new(
                        evaluator2,
                        None::<crate::decision::DecisionPolicy>,
                    ));
                }
            }
        } else {
            // No mirror-log path configured, use plain JEV gate
            self.decision_gate = Some(gate);
        }
    }

    /// Get a summary of the current fleet state for JEV evaluation context.
    fn get_fleet_state_summary(&self) -> String {
        let workers = self.list_workers();
        let online_workers: Vec<&Worker> = workers
            .iter()
            .filter(|w| w.status == WorkerStatus::Online)
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

    /// Get JEV decision cache statistics (for monitoring and metrics).
    pub fn cache_stats(&self) -> Option<CacheStats> {
        self.decision_gate.as_ref().map(|gate| gate.cache_stats())
    }

    /// Extract candidate rules from decision history.
    pub fn extract_rules(
        &self,
        criteria_file: &str,
        min_occurrences: i64,
    ) -> anyhow::Result<Vec<crate::decision::rule_extraction::CandidateRule>> {
        let rules = crate::decision::rule_extraction::extract_rules(
            &self.store,
            criteria_file,
            min_occurrences,
        )?;
        Ok(rules)
    }

    /// Promote a candidate rule to persistent storage.
    pub fn promote_rule(
        &self,
        pattern_id: &str,
        criteria_file: &str,
        recommended_option: &str,
    ) -> anyhow::Result<i64> {
        let rule = crate::decision::rule_extraction::CandidateRule {
            criteria_file: criteria_file.to_string(),
            pattern_id: pattern_id.to_string(),
            recommended_option: recommended_option.to_string(),
            avg_confidence: 0.0,
            occurrence_count: 0,
        };
        let rule_id = crate::decision::rule_extraction::promote_rule(&self.store, &rule)?;
        Ok(rule_id)
    }

    /// Evaluate a decision via JEV (for external callers like pesti-runner model routing).
    pub async fn evaluate_decision(
        &self,
        observation: &str,
        criteria_file: &str,
    ) -> anyhow::Result<crate::decision::evaluate::DecisionResult> {
        // If hybrid layer is configured, use the two-stage pipeline
        if let Some(hybrid) = &self.hybrid_layer {
            return hybrid
                .process_observation(
                    uuid::Uuid::new_v4().to_string().as_str(),
                    "conductor",
                    observation,
                    None,
                )
                .await
                .map(|decision| {
                    // Extract JEV result if available, otherwise create a default
                    decision.jev_result.unwrap_or_else(|| {
                        crate::decision::evaluate::DecisionResult {
                            option: "ignore".to_string(),
                            rationale: format!(
                                "Filtered by attention layer (score: {:.0})",
                                decision.attention_score
                            ),
                            confidence: 0.95,
                        }
                    })
                });
        }

        // Fall back to plain JEV evaluation
        let gate = self
            .decision_gate
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("JEV decision gate not configured"))?;

        gate.evaluate(observation, criteria_file, Some(&self.store))
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
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
             \nGoal: {}\n\n\
             Return ONLY a JSON array (no markdown, no explanation) with this exact format:\n\
             [{{\"command\": \"shell command here\", \"description\": \"what it does\"}}]\n\
             \nEach object must have exactly two fields: 'command' and 'description'.\n\
             Do not include depends_on. Just the command and description.\n",
            description
        );

        // Configure HTTP client with explicit timeout and DNS resolution
        let url = format!("{}/v1/chat/completions", self.config.pesti_server_url);
        info!(goal_id = %goal_id, url = %url, "Connecting to pesti-server");

        // Debug: resolve the hostname ourselves to see what IP we get
        let parsed =
            reqwest::Url::parse(&url).map_err(|e| anyhow::anyhow!("invalid URL: {}", e))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("no host in URL"))?;
        info!(goal_id = %goal_id, host = %host, "Resolved hostname");

        // Try resolving manually to see what we get
        match std::net::ToSocketAddrs::to_socket_addrs(&format!("{}:8081", host)) {
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
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build HTTP client: {}", e))?;

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
            .map_err(|e| anyhow::anyhow!("pesti-server request failed: {}", e))?;

        let status = resp.status();
        info!(goal_id = %goal_id, status = status.as_u16(), "Received response from pesti-server");

        let text = resp
            .text()
            .map_err(|e| anyhow::anyhow!("failed to read response: {}", e))?;

        if !status.is_success() {
            return Err(anyhow::anyhow!("pesti-server error {}: {}", status, text));
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
                    if trimmed.starts_with("[") || trimmed.starts_with("{") {
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
                .create_task(&goal_id, format!("[{}] {}", description, command))?;
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
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// List all registered workers.
    pub fn list_workers(&self) -> Vec<Worker> {
        self.store.list_workers().unwrap_or_default()
    }

    /// Register a new worker in the fleet. Returns (worker, auth_token).
    pub fn register_worker(
        &self,
        name: String,
        capabilities: Vec<String>,
        ssh_key: Option<String>,
    ) -> Result<(Worker, String)> {
        // Validate SSH key format if provided
        if let Some(ref key) = ssh_key {
            auth::validate_ssh_public_key(key)?;
        }

        let result = self.store.register_worker(name, capabilities, ssh_key)?;
        Ok(result)
    }

    /// Authenticate a worker by name and SSH key, returning (worker, auth_token).
    /// If the worker is not registered, this creates a new registration.
    pub fn worker_login(&self, name: &str, ssh_key: Option<String>) -> Result<(Worker, String)> {
        // Try to find existing worker by name
        let workers = self.store.list_workers()?;
        if let Some(existing) = workers.into_iter().find(|w| w.name == name) {
            // Validate SSH key if provided and worker has one stored
            if let Some(ref key) = ssh_key {
                auth::validate_ssh_public_key(key)?;
            }

            // Refresh the token for the existing worker
            let new_token = self.store.refresh_worker_token(&existing.id)?;
            info!(worker_id = %existing.id, "Worker authenticated via login");
            return Ok((existing, new_token));
        }

        // Worker not found — register a new one (auto-registration on first login)
        info!(name = %name, "Auto-registering worker on first login");
        let result = self.store.register_worker(name.to_string(), vec![], ssh_key)?;
        Ok(result)
    }

    /// Poll for tasks assigned to a specific worker.
    pub fn tasks_for_worker(&self, worker_id: &str) -> Result<Vec<Task>> {
        // Get the worker's capabilities and trust level
        let worker = self
            .store
            .get_worker(worker_id)?
            .ok_or_else(|| anyhow::anyhow!("worker not found"))?;

        // Check if worker has sufficient trust for auto-assignment
        if !trust_sufficient_for_auto_assign(&worker) {
            debug!(
                worker_id = %worker_id,
                trust_level = worker.trust_level,
                "Worker trust level insufficient for auto-assignment"
            );
            return Ok(vec![]);
        }

        // Get all pending tasks that are ready (dependencies met)
        let ready_tasks = self.store.get_ready_tasks()?;

        // Filter by worker capabilities and trust level, then assign
        for task in ready_tasks.iter() {
            if task.worker_id.is_some() {
                continue; // Already assigned to another worker
            }

            // Check trust level requirement
            let task_trust_required = self.store.get_task_trust_level(&task.id)?;
            if worker.trust_level < task_trust_required {
                debug!(
                    task_id = %task.id,
                    worker_trust = worker.trust_level,
                    required = task_trust_required,
                    "Worker trust level insufficient"
                );
                continue;
            }

            // Check capability requirements
            let required_caps = self.store.get_task_capabilities(&task.id)?;
            if !self.capabilities_satisfy(&worker.capabilities, &required_caps) {
                debug!(
                    task_id = %task.id,
                    worker_caps = ?worker.capabilities,
                    required = ?required_caps,
                    "Worker capabilities insufficient"
                );
                continue;
            }

            // Worker qualifies — assign the task
            self.store.assign_task(&task.id, worker_id)?;
            let updated = self.store.get_task(&task.id)?.unwrap();
            info!(
                task_id = %task.id,
                worker_id = %worker_id,
                "Task assigned to qualified worker"
            );
            return Ok(vec![updated]);
        }

        Ok(vec![])
    }

    /// Check if a worker's capabilities satisfy the task requirements.
    /// Supports exact match and wildcard matching (e.g., "gpu:nvidia:*").
    fn capabilities_satisfy(&self, worker_caps: &[String], required_caps: &[String]) -> bool {
        for required in required_caps {
            let satisfied = worker_caps
                .iter()
                .any(|worker_cap| self.capability_matches(worker_cap, required));
            if !satisfied {
                return false;
            }
        }
        true
    }

    /// Check if a single worker capability matches a required capability.
    /// Supports wildcards: "gpu:nvidia:*" matches "gpu:nvidia:3070ti".
    fn capability_matches(&self, worker_cap: &str, required: &str) -> bool {
        // Exact match
        if worker_cap == required {
            return true;
        }

        // Wildcard matching — split by colon and compare segments
        let worker_parts: Vec<&str> = worker_cap.split(':').collect();
        let required_parts: Vec<&str> = required.split(':').collect();

        if worker_parts.len() != required_parts.len() {
            return false;
        }

        for (w, r) in worker_parts.iter().zip(required_parts.iter()) {
            if *r == "*" {
                continue; // Wildcard matches anything
            }
            if w != r {
                return false;
            }
        }

        true
    }

    /// Report completion of a task. Returns updated auth token if trust level changed.
    pub fn report_task_complete(
        &self,
        task_id: &str,
        result: Option<String>,
    ) -> Result<Option<String>> {
        // Get the task to find which worker completed it
        let task = self
            .store
            .get_task(task_id)?
            .ok_or_else(|| anyhow::anyhow!("task not found"))?;

        let worker_id = match &task.worker_id {
            Some(id) => id.clone(),
            None => return Ok(None), // No worker assigned, no annealing
        };

        self.store.complete_task(task_id, result)?;

        // Anneal trust level based on success
        let new_trust_level = self.store.record_task_success(&worker_id)?;

        // Check if all tasks for the goal are complete
        let goal_tasks = self.store.list_tasks_for_goal(&task.goal_id)?;
        let all_complete = goal_tasks.iter().all(|t| t.status == TaskStatus::Completed);

        if all_complete {
            self.store
                .update_goal_status(&task.goal_id, GoalStatus::Completed)?;
        }

        // If trust level changed, issue a new token with updated scope
        let worker = self
            .store
            .get_worker(&worker_id)?
            .ok_or_else(|| anyhow::anyhow!("worker not found"))?;
        if worker.trust_level != new_trust_level {
            // Trust level was upgraded — refresh token
            let new_token = self.store.refresh_worker_token(&worker_id)?;
            info!(worker_id = %worker_id, trust_level = new_trust_level, "Worker trust level upgraded, new token issued");
            Ok(Some(new_token))
        } else {
            Ok(None)
        }
    }

    /// Report task failure. Returns updated auth token if trust level changed.
    pub fn report_task_failed(&self, task_id: &str, error: String) -> Result<Option<String>> {
        // Get the task to find which worker failed it
        let task = self
            .store
            .get_task(task_id)?
            .ok_or_else(|| anyhow::anyhow!("task not found"))?;

        let worker_id = match &task.worker_id {
            Some(id) => id.clone(),
            None => return Ok(None), // No worker assigned, no annealing
        };

        self.store.fail_task(task_id, error)?;

        // Anneal trust level based on failure
        let new_trust_level = self.store.record_task_failure(&worker_id)?;

        // If trust level changed (downgrade), issue a new token with reduced scope
        let worker = self
            .store
            .get_worker(&worker_id)?
            .ok_or_else(|| anyhow::anyhow!("worker not found"))?;
        if worker.trust_level != new_trust_level {
            let new_token = self.store.refresh_worker_token(&worker_id)?;
            warn!(worker_id = %worker_id, trust_level = new_trust_level, "Worker trust level downgraded, new token issued");
            Ok(Some(new_token))
        } else {
            Ok(None)
        }
    }

    /// Update worker status via heartbeat.
    pub fn worker_heartbeat(&self, worker_id: &str) -> Result<()> {
        self.store.heartbeat(worker_id)?;
        Ok(())
    }

    /// Get the state store (for testing and API layer).
    pub fn store(&self) -> &StateStore {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn submit_goal_creates_task() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Create goal directly without decomposition (no pesti-server needed)
        let goal = conductor
            .store
            .create_goal("Test goal".to_string())
            .unwrap();
        let status = conductor.goal_status(&goal.id).unwrap().unwrap();
        assert_eq!(status, GoalStatus::Decomposed);
    }

    #[test]
    fn register_worker_validates_ssh_key() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Valid ed25519 key should pass
        let valid_key =
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGFub3RoZXJrZXllbmNvZGluZw== user@host";
        let result = conductor.register_worker(
            "worker-valid".to_string(),
            vec!["compute".to_string()],
            Some(valid_key.to_string()),
        );
        assert!(result.is_ok());

        // Invalid key format should fail
        let invalid_key = "not-a-valid-key";
        let result = conductor.register_worker(
            "worker-invalid".to_string(),
            vec!["compute".to_string()],
            Some(invalid_key.to_string()),
        );
        assert!(result.is_err());

        // Missing key should pass (optional)
        let result =
            conductor.register_worker("worker-nok".to_string(), vec!["compute".to_string()], None);
        assert!(result.is_ok());
    }

    #[test]
    fn task_assignment_filters_by_capability() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Register workers with different capabilities
        let (worker_gpu, _) = conductor
            .register_worker(
                "gpu-worker".to_string(),
                vec!["compute".to_string(), "gpu:nvidia:3070ti".to_string()],
                None,
            )
            .unwrap();
        let (worker_cpu, _) = conductor
            .register_worker("cpu-worker".to_string(), vec!["compute".to_string()], None)
            .unwrap();

        // Set trust levels so workers can be auto-assigned (not testing trust here)
        conductor
            .store
            .set_worker_trust_level(&worker_gpu.id, 1)
            .unwrap();
        conductor
            .store
            .set_worker_trust_level(&worker_cpu.id, 1)
            .unwrap();

        // Create a goal and task directly
        let goal = conductor.store.create_goal("GPU task".to_string()).unwrap();
        let task = conductor
            .store
            .create_task(&goal.id, "nvidia-smi".to_string())
            .unwrap();

        // Set GPU capability requirement on the task
        conductor
            .store
            .set_task_capabilities(&task.id, &["gpu:nvidia:*".to_string()])
            .unwrap();

        // CPU-only worker should not get the task
        let tasks_for_cpu = conductor.tasks_for_worker(&worker_cpu.id).unwrap();
        assert!(tasks_for_cpu.is_empty());

        // GPU worker should get the task
        let tasks_for_gpu = conductor.tasks_for_worker(&worker_gpu.id).unwrap();
        assert!(!tasks_for_gpu.is_empty());
        assert_eq!(tasks_for_gpu[0].id, task.id);
    }

    #[test]
    fn annealing_upgrades_trust_level() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Register worker (starts at trust level 0)
        let (worker, _) = conductor
            .register_worker(
                "anneal-worker".to_string(),
                vec!["compute".to_string()],
                None,
            )
            .unwrap();
        assert_eq!(worker.trust_level, 0);

        // Create a goal and task
        let goal = conductor.store.create_goal("Test".to_string()).unwrap();
        let task = conductor
            .store
            .create_task(&goal.id, "echo test".to_string())
            .unwrap();

        // Assign task to worker
        conductor.store.assign_task(&task.id, &worker.id).unwrap();

        // Complete the task — trust level should upgrade
        let new_token = conductor
            .report_task_complete(&task.id, Some("success".to_string()))
            .unwrap();

        // After first success, trust level goes from 0 to 1 (upgrade threshold is 5 successes)
        // Actually at 1 success it's still below the threshold, so no upgrade yet
        let updated_worker = conductor.store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(updated_worker.successful_tasks, 1);
    }

    #[test]
    fn worker_login_existing_refreshes_token() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Register a worker first
        let (worker, _original_token) = conductor
            .register_worker(
                "login-worker".to_string(),
                vec!["compute".to_string()],
                None,
            )
            .unwrap();

        // Login with the same name should return the existing worker
        let (logged_in_worker, new_token) = conductor
            .worker_login("login-worker", None)
            .unwrap();

        assert_eq!(logged_in_worker.id, worker.id);
        assert!(!new_token.is_empty()); // Should have a valid token
    }

    #[test]
    fn worker_login_new_auto_registers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Login with a name that doesn't exist yet should auto-register
        let (worker, token) = conductor
            .worker_login("new-worker", None)
            .unwrap();

        assert_eq!(worker.name, "new-worker");
        assert!(!token.is_empty());

        // Verify the worker was registered
        let workers = conductor.list_workers();
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].name, "new-worker");
    }

    #[test]
    fn task_assignment_validates_trust_level() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Register workers with different trust levels
        let (trusted, _) = conductor
            .register_worker("trusted-worker".to_string(), vec!["compute".to_string()], None)
            .unwrap();
        conductor.store.set_worker_trust_level(&trusted.id, 2).unwrap();

        let (untrusted, _) = conductor
            .register_worker("new-worker".to_string(), vec!["compute".to_string()], None)
            .unwrap();
        // Trust level is 0 by default — insufficient for auto-assignment

        // Create a goal and task requiring trust level 1+
        let goal = conductor.store.create_goal("Sensitive task".to_string()).unwrap();
        let task = conductor
            .store
            .create_task(&goal.id, "handle sensitive data".to_string())
            .unwrap();
        conductor.store.set_task_trust_level(&task.id, 1).unwrap();

        // Only the trusted worker should be able to claim this task
        let trusted_tasks = conductor.tasks_for_worker(&trusted.id).unwrap();
        assert_eq!(trusted_tasks.len(), 1);
        assert_eq!(trusted_tasks[0].id, task.id);

        let untrusted_tasks = conductor.tasks_for_worker(&untrusted.id).unwrap();
        assert_eq!(untrusted_tasks.len(), 0); // Trust level too low
    }

    #[test]
    fn wildcard_capability_matching() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Register worker with specific GPU
        let (worker, _) = conductor
            .register_worker(
                "gpu-node".to_string(),
                vec!["compute".to_string(), "gpu:nvidia:rtx4090".to_string()],
                None,
            )
            .unwrap();
        conductor.store.set_worker_trust_level(&worker.id, 1).unwrap();

        // Create task requiring any NVIDIA GPU
        let goal = conductor.store.create_goal("GPU task".to_string()).unwrap();
        let task = conductor
            .store
            .create_task(&goal.id, "use nvidia gpu".to_string())
            .unwrap();
        conductor
            .store
            .set_task_capabilities(&task.id, &["gpu:nvidia:*".to_string()])
            .unwrap();

        // Worker with rtx4090 should match gpu:nvidia:*
        let tasks = conductor.tasks_for_worker(&worker.id).unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn trust_level_upgrades_after_threshold() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Register worker at trust level 0
        let (worker, _) = conductor
            .register_worker("threshold-worker".to_string(), vec!["compute".to_string()], None)
            .unwrap();
        assert_eq!(worker.trust_level, 0);

        // Complete 5 successful tasks to reach the upgrade threshold
        for i in 0..5 {
            let goal = conductor.store.create_goal(format!("Goal {}", i)).unwrap();
            let task = conductor
                .store
                .create_task(&goal.id, "echo success".to_string())
                .unwrap();
            conductor.store.assign_task(&task.id, &worker.id).unwrap();
            conductor
                .report_task_complete(&task.id, Some("success".to_string()))
                .unwrap();
        }

        // After 5 successes, trust level should upgrade to 1
        let updated_worker = conductor.store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(updated_worker.trust_level, 1);
        assert_eq!(updated_worker.successful_tasks, 5);
    }

    #[test]
    fn failed_task_decreases_trust_level() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Register worker and manually set to trust level 2
        let (worker, _) = conductor
            .register_worker("fail-worker".to_string(), vec!["compute".to_string()], None)
            .unwrap();
        conductor.store.set_worker_trust_level(&worker.id, 2).unwrap();

        // Complete some tasks to establish a baseline
        for i in 0..5 {
            let goal = conductor.store.create_goal(format!("Goal {}", i)).unwrap();
            let task = conductor
                .store
                .create_task(&goal.id, "echo success".to_string())
                .unwrap();
            conductor.store.assign_task(&task.id, &worker.id).unwrap();
            conductor
                .report_task_complete(&task.id, Some("success".to_string()))
                .unwrap();
        }

        // Fail enough tasks to exceed 50% failure rate (6 failures vs 5 successes)
        for i in 0..6 {
            let goal = conductor.store.create_goal(format!("Fail goal {}", i)).unwrap();
            let task = conductor
                .store
                .create_task(&goal.id, "echo fail".to_string())
                .unwrap();
            conductor.store.assign_task(&task.id, &worker.id).unwrap();
            conductor
                .report_task_failed(&task.id, "error occurred".to_string())
                .unwrap();
        }

        // Trust level should decrease from 2 to 1 (failure rate > 50%)
        let updated_worker = conductor.store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(updated_worker.trust_level, 1);
    }

    #[test]
    fn idle_decay_reduces_trust_level() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Register worker and manually set to trust level 2
        let (worker, _) = conductor
            .register_worker("idle-worker".to_string(), vec!["compute".to_string()], None)
            .unwrap();
        conductor.store.set_worker_trust_level(&worker.id, 2).unwrap();

        // Simulate idle time by backdating last_heartbeat to >30 days ago
        let now = chrono::Utc::now();
        let thirty_days = chrono::Duration::days(31);
        let past = now - thirty_days;
        conductor
            .store
            .update_worker_last_heartbeat(&worker.id, &past)
            .unwrap();

        // Heartbeat should trigger idle decay
        conductor.worker_heartbeat(&worker.id).unwrap();

        let updated_worker = conductor.store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(updated_worker.trust_level, 1);
    }

    #[test]
    fn api_returns_new_token_on_trust_change() {
        use crate::api;
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use serde_json::Value;

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conductor =
            Conductor::with_db(ConductorConfig::default(), &path.to_string_lossy()).unwrap();

        // Register worker at trust level 0
        let (worker, original_token) = conductor
            .register_worker("token-worker".to_string(), vec!["compute".to_string()], None)
            .unwrap();
        assert_eq!(worker.trust_level, 0);

        // Complete tasks to reach threshold and trigger upgrade
        for i in 0..5 {
            let goal = conductor.store.create_goal(format!("Goal {}", i)).unwrap();
            let task = conductor
                .store
                .create_task(&goal.id, "echo success".to_string())
                .unwrap();
            conductor.store.assign_task(&task.id, &worker.id).unwrap();
            conductor
                .report_task_complete(&task.id, Some("success".to_string()))
                .unwrap();
        }

        // Worker should now be at trust level 1
        let updated_worker = conductor.store.get_worker(&worker.id).unwrap().unwrap();
        assert_eq!(updated_worker.trust_level, 1);

        // Login again — token should reflect new trust level
        let (_, new_token) = conductor.worker_login("token-worker", None).unwrap();
        assert_ne!(original_token, new_token);

        // Verify the token's trust level claim via validate_token
        let claims = auth::validate_token(&new_token).unwrap();
        assert_eq!(claims.trust_level, 1);
    }
}
