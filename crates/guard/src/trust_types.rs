//! Trust-related types for the guard system.
//!
//! Types: `TrustScore`, `TrustLayer`, `ReviewAction`, `ReviewRecord`,
//! `AnnealConfig`, `AnnealResult`, `RetrievalBand`, `PidTrustRecord`, `RevokedLogEntry`.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::memory_types::NodeKind;

// ============================================================================
// TrustScore
// ============================================================================

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
