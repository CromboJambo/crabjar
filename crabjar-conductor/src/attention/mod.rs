//! Hybrid Attention + Semantic Decision Layer
//!
//! Two-stage decision pipeline:
//! 1. Attention Triage (Mirror-Log): Fast rule-based scoring filters ~90% of observations
//! 2. Semantic Evaluation (JEV): Deep embedding matching for high-attention items only
//!
//! This module provides the attention layer integration with crabjar-conductor's
//! existing JEV decision gate infrastructure.

use crate::decision::{DecisionGate, DecisionResult};
use anyhow::Result;
use chrono::{Duration, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Attention score thresholds for escalation decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionThresholds {
    /// Below this: ignore (fast filter)
    pub low: f64,
    /// Above this: escalate to JEV semantic evaluation
    pub high: f64,
}

impl Default for AttentionThresholds {
    fn default() -> Self {
        Self {
            low: 30.0,
            high: 70.0,
        }
    }
}

/// Decision policy configuration for the hybrid model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridDecisionPolicy {
    pub attention_thresholds: AttentionThresholds,
    /// JEV criteria files per observation source type
    pub jev_criteria_files: Option<std::collections::HashMap<String, String>>,
}

impl Default for HybridDecisionPolicy {
    fn default() -> Self {
        Self {
            attention_thresholds: AttentionThresholds::default(),
            jev_criteria_files: None,
        }
    }
}

/// Outcome of the hybrid two-stage decision pipeline.
#[derive(Debug, Clone)]
pub struct HybridDecision {
    /// Stage 1 attention score (0-100)
    pub attention_score: f64,
    /// Whether the item escalated to JEV evaluation
    pub escalated_to_jev: bool,
    /// JEV decision result (if escalated)
    pub jev_result: Option<DecisionResult>,
    /// Final action determined by the pipeline
    pub action: HybridAction,
}

/// Final action types from the hybrid decision pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum HybridAction {
    /// Item filtered out by attention layer (low score)
    Ignore,
    /// Item logged for monitoring but no action taken
    Log,
    /// Item escalated to JEV and execution is recommended
    Execute(String),
    /// Item escalated to JEV but should be held pending more data
    Hold(String),
}

/// Hybrid attention + semantic decision layer.
pub struct HybridDecisionLayer {
    /// Mirror-log database connection for attention scoring
    mirror_log_conn: Arc<Mutex<Connection>>,
    /// JEV decision gate for semantic evaluation
    jev_gate: DecisionGate,
    /// Configurable thresholds and policy
    policy: HybridDecisionPolicy,
}

impl HybridDecisionLayer {
    /// Create a new hybrid decision layer.
    pub fn new(
        mirror_log_db_path: &str,
        jev_gate: DecisionGate,
        policy: Option<HybridDecisionPolicy>,
    ) -> Result<Self> {
        let conn = Connection::open(mirror_log_db_path)?;
        // Enable WAL mode for better concurrent access
        conn.execute_batch("PRAGMA journal_mode=WAL")?;
        Ok(Self {
            mirror_log_conn: Arc::new(Mutex::new(conn)),
            jev_gate,
            policy: policy.unwrap_or_default(),
        })
    }

    /// Process an observation through the two-stage pipeline.
    pub async fn process_observation(
        &self,
        obs_id: &str,
        source: &str,
        content: &str,
        timestamp: Option<i64>,
    ) -> Result<HybridDecision> {
        let ts = timestamp.unwrap_or_else(|| Utc::now().timestamp());

        // Stage 1: Attention triage (fast, rule-based)
        let attention_score = self
            .calculate_attention(obs_id, source, content, ts)
            .await?;
        debug!(
            obs_id = %obs_id,
            attention_score = attention_score,
            "Stage 1: attention score calculated"
        );

        // Apply threshold logic
        if attention_score < self.policy.attention_thresholds.low {
            return Ok(HybridDecision {
                attention_score,
                escalated_to_jev: false,
                jev_result: None,
                action: HybridAction::Ignore,
            });
        }

        if attention_score < self.policy.attention_thresholds.high {
            // Log for monitoring but don't escalate to JEV
            return Ok(HybridDecision {
                attention_score,
                escalated_to_jev: false,
                jev_result: None,
                action: HybridAction::Log,
            });
        }

        // Stage 2: Semantic evaluation via JEV decision gate
        info!(obs_id = %obs_id, "Escalating to JEV semantic evaluation");
        let criteria_file = self.get_criteria_for_source(source)?;

        match self.jev_gate.evaluate(content, &criteria_file, None).await {
            Ok(jev_result) => {
                if !jev_result.option.is_empty() {
                    Ok(HybridDecision {
                        attention_score,
                        escalated_to_jev: true,
                        jev_result: Some(jev_result.clone()),
                        action: HybridAction::Execute(jev_result.option),
                    })
                } else {
                    Ok(HybridDecision {
                        attention_score,
                        escalated_to_jev: true,
                        jev_result: Some(jev_result.clone()),
                        action: HybridAction::Hold(jev_result.rationale),
                    })
                }
            }
            Err(e) => {
                warn!(obs_id = %obs_id, error = %e, "JEV evaluation failed");
                // Graceful degradation: hold if JEV fails
                Ok(HybridDecision {
                    attention_score,
                    escalated_to_jev: true,
                    jev_result: None,
                    action: HybridAction::Hold(format!("JEV evaluation failed: {}", e)),
                })
            }
        }
    }

    /// Calculate attention score using mirror-log's decay/access frequency model.
    async fn calculate_attention(
        &self,
        obs_id: &str,
        source: &str,
        content: &str,
        timestamp: i64,
    ) -> Result<f64> {
        let conn = self.mirror_log_conn.lock().await;

        // Insert or update observation in mirror-log for attention tracking
        let now = Utc::now().timestamp();
        let age_seconds = now - timestamp;

        // Temporal decay: newer items score higher
        // Half-life of 7 days (604800 seconds)
        let temporal_score = if age_seconds <= 0 {
            100.0
        } else {
            let half_life: f64 = 604800.0;
            let decay_factor = (-age_seconds as f64 / half_life * std::f64::consts::LN_2).exp();
            decay_factor * 100.0
        };

        // Source type prioritization: alerts > metrics > journal
        let source_weight = match source {
            "alert" | "critical" => 1.3,
            "metrics" | "monitoring" => 1.1,
            _ => 1.0,
        };

        // Content length heuristic: longer content tends to be more significant
        let length_factor = if content.len() > 500 {
            1.2
        } else if content.len() > 100 {
            1.1
        } else {
            1.0
        };

        // Combined score (clamped to 0-100)
        let raw_score = temporal_score * source_weight * length_factor;
        let score = raw_score.clamp(0.0, 100.0);

        // Track access for future attention calculations
        self.track_access_internal(conn.deref(), obs_id)?;

        Ok(score)
    }

    /// Get JEV criteria file path for a given observation source type.
    fn get_criteria_for_source(&self, source: &str) -> Result<String> {
        if let Some(criteria_map) = &self.policy.jev_criteria_files {
            if let Some(file) = criteria_map.get(source) {
                return Ok(file.clone());
            }
        }

        // Default criteria file for unknown sources
        Ok("specs/criteria/autonomous-routing-evaluation.json".to_string())
    }

    /// Track access to an observation (updates decay score).
    fn track_access_internal(&self, conn: &Connection, obs_id: &str) -> Result<()> {
        // Create decay tracking row if it doesn't exist
        conn.execute(
            "INSERT OR IGNORE INTO decay (event_id, access_count, last_accessed, pinned) \
             VALUES (?, 1, unixepoch(), 0)",
            [obs_id],
        )?;

        // Update access count and timestamp
        conn.execute(
            "UPDATE decay SET access_count = access_count + 1, last_accessed = unixepoch() \
             WHERE event_id = ?",
            [obs_id],
        )?;

        Ok(())
    }

    /// Get attention statistics from mirror-log.
    pub async fn get_attention_stats(&self) -> Result<AttentionStats> {
        let conn = self.mirror_log_conn.lock().await;

        let total_events: i64 =
            conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;

        let active_events: i64 = conn.query_row(
            "SELECT COUNT(*) FROM decay WHERE last_accessed > unixepoch() - 86400 * 7",
            [],
            |row| row.get(0),
        )?;

        Ok(AttentionStats {
            total_events,
            active_events,
            attention_thresholds: self.policy.attention_thresholds.clone(),
        })
    }
}

/// Attention layer statistics.
#[derive(Debug, Clone, Serialize)]
pub struct AttentionStats {
    pub total_events: i64,
    pub active_events: i64,
    pub attention_thresholds: AttentionThresholds,
}

impl AttentionStats {
    pub fn active_percentage(&self) -> f64 {
        if self.total_events == 0 {
            0.0
        } else {
            (self.active_events as f64 / self.total_events as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{BackendConfig, DecisionPolicy};
    use tempfile::tempdir;

    fn create_test_layer() -> (HybridDecisionLayer, std::path::PathBuf, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let path_str = db_path.to_string_lossy().to_string();

        // Initialize mirror-log schema in test database
        let conn = Connection::open(&path_str).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (id TEXT PRIMARY KEY, timestamp INTEGER, source TEXT, content TEXT, meta TEXT)",
            [],
        ).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS decay (event_id TEXT PRIMARY KEY, access_count INTEGER, last_accessed INTEGER, pinned INTEGER)",
            [],
        ).unwrap();

        let backend_config = BackendConfig {
            url: "http://localhost:8081".to_string(),
            model_id: "test-model".to_string(),
        };
        let jev_gate = DecisionGate::new(
            crate::decision::evaluate::JevEvaluator::new(backend_config),
            None::<DecisionPolicy>,
        );

        let layer = HybridDecisionLayer::new(&path_str, jev_gate, None).unwrap();
        (layer, db_path, dir)
    }

    #[tokio::test]
    async fn test_attention_score_new_item() {
        let (layer, _db_path, _dir) = create_test_layer();

        // New item should have high attention score
        let decision = layer
            .process_observation(
                "test-obs-1",
                "alert",
                "Test observation content",
                Some(Utc::now().timestamp()),
            )
            .await
            .unwrap();

        assert!(
            decision.attention_score > 70.0,
            "New alert should score high"
        );
    }

    #[tokio::test]
    async fn test_attention_score_old_item() {
        let (layer, _db_path, _dir) = create_test_layer();

        // Old item (30 days ago) should have low attention score
        let old_ts = Utc::now().timestamp() - (30 * 24 * 60 * 60);
        let decision = layer
            .process_observation(
                "test-obs-2",
                "journal",
                "Old observation content",
                Some(old_ts),
            )
            .await
            .unwrap();

        assert!(
            decision.attention_score < 30.0,
            "Old journal item should score low"
        );
    }

    #[tokio::test]
    async fn test_attention_thresholds() {
        let (layer, _db_path, _dir) = create_test_layer();

        // Item with moderate attention should be logged but not escalated
        // 3 days old with metrics source: decay ~87%, score ~96% -> high enough to escalate
        // Let's use 10 days old: decay ~52%, score ~57% -> Log range
        let mid_ts = Utc::now().timestamp() - (10 * 24 * 60 * 60);
        let decision = layer.process_observation(
            "test-obs-3",
            "metrics",
            "Moderate attention item with sufficient length to hit the Log range when decayed appropriately for testing purposes.",
            Some(mid_ts),
        ).await.unwrap();

        // Should be in the Log range (between low and high thresholds)
        assert!(
            matches!(decision.action, HybridAction::Log),
            "Expected Log action but got: {:?}",
            decision.action
        );
    }
}
