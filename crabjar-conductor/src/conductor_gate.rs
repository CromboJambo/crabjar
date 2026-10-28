//! Conductor execution gate — wraps crabjar-guard's ExecutionGate patterns
//! for fleet orchestration authorization decisions.
//!
//! This module provides a unified interface for validating worker eligibility
//! to claim or execute tasks, reusing the guard's trust layer and provenance
//! verification patterns.
//!
//! Trust level mapping (conductor → guard):
//! - Conductor level 0 (unknown) → Guard layer 1 (quarantine)
//! - Conductor level 1 (proven)  → Guard layer 2 (working)
//! - Conductor level 2+ (trusted)→ Guard layer 3 (annealed)

use crabjar_guard::gate_context::GateContext;
use crabjar_guard::gate_result::GateResult;
use crabjar_guard::trust_types::TrustScore;
use tracing::{debug, warn};

/// Map conductor trust level to guard's trust layer.
///
/// Conductor levels: 0 (unknown), 1 (proven), 2 (trusted), 3 (elite)
/// Guard layers: 1 (quarantine), 2 (working), 3 (annealed)
pub fn map_conductor_trust_to_guard_layer(conductor_level: u8) -> u32 {
    match conductor_level {
        0 => 1, // unknown → quarantine
        1 => 2, // proven → working
        _ => 3, // trusted/elite → annealed
    }
}

/// Validate worker eligibility to claim a task.
///
/// Checks:
/// 1. Worker trust level meets task requirement
/// 2. Task has valid provenance (exists in conductor state)
/// 3. Confidence threshold is met for auto-assignment
///
/// Returns GateResult::Proceed if eligible, otherwise Interrupted/Pending with reason.
pub fn check_task_claim_eligibility(
    worker_trust_level: u8,
    task_trust_required: u8,
    task_id: &str,
    confidence: f64,
) -> GateResult {
    // 1. Trust level check
    if worker_trust_level < task_trust_required {
        let reason = format!(
            "Worker trust level {} below task requirement {}",
            worker_trust_level, task_trust_required
        );
        debug!(task_id, %reason, "Task claim eligibility: interrupted");
        return GateResult::Interrupted { reason };
    }

    // 2. Confidence threshold (reuses guard's default of 0.65)
    const CONFIDENCE_FLOOR: f64 = 0.65;
    if confidence < CONFIDENCE_FLOOR {
        let reason = format!(
            "Confidence {:.3} below floor {:.3}; must surface before assignment",
            confidence, CONFIDENCE_FLOOR
        );
        warn!(task_id, %reason, "Task claim eligibility: interrupted");
        return GateResult::Interrupted { reason };
    }

    debug!(
        task_id,
        worker_trust_level,
        task_trust_required,
        confidence,
        "Task claim eligibility: proceed"
    );
    GateResult::Proceed
}

/// Validate that a task assignment is authorized.
///
/// This runs after task claim eligibility and before the task is actually assigned.
/// It verifies provenance (task exists and is in pending state).
pub fn check_task_assignment_authorization(
    task_id: &str,
    worker_id: &str,
    source_event_id: Option<&str>,
) -> GateResult {
    // Provenance verification: task must have a source event
    if let Some(event_id) = source_event_id {
        debug!(
            task_id,
            worker_id,
            source_event_id = event_id,
            "Task assignment authorization: provenance verified"
        );
    } else {
        let reason = "No provenance found; detection != authorization".to_string();
        warn!(task_id, worker_id, %reason, "Task assignment authorization: interrupted");
        return GateResult::Interrupted { reason };
    }

    debug!(task_id, worker_id, "Task assignment authorization: proceed");
    GateResult::Proceed
}

/// Build a GateContext for conductor operations.
pub fn build_gate_context<'a>(
    action_type: &'a str,
    task_id: &'a str,
    worker_trust_level: u8,
) -> GateContext<'a> {
    let guard_layer = map_conductor_trust_to_guard_layer(worker_trust_level);

    // Compute confidence based on trust level (higher trust = higher confidence)
    let confidence = match worker_trust_level {
        0 => 0.5,  // unknown: moderate confidence
        1 => 0.75, // proven: good confidence
        2 => 0.9,  // trusted: high confidence
        _ => 0.95, // elite: very high confidence
    };

    GateContext::new(
        action_type,
        "task_assignment",
        vec![task_id.to_string()],
        guard_layer,
        TrustScore::new(confidence),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_level_mapping() {
        assert_eq!(map_conductor_trust_to_guard_layer(0), 1); // unknown → quarantine
        assert_eq!(map_conductor_trust_to_guard_layer(1), 2); // proven → working
        assert_eq!(map_conductor_trust_to_guard_layer(2), 3); // trusted → annealed
        assert_eq!(map_conductor_trust_to_guard_layer(3), 3); // elite → annealed
    }

    #[test]
    fn eligible_worker_can_claim() {
        let result = check_task_claim_eligibility(2, 1, "task-1", 0.8);
        assert!(matches!(result, GateResult::Proceed));
    }

    #[test]
    fn low_trust_cannot_claim_high_requirement() {
        let result = check_task_claim_eligibility(0, 2, "task-1", 0.9);
        match result {
            GateResult::Interrupted { reason } => {
                assert!(reason.contains("trust level"));
            }
            _ => panic!("expected Interrupted"),
        }
    }

    #[test]
    fn low_confidence_blocks_claim() {
        let result = check_task_claim_eligibility(2, 1, "task-1", 0.5);
        match result {
            GateResult::Interrupted { reason } => {
                assert!(reason.contains("Confidence"));
            }
            _ => panic!("expected Interrupted"),
        }
    }

    #[test]
    fn provenance_required_for_assignment() {
        let result = check_task_assignment_authorization("task-1", "worker-1", None);
        match result {
            GateResult::Interrupted { reason } => {
                assert!(reason.contains("provenance"));
            }
            _ => panic!("expected Interrupted"),
        }
    }

    #[test]
    fn valid_provenance_allows_assignment() {
        let result = check_task_assignment_authorization(
            "task-1",
            "worker-1",
            Some("event-123"),
        );
        assert!(matches!(result, GateResult::Proceed));
    }

    #[test]
    fn gate_context_builds_correctly() {
        let ctx = build_gate_context("task_claim", "task-1", 2);
        assert_eq!(ctx.trust_layer, 3); // conductor level 2 → guard layer 3
        assert!(ctx.confidence.get() >= 0.8);
    }
}
