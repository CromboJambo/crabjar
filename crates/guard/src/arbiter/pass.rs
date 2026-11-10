//! Evaluation passes — the "subprocesses" that contribute evidence to arbitration.
//!
//! Each pass is an independent evaluator with its own assumptions and speed profile.
//! The arbiter runs passes in order, accumulating their confidence deltas until
//! saturation or exhaustion. Passes can be fast (pattern matching) or slow
//! (semantic analysis), but within a single arbitration they run sequentially —
//! async multiplexing happens at the orchestrator layer.

#[cfg(test)]
use crate::action::ActionRequest;
use crate::action::{GuardedAction, Pending};
use crate::arbiter::types::{ArbiterState, ArbitrationStep, Evidence};
use crate::command_risk::{CommandRisk, HIGH_RISK_COMMANDS, MEDIUM_RISK_COMMANDS};
use std::fmt;

// ─── Evaluation Pass Trait ───────────────────────────────────────────────────

/// A single evaluation pass within the arbiter's belief-revision loop.
///
/// Each pass examines one aspect of the action and contributes a confidence delta:
/// - Positive delta favors approval
/// - Negative delta favors denial
/// - Zero delta means "no signal" (pass abstains)
///
/// Passes are stateless with respect to each other — they receive the accumulated
/// [`ArbiterState`] as read-only context, enabling belief revision without shared mutation.
pub trait EvaluationPass: fmt::Debug + Send + Sync {
    /// Human-readable name for this pass (used in evidence records).
    fn name(&self) -> &str;

    /// Evaluate the action and return a confidence delta with supporting evidence.
    ///
    /// # Returns
    /// - `Ok(ArbitrationStep::Continue(delta))` — contributed delta, keep going
    /// - `Ok(ArbitrationStep::Approve(evidence))` — this pass alone is decisive for approval
    /// - `Ok(ArbitrationStep::Deny(evidence))` — this pass alone is decisive for denial
    /// - `Ok(ArbitrationStep::Excuse(excuse))` — can't evaluate, here's why
    /// - `Err(e)` — unexpected error (should be rare; passes should prefer Excuse)
    fn evaluate(
        &self,
        action: &GuardedAction<Pending>,
        state: &ArbiterState,
    ) -> Result<ArbitrationStep, PassError>;
}

/// Error from an evaluation pass. Distinct from [`ArbitrationExcuse`] — this is
/// a genuine failure (bug, I/O error), not "I looked and couldn't decide."
#[derive(Debug, thiserror::Error)]
pub enum PassError {
    #[error("Pass '{name}' failed: {detail}")]
    Failed { name: String, detail: String },

    #[error("Pass '{name}' timed out")]
    Timeout { name: String },
}

// ─── Built-in Passes ─────────────────────────────────────────────────────────

/// Fast pattern-matching pass against known command risk lists.
///
/// This is the "juggling with 2 hands" pass — once you know the patterns,
/// it's fast and automatic. High-risk commands get a strong negative delta;
/// low-risk/known-safe commands get a positive delta.
#[derive(Debug)]
pub struct CommandRiskPass;

impl Default for CommandRiskPass {
    fn default() -> Self {
        Self
    }
}

impl EvaluationPass for CommandRiskPass {
    fn name(&self) -> &str {
        "command_risk"
    }

    fn evaluate(
        &self,
        action: &GuardedAction<Pending>,
        _state: &ArbiterState,
    ) -> Result<ArbitrationStep, PassError> {
        let request = action.as_ref();
        let command = extract_command(&request.payload);

        let risk = assess_command_risk(&command);

        match risk {
            CommandRisk::High | CommandRisk::Unauthorized => {
                let evidence = Evidence::new(
                    self.name(),
                    1,
                    -0.6,
                    format!(
                        "Command '{command}' matches {}-risk pattern",
                        if risk == CommandRisk::Unauthorized {
                            "unauthorized"
                        } else {
                            "high"
                        }
                    ),
                );
                Ok(ArbitrationStep::Deny(evidence))
            }
            CommandRisk::Medium => {
                let evidence = Evidence::new(
                    self.name(),
                    1,
                    -0.2,
                    format!("Command '{command}' matches medium-risk pattern"),
                );
                Ok(ArbitrationStep::Continue(evidence.delta, evidence))
            }
            CommandRisk::Low => {
                let evidence = Evidence::new(
                    self.name(),
                    1,
                    0.3,
                    format!("Command '{command}' is low-risk"),
                );
                Ok(ArbitrationStep::Continue(evidence.delta, evidence))
            }
        }
    }
}

/// Trust layer pass — higher trust layers get more benefit of the doubt.
#[derive(Debug)]
pub struct TrustLayerPass;

impl Default for TrustLayerPass {
    fn default() -> Self {
        Self
    }
}

impl EvaluationPass for TrustLayerPass {
    fn name(&self) -> &str {
        "trust_layer"
    }

    fn evaluate(
        &self,
        action: &GuardedAction<Pending>,
        _state: &ArbiterState,
    ) -> Result<ArbitrationStep, PassError> {
        let request = action.as_ref();
        let layer = request.trust_layer;

        // Trust layers 1-5, where 5 is highest trust
        let delta = match layer {
            0..=1 => -0.1, // Low trust: slight penalty
            2 => 0.0,      // Neutral
            3 => 0.1,      // Moderate trust
            4 => 0.2,      // High trust
            _ => 0.3,      // Very high trust
        };

        let evidence = Evidence::new(
            self.name(),
            1,
            delta,
            format!("Trust layer {layer} (higher = more trusted)"),
        );
        Ok(ArbitrationStep::Continue(evidence.delta, evidence))
    }
}

/// Confidence score pass — uses the action's existing confidence value.
#[derive(Debug)]
pub struct ConfidencePass;

impl Default for ConfidencePass {
    fn default() -> Self {
        Self
    }
}

impl EvaluationPass for ConfidencePass {
    fn name(&self) -> &str {
        "confidence"
    }

    fn evaluate(
        &self,
        action: &GuardedAction<Pending>,
        _state: &ArbiterState,
    ) -> Result<ArbitrationStep, PassError> {
        let request = action.as_ref();
        let confidence = request.confidence.get();

        // Map [0.0, 1.0] confidence to [-0.3, +0.3] delta
        // Below 0.5 is slightly negative, above 0.5 is slightly positive
        let delta = (confidence - 0.5) * 0.6;

        let evidence = Evidence::new(
            self.name(),
            1,
            delta,
            format!("Action confidence {confidence:.2}"),
        );
        Ok(ArbitrationStep::Continue(evidence.delta, evidence))
    }
}

/// Provenance pass — checks if the action has proper source attribution.
/// Actions without provenance get a penalty (detection != authorization).
#[derive(Debug)]
pub struct ProvenancePass;

impl Default for ProvenancePass {
    fn default() -> Self {
        Self
    }
}

impl EvaluationPass for ProvenancePass {
    fn name(&self) -> &str {
        "provenance"
    }

    fn evaluate(
        &self,
        action: &GuardedAction<Pending>,
        _state: &ArbiterState,
    ) -> Result<ArbitrationStep, PassError> {
        let request = action.as_ref();

        match (&request.source_event_id, &request.source_node_id) {
            (Some(_), Some(_)) => {
                // Full provenance: event + node
                let evidence = Evidence::new(
                    self.name(),
                    1,
                    0.2,
                    "Full provenance: source event and memory node present",
                );
                Ok(ArbitrationStep::Continue(evidence.delta, evidence))
            }
            (Some(_), None) | (None, Some(_)) => {
                // Partial provenance
                let evidence = Evidence::new(
                    self.name(),
                    1,
                    0.05,
                    "Partial provenance: only one source reference present",
                );
                Ok(ArbitrationStep::Continue(evidence.delta, evidence))
            }
            (None, None) => {
                // No provenance at all — this is a red flag
                let evidence = Evidence::new(
                    self.name(),
                    1,
                    -0.3,
                    "No provenance: action has no source event or memory node",
                );
                Ok(ArbitrationStep::Continue(evidence.delta, evidence))
            }
        }
    }
}

/// History pass — compares against past decisions for similar actions.
/// (Stub implementation; full version would query GuardDb.)
#[derive(Debug, Default)]
pub struct HistoryPass {
    /// Pre-loaded similarity scores from past decisions (for testing).
    /// In production, this would be populated by querying GuardDb.
    pub similar_approved: usize,
    pub similar_denied: usize,
}

impl EvaluationPass for HistoryPass {
    fn name(&self) -> &str {
        "history"
    }

    fn evaluate(
        &self,
        _action: &GuardedAction<Pending>,
        _state: &ArbiterState,
    ) -> Result<ArbitrationStep, PassError> {
        let total = self.similar_approved + self.similar_denied;

        if total == 0 {
            // No history to compare against — abstain with zero delta and neutral evidence
            let evidence = Evidence::new(
                self.name(),
                1,
                0.0,
                "No historical data available for comparison",
            );
            return Ok(ArbitrationStep::Continue(evidence.delta, evidence));
        }

        let approval_ratio = self.similar_approved as f64 / total as f64;

        // If 80%+ of similar actions were approved, slight positive signal
        // If 80%+ were denied, slight negative signal
        let delta = if approval_ratio >= 0.8 {
            0.15
        } else if approval_ratio <= 0.2 {
            -0.15
        } else {
            0.0 // Mixed history — no clear signal
        };

        let evidence = Evidence::new(
            self.name(),
            1,
            delta,
            format!(
                "History: {}/{} similar actions approved",
                self.similar_approved, total
            ),
        );
        Ok(ArbitrationStep::Continue(evidence.delta, evidence))
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Extract the command name from a payload string.
fn extract_command(payload: &str) -> String {
    payload.split_whitespace().next().unwrap_or("").to_string()
}

/// Assess command risk using the existing risk lists.
fn assess_command_risk(command: &str) -> CommandRisk {
    let cmd_lower = command.to_lowercase();

    if HIGH_RISK_COMMANDS.iter().any(|c| c == &cmd_lower) {
        return CommandRisk::High;
    }
    if MEDIUM_RISK_COMMANDS.iter().any(|c| c == &cmd_lower) {
        return CommandRisk::Medium;
    }
    CommandRisk::Low
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust_types::TrustScore;

    fn make_action(payload: &str, trust_layer: u32, confidence: f64) -> GuardedAction<Pending> {
        let request = ActionRequest::new(
            "test",
            "execute",
            payload,
            trust_layer,
            TrustScore::new(confidence),
        );
        GuardedAction::new(request)
    }

    #[test]
    fn command_risk_pass_high_risk_denies() {
        let pass = CommandRiskPass;
        let action = make_action("rm -rf /tmp/test", 3, 0.8);
        let state = ArbiterState::new();

        match pass.evaluate(&action, &state).unwrap() {
            ArbitrationStep::Deny(e) => {
                assert!(e.delta < 0.0);
                assert!(e.rationale.contains("high-risk"));
            }
            other => panic!("Expected Deny, got {:?}", other),
        }
    }

    #[test]
    fn command_risk_pass_low_risk_continues() {
        let pass = CommandRiskPass;
        let action = make_action("echo hello", 3, 0.8);
        let state = ArbiterState::new();

        match pass.evaluate(&action, &state).unwrap() {
            ArbitrationStep::Continue(delta, _) => {
                assert!(delta > 0.0);
            }
            other => panic!("Expected Continue, got {:?}", other),
        }
    }

    #[test]
    fn trust_layer_pass_high_trust_positive() {
        let pass = TrustLayerPass;
        let action = make_action("echo hello", 5, 0.8);
        let state = ArbiterState::new();

        match pass.evaluate(&action, &state).unwrap() {
            ArbitrationStep::Continue(delta, _) => {
                assert!(delta > 0.0);
            }
            other => panic!("Expected Continue, got {:?}", other),
        }
    }

    #[test]
    fn confidence_pass_above_half_positive() {
        let pass = ConfidencePass;
        let action = make_action("echo hello", 3, 0.9);
        let state = ArbiterState::new();

        match pass.evaluate(&action, &state).unwrap() {
            ArbitrationStep::Continue(delta, _) => {
                assert!(delta > 0.0);
            }
            other => panic!("Expected Continue, got {:?}", other),
        }
    }

    #[test]
    fn provenance_pass_full_provenance_positive() {
        let pass = ProvenancePass;
        let request = ActionRequest::new("test", "execute", "echo", 3, TrustScore::new(0.8))
            .with_source("event-1")
            .with_node("node-1");
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let state = ArbiterState::new();

        match pass.evaluate(&action, &state).unwrap() {
            ArbitrationStep::Continue(delta, _) => {
                assert!(delta > 0.0);
            }
            other => panic!("Expected Continue, got {:?}", other),
        }
    }

    #[test]
    fn provenance_pass_no_provenance_negative() {
        let pass = ProvenancePass;
        let action = make_action("echo hello", 3, 0.8);
        let state = ArbiterState::new();

        match pass.evaluate(&action, &state).unwrap() {
            ArbitrationStep::Continue(delta, _) => {
                assert!(delta < 0.0);
            }
            other => panic!("Expected Continue, got {:?}", other),
        }
    }

    #[test]
    fn history_pass_no_history_abstains() {
        let pass = HistoryPass::default();
        let action = make_action("echo hello", 3, 0.8);
        let state = ArbiterState::new();

        match pass.evaluate(&action, &state).unwrap() {
            ArbitrationStep::Continue(delta, _) => {
                assert_eq!(delta, 0.0);
            }
            other => panic!("Expected Continue(0.0), got {:?}", other),
        }
    }

    #[test]
    fn history_pass_mostly_approved_positive() {
        let pass = HistoryPass {
            similar_approved: 8,
            similar_denied: 2,
        };
        let action = make_action("echo hello", 3, 0.8);
        let state = ArbiterState::new();

        match pass.evaluate(&action, &state).unwrap() {
            ArbitrationStep::Continue(delta, _) => {
                assert!(delta > 0.0);
            }
            other => panic!("Expected Continue(>0), got {:?}", other),
        }
    }
}
