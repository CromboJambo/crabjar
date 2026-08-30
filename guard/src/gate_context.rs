//! Gate context — the data needed for a gate check.
use crate::context_budget::ContextBudget;
use crate::trust::TrustScore;

/// Context for gate checks.
///
/// Carries all the information needed to evaluate whether an action
/// should proceed, be interrupted, or deferred for review.
pub struct GateContext<'a> {
    pub action_type: &'a str,
    pub command: &'a str,
    pub args: Vec<String>,
    pub trust_layer: u32,
    pub confidence: TrustScore,
    pub source_event_id: Option<&'a str>,
    pub can_interrupt: bool,
    /// PID of the calling process (for pid_trust lookup)
    pub pid: Option<i32>,
    /// Scope of the action — project/identity isolation
    pub scope: Option<crate::scope::Scope>,
    /// Scope of the target resource being accessed
    pub target_scope: Option<crate::scope::Scope>,
    /// Authorization for cross-scope operations (optional bypass).
    /// When set and valid, allows an action to proceed even when
    /// actor_scope cannot directly access target_scope.
    pub cross_scope_auth: Option<crate::scope::CrossScopeAuth>,
    /// Known domains/URLs associated with this action.
    /// Populated by callers (orchestrator, exec handler) who know the actual
    /// network destinations. The gate checks these against the domain allowlist.
    /// If empty, the gate skips domain checking (caller must have verified).
    pub domains: Vec<String>,
    /// Cumulative context budget for this action's scope.
    /// If None, context budget is not checked (permissive mode).
    pub context_budget: Option<ContextBudget>,
    /// Token count of the context that would be injected by this action.
    /// Only relevant when context_budget is Some.
    pub context_fragment_tokens: Option<usize>,
}

impl<'a> GateContext<'a> {
    /// Create a new GateContext with default values.
    pub fn new(
        action_type: &'a str,
        command: &'a str,
        args: Vec<String>,
        trust_layer: u32,
        confidence: TrustScore,
    ) -> Self {
        Self {
            action_type,
            command,
            args,
            trust_layer,
            confidence,
            source_event_id: None,
            can_interrupt: true,
            pid: None,
            scope: None,
            target_scope: None,
            cross_scope_auth: None,
            domains: Vec::new(),
            context_budget: None,
            context_fragment_tokens: None,
        }
    }

    /// Set the source event ID for provenance tracking.
    pub fn with_source_event(mut self, id: &'a str) -> Self {
        self.source_event_id = Some(id);
        self
    }

    /// Set whether the action can be interrupted.
    pub fn with_can_interrupt(mut self, can_interrupt: bool) -> Self {
        self.can_interrupt = can_interrupt;
        self
    }

    /// Set the PID of the calling process.
    pub fn with_pid(mut self, pid: i32) -> Self {
        self.pid = Some(pid);
        self
    }

    /// Set the scope of the action.
    pub fn with_scope(mut self, scope: crate::scope::Scope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Set the target scope of the action.
    pub fn with_target_scope(mut self, scope: crate::scope::Scope) -> Self {
        self.target_scope = Some(scope);
        self
    }

    /// Set cross-scope authorization for bypassing scope isolation.
    pub fn with_cross_scope_auth(mut self, auth: crate::scope::CrossScopeAuth) -> Self {
        self.cross_scope_auth = Some(auth);
        self
    }

    /// Set the context budget for this action.
    ///
    /// When Some, the gate will check that the action's context fragments
    /// fit within the budget. Per Q12: bounds are loose — a warning is
    /// logged at 80% utilization, but the action is allowed through.
    /// Hard rejection only happens at 100%.
    pub fn with_context_budget(mut self, budget: ContextBudget, fragment_tokens: usize) -> Self {
        self.context_budget = Some(budget);
        self.context_fragment_tokens = Some(fragment_tokens);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_context_new() {
        let ctx = GateContext::new(
            "test_action",
            "echo",
            vec!["hello".to_string()],
            3,
            TrustScore::new(0.8),
        );
        assert_eq!(ctx.action_type, "test_action");
        assert_eq!(ctx.command, "echo");
        assert_eq!(ctx.trust_layer, 3);
        assert_eq!(ctx.confidence.get(), 0.8);
        assert!(ctx.can_interrupt);
        assert!(ctx.pid.is_none());
        assert!(ctx.scope.is_none());
        assert!(ctx.target_scope.is_none());
    }

    #[test]
    fn gate_context_with_source_event() {
        let ctx = GateContext::new("test", "echo", vec![], 0, TrustScore::new(0.5))
            .with_source_event("evt-123");
        assert_eq!(ctx.source_event_id, Some("evt-123"));
    }

    #[test]
    fn gate_context_with_can_interrupt() {
        let ctx = GateContext::new("test", "echo", vec![], 0, TrustScore::new(0.5))
            .with_can_interrupt(false);
        assert!(!ctx.can_interrupt);
    }

    #[test]
    fn gate_context_with_pid() {
        let ctx = GateContext::new("test", "echo", vec![], 0, TrustScore::new(0.5)).with_pid(1234);
        assert_eq!(ctx.pid, Some(1234));
    }

    #[test]
    fn gate_context_with_cross_scope_auth() {
        let actor = crate::scope::Scope::project("project-a");
        let target = crate::scope::Scope::project("project-b");
        let auth = crate::scope::CrossScopeAuth::new(
            target.clone(),
            actor.clone(),
            "migration",
            "admin-policy",
        );
        let ctx = GateContext::new("test", "echo", vec![], 0, TrustScore::new(0.5))
            .with_scope(actor)
            .with_target_scope(target)
            .with_cross_scope_auth(auth);
        assert!(ctx.cross_scope_auth.is_some());
    }
}
