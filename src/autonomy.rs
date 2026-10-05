//! Agent autonomy with guard-based approval flow.
//!
//! Maps actions to risk levels, auto-approves low-risk actions,
//! queues high-risk for user review via crabjar's existing guard system.
//! Includes query caching (L3-style) and study material generation.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

// ============================================================================
// Action risk classification
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Auto-approve: read-only, idempotent, reversible operations
    L0_Auto = 0,
    /// Low risk: local changes that are easily reversible
    L1_Low = 1,
    /// Medium risk: changes that affect shared state or require review
    L2_Medium = 2,
    /// High risk: destructive or hard-to-reverse operations
    L3_High = 3,
    /// Critical: requires explicit user approval every time
    L4_Critical = 4,
}

impl RiskLevel {
    pub fn auto_approve(&self) -> bool {
        matches!(self, RiskLevel::L0_Auto | RiskLevel::L1_Low)
    }

    pub fn requires_review(&self) -> bool {
        matches!(self, RiskLevel::L2_Medium | RiskLevel::L3_High | RiskLevel::L4_Critical)
    }
}

/// Classify an action type to its risk level.
pub fn classify_action(action_type: &str) -> RiskLevel {
    match action_type {
        // L0: Auto-approve (read-only, idempotent)
        "file_read" | "search_files" | "list_directory" | "git_status"
        | "git_log" | "cargo_check" | "cargo_test" | "cargo_clippy"
        | "crabjar_state_list" | "crabjar_guard_pending" => RiskLevel::L0_Auto,

        // L1: Low risk (local, reversible)
        "file_write" | "file_patch" | "git_commit" | "git_add"
        | "cargo_build" | "cargo_fmt" | "create_branch"
        | "crabjar_state_update" => RiskLevel::L1_Low,

        // L2: Medium risk (shared state, needs review)
        "git_push" | "git_merge" | "deploy_local"
        | "modify_config" | "install_dependency" => RiskLevel::L2_Medium,

        // L3: High risk (destructive or hard to reverse)
        "delete_file" | "git_reset" | "git_tag"
        | "deploy_remote" | "modify_guard_policy" => RiskLevel::L3_High,

        // L4: Critical (always require explicit approval)
        "sudo_command" | "network_change" | "user_management"
        | "crabjar_guard_modify" => RiskLevel::L4_Critical,

        _ => RiskLevel::L2_Medium, // Default to review for unknown actions
    }
}

// ============================================================================
// Query cache (L3-style)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedQuery {
    pub query: String,
    pub result: String,
    pub timestamp: i64,
    pub ttl_seconds: u64,
    pub source: String, // "guard", "trust_layer", "environment"
}

impl CachedQuery {
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        (now - self.timestamp) > self.ttl_seconds as i64
    }
}

/// Simple in-memory query cache with TTL.
pub struct QueryCache {
    entries: HashMap<String, CachedQuery>,
    max_size: usize,
}

impl QueryCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
        }
    }

    pub fn get(&self, key: &str) -> Option<&CachedQuery> {
        self.entries.get(key).filter(|e| !e.is_expired())
    }

    pub fn set(&mut self, key: String, query: String, result: String, ttl_seconds: u64, source: String) {
        // Evict oldest if at capacity
        if self.entries.len() >= self.max_size && !self.entries.contains_key(&key) {
            let oldest_key = self.entries.iter().min_by_key(|(_, v)| v.timestamp).map(|(k, _)| k.clone());
            if let Some(k) = oldest_key {
                self.entries.remove(&k);
            }
        }

        self.entries.insert(key, CachedQuery {
            query,
            result,
            timestamp: Utc::now().timestamp(),
            ttl_seconds,
            source,
        });
    }

    pub fn stats(&self) -> (usize, usize) {
        let total = self.entries.len();
        let expired = self.entries.values().filter(|e| e.is_expired()).count();
        (total, expired)
    }
}

// ============================================================================
// Study material generation
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudyMaterial {
    pub topic: String,
    pub title: String,
    pub content: String,
    pub sources: Vec<String>,
    pub created_at: i64,
}

/// Generate study material based on autonomous actions taken.
pub fn generate_study_material(topic: &str, actions_taken: &[String], sources: &[String]) -> StudyMaterial {
    let title = format!("Understanding {}", topic);
    let content = format!(
        "# {}\n\n## Context\n\nThis material was generated based on the following autonomous actions:\n\n{}\n\n## Key Concepts\n\n- What risk levels mean in crabjar's guard system\n- Why certain actions require approval while others are auto-approved\n- How to override or adjust autonomy settings\n\n## Sources\n\n{}\n",
        topic,
        actions_taken.join("\n"),
        sources.join("\n")
    );

    StudyMaterial {
        topic: topic.to_string(),
        title,
        content,
        sources: sources.to_vec(),
        created_at: Utc::now().timestamp(),
    }
}

/// Save study material to user_courses directory.
pub fn save_study_material(material: &StudyMaterial, output_dir: &PathBuf) -> Result<PathBuf, String> {
    fs::create_dir_all(output_dir).map_err(|e| format!("Failed to create dir: {}", e))?;

    let filename = format!(
        "{}_{}.md",
        material.topic.replace(' ', "-").to_lowercase(),
        chrono::DateTime::from_timestamp(material.created_at, 0)
            .map(|dt| dt.format("%Y%m%d_%H%M%S").to_string())
            .unwrap_or("unknown".to_string())
    );

    let path = output_dir.join(filename);
    fs::write(&path, &material.content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(path)
}

// ============================================================================
// Autonomy audit log
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyEntry {
    pub action_type: String,
    pub risk_level: u8,
    pub decision: String, // "auto_approved", "queued", "approved", "rejected"
    pub reason: Option<String>,
    pub timestamp: i64,
}

/// Log an autonomy decision to ~/.crabjar/autonomy-log.jsonl
pub fn log_autonomy_decision(entry: &AutonomyEntry) -> Result<(), String> {
    let home = std::env::var("HOME").map(PathBuf::from).map_err(|e| format!("Failed to get HOME env var: {}", e))?;
    let crabjar_dir = home.join(".crabjar");
    fs::create_dir_all(&crabjar_dir).map_err(|e| format!("Failed to create .crabjar dir: {}", e))?;

    let log_path = crabjar_dir.join("autonomy-log.jsonl");
    let line = serde_json::to_string(entry).map_err(|e| format!("Failed to serialize entry: {}", e))?;

    use std::fs::OpenOptions;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Failed to open log file: {}", e))?;

    writeln!(file, "{}", line).map_err(|e| format!("Failed to write to log: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_action_read_only() {
        assert_eq!(classify_action("file_read"), RiskLevel::L0_Auto);
        assert_eq!(classify_action("git_status"), RiskLevel::L0_Auto);
    }

    #[test]
    fn test_classify_action_low_risk() {
        assert_eq!(classify_action("file_write"), RiskLevel::L1_Low);
        assert_eq!(classify_action("git_commit"), RiskLevel::L1_Low);
    }

    #[test]
    fn test_classify_action_medium_risk() {
        assert_eq!(classify_action("git_push"), RiskLevel::L2_Medium);
        assert_eq!(classify_action("deploy_local"), RiskLevel::L2_Medium);
    }

    #[test]
    fn test_classify_action_high_risk() {
        assert_eq!(classify_action("delete_file"), RiskLevel::L3_High);
        assert_eq!(classify_action("git_reset"), RiskLevel::L3_High);
    }

    #[test]
    fn test_classify_action_critical() {
        assert_eq!(classify_action("sudo_command"), RiskLevel::L4_Critical);
    }

    #[test]
    fn test_query_cache_set_get() {
        let mut cache = QueryCache::new(10);
        cache.set(
            "test-key".to_string(),
            "What is the risk level of git_push?".to_string(),
            "L2_Medium".to_string(),
            3600,
            "guard".to_string(),
        );

        let cached = cache.get("test-key");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().result, "L2_Medium");
    }

    #[test]
    fn test_query_cache_expiry() {
        let mut cache = QueryCache::new(10);
        // Manually set an expired entry (timestamp 2 seconds ago, TTL of 1 second)
        cache.entries.insert("expired-key".to_string(), CachedQuery {
            query: "query".to_string(),
            result: "result".to_string(),
            timestamp: Utc::now().timestamp() - 2,
            ttl_seconds: 1,
            source: "guard".to_string(),
        });

        assert!(cache.get("expired-key").is_none());
    }

    #[test]
    fn test_generate_study_material() {
        let material = generate_study_material(
            "Guard System",
            &["git_push origin main".to_string()],
            &["https://crabjar.dev/docs/guard".to_string()],
        );

        assert_eq!(material.topic, "Guard System");
        assert!(material.title.contains("Guard System"));
        assert!(material.content.contains("git_push"));
    }
}