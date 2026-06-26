//! Trust-related types and operations for the guard system.
//!
//! Types: `TrustScore`, `TrustLayer`, `ReviewAction`, `ReviewRecord`,
//! `AnnealConfig`, `AnnealResult`, `RetrievalBand`, `PidTrustRecord`, `RevokedLogEntry`.
//!
//! Operations: `TrustManager` — DB-backed trust layer management.

use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::{debug, info, warn};

use crate::guard_db::{GuardDb, GuardDbError};
use crate::memory_types::NodeKind;

// ============================================================================
// TrustScore
// ============================================================================

/// Confidence score for a memory node, 0.0 (unknown) to 1.0 (certain).
/// Decays over time unless reinforced by successful outcomes.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TrustScore(f64);

impl TrustScore {
    pub const fn new(score: f64) -> Self {
        Self(score.clamp(0.0, 1.0))
    }

    pub const fn get(&self) -> f64 {
        self.0
    }

    pub const fn is_zero(&self) -> bool {
        self.0 == 0.0
    }

    pub fn reinforce(&self, delta: f64) -> Self {
        Self::new(self.0 + delta)
    }

    pub fn decay(&self, rate: f64) -> Self {
        Self::new(self.0 - rate)
    }

    pub fn interpolate(&self, other: &Self, weight: f64) -> Self {
        let blended = self.0 * (1.0 - weight) + other.0 * weight;
        Self::new(blended)
    }
}

impl Default for TrustScore {
    fn default() -> Self {
        Self(0.5)
    }
}

impl fmt::Display for TrustScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

// ============================================================================
// TrustLayer
// ============================================================================

/// Configurable trust layer band
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustLayer {
    pub id: u32,
    pub(crate) name: String,
    pub min_confidence: f64,
    pub max_confidence: f64,
    pub auto_execute: bool,
    pub requires_review: bool,
    pub(crate) description: Option<String>,
}

impl TrustLayer {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn contains_score(&self, score: TrustScore) -> bool {
        score.get() >= self.min_confidence && score.get() < self.max_confidence
    }
}

// ============================================================================
// Review types
// ============================================================================

/// Review action taken by human or automated reviewer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewAction {
    Approve,
    Reject,
    Modify,
    Escalate,
}

impl fmt::Display for ReviewAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReviewAction::Approve => write!(f, "approve"),
            ReviewAction::Reject => write!(f, "reject"),
            ReviewAction::Modify => write!(f, "modify"),
            ReviewAction::Escalate => write!(f, "escalate"),
        }
    }
}

/// Record of a human or automated review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRecord {
    pub id: String,
    pub node_id: String,
    pub reviewer: String,
    pub action: ReviewAction,
    pub old_confidence: Option<TrustScore>,
    pub new_confidence: Option<TrustScore>,
    pub old_trust_layer: Option<u32>,
    pub new_trust_layer: Option<u32>,
    pub notes: Option<String>,
    pub created_at: i64,
}

// ============================================================================
// Annealing types
// ============================================================================

/// Annealing configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnealConfig {
    pub decay_rate: f64,
    pub reinforce_threshold: f64,
    pub anneal_interval_seconds: u64,
    pub max_anneal_passes: u32,
    pub confidence_floor: f64,
    pub auto_anneal_enabled: bool,
}

impl Default for AnnealConfig {
    fn default() -> Self {
        Self {
            decay_rate: 0.02,
            reinforce_threshold: 0.7,
            anneal_interval_seconds: 3600,
            max_anneal_passes: 10,
            confidence_floor: 0.05,
            auto_anneal_enabled: true,
        }
    }
}

/// Result of an annealing pass
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnealResult {
    pub nodes_processed: usize,
    pub nodes_upgraded: usize,
    pub nodes_downgraded: usize,
    pub nodes_decayed: usize,
    pub edges_pruned: usize,
    pub pass_number: u32,
    pub timestamp: i64,
}

// ============================================================================
// Retrieval
// ============================================================================

/// Retrieval band filter for querying memory nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalBand {
    pub min_trust_layer: u32,
    pub max_trust_layer: u32,
    pub min_confidence: f64,
    pub kinds: Option<Vec<NodeKind>>,
    pub max_results: usize,
}

impl Default for RetrievalBand {
    fn default() -> Self {
        Self {
            min_trust_layer: 0,
            max_trust_layer: 3,
            min_confidence: 0.0,
            kinds: None,
            max_results: 100,
        }
    }
}

impl RetrievalBand {
    pub fn working_and_above() -> Self {
        Self {
            min_trust_layer: 2,
            max_trust_layer: 3,
            min_confidence: 0.5,
            kinds: None,
            max_results: 50,
        }
    }

    pub fn annealed_only() -> Self {
        Self {
            min_trust_layer: 3,
            max_trust_layer: 3,
            min_confidence: 0.8,
            kinds: None,
            max_results: 25,
        }
    }

    pub fn with_kinds(mut self, kinds: Vec<NodeKind>) -> Self {
        self.kinds = Some(kinds);
        self
    }
}

// ============================================================================
// PidTrustRecord & RevokedLogEntry
// ============================================================================

/// PID trust record for per-process authorization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PidTrustRecord {
    pub pid: i32,
    pub trust_layer: u32,
    pub use_count: u64,
    pub last_use: i64,
    pub auto_grant: bool,
    pub decay_interval: i64,
    pub decay_rate: f64,
}

impl Default for PidTrustRecord {
    fn default() -> Self {
        Self {
            pid: 0,
            trust_layer: 0,
            use_count: 0,
            last_use: 0,
            auto_grant: false,
            decay_interval: 3600,
            decay_rate: 0.02,
        }
    }
}

/// Record of a guided revocation exit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokedLogEntry {
    pub id: i64,
    pub pid: i32,
    pub command: String,
    pub revoked_at: i64,
    pub reason: String,
    pub old_layer: u32,
    pub new_layer: u32,
}

// ============================================================================
// TrustManager (DB-backed trust layer operations)
// ============================================================================

/// Manages trust layers and confidence scoring for memory nodes.
/// Trust layers define bands of confidence that determine auto-execute and review behavior.
pub struct TrustManager<'a> {
    db: &'a GuardDb,
}

impl<'a> TrustManager<'a> {
    pub fn new(db: &'a GuardDb) -> Self {
        Self { db }
    }

    /// List all configured trust layers.
    pub fn list_layers(&self) -> Result<Vec<TrustLayer>, GuardDbError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, min_confidence, max_confidence, auto_execute, requires_review, description FROM trust_layers ORDER BY id",
        )?;

        let layers: Vec<TrustLayer> = stmt
            .query_map([], |row| {
                Ok(TrustLayer {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    min_confidence: row.get(2)?,
                    max_confidence: row.get(3)?,
                    auto_execute: row.get(4)?,
                    requires_review: row.get(5)?,
                    description: row.get(6)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(layers)
    }

    /// Find the trust layer for a given confidence score.
    pub fn layer_for_score(
        &self,
        score: TrustScore,
    ) -> Result<Option<TrustLayer>, GuardDbError> {
        let layers = self.list_layers()?;
        for layer in &layers {
            if layer.contains_score(score) {
                return Ok(Some(layer.clone()));
            }
        }
        // If score exceeds all layers, return the highest layer
        layers
            .last()
            .cloned()
            .map(Some)
            .ok_or(GuardDbError::SchemaError(
                "No trust layers configured".into(),
            ))
    }

    /// Check if a node at a given trust layer can auto-execute.
    pub fn can_auto_execute(&self, trust_layer_id: u32) -> Result<bool, GuardDbError> {
        let conn = self.db.conn();
        let auto: bool = conn
            .query_row(
                "SELECT auto_execute FROM trust_layers WHERE id = ?1",
                params![trust_layer_id],
                |r| r.get(0),
            )
            .map_err(|_| GuardDbError::SchemaError("Trust layer not found".into()))?;
        Ok(auto)
    }

    /// Check if a node at a given trust layer requires human review.
    pub fn requires_review(&self, trust_layer_id: u32) -> Result<bool, GuardDbError> {
        let conn = self.db.conn();
        let review: bool = conn
            .query_row(
                "SELECT requires_review FROM trust_layers WHERE id = ?1",
                params![trust_layer_id],
                |r| r.get(0),
            )
            .map_err(|_| GuardDbError::SchemaError("Trust layer not found".into()))?;
        Ok(review)
    }

    /// Update a node's trust layer based on its current confidence score.
    /// Returns the old and new layer IDs.
    pub fn update_node_trust_layer(
        &self,
        node_id: &str,
        new_confidence: TrustScore,
    ) -> Result<Option<(u32, u32)>, GuardDbError> {
        // Compute new layer ID outside the lock to avoid deadlock
        let new_layer = self.layer_for_score(new_confidence)?.map(|l| l.id);

        let result = {
            let conn = self.db.conn();

            let old_layer: u32 = conn
                .query_row(
                    "SELECT trust_layer FROM memory_nodes WHERE id = ?1",
                    params![node_id],
                    |r| r.get(0),
                )
                .map_err(|_| GuardDbError::SchemaError("Node not found".into()))?;

            let final_layer = new_layer.unwrap_or(old_layer);

            conn.execute(
                "UPDATE memory_nodes SET confidence = ?1, trust_layer = ?2, last_touched = unixepoch() WHERE id = ?3",
                params![new_confidence.get(), final_layer, node_id],
            )?;

            (old_layer, final_layer)
        }; // lock dropped

        let (old_layer, new_layer) = result;
        if old_layer != new_layer {
            info!(
                node = node_id,
                old_layer = old_layer,
                new_layer = new_layer,
                confidence = new_confidence.get(),
                "Trust layer transition"
            );
            Ok(Some((old_layer, new_layer)))
        } else {
            Ok(None)
        }
    }

    /// Record a human review of a memory node.
    pub fn record_review(&self, record: &ReviewRecord) -> Result<(), GuardDbError> {
        let conn = self.db.conn();

        conn.execute(
            "INSERT INTO review_records (id, node_id, reviewer, action, old_confidence, new_confidence, old_trust_layer, new_trust_layer, notes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, unixepoch())",
            params![
                record.id,
                record.node_id,
                record.reviewer,
                format!("{}", record.action),
                record.old_confidence.map(|s| s.get()),
                record.new_confidence.map(|s| s.get()),
                record.old_trust_layer,
                record.new_trust_layer,
                &record.notes,
            ],
        )?;

        debug!(
            review_id = record.id,
            node = record.node_id,
            action = %record.action,
            "Review recorded"
        );

        Ok(())
    }

    /// Compute effective confidence for a node, factoring in supporting and contradicting evidence.
    pub fn effective_confidence(&self, node_id: &str) -> Result<TrustScore, GuardDbError> {
        let conn = self.db.conn();

        let base_confidence: f64 = conn
            .query_row(
                "SELECT confidence FROM memory_nodes WHERE id = ?1",
                params![node_id],
                |r| r.get(0),
            )
            .map_err(|_| GuardDbError::SchemaError("Node not found".into()))?;

        let support_sum: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(e.weight * mn.confidence), 0)
             FROM memory_edges e
             JOIN memory_nodes mn ON e.from_id = mn.id
             WHERE e.to_id = ?1 AND e.relation = 'supports'",
                params![node_id],
                |r| r.get(0),
            )
            .unwrap_or(0.0);

        let contradict_sum: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(e.weight * mn.confidence), 0)
             FROM memory_edges e
             JOIN memory_nodes mn ON e.from_id = mn.id
             WHERE e.to_id = ?1 AND e.relation = 'contradicts'",
                params![node_id],
                |r| r.get(0),
            )
            .unwrap_or(0.0);

        let total_evidence = support_sum + contradict_sum;
        let effective = if total_evidence > 0.0 {
            let evidence_ratio = support_sum / total_evidence;
            0.6 * base_confidence + 0.4 * evidence_ratio
        } else {
            base_confidence
        };

        Ok(TrustScore::new(effective))
    }

    /// Reinforce a node's confidence after a successful outcome.
    pub fn reinforce(&self, node_id: &str, delta: f64) -> Result<TrustScore, GuardDbError> {
        let new_score = {
            let conn = self.db.conn();

            let current: f64 = conn
                .query_row(
                    "SELECT confidence FROM memory_nodes WHERE id = ?1",
                    params![node_id],
                    |r| r.get(0),
                )
                .map_err(|_| GuardDbError::SchemaError("Node not found".into()))?;

            TrustScore::new(current + delta)
        }; // lock dropped

        debug!(
            node = node_id,
            new = new_score.get(),
            "Confidence reinforced"
        );
        let _ = self.update_node_trust_layer(node_id, new_score);
        Ok(new_score)
    }

    /// Get the guard DB connection.
    pub fn conn(&self) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
        self.db.conn()
    }

    /// Decay a node's confidence based on time and usage.
    pub fn decay(&self, node_id: &str, rate: f64) -> Result<TrustScore, GuardDbError> {
        let config = self.db.load_anneal_config()?;
        let new_score = {
            let conn = self.db.conn();

            let current: f64 = conn
                .query_row(
                    "SELECT confidence FROM memory_nodes WHERE id = ?1",
                    params![node_id],
                    |r| r.get(0),
                )
                .map_err(|_| GuardDbError::SchemaError("Node not found".into()))?;

            TrustScore::new((current - rate).max(config.confidence_floor))
        }; // lock dropped

        warn!(node = node_id, new = new_score.get(), "Confidence decayed");
        let _ = self.update_node_trust_layer(node_id, new_score);
        Ok(new_score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_score_new_clamps_to_one() {
        assert_eq!(TrustScore::new(1.5).get(), 1.0);
        assert_eq!(TrustScore::new(-0.5).get(), 0.0);
        assert_eq!(TrustScore::new(0.5).get(), 0.5);
    }

    #[test]
    fn trust_score_is_zero() {
        assert!(TrustScore::new(0.0).is_zero());
        assert!(!TrustScore::new(0.001).is_zero());
    }

    #[test]
    fn trust_score_default() {
        let score = TrustScore::default();
        assert_eq!(score.get(), 0.5);
    }

    #[test]
    fn trust_score_reinforce() {
        let base = TrustScore::new(0.5);
        let reinforced = base.reinforce(0.3);
        assert_eq!(reinforced.get(), 0.8);
    }

    #[test]
    fn trust_score_decay() {
        let base = TrustScore::new(0.5);
        let decayed = base.decay(0.2);
        assert_eq!(decayed.get(), 0.3);
    }

    #[test]
    fn trust_score_interpolate() {
        let a = TrustScore::new(0.0);
        let b = TrustScore::new(1.0);
        let blended = a.interpolate(&b, 0.5);
        assert_eq!(blended.get(), 0.5);
    }

    #[test]
    fn trust_score_display() {
        let score = TrustScore::new(0.1234);
        assert_eq!(format!("{}", score), "0.123");
    }

    #[test]
    fn trust_layer_contains_score() {
        let layer = TrustLayer {
            id: 1,
            name: "working".to_string(),
            min_confidence: 0.5,
            max_confidence: 0.9,
            auto_execute: false,
            requires_review: true,
            description: Some("test".to_string()),
        };
        assert!(layer.contains_score(TrustScore::new(0.6)));
        assert!(layer.contains_score(TrustScore::new(0.8)));
        assert!(!layer.contains_score(TrustScore::new(0.4)));
        assert!(!layer.contains_score(TrustScore::new(0.9)));
    }

    #[test]
    fn trust_layer_name() {
        let layer = TrustLayer {
            id: 1,
            name: "working".to_string(),
            min_confidence: 0.0,
            max_confidence: 1.0,
            auto_execute: false,
            requires_review: false,
            description: None,
        };
        assert_eq!(layer.name(), "working");
    }

    #[test]
    fn trust_layer_description() {
        let layer = TrustLayer {
            id: 1,
            name: "test".to_string(),
            min_confidence: 0.0,
            max_confidence: 1.0,
            auto_execute: false,
            requires_review: false,
            description: Some("desc".to_string()),
        };
        assert_eq!(layer.description(), Some("desc"));
    }

    #[test]
    fn review_action_display_approve() {
        assert_eq!(format!("{}", ReviewAction::Approve), "approve");
    }

    #[test]
    fn review_action_display_reject() {
        assert_eq!(format!("{}", ReviewAction::Reject), "reject");
    }

    #[test]
    fn review_action_display_modify() {
        assert_eq!(format!("{}", ReviewAction::Modify), "modify");
    }

    #[test]
    fn review_action_display_escalate() {
        assert_eq!(format!("{}", ReviewAction::Escalate), "escalate");
    }

    #[test]
    fn review_action_equality() {
        assert_eq!(ReviewAction::Approve, ReviewAction::Approve);
        assert_ne!(ReviewAction::Approve, ReviewAction::Reject);
    }

    #[test]
    fn anneal_config_default() {
        let config = AnnealConfig::default();
        assert_eq!(config.decay_rate, 0.02);
        assert_eq!(config.reinforce_threshold, 0.7);
        assert_eq!(config.anneal_interval_seconds, 3600);
        assert_eq!(config.max_anneal_passes, 10);
        assert_eq!(config.confidence_floor, 0.05);
        assert!(config.auto_anneal_enabled);
    }

    #[test]
    fn retrieval_band_default() {
        let band = RetrievalBand::default();
        assert_eq!(band.min_trust_layer, 0);
        assert_eq!(band.max_trust_layer, 3);
        assert_eq!(band.min_confidence, 0.0);
        assert!(band.kinds.is_none());
        assert_eq!(band.max_results, 100);
    }

    #[test]
    fn retrieval_band_working_and_above() {
        let band = RetrievalBand::working_and_above();
        assert_eq!(band.min_trust_layer, 2);
        assert_eq!(band.max_trust_layer, 3);
        assert_eq!(band.min_confidence, 0.5);
        assert_eq!(band.max_results, 50);
    }

    #[test]
    fn retrieval_band_annealed_only() {
        let band = RetrievalBand::annealed_only();
        assert_eq!(band.min_trust_layer, 3);
        assert_eq!(band.max_trust_layer, 3);
        assert_eq!(band.min_confidence, 0.8);
        assert_eq!(band.max_results, 25);
    }

    #[test]
    fn retrieval_band_with_kinds() {
        let band = RetrievalBand::default()
            .with_kinds(vec![NodeKind::Fact, NodeKind::Rule]);
        assert!(band.kinds.is_some());
        assert_eq!(band.kinds.as_ref().unwrap().len(), 2);
    }
}
