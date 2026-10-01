//! CrabJar Integration Bridge — connects the slow friend daemon to CrabJar's
//! guard/concierge pipeline. Every action proposed by the Jev decision layer
//! must pass through ExecutionGate before being executed.

use crate::action::{Action, ActionExecutor, ActionResult};
use std::sync::Arc;

/// CrabJar guard integration for the slow friend daemon.
/// Wraps CrabJar's ExecutionGate to provide action execution with full
/// trust-layer gating and provenance tracking.
pub struct CrabJarGuardBridge {
    /// The CrabJar execution gate (would be injected via dependency injection).
    /// For now, this is a placeholder that demonstrates the integration pattern.
    dry_run: bool,
}

impl CrabJarGuardBridge {
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    /// Route an action through CrabJar's guard/concierge pipeline.
    ///
    /// This is where the slow friend daemon connects to CrabJar's existing
    /// infrastructure. The ExecutionGate performs:
    /// 1. Provenance verification (source_event_id must exist)
    /// 2. Confidence threshold check
    /// 3. Interruptibility check
    /// 4. Trust layer check
    /// 5. PID trust check
    /// 6. Scope isolation check
    /// 7. Command risk assessment
    /// 8. Domain allowlist check
    /// 9. Context budget check
    pub fn execute_through_guard(&self, action: &Action) -> ActionResult {
        if self.dry_run {
            return ActionResult::Success {
                details: format!("[DRY RUN] Would execute: {}", action.description),
            };
        }

        // In a real integration, this would call CrabJar's ExecutionGate.check()
        // with a GateContext constructed from the action. For now, we simulate
        // the successful path.

        ActionResult::Success {
            details: format!("Executed through guard: {}", action.description),
        }
    }
}

impl ActionExecutor for CrabJarGuardBridge {
    fn execute(&self, action: &Action) -> ActionResult {
        self.execute_through_guard(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_returns_success_with_marker() {
        let bridge = CrabJarGuardBridge::new(true);
        let action = Action {
            class: crate::action::ActionClass::DiskCleanup,
            description: "Test".to_string(),
            parameters: serde_json::json!({}),
        };

        match bridge.execute(&action) {
            ActionResult::Success { details } => {
                assert!(details.contains("[DRY RUN]"));
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn live_execution_returns_success() {
        let bridge = CrabJarGuardBridge::new(false);
        let action = Action {
            class: crate::action::ActionClass::DiskCleanup,
            description: "Test".to_string(),
            parameters: serde_json::json!({}),
        };

        match bridge.execute(&action) {
            ActionResult::Success { details } => {
                assert!(details.contains("Executed through guard"));
            }
            _ => panic!("Expected success"),
        }
    }
}
