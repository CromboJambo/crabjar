//! The JEV Arbiter — belief-revision decision engine for queued actions.
//!
//! This is the "flexibility layer" that sits above the deterministic gate.
//! When `GateConcierge::enforce()` produces a `Queued` outcome, the arbiter
//! takes over and runs multiple evaluation passes, accumulating confidence
//! until it either:
//! - Crosses the approval threshold → auto-approve with recorded judgment
//! - Crosses the denial threshold → auto-deny with recorded reasoning
//! - Exhausts its passes without saturation → escalate to human with a
//!   structured excuse explaining why it couldn't decide
//!
//! The key insight: human decisions aren't based on fully-defined criteria.
//! We accumulate evidence from different angles (some fast, some slow) and
//! decide when we're confident enough. The arbiter formalizes this process
//! while keeping the deterministic layer fast and type-safe.

use crate::action::{GuardedAction, Pending};
use crate::arbiter::pass::EvaluationPass;
use crate::arbiter::types::{
    ArbitrationExcuse, ArbitrationRecord, ArbitrationResult, ArbitrationStep, ArbiterState, Evidence,
};
use std::sync::Arc;

// ─── JevArbiter ──────────────────────────────────────────────────────────────

/// The arbiter that runs evaluation passes and accumulates confidence.
///
/// # Design Principles
/// - **Implicit assumptions**: Each pass encodes an assumption about what makes
///   an action safe/risky. The arbiter doesn't need to enumerate all criteria —
///   passes contribute their votes and the aggregate decides.
/// - **No throttling**: Fast passes (pattern matching) don't wait for slow ones
///   (semantic analysis). Within a single arbitration, passes run sequentially
///   in registered order; async multiplexing happens at the orchestrator layer.
/// - **Structured excuses**: When the arbiter can't decide, it explains why.
///   The excuse is attached to the decision record for human review.
/// - **Typestate safety**: The arbiter operates on `GuardedAction<Pending>` and
///   can only produce `TrustApproved` or `Denied` transitions through the
///   typestate methods. Invalid states are unrepresentable at compile time.
pub struct JevArbiter {
    /// Confidence threshold for auto-approval (e.g., 0.85).
    pub approve_threshold: f64,
    /// Confidence threshold for auto-denial (e.g., -0.85).
    pub deny_threshold: f64,
    /// Maximum number of passes before escalating to human.
    pub max_passes: u32,
    /// Registered evaluation passes, run in order.
    passes: Vec<Arc<dyn EvaluationPass>>,
}

impl Default for JevArbiter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for JevArbiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JevArbiter")
            .field("approve_threshold", &self.approve_threshold)
            .field("deny_threshold", &self.deny_threshold)
            .field("max_passes", &self.max_passes)
            .field(
                "passes",
                &self
                    .passes
                    .iter()
                    .map(|p| p.name().to_string())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl JevArbiter {
    /// Create a new arbiter with default thresholds and no passes.
    pub fn new() -> Self {
        Self {
            approve_threshold: 0.7,
            deny_threshold: -0.7,
            max_passes: 5,
            passes: Vec::new(),
        }
    }

    /// Set the approval confidence threshold.
    pub fn with_approve_threshold(mut self, threshold: f64) -> Self {
        self.approve_threshold = threshold;
        self
    }

    /// Set the denial confidence threshold.
    pub fn with_deny_threshold(mut self, threshold: f64) -> Self {
        self.deny_threshold = threshold;
        self
    }

    /// Set the maximum number of passes before escalation.
    pub fn with_max_passes(mut self, max: u32) -> Self {
        self.max_passes = max;
        self
    }

    /// Register an evaluation pass. Passes run in registration order.
    pub fn register_pass(mut self, pass: Arc<dyn EvaluationPass>) -> Self {
        self.passes.push(pass);
        self
    }

    /// Create an arbiter with the standard built-in passes registered.
    pub fn with_standard_passes() -> Self {
        use crate::arbiter::pass::{CommandRiskPass, ConfidencePass, HistoryPass, ProvenancePass, TrustLayerPass};

        Self::new().register_pass(Arc::new(CommandRiskPass)).register_pass(Arc::new(TrustLayerPass)).register_pass(Arc::new(ConfidencePass)).register_pass(Arc::new(ProvenancePass)).register_pass(Arc::new(HistoryPass::default()))
    }

    /// Run the arbitration loop on a queued action.
    ///
    /// # Process
    /// 1. Start with confidence = 0.0
    /// 2. For each pass (up to `max_passes`):
    ///    a. Run the pass, get its delta and evidence
    ///    b. Add delta to running confidence
    ///    c. Check if confidence crossed a threshold → terminate early
    /// 3. If all passes complete without saturation → escalate with excuse
    ///
    /// # Returns
    /// - `ArbitrationResult::Approved` — confidence ≥ approve_threshold
    /// - `ArbitrationResult::Denied` — confidence ≤ deny_threshold
    /// - `ArbitrationResult::Escalated` — max passes reached without saturation
    pub fn arbitrate(&self, action: GuardedAction<Pending>) -> ArbitrationResult {
        if self.passes.is_empty() {
            // No passes registered — can't evaluate, escalate immediately
            let excuse = ArbitrationExcuse::MissingContext {
                required: "At least one evaluation pass".to_string(),
                available: "No passes registered".to_string(),
            };
            return ArbitrationResult::Escalated(excuse, Vec::new());
        }

        let mut state = ArbiterState::new();

        for (i, pass) in self.passes.iter().enumerate() {
            let pass_num = (i % self.max_passes as usize) + 1;

            // Run the pass
            let step = match pass.evaluate(&action, &state) {
                Ok(step) => step,
                Err(e) => {
                    // Pass failed unexpectedly — record it and continue with other passes
                    tracing::warn!("Arbiter pass '{}' failed: {}", pass.name(), e);
                    state.record_pass(0.0, vec![Evidence::new(
                        pass.name(),
                        pass_num as u32,
                        0.0,
                        format!("Pass failed: {e}"),
                    )]);
                    continue;
                }
            };

            // Handle the step result
            match step {
                ArbitrationStep::Continue(delta, evidence) => {
                    state.record_pass(delta, vec![evidence]);
                }
                ArbitrationStep::Approve(evidence) => {
                    // This pass alone is decisive for approval
                    state.record_pass(evidence.delta, vec![evidence]);
                    return self.finalize_approved(action, &state);
                }
                ArbitrationStep::Deny(evidence) => {
                    // This pass alone is decisive for denial
                    state.record_pass(evidence.delta, vec![evidence]);
                    return self.finalize_denied(action, &state);
                }
                ArbitrationStep::Excuse(excuse) => {
                    // This pass can't evaluate — record the excuse and continue
                    tracing::debug!("Arbiter pass '{}' excused: {}", pass.name(), excuse);
                    state.record_pass(0.0, Vec::new());
                    // If this is the only pass, escalate immediately
                    if self.passes.len() == 1 {
                        return ArbitrationResult::Escalated(excuse, state.evidence.clone());
                    }
                }
            }

            // Check for saturation after each pass
            if state.confidence >= self.approve_threshold {
                return self.finalize_approved(action, &state);
            }
            if state.confidence <= self.deny_threshold {
                return self.finalize_denied(action, &state);
            }

            // Safety cap: stop after max_passes even if more passes are registered
            if state.passes_completed >= self.max_passes {
                break;
            }
        }

        // Exhausted passes without saturation — escalate with structured excuse
        let excuse = ArbitrationExcuse::MaxPassesExceeded {
            final_confidence: state.confidence,
            passes_completed: state.passes_completed,
        };
        tracing::info!(
            "Arbiter escalated after {} passes (confidence {:.3}): {}",
            state.passes_completed,
            state.confidence,
            excuse
        );
        ArbitrationResult::Escalated(excuse, state.evidence.clone())
    }

    /// Finalize an approved decision, transitioning the action's typestate.
    fn finalize_approved(
        &self,
        action: GuardedAction<Pending>,
        state: &ArbiterState,
    ) -> ArbitrationResult {
        let confidence = state.confidence;

        // Transition Pending → TrustApproved via typestate (consuming)
        let approved = action.approve();

        tracing::info!(
            action_id = %approved.as_ref().id,
            confidence = confidence,
            passes = state.passes_completed,
            "Arbiter approved action"
        );

        ArbitrationResult::Approved(approved, confidence, state.evidence.clone())
    }

    /// Finalize a denied decision, transitioning the action's typestate.
    fn finalize_denied(
        &self,
        action: GuardedAction<Pending>,
        state: &ArbiterState,
    ) -> ArbitrationResult {
        let confidence = state.confidence;

        // Transition Pending → Denied via typestate (consuming)
        let denied = action.deny();

        tracing::info!(
            action_id = %denied.as_ref().id,
            confidence = confidence,
            passes = state.passes_completed,
            "Arbiter denied action"
        );

        ArbitrationResult::Denied(denied, confidence, state.evidence.clone())
    }

    /// Create an arbitration record for persistence.
    pub fn create_record(&self, action_id: &str, result: &ArbitrationResult) -> ArbitrationRecord {
        let mut record = ArbitrationRecord::from_result(action_id, result);
        record.passes_completed = match result {
            ArbitrationResult::Approved(_, _, ev) | ArbitrationResult::Denied(_, _, ev) => {
                ev.iter().map(|e| e.pass_number).max().unwrap_or(0)
            }
            ArbitrationResult::Escalated(ArbitrationExcuse::MaxPassesExceeded { passes_completed, .. }, _) => {
                *passes_completed
            }
            _ => 0,
        };
        record
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ActionRequest;
    use crate::arbiter::pass::{CommandRiskPass, ConfidencePass, ProvenancePass, TrustLayerPass};
    use crate::trust_types::TrustScore;

    fn make_action(payload: &str, trust_layer: u32, confidence: f64) -> GuardedAction<Pending> {
        let request = ActionRequest::new("test", "execute", payload, trust_layer, TrustScore::new(confidence));
        GuardedAction::new(request)
    }

    #[test]
    fn arbiter_no_passes_escalates() {
        let arbiter = JevArbiter::new();
        let action = make_action("echo hello", 3, 0.8);

        match arbiter.arbitrate(action) {
            ArbitrationResult::Escalated(excuse, _) => {
                assert!(matches!(excuse, ArbitrationExcuse::MissingContext { .. }));
            }
            other => panic!("Expected Escalated, got {:?}", other),
        }
    }

    #[test]
    fn arbiter_high_risk_command_denies() {
        let arbiter = JevArbiter::new().register_pass(Arc::new(CommandRiskPass));
        let action = make_action("rm -rf /tmp/test", 3, 0.8);

        match arbiter.arbitrate(action) {
            ArbitrationResult::Denied(_, confidence, evidence) => {
                assert!(confidence < 0.0);
                assert!(!evidence.is_empty());
            }
            other => panic!("Expected Denied, got {:?}", other),
        }
    }

    #[test]
    fn arbiter_low_risk_high_trust_approves() {
        let arbiter = JevArbiter::new()
            .with_approve_threshold(0.5)
            .register_pass(Arc::new(CommandRiskPass))
            .register_pass(Arc::new(TrustLayerPass))
            .register_pass(Arc::new(ConfidencePass));

        let action = make_action("echo hello", 5, 0.95);

        match arbiter.arbitrate(action) {
            ArbitrationResult::Approved(_, confidence, evidence) => {
                assert!(confidence >= 0.5);
                assert!(!evidence.is_empty());
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }

    #[test]
    fn arbiter_mixed_signals_escalates() {
        // Low trust + no provenance + medium confidence = mixed signals
        let arbiter = JevArbiter::new()
            .with_approve_threshold(0.8)
            .with_deny_threshold(-0.8)
            .register_pass(Arc::new(CommandRiskPass))
            .register_pass(Arc::new(TrustLayerPass))
            .register_pass(Arc::new(ConfidencePass))
            .register_pass(Arc::new(ProvenancePass));

        let action = make_action("echo hello", 1, 0.5);

        match arbiter.arbitrate(action) {
            ArbitrationResult::Escalated(excuse, _) => {
                assert!(matches!(excuse, ArbitrationExcuse::MaxPassesExceeded { .. }));
            }
            other => panic!("Expected Escalated, got {:?}", other),
        }
    }

    #[test]
    fn arbiter_record_creation() {
        let arbiter = JevArbiter::with_standard_passes();
        let action = make_action("echo hello", 3, 0.8);
        let result = arbiter.arbitrate(action);
        let record = arbiter.create_record("action-123", &result);

        assert_eq!(record.action_id, "action-123");
        assert!(!record.result.is_empty());
    }

    #[test]
    fn arbiter_respects_max_passes() {
        // Register more passes than max_passes allows
        let arbiter = JevArbiter::new()
            .with_max_passes(2)
            .register_pass(Arc::new(CommandRiskPass))
            .register_pass(Arc::new(TrustLayerPass))
            .register_pass(Arc::new(ConfidencePass))
            .register_pass(Arc::new(ProvenancePass));

        let action = make_action("echo hello", 3, 0.5);
        let result = arbiter.arbitrate(action);

        // Should only have run 2 passes
        if let ArbitrationResult::Escalated(
            ArbitrationExcuse::MaxPassesExceeded {
                passes_completed, ..
            },
            _,
        ) = &result
        {
            assert!(*passes_completed <= 2);
        }
    }

    #[test]
    fn arbiter_standard_passes_work() {
        let arbiter = JevArbiter::with_standard_passes();
        let action = make_action("echo hello", 3, 0.8);
        let result = arbiter.arbitrate(action);

        // Should complete without panicking
        assert!(result.is_approved() || result.is_denied() || result.is_escalated());
    }
}
