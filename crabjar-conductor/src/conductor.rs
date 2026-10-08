//! Core orchestration logic — goal decomposition and task lifecycle management.

use crate::state_store::{Goal, GoalStatus, StateStore, Task, TaskStatus, Worker, WorkerStatus};
use anyhow::Result;

/// Configuration for the conductor service.
pub struct ConductorConfig {
    pub listen_addr: String,
    pub listen_port: u16,
    pub pesti_server_url: String,
}

impl Default for ConductorConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port: 8091,
            pesti_server_url: "http://localhost:8000".to_string(),
        }
    }
}

/// The conductor — receives goals, decomposes them into tasks, manages lifecycle.
pub struct Conductor {
    store: StateStore,
    config: ConductorConfig,
}

impl Conductor {
    /// Create a new conductor with an in-memory database (for testing).
    pub fn new(config: ConductorConfig) -> Result<Self> {
        let store = StateStore::open(":memory:")?;
        Ok(Self { store, config })
    }

    /// Create a conductor with a specific database path.
    pub fn with_db(config: ConductorConfig, db_path: &str) -> Result<Self> {
        let store = StateStore::open(db_path)?;
        Ok(Self { store, config })
    }

    /// Submit a new goal for decomposition and execution.
    pub fn submit_goal(&self, description: String) -> Result<Goal> {
        // Create the goal record
        let goal = self.store.create_goal(description.clone())?;

        // Decompose into tasks using pesti-server (LLM inference)
        let _tasks = self.decompose(goal.id.clone(), &description)?;

        // Mark as running once decomposition is complete
        self.store.update_goal_status(&goal.id, GoalStatus::Running)?;

        Ok(goal)
    }

    /// Decompose a goal into executable tasks using LLM inference.
    fn decompose(&self, goal_id: String, description: &str) -> Result<Vec<Task>> {
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

        let client = reqwest::blocking::Client::new();
        let url = format!("{}/v1/chat/completions", self.config.pesti_server_url);

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

        let resp = client.post(&url)
            .json(&body)
            .send()
            .map_err(|e| anyhow::anyhow!("pesti-server request failed: {}", e))?;

        let status = resp.status();
        let text = resp.text().map_err(|e| anyhow::anyhow!("failed to read response: {}", e))?;

        if !status.is_success() {
            return Err(anyhow::anyhow!("pesti-server error {}: {}", status, text));
        }

        // Parse the LLM response
        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("failed to parse pesti response as JSON: {} - {}", e, &text[..std::cmp::min(200, text.len())]))?;

        // Extract content from chat completion format
        let content = parsed["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("no content in response"))?
            .trim();

        // Strip markdown code fences if present
        let json_content = if content.starts_with("```") {
            // Remove leading ```json or ``` and trailing ```
            let mut lines: Vec<&str> = content.lines().collect();
            // Skip opening fence
            while !lines.is_empty() && lines[0].trim().starts_with("```") {
                lines.remove(0);
            }
            // Skip closing fence
            while !lines.is_empty() && lines.last().unwrap().trim().starts_with("```") {
                lines.pop();
            }
            lines.join("\n").trim().to_string()
        } else {
            content.to_string()
        };

        // Try to parse the content as JSON array of tasks
        let task_specs: Vec<serde_json::Value> = serde_json::from_str(&json_content)
            .map_err(|e| anyhow::anyhow!("failed to parse task specs from LLM: {} - {}", e, &json_content[..std::cmp::min(200, json_content.len())]))?;

        // Create tasks in the store
        let mut tasks = Vec::new();
        for spec in task_specs {
            let command = spec["command"].as_str()
                .ok_or_else(|| anyhow::anyhow!("task spec missing 'command'"))?
                .to_string();
            let description = spec["description"].as_str().unwrap_or(&command).to_string();

            let task = self.store.create_task(&goal_id, format!("[{}] {}", description, command))?;
            tasks.push(task);
        }

        Ok(tasks)
    }

    /// Get status of a goal and its tasks.
    pub fn goal_status(&self, goal_id: &str) -> Result<Option<GoalStatus>> {
        let goal = self.store.get_goal(goal_id)?;
        Ok(goal.map(|g| g.status))
    }

    /// Get full goal details.
    pub fn get_goal(&self, goal_id: &str) -> Result<Option<Goal>> {
        self.store.get_goal(goal_id).map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// List all registered workers.
    pub fn list_workers(&self) -> Vec<Worker> {
        self.store.list_workers().unwrap_or_default()
    }

    /// Register a new worker in the fleet.
    pub fn register_worker(&self, name: String, capabilities: Vec<String>) -> Result<Worker> {
        let worker = self.store.register_worker(name, capabilities)?;
        Ok(worker)
    }

    /// Poll for tasks assigned to a specific worker.
    pub fn tasks_for_worker(&self, _worker_id: &str) -> Result<Vec<Task>> {
        // Get all pending tasks that are ready (dependencies met)
        let ready_tasks = self.store.get_ready_tasks()?;
        
        // In production, filter by worker capabilities and assign
        // For now, return the first unassigned task if any
        for task in ready_tasks.iter() {
            if task.worker_id.is_none() {
                // Assign this task to the requesting worker
                self.store.assign_task(&task.id, _worker_id)?;
                let updated = self.store.get_task(&task.id)?.unwrap();
                return Ok(vec![updated]);
            }
        }
        
        Ok(vec![])
    }

    /// Report completion of a task.
    pub fn report_task_complete(&self, task_id: &str, result: Option<String>) -> Result<()> {
        self.store.complete_task(task_id, result)?;
        
        // Check if all tasks for the goal are complete
        let task = self.store.get_task(task_id)?.ok_or_else(|| anyhow::anyhow!("task not found"))?;
        let goal_tasks = self.store.list_tasks_for_goal(&task.goal_id)?;
        let all_complete = goal_tasks.iter().all(|t| t.status == TaskStatus::Completed);
        
        if all_complete {
            self.store.update_goal_status(&task.goal_id, GoalStatus::Completed)?;
        }
        
        Ok(())
    }

    /// Report task failure.
    pub fn report_task_failed(&self, task_id: &str, error: String) -> Result<()> {
        self.store.fail_task(task_id, error)?;
        Ok(())
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

    #[tokio::test]
    async fn submit_goal_creates_task() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let store = StateStore::open(&path).unwrap();
        let conductor = Conductor {
            store,
            config: ConductorConfig::default(),
        };

        let goal = conductor.submit_goal("Test goal".to_string()).unwrap();
        let status = conductor.goal_status(&goal.id).unwrap().unwrap();
        assert_eq!(status, GoalStatus::Running);
    }
}