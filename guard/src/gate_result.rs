/// Result of a gate check.
///
/// Represents the outcome of an `ExecutionGate::check()` call:
/// whether to proceed, interrupt, or defer for review.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateResult {
    Proceed,
    Interrupted {
        reason: String,
    },
    Pending,
    DryRun,
    /// Process was revoked — guide out gracefully, don't block hard.
    Revoked {
        reason: String,
    },
    /// Context budget exhausted — no more fragments can be injected.
    /// The agent should continue with reduced context and try again later.
    ContextExhausted {
        used: usize,
        budget: usize,
        remaining: usize,
    },
}

impl GateResult {
    /// Returns `true` if the gate allows execution to proceed.
    pub fn is_proceed(&self) -> bool {
        matches!(self, GateResult::Proceed)
    }

    /// Returns `true` if the gate interrupted execution.
    pub fn is_interrupted(&self) -> bool {
        matches!(self, GateResult::Interrupted { .. })
    }

    /// Returns `true` if the gate deferred for review.
    pub fn is_pending(&self) -> bool {
        matches!(self, GateResult::Pending)
    }

    /// Returns `true` if this is a dry-run result.
    pub fn is_dry_run(&self) -> bool {
        matches!(self, GateResult::DryRun)
    }

    /// Returns `true` if the process was revoked.
    pub fn is_revoked(&self) -> bool {
        matches!(self, GateResult::Revoked { .. })
    }

    /// Returns `true` if the context budget was exhausted.
    pub fn is_context_exhausted(&self) -> bool {
        matches!(self, GateResult::ContextExhausted { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_result_is_proceed() {
        assert!(GateResult::Proceed.is_proceed());
        assert!(!GateResult::Pending.is_proceed());
        assert!(!GateResult::Interrupted { reason: "test".to_string() }.is_proceed());
    }

    #[test]
    fn gate_result_is_interrupted() {
        assert!(GateResult::Interrupted { reason: "test".to_string() }.is_interrupted());
        assert!(!GateResult::Proceed.is_interrupted());
    }

    #[test]
    fn gate_result_is_pending() {
        assert!(GateResult::Pending.is_pending());
        assert!(!GateResult::Proceed.is_pending());
    }

    #[test]
    fn gate_result_is_dry_run() {
        assert!(GateResult::DryRun.is_dry_run());
        assert!(!GateResult::Proceed.is_dry_run());
    }

    #[test]
    fn gate_result_is_revoked() {
        assert!(GateResult::Revoked { reason: "test".to_string() }.is_revoked());
        assert!(!GateResult::Proceed.is_revoked());
    }

    #[test]
    fn gate_result_equality() {
        let r1 = GateResult::Proceed;
        let r2 = GateResult::Proceed;
        assert_eq!(r1, r2);

        let r3 = GateResult::Interrupted { reason: "a".to_string() };
        let r4 = GateResult::Interrupted { reason: "b".to_string() };
        assert_ne!(r3, r4); // Interrupted variants differ by reason
    }
}
