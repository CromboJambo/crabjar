//! Arbiter core types — evidence accumulation and belief-revision model.
//!
//! The arbiter is not a one-shot classifier. It runs multiple evaluation passes,
//! each contributing a confidence delta. Confidence accumulates until it crosses
//! an approval/denial threshold, or the arbiter exhausts its passes and escalates
//! with a structured excuse explaining why it couldn't decide.
//!
//! This mirrors how human judgment actually works: we don't have fully-defined
//! criteria for every decision. We accumulate evidence from different angles,
//! some fast (pattern recognition), some slow (semantic analysis), and decide
//! when we're confident enough — or admit we can't and escalate.

use crate::action::{Denied, GuardedAction, TrustApproved};
use serde::{Deserialize, Serialize};
use std::fmt;

// ─── Evidence ────────────────────────────────────────────────────────────────

/// A single piece of evidence gathered by an evaluation pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Which pass produced this evidence.
    pub source: String,
    /// Pass number (1-indexed) when this was gathered.
    pub pass_number: u32,
    /// The confidence delta this evidence contributed (+ favors approval, - favors denial).
    pub delta: f64,
    /// Human-readable explanation of what was observed.
    pub rationale: String,
}

impl Evidence {
    pub fn new(source: impl Into<String>, pass_number: u32, delta: f64, rationale: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            pass_number,
            delta,
            rationale: rationale.into(),
        }
    }

    /// Returns true if this evidence favors approval.
    pub fn is_pro(&self) -> bool {
        self.delta > 0.0
    }

    /// Returns true if this evidence favors denial.
    pub fn is_con(&self) -> bool {
        self.delta < 0.0
    }
}

// ─── Arbitration Step ────────────────────────────────────────────────────────

/// The result of a single evaluation pass within one arbitration round.
pub enum ArbitrationStep {
    /// Continue accumulating; this pass contributed the given delta and evidence.
    Continue(f64, Evidence),
    /// This pass alone is decisive: approve with the given evidence.
    Approve(Evidence),
    /// This pass alone is decisive: deny with the given evidence.
    Deny(Evidence),
    /// This pass cannot evaluate and explains why (structured excuse).
    Excuse(ArbitrationExcuse),
}

impl fmt::Debug for ArbitrationStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Continue(delta, _) => write!(f, "Continue({delta})"),
            Self::Approve(e) => write!(f, "Approve(pass={}, delta={})", e.source, e.delta),
            Self::Deny(e) => write!(f, "Deny(pass={}, delta={})", e.source, e.delta),
            Self::Excuse(e) => write!(f, "Excuse({e:?})"),
        }
    }
}

// ─── Arbitration Excuse ──────────────────────────────────────────────────────

/// Structured reason why the arbiter (or a pass) couldn't reach a conclusion.
///
/// An excuse is not an error — it's graceful acknowledgment that the available
/// evidence is insufficient or contradictory. The excuse is attached to the
/// decision record so a human reviewer sees *why* escalation happened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArbitrationExcuse {
    /// Evidence points in conflicting directions; no clear winner.
    ContradictoryEvidence {
        pro: Vec<Evidence>,
        con: Vec<Evidence>,
    },
    /// Required context is missing for a confident decision.
    MissingContext {
        required: String,
        available: String,
    },
    /// The action's scope doesn't match what the arbiter can evaluate.
    ScopeViolation {
        expected: String,
        found: String,
    },
    /// All passes completed but confidence never crossed a threshold.
    MaxPassesExceeded {
        final_confidence: f64,
        passes_completed: u32,
    },
}

impl fmt::Display for ArbitrationExcuse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContradictoryEvidence { pro, con } => write!(
                f,
                "Contradictory evidence: {} pro vs {} con",
                pro.len(),
                con.len()
            ),
            Self::MissingContext { required, available } => {
                write!(f, "Missing context: need '{required}', have '{available}'")
            }
            Self::ScopeViolation { expected, found } => {
                write!(f, "Scope violation: expected '{expected}', found '{found}'")
            }
            Self::MaxPassesExceeded { final_confidence, passes_completed } => write!(
                f,
                "Confidence {final_confidence:.3} after {passes_completed} passes — threshold not reached"
            ),
        }
    }
}

// ─── Arbiter State ───────────────────────────────────────────────────────────

/// Accumulated state across arbitration passes.
///
/// Each pass sees the full history of previous passes' evidence, enabling
/// belief revision: a later pass can see that an earlier pass was wrong and
/// contribute a corrective delta.
#[derive(Debug, Clone, Default)]
pub struct ArbiterState {
    /// Running confidence score (sum of all deltas so far).
    pub confidence: f64,
    /// All evidence gathered across all passes.
    pub evidence: Vec<Evidence>,
    /// Number of passes completed.
    pub passes_completed: u32,
}

impl ArbiterState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a pass's contribution and update running confidence.
    pub fn record_pass(&mut self, delta: f64, evidence: Vec<Evidence>) {
        self.confidence += delta;
        self.passes_completed += 1;
        self.evidence.extend(evidence);
    }

    /// Returns evidence favoring approval.
    pub fn pro_evidence(&self) -> Vec<&Evidence> {
        self.evidence.iter().filter(|e| e.is_pro()).collect()
    }

    /// Returns evidence favoring denial.
    pub fn con_evidence(&self) -> Vec<&Evidence> {
        self.evidence.iter().filter(|e| e.is_con()).collect()
    }

    /// Returns true if confidence has saturated in either direction.
    pub fn is_saturated(&self, approve_threshold: f64, deny_threshold: f64) -> bool {
        self.confidence >= approve_threshold || self.confidence <= deny_threshold
    }
}

// ─── Arbitration Result ──────────────────────────────────────────────────────

/// Final result of an arbitration run.
pub enum ArbitrationResult {
    /// Confidence crossed the approval threshold; action is approved.
    Approved(GuardedAction<TrustApproved>, f64, Vec<Evidence>),
    /// Confidence crossed the denial threshold; action is denied.
    Denied(GuardedAction<Denied>, f64, Vec<Evidence>),
    /// Arbiter couldn't reach a confident decision; escalate to human with excuse.
    Escalated(ArbitrationExcuse, Vec<Evidence>),
}

impl ArbitrationResult {
    /// Returns true if the action was approved by the arbiter.
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved(_, _, _))
    }

    /// Returns true if the action was denied by the arbiter.
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Denied(_, _, _))
    }

    /// Returns true if the arbiter escalated to human review.
    pub fn is_escalated(&self) -> bool {
        matches!(self, Self::Escalated(_, _))
    }

    /// Returns all evidence gathered during arbitration (for any outcome).
    pub fn evidence(&self) -> &[Evidence] {
        match self {
            Self::Approved(_, _, e) | Self::Denied(_, _, e) | Self::Escalated(_, e) => e,
        }
    }

    /// Returns the final confidence score (for approved/denied outcomes).
    pub fn confidence(&self) -> Option<f64> {
        match self {
            Self::Approved(_, c, _) | Self::Denied(_, c, _) => Some(*c),
            Self::Escalated(ArbitrationExcuse::MaxPassesExceeded { final_confidence, .. }, _) => {
                Some(*final_confidence)
            }
            _ => None,
        }
    }

    /// Returns the excuse if this was an escalation.
    pub fn excuse(&self) -> Option<&ArbitrationExcuse> {
        match self {
            Self::Escalated(e, _) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Debug for ArbitrationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Approved(a, conf, ev) => f
                .debug_struct("ArbitrationResult::Approved")
                .field("action_id", &a.as_ref().id)
                .field("confidence", conf)
                .field("evidence_count", &ev.len())
                .finish(),
            Self::Denied(a, conf, ev) => f
                .debug_struct("ArbitrationResult::Denied")
                .field("action_id", &a.as_ref().id)
                .field("confidence", conf)
                .field("evidence_count", &ev.len())
                .finish(),
            Self::Escalated(excuse, ev) => f
                .debug_struct("ArbitrationResult::Escalated")
                .field("excuse", excuse)
                .field("evidence_count", &ev.len())
                .finish(),
        }
    }
}

// ─── Decision Record ─────────────────────────────────────────────────────────

/// Persistent record of an arbitration decision, stored alongside the action.
///
/// Follows crabjar's "every abstraction carries doubt" principle: includes
/// assumptions, blind spots, and staleness metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrationRecord {
    pub action_id: String,
    pub result: String, // "approved" | "denied" | "escalated"
    pub final_confidence: Option<f64>,
    pub passes_completed: u32,
    pub evidence: Vec<Evidence>,
    pub excuse: Option<ArbitrationExcuse>,
    /// Assumptions the arbiter made during evaluation.
    pub assumptions: Vec<String>,
    /// Known blind spots — things the arbiter didn't evaluate.
    pub blind_spots: Vec<String>,
    pub decided_at: i64,
}

impl ArbitrationRecord {
    pub fn from_result(action_id: impl Into<String>, result: &ArbitrationResult) -> Self {
        let (result_str, confidence, excuse) = match result {
            ArbitrationResult::Approved(_, c, _) => ("approved", Some(*c), None),
            ArbitrationResult::Denied(_, c, _) => ("denied", Some(*c), None),
            ArbitrationResult::Escalated(e, _) => ("escalated", None, Some(e.clone())),
        };

        Self {
            action_id: action_id.into(),
            result: result_str.to_string(),
            final_confidence: confidence,
            passes_completed: 0, // filled in by arbiter
            evidence: result.evidence().to_vec(),
            excuse,
            assumptions: vec![
                "Evaluation passes ran in registered order".into(),
                "Confidence thresholds are static for this run".into(),
            ],
            blind_spots: vec!["Cross-session context not evaluated".into()],
            decided_at: chrono::Utc::now().timestamp(),
        }
    }
}
