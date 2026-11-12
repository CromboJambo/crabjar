//! Worker management methods for the conductor.

use crate::audit::{AuditContext, AuditEventType};
use crate::auth;
use crate::conductor::Conductor;
use crate::gate::trust_sufficient_for_auto_assign;
use anyhow::Result;
use serde_json::json;
use tracing::{debug, info, warn};

impl Conductor {
    /// List all registered workers.
    pub fn list_workers(&self) -> Vec<crate::state_store::Worker> {
        self.store.list_workers().unwrap_or_default()
    }

    /// Register a new worker in the fleet. Returns (worker, `auth_token`).
    pub fn register_worker(
        &self,
        name: String,
        capabilities: Vec<String>,
        ssh_key: Option<String>,
    ) -> Result<(crate::state_store::Worker, String)> {
        // Validate SSH key format if provided
        if let Some(ref key) = ssh_key {
            auth::validate_ssh_public_key(key)?;
        }

        let result = self
            .store
            .register_worker(name.clone(), capabilities, ssh_key)?;

        // Audit: log worker registration
        if let Some(logger) = &self.audit_logger {
            let logger = logger.clone();
            let worker_id = result.0.id.clone();
            tokio::spawn(async move {
                let logger = logger.lock().await;
                let ctx = AuditContext {
                    task_id: None,
                    worker_id: Some(worker_id),
                    outcome: "worker_registered".to_string(),
                    duration_ms: None,
                    metadata: json!({}),
                };
                let _ = logger.log_event(AuditEventType::RouteDecision, ctx).await;
            });
        }

        Ok(result)
    }

    /// Authenticate a worker by name and SSH key, returning (worker, `auth_token`).
    /// If the worker is not registered, this creates a new registration.
    pub fn worker_login(
        &self,
        name: &str,
        ssh_key: Option<String>,
    ) -> Result<(crate::state_store::Worker, String)> {
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
        let result = self
            .store
            .register_worker(name.to_string(), vec![], ssh_key)?;
        Ok(result)
    }

    /// Poll for tasks assigned to a specific worker.
    pub fn tasks_for_worker(&self, worker_id: &str) -> Result<Vec<crate::state_store::Task>> {
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
        for task in &ready_tasks {
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

            // Resource-aware scheduling: verify the worker has enough resources.
            // Fail-open: if riod is unavailable, assignment proceeds as before.
            if !self.riod_checker().can_satisfy(&task.resources) {
                debug!(
                    task_id = %task.id,
                    worker_id = %worker_id,
                    cpu = task.resources.cpu,
                    memory_mb = task.resources.memory_mb,
                    gpu = task.resources.gpu,
                    "Worker resources insufficient (riod check)"
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

        // Audit: log task completion with worker and goal context
        if let Some(logger) = &self.audit_logger {
            let logger = logger.clone();
            let task_id = task_id.to_string();
            let worker_id = worker_id.clone();
            let goal_id = task.goal_id.clone();
            tokio::spawn(async move {
                let logger = logger.lock().await;
                let ctx = AuditContext {
                    task_id: Some(task_id),
                    worker_id: Some(worker_id),
                    outcome: "task_completed".to_string(),
                    duration_ms: None,
                    metadata: json!({"goal_id": goal_id}),
                };
                let _ = logger.log_event(AuditEventType::TaskCompleted, ctx).await;
            });
        }

        // Anneal trust level based on success
        let new_trust_level = self.store.record_task_success(&worker_id)?;

        // Check if all tasks for the goal are complete
        let goal_tasks = self.store.list_tasks_for_goal(&task.goal_id)?;
        let all_complete = goal_tasks
            .iter()
            .all(|t| t.status == crate::state_store::TaskStatus::Completed);

        if all_complete {
            self.store
                .update_goal_status(&task.goal_id, crate::state_store::GoalStatus::Completed)?;
        }

        // If trust level changed, issue a new token with updated scope
        let worker = self
            .store
            .get_worker(&worker_id)?
            .ok_or_else(|| anyhow::anyhow!("worker not found"))?;
        if worker.trust_level == new_trust_level {
            Ok(None)
        } else {
            // Trust level was upgraded — refresh token
            let new_token = self.store.refresh_worker_token(&worker_id)?;
            info!(worker_id = %worker_id, trust_level = new_trust_level, "Worker trust level upgraded, new token issued");

            // Audit: log trust level change
            if let Some(logger) = &self.audit_logger {
                let logger = logger.clone();
                let worker_id = worker_id.clone();
                tokio::spawn(async move {
                    let logger = logger.lock().await;
                    let ctx = AuditContext {
                        task_id: None,
                        worker_id: Some(worker_id),
                        outcome: format!("trust_upgrade:{new_trust_level}"),
                        duration_ms: None,
                        metadata: json!({}),
                    };
                    let _ = logger.log_event(AuditEventType::WorkerPromoted, ctx).await;
                });
            }

            Ok(Some(new_token))
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

        // Audit: log task failure with worker and goal context
        if let Some(logger) = &self.audit_logger {
            let logger = logger.clone();
            let task_id = task_id.to_string();
            let worker_id = worker_id.clone();
            let goal_id = task.goal_id.clone();
            tokio::spawn(async move {
                let logger = logger.lock().await;
                let ctx = AuditContext {
                    task_id: Some(task_id),
                    worker_id: Some(worker_id),
                    outcome: "task_failed".to_string(),
                    duration_ms: None,
                    metadata: json!({"goal_id": goal_id}),
                };
                let _ = logger.log_event(AuditEventType::TaskFailed, ctx).await;
            });
        }

        // Anneal trust level based on failure
        let new_trust_level = self.store.record_task_failure(&worker_id)?;

        // If trust level changed (downgrade), issue a new token with reduced scope
        let worker = self
            .store
            .get_worker(&worker_id)?
            .ok_or_else(|| anyhow::anyhow!("worker not found"))?;
        if worker.trust_level == new_trust_level {
            Ok(None)
        } else {
            let new_token = self.store.refresh_worker_token(&worker_id)?;
            warn!(worker_id = %worker_id, trust_level = new_trust_level, "Worker trust level downgraded, new token issued");

            // Audit: log trust level change
            if let Some(logger) = &self.audit_logger {
                let logger = logger.clone();
                let worker_id = worker_id.clone();
                tokio::spawn(async move {
                    let logger = logger.lock().await;
                    let ctx = AuditContext {
                        task_id: None,
                        worker_id: Some(worker_id),
                        outcome: format!("trust_downgrade:{new_trust_level}"),
                        duration_ms: None,
                        metadata: json!({}),
                    };
                    let _ = logger.log_event(AuditEventType::WorkerDemoted, ctx).await;
                });
            }

            Ok(Some(new_token))
        }
    }

    /// Update worker status via heartbeat.
    pub fn worker_heartbeat(&self, worker_id: &str) -> Result<()> {
        self.store.heartbeat(worker_id)?;
        Ok(())
    }
}
