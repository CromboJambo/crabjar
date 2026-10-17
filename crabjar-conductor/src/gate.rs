//! Conductor execution gate — reuses guard's ExecutionGate patterns for
//! task assignment and fleet orchestration authorization.
//!
//! The conductor uses the same provenance-based, trust-layer gating as
//! crabjar-guard to ensure task assignment decisions are authorized,
//! reversible, and auditable.

use anyhow::Result;
use crabjar_guard::{ExecutionGate, GateContext, TrustScore};

use crate::state_store::{Task, Worker};

/// Map conductor trust levels (0-3) to guard trust layers (1-3).
/// Conductor level 0 = unknown/unproven → guard layer 1 (quarantine).
pub fn conductor_level_to_guard_layer(level: u8) -> u32 {
    match level {
        0 => 1, // Unknown → quarantine
        1 => 2, // Proven → working
        2 => 3, // Trusted → annealed
        _ => 3, // Cap at annealed
    }
}

/// Check if a worker can claim a task based on trust layer requirements.
/// Uses guard's ExecutionGate pattern with provenance verification.
pub fn can_claim_task(gate: &ExecutionGate<'_>, worker: &Worker, task: &Task) -> Result<bool> {
    // Build a gate context for the claim decision
    let trust_layer = conductor_level_to_guard_layer(worker.trust_level);

    let ctx = GateContext::new(
        "task_claim",
        "conductor",
        vec![],
        trust_layer,
        TrustScore::new(0.95),
    )
    .with_can_interrupt(true);

    // Run the gate check — this verifies provenance, trust layer, etc.
    match gate.check(ctx)? {
        crabjar_guard::GateResult::Proceed => Ok(true),
        _ => Ok(false),
    }
}

/// Check if a worker's trust level is sufficient for task auto-assignment.
/// Trust layer 2+ can be auto-assigned; layer 1 requires manual review.
pub fn trust_sufficient_for_auto_assign(worker: &Worker) -> bool {
    let layer = conductor_level_to_guard_layer(worker.trust_level);
    layer >= 2 // WORKING layer (id=2) or above
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_store::WorkerStatus;

    #[test]
    fn trust_level_mapping() {
        assert_eq!(conductor_level_to_guard_layer(0), 1); // Unknown → quarantine
        assert_eq!(conductor_level_to_guard_layer(1), 2); // Proven → working
        assert_eq!(conductor_level_to_guard_layer(2), 3); // Trusted → annealed
        assert_eq!(conductor_level_to_guard_layer(3), 3); // Cap at annealed
    }

    #[test]
    fn auto_assign_requires_working_layer() {
        let unknown = Worker {
            id: "w1".to_string(),
            name: "unknown".to_string(),
            ssh_key: None,
            capabilities: vec![],
            status: WorkerStatus::Online,
            trust_level: 0,
            successful_tasks: 0,
            failed_tasks: 0,
        };
        assert!(!trust_sufficient_for_auto_assign(&unknown));

        let proven = Worker {
            id: "w2".to_string(),
            name: "proven".to_string(),
            ssh_key: None,
            capabilities: vec![],
            status: WorkerStatus::Online,
            trust_level: 1,
            successful_tasks: 5,
            failed_tasks: 0,
        };
        assert!(trust_sufficient_for_auto_assign(&proven));
    }
}
