//! ExecutionGate — the single authorization boundary for all tool execution.
//!
//! This is the single point where the system transitions from detection to action.
//! Detection != authorization: knowing *** happened does not grant the right to change what happens.

use std::path::PathBuf;
use tracing::{debug, info, warn};

pub use crate::command_risk::{CommandRisk, HIGH_RISK_COMMANDS, MEDIUM_RISK_COMMANDS};
pub use crate::context_budget::ContextBudget;
pub use crate::domain_allowlist::{DomainAllowlist, DomainCheckError};
use crate::gate_context::GateContext;
use crate::gate_result::GateResult;
use crate::guard_db::{GuardDb, GuardDbError};
pub use crate::policy_types::{PolicyContext, PolicyEngine, PolicyResult};
use crate::risk_config::RiskConfig;
use crate::trust::TrustManager;

mod command_risk;
mod pid_trust;
mod policy;
mod provenance;

/// Execution gate that combines trust-layer gating with command security checks.
pub struct ExecutionGate<'a> {
    trust: TrustManager<'a>,
    dry_run: bool,
    risk_config: RiskConfig,
    domain_allowlist: DomainAllowlist,
    /// Optional policy engine for declarative policy evaluation.
    /// When set, runs as a pre-check before the existing gate logic.
    /// If no policy engine is configured, falls back to static gate behavior (backward compatible).
    policy_engine: Option<Box<dyn PolicyEngine>>,
}

impl<'a> ExecutionGate<'a> {
    pub fn new(db: &'a GuardDb, dry_run: bool, _root: impl Into<PathBuf>) -> Self {
        let _ = _root;
        Self {
            trust: TrustManager::new(db),
            dry_run,
            risk_config: RiskConfig::default(),
            domain_allowlist: DomainAllowlist::new(),
            policy_engine: None,
        }
    }

    /// Set the policy engine for this gate.
    pub fn with_policy_engine(mut self, engine: Box<dyn PolicyEngine>) -> Self {
        self.policy_engine = Some(engine);
        self
    }

    pub fn with_risk_config(mut self, risk_config: RiskConfig) -> Self {
        self.risk_config = risk_config;
        self
    }

    /// Run the full gate check before executing an action.
    ///
    /// The gate enforces:
    /// 1. Raw data reference: the event must reference raw data, not interpreted summaries
    /// 2. Uncertainty exposure: if confidence is below threshold, surface it before executing
    /// 3. Interruptibility: allow the gate to return Interrupted instead of executing
    /// 4. Trust layer check: auto-execute only for trusted layers
    pub fn check(&self, ctx: GateContext<'_>) -> Result<GateResult, GuardDbError> {
        // Policy engine pre-check (if configured).
        if let Some(gate_result) = policy::run_policy_check(self.policy_engine.as_deref(), &ctx) {
            debug!(
                action = %ctx.action_type,
                policy_source = self.policy_engine.as_ref().map(|e| e.source_description()),
                "Policy engine pre-check returned: {:?}",
                gate_result
            );
            return Ok(gate_result);
        }

        // 1. Dry-run check
        if self.dry_run {
            info!(
                action = %ctx.action_type,
                trust_layer = ctx.trust_layer,
                "Dry-run: would execute action"
            );
            return Ok(GateResult::DryRun);
        }

        // 2. Provenance verification: source_event_id must exist in action_requests
        let provenance_exists = if let Some(id) = ctx.source_event_id {
            provenance::verify_provenance(&self.trust.conn(), id)?
        } else {
            false
        };

        if !provenance_exists {
            let reason = "No provenance found in GuardDb; detection != authorization".to_string();
            warn!(action = %ctx.action_type, %reason, "Gate interrupted");
            return Ok(GateResult::Interrupted { reason });
        }

        // 3. Confidence threshold
        if ctx.confidence.get() < self.risk_config.confidence_floor {
            let reason = format!(
                "Confidence {:.3} below floor {:.3}; must surface before execution",
                ctx.confidence.get(),
                self.risk_config.confidence_floor
            );
            warn!(
                action = %ctx.action_type,
                confidence = ctx.confidence.get(),
                %reason,
                "Gate interrupted"
            );
            return Ok(GateResult::Interrupted { reason });
        }

        // 4. Interruptibility check
        if !ctx.can_interrupt {
            let reason =
                "Action cannot be interrupted; gate safety requirement not met".to_string();
            warn!(action = %ctx.action_type, %reason, "Gate interrupted");
            return Ok(GateResult::Interrupted { reason });
        }

        // 5. Trust layer check
        let can_auto = self.trust.can_auto_execute(ctx.trust_layer)?;
        let needs_review = self.trust.requires_review(ctx.trust_layer)?;

        if needs_review {
            debug!(
                action = %ctx.action_type,
                trust_layer = ctx.trust_layer,
                "Action requires human review"
            );
            return Ok(GateResult::Pending);
        }

        if !can_auto {
            let reason = format!(
                "Trust layer {} does not allow auto-execute",
                ctx.trust_layer
            );
            return Ok(GateResult::Interrupted { reason });
        }

        // 6. PID trust check (Option B: per-process trust layers)
        if let Some(pid) = ctx.pid {
            if let Some(gate_result) =
                pid_trust::check_pid_trust(&self.trust.conn(), pid, ctx.command, &self.risk_config)?
            {
                return Ok(gate_result);
            }
        }

        // 7. Scope isolation check
        #[allow(clippy::collapsible_if)]
        if let (Some(actor_scope), Some(target_scope)) = (&ctx.scope, &ctx.target_scope) {
            if !actor_scope.can_access(target_scope) {
                // Check for valid CrossScopeAuth bypass
                let cross_scope_allowed = ctx.cross_scope_auth.as_ref().is_some_and(|auth| {
                    auth.is_valid(3600)
                        && auth.actor_scope == *actor_scope
                        && auth.target_scope == *target_scope
                });

                if !cross_scope_allowed {
                    let reason = format!(
                        "Scope isolation: {} cannot access {}",
                        actor_scope.to_scope_string(),
                        target_scope.to_scope_string()
                    );
                    warn!(
                        action = %ctx.action_type,
                        actor = %actor_scope,
                        target = %target_scope,
                        %reason,
                        "Scope isolation blocked"
                    );
                    return Ok(GateResult::Interrupted { reason });
                }

                debug!(
                    action = %ctx.action_type,
                    actor = %actor_scope,
                    target = %target_scope,
                    reason = ctx.cross_scope_auth.as_ref().map(|a| a.reason.as_str()).unwrap_or(""),
                    "Cross-scope operation authorized via CrossScopeAuth"
                );
            }
        }

        // 8. Command risk assessment
        let risk = command_risk::assess_command_risk(ctx.command, &ctx.args, &self.risk_config);
        match risk {
            CommandRisk::High => {
                return Ok(GateResult::Interrupted {
                    reason: format!("High-risk command '{}' detected", ctx.command),
                });
            }
            CommandRisk::Medium => {
                debug!(
                    action = %ctx.action_type,
                    command = %ctx.command,
                    "Medium-risk command requires review"
                );
                return Ok(GateResult::Pending);
            }
            CommandRisk::Low => {
                debug!(action = %ctx.action_type, "Low-risk command approved");
            }
            CommandRisk::Unauthorized => {
                return Ok(GateResult::Interrupted {
                    reason: "Unauthorized action: detection != authorization".to_string(),
                });
            }
        }

        // 9. Domain allowlist check (if domains are known)
        if !ctx.domains.is_empty() {
            for domain in &ctx.domains {
                match self
                    .domain_allowlist
                    .check_for_trust_layer(domain, ctx.trust_layer)
                {
                    Ok(trust_level) => {
                        debug!(
                            action = %ctx.action_type,
                            domain = %domain,
                            trust_level = %trust_level,
                            "Domain allowlist: checked"
                        );
                    }
                    Err(e) => {
                        let reason = format!(
                            "Domain allowlist: {} for domain '{}' at trust layer {}",
                            e, domain, ctx.trust_layer
                        );
                        warn!(
                            action = %ctx.action_type,
                            domain = %domain,
                            %reason,
                            "Domain allowlist blocked"
                        );
                        return Ok(GateResult::Interrupted { reason });
                    }
                }
            }
        }

        // 10. Context budget check (Q12: wire in but leave pretty open)
        if let (Some(budget), fragment_tokens) = (&ctx.context_budget, ctx.context_fragment_tokens)
        {
            // Per-fragment hard cap (Codex spec: 10K tokens)
            if let Some(tokens) = fragment_tokens
                && tokens > crate::context_budget::MAX_TOKENS_PER_FRAGMENT
            {
                return Ok(GateResult::OversizedFragment {
                    actual: tokens,
                    max: crate::context_budget::MAX_TOKENS_PER_FRAGMENT,
                });
            }

            // Warn at 80% utilization (loose bounds per Q12)
            if let Some(remaining) = budget.warn_if_approaching() {
                warn!(
                    action = %ctx.action_type,
                    used = budget.used(),
                    remaining = remaining,
                    budget = budget.budget(),
                    "Context budget approaching limit"
                );
            }

            // Reserve tokens for this action's context
            if let Some(tokens) = fragment_tokens
                && !budget.can_fit(tokens)
            {
                return Ok(GateResult::ContextExhausted {
                    used: budget.used(),
                    budget: budget.budget(),
                    remaining: budget.remaining(),
                });
            }
        }

        Ok(GateResult::Proceed)
    }

    /// Gate knowledge write based on source.
    ///
    /// External-sourced writes always land in quarantine (pending) regardless
    /// of confidence. User/Agent writes follow normal trust layer gating.
    pub fn check_knowledge_write(&self, source: &str) -> Result<GateResult, GuardDbError> {
        match source {
            "external" => {
                debug!(
                    source = source,
                    "Knowledge write from external source → quarantine"
                );
                Ok(GateResult::Pending)
            }
            "agent" | "system" => {
                debug!(source = source, "Knowledge write from trusted source");
                Ok(GateResult::Proceed)
            }
            "user" => {
                debug!(source = source, "User writes are trusted");
                Ok(GateResult::Proceed)
            }
            _ => {
                warn!(
                    source = source,
                    "Unknown knowledge write source → quarantine"
                );
                Ok(GateResult::Pending)
            }
        }
    }
}
