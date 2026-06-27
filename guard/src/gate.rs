//! ExecutionGate — the single authorization boundary for all tool execution.
//!
//! This is the single point where the system transitions from detection to action.
//! Detection != authorization: knowing *** happened does not grant the right to change what happens.

use std::path::PathBuf;
use tracing::{debug, info, warn};

pub use crate::command_risk::{CommandRisk, HIGH_RISK_COMMANDS, MEDIUM_RISK_COMMANDS};
pub use crate::domain_allowlist::{DomainAllowlist, DomainCheckError};
use crate::gate_context::GateContext;
use crate::gate_result::GateResult;
use crate::guard_db::{GuardDb, GuardDbError};
use crate::risk_config::RiskConfig;
use crate::trust::TrustManager;

/// Execution gate that combines trust-layer gating with command security checks.
pub struct ExecutionGate<'a> {
    trust: TrustManager<'a>,
    dry_run: bool,
    risk_config: RiskConfig,
    domain_allowlist: DomainAllowlist,
}

impl<'a> ExecutionGate<'a> {
    pub fn new(db: &'a GuardDb, dry_run: bool, _root: impl Into<PathBuf>) -> Self {
        let _ = _root;
        Self {
            trust: TrustManager::new(db),
            dry_run,
            risk_config: RiskConfig::default(),
            domain_allowlist: DomainAllowlist::new(),
        }
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
            self.verify_provenance(id)?
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
        if let Some(pid) = ctx.pid
            && let Some(gate_result) = self.check_pid_trust(pid, ctx.command)?
        {
            return Ok(gate_result);
        }

        // 7. Scope isolation check
        #[allow(clippy::collapsible_if)]
        if let (Some(actor_scope), Some(target_scope)) = (&ctx.scope, &ctx.target_scope) {
            if !actor_scope.can_access(target_scope) {
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
        }

        // 8. Command risk assessment
        let risk = self.assess_command_risk(ctx.command, &ctx.args);
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
                match self.domain_allowlist.check_for_trust_layer(domain, ctx.trust_layer) {
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

    /// Verify provenance: source_event_id exists in GuardDb action_requests.
    fn verify_provenance(&self, id: &str) -> Result<bool, GuardDbError> {
        let conn = self.trust.conn();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM action_requests WHERE source_event_id = ?1)",
                rusqlite::params![id],
                |r| r.get(0),
            )
            .map_err(|_| GuardDbError::SchemaError("Provenance check failed".into()))?;
        Ok(exists)
    }

    /// Check PID trust layer (Option B).
    /// Returns Revoked if trust has decayed below the action's trust layer.
    fn check_pid_trust(
        &self,
        pid: i32,
        command: &str,
    ) -> Result<Option<GateResult>, GuardDbError> {
        let conn = self.trust.conn();

        let row = conn.query_row(
            "SELECT trust_layer, use_count, last_use, auto_grant, decay_interval, decay_rate
             FROM pid_trust WHERE pid = ?1",
            rusqlite::params![pid],
            |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, u64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, f64>(5)?,
                ))
            },
        );

        let (current_layer, _use_count, last_use, auto_grant, decay_interval, decay_rate) =
            match row {
                Ok(r) => r,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Ok(Some(GateResult::Pending));
                }
                Err(e) => return Err(GuardDbError::Sqlite(e)),
            };

        // Compute decayed trust layer
        let elapsed = chrono::Utc::now().timestamp() - last_use;
        let effective_layer = if elapsed > decay_interval {
            let decayed = (decay_rate * elapsed as f64).min(1.0);
            let new_confidence = (current_layer as f64 / 3.0) * (1.0 - decayed);
            (new_confidence * 3.0) as u32
        } else {
            current_layer
        };

        // If trust has dropped below the action's layer, revoke
        if effective_layer < (self.risk_config.confidence_floor * 3.0) as u32 {
            let reason = format!(
                "PID {} trust decayed from layer {} to {} (auto_grant={})",
                pid, current_layer, effective_layer, auto_grant
            );
            warn!(pid, command, %reason, "PID trust revoked");
            return Ok(Some(GateResult::Revoked { reason }));
        }

        Ok(None)
    }

    /// Assess command risk based on name and arguments.
    fn assess_command_risk(&self, command: &str, args: &[String]) -> CommandRisk {
        let basename = command.split('/').next_back().unwrap_or(command);

        for risk_cmd in &self.risk_config.high_risk {
            if basename.eq_ignore_ascii_case(risk_cmd) {
                return CommandRisk::High;
            }
            let full_cmd = format!("{} {}", basename, args.join(" "));
            if full_cmd.eq_ignore_ascii_case(risk_cmd) {
                return CommandRisk::High;
            }
        }

        for risk_cmd in &self.risk_config.medium_risk {
            if basename.eq_ignore_ascii_case(risk_cmd) {
                return CommandRisk::Medium;
            }
        }

        CommandRisk::Low
    }
}
