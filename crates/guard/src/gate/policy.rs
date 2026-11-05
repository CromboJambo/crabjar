use crate::gate_context::GateContext;
use crate::policy_types::{PolicyContext, PolicyEngine, PolicyResult};

/// Convert a GateContext into a PolicyContext for evaluation.
pub fn to_policy_context(ctx: &GateContext<'_>) -> PolicyContext {
    PolicyContext {
        action_type: ctx.action_type.to_string(),
        command: ctx.command.to_string(),
        args: ctx.args.clone(),
        trust_layer: ctx.trust_layer,
        confidence: ctx.confidence,
        source_event_id: ctx.source_event_id.map(|s| s.to_string()),
        can_interrupt: ctx.can_interrupt,
        pid: ctx.pid,
        scope: ctx.scope.as_ref().cloned(),
        target_scope: ctx.target_scope.as_ref().cloned(),
        domains: ctx.domains.clone(),
        context_budget: ctx.context_budget.clone(),
        context_fragment_tokens: ctx.context_fragment_tokens,
    }
}

/// Run the policy engine pre-check if configured.
/// Returns `Some(GateResult)` if the policy engine blocks or modifies the check,
/// or `None` to proceed with normal gate logic.
pub fn run_policy_check(
    policy_engine: Option<&dyn PolicyEngine>,
    ctx: &GateContext<'_>,
) -> Option<crate::gate_result::GateResult> {
    let engine = policy_engine?;
    let policy_ctx = to_policy_context(ctx);

    match engine.evaluate(&policy_ctx) {
        PolicyResult::Proceed => None,
        PolicyResult::Interrupted { reason } => {
            Some(crate::gate_result::GateResult::Interrupted { reason })
        }
        PolicyResult::Pending => Some(crate::gate_result::GateResult::Pending),
        PolicyResult::DryRun => Some(crate::gate_result::GateResult::DryRun),
        PolicyResult::Revoked { reason } => {
            Some(crate::gate_result::GateResult::Revoked { reason })
        }
        PolicyResult::ContextExhausted {
            used,
            budget,
            remaining,
        } => Some(crate::gate_result::GateResult::ContextExhausted {
            used,
            budget,
            remaining,
        }),
        PolicyResult::OversizedFragment { actual, max } => {
            Some(crate::gate_result::GateResult::OversizedFragment { actual, max })
        }
    }
}
