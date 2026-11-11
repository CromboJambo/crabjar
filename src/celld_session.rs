//! Celld session store client — durable KV storage for agent workloads.
//!
//! Each agent session maps to one celld cell. State (progress, observations,
//! pending work) is stored in the cell's built-in SQLite, surviving node
//! restarts and enabling resume after failure.
//!
//! Config: CELLD_SESSION_URL (default http://127.0.0.1:8789)

use serde::{Deserialize, Serialize};
use std::env;

const DEFAULT_URL: &str = "http://127.0.0.1:8789";

/// Metadata about a stored entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMeta {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub written_at: u64,
    pub size: usize,
}

/// A single entry in the session store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<EntryMeta>,
}

/// Response from the session store worker.
#[derive(Debug, Deserialize)]
struct WorkerResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    value: Option<serde_json::Value>,
    #[serde(default)]
    meta: Option<EntryMeta>,
    #[serde(default)]
    count: Option<usize>,
    #[serde(default)]
    entries: Option<Vec<SessionEntry>>,
}

/// Client for a single agent session in the celld session store.
pub struct SessionClient {
    base_url: String,
    session_id: String,
}

impl SessionClient {
    /// Create a client for the given session ID.
    pub fn new(session_id: &str) -> Self {
        let base_url = env::var("CELLD_SESSION_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
        Self {
            base_url,
            session_id: session_id.to_string(),
        }
    }

    fn url(&self, key: Option<&str>, list: bool) -> String {
        let mut url = format!("{}/?session={}", self.base_url, self.session_id);
        if list {
            url.push_str("&list=1");
        } else if let Some(k) = key {
            url.push_str(&format!("&key={}", k));
        }
        url
    }

    fn run_curl(&self, args: &[&str]) -> Result<WorkerResponse, String> {
        let output = std::process::Command::new("curl")
            .args(["-s", "--max-time", "5"])
            .args(args)
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "curl exited {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        serde_json::from_slice(&output.stdout).map_err(|e| format!("JSON parse error: {}", e))
    }

    /// Store a value (create or replace). Returns metadata on success.
    pub fn put(
        &self,
        key: &str,
        value: &serde_json::Value,
        entry_type: &str,
    ) -> Result<EntryMeta, String> {
        let body = serde_json::json!({ "value": value, "type": entry_type });
        let resp = self.run_curl(&[
            "-X",
            "PUT",
            "-H",
            "Content-Type: application/json",
            "-d",
            &body.to_string(),
            &self.url(Some(key), false),
        ])?;

        if !resp.ok {
            return Err(resp.error.unwrap_or_else(|| "put failed".into()));
        }
        resp.meta
            .ok_or_else(|| "worker returned no metadata".into())
    }

    /// Retrieve a value by key. Returns None if not found.
    pub fn get(&self, key: &str) -> Result<Option<serde_json::Value>, String> {
        let resp = self.run_curl(&[&self.url(Some(key), false)])?;
        if !resp.ok {
            if resp.error.as_deref() == Some("not_found") {
                return Ok(None);
            }
            return Err(resp.error.unwrap_or_else(|| "get failed".into()));
        }
        Ok(resp.value)
    }

    /// List all entries in the session with metadata.
    pub fn list(&self) -> Result<Vec<SessionEntry>, String> {
        let resp = self.run_curl(&[&self.url(None, true)])?;
        if !resp.ok {
            return Err(resp.error.unwrap_or_else(|| "list failed".into()));
        }
        Ok(resp.entries.unwrap_or_default())
    }

    /// Delete an entry by key.
    pub fn delete(&self, key: &str) -> Result<(), String> {
        let resp = self.run_curl(&["-X", "DELETE", &self.url(Some(key), false)])?;
        if !resp.ok {
            return Err(resp.error.unwrap_or_else(|| "delete failed".into()));
        }
        Ok(())
    }
}

/// A workload step that can be checkpointed and resumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadStep {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub status: StepStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// A checkpointed workload that can be resumed after interruption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointedWorkload {
    pub name: String,
    pub created_at: u64,
    pub steps: Vec<WorkloadStep>,
}

impl CheckpointedWorkload {
    pub fn new(name: &str, steps: Vec<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            name: name.to_string(),
            created_at: now,
            steps: steps
                .into_iter()
                .enumerate()
                .map(|(i, desc)| WorkloadStep {
                    id: format!("step-{i:02}"),
                    description: desc,
                    status: StepStatus::Pending,
                    result: None,
                    completed_at: None,
                })
                .collect(),
        }
    }

    pub fn next_pending(&self) -> Option<usize> {
        self.steps.iter().position(|s| s.status == StepStatus::Pending)
    }

    pub fn is_complete(&self) -> bool {
        self.steps.iter().all(|s| matches!(s.status, StepStatus::Completed | StepStatus::Failed))
    }

    pub fn progress_summary(&self) -> String {
        let total = self.steps.len();
        let done = self.steps.iter().filter(|s| s.status == StepStatus::Completed).count();
        let failed = self.steps.iter().filter(|s| s.status == StepStatus::Failed).count();
        format!("{}/{} complete, {} failed", done, total, failed)
    }
}
