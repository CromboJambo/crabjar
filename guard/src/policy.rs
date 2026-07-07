//! Static policy engine — Rust-native declarative policy evaluation.
//!
//! Implements `PolicyEngine` using parameterized TOML configuration instead of
//! interpreted scripting. This provides:
//! - Zero startup cost (no interpreter initialization)
//! - Compile-time type safety (config validated at load time)
//! - Hot-reload support (reload config from file without restart)
//! - Backward compatibility (existing gate logic unchanged when no policy engine is set)
//!
//! ## Design Notes
//!
//! The static engine evaluates a subset of gate checks that are naturally
//! declarative: dangerous command lists, confidence floors, trust layer minimums,
//! scope isolation toggles, and domain allowlist modes. Complex conditional logic
//! (time-based policies, rate limiting, multi-factor approval) is deferred to a
//! future Starlark backend if/when demand emerges.

use std::path::Path;
use tracing::debug;

use crate::policy_types::*;

/// Static policy engine backed by TOML configuration.
///
/// Evaluates actions against configurable thresholds and rules loaded from
/// a policy file. Supports hot-reload via `reload()` method.
pub struct StaticPolicyEngine {
    config: PolicyConfig,
}

impl StaticPolicyEngine {
    /// Create a new static policy engine with default configuration.
    pub fn new() -> Self {
        Self {
            config: PolicyConfig::default(),
        }
    }

    /// Create a new static policy engine from a TOML file.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let config = PolicyConfig::load(path)?;
        Ok(Self { config })
    }

    /// Evaluate an action against the current policy configuration.
    ///
    /// Checks are applied in order of increasing cost (fast path first):
    /// 1. Dangerous command check (string match, O(n))
    /// 2. Confidence floor check (float comparison)
    /// 3. Trust layer minimum check (integer comparison)
    /// 4. Scope isolation check (if enabled)
    /// 5. Domain allowlist check (if domains are known)
    /// 6. Context budget check (if configured and active)
    pub fn evaluate(&self, ctx: &PolicyContext) -> PolicyResult {
        // 1. Dangerous command check
        if self.is_dangerous_command(&ctx.command, &ctx.args) {
            return PolicyResult::Interrupted {
                reason: format!("Dangerous command '{}' blocked by policy", ctx.command),
            };
        }

        // 2. Confidence floor check
        if ctx.confidence.get() < self.config.confidence_floor {
            return PolicyResult::Interrupted {
                reason: format!(
                    "Confidence {:.3} below policy floor {:.3}",
                    ctx.confidence.get(),
                    self.config.confidence_floor,
                ),
            };
        }

        // 3. Trust layer minimum check
        if ctx.trust_layer < self.config.min_trust_layer {
            return PolicyResult::Interrupted {
                reason: format!(
                    "Trust layer {} below policy minimum {}",
                    ctx.trust_layer, self.config.min_trust_layer
                ),
            };
        }

        // 4. Scope isolation check (configurable)
        if self.config.enforce_scope_isolation
            && let Some(result) = self.check_scope_isolation(ctx)
        {
            return result;
        }

        // 5. Domain allowlist check
        if !ctx.domains.is_empty()
            && let Some(result) = self.check_domain_allowlist(ctx)
        {
            return result;
        }

        // 6. Context budget check (if configured and active)
        if self.config.max_context_budget_tokens > 0
            && let Some(result) = self.check_context_budget(ctx)
        {
            return result;
        }

        PolicyResult::Proceed
    }

    /// Check if a command matches any dangerous command pattern.
    fn is_dangerous_command(&self, command: &str, args: &[String]) -> bool {
        let basename = command.split('/').next_back().unwrap_or(command);
        let full_cmd = format!("{} {}", basename, args.join(" "));

        for pattern in &self.config.dangerous_commands {
            if basename.eq_ignore_ascii_case(pattern) || full_cmd.eq_ignore_ascii_case(pattern) {
                return true;
            }
        }
        false
    }

    /// Check scope isolation between actor and target scopes.
    fn check_scope_isolation(&self, ctx: &PolicyContext) -> Option<PolicyResult> {
        if let (Some(actor), Some(target)) = (&ctx.scope, &ctx.target_scope)
            && !actor.can_access(target)
        {
            return Some(PolicyResult::Interrupted {
                reason: format!(
                    "Scope isolation: {} cannot access {}",
                    actor.to_scope_string(),
                    target.to_scope_string()
                ),
            });
        }
        None
    }

    /// Check domain allowlist against configured mode.
    fn check_domain_allowlist(&self, ctx: &PolicyContext) -> Option<PolicyResult> {
        let allowlist = crate::domain_allowlist::DomainAllowlist::new();

        for domain in &ctx.domains {
            match self.config.domain_allowlist_mode {
                DomainAllowlistMode::Strict => {
                    if !allowlist.is_allowed(domain) {
                        return Some(PolicyResult::Interrupted {
                            reason: format!("Domain '{}' not in allowlist (strict mode)", domain),
                        });
                    }
                }
                DomainAllowlistMode::Permissive => {
                    // Allow but log unlisted domains
                    if !allowlist.is_allowed(domain) {
                        debug!(domain, "Policy: permissive mode — allowing unlisted domain");
                    }
                }
            }
        }
        None
    }

    /// Check context budget against configured limits.
    fn check_context_budget(&self, ctx: &PolicyContext) -> Option<PolicyResult> {
        // Per-fragment hard cap (use config value or default 10K)
        let max_fragment = if self.config.max_fragment_tokens > 0 {
            self.config.max_fragment_tokens
        } else {
            crate::context_budget::MAX_TOKENS_PER_FRAGMENT
        };

        if let Some(tokens) = ctx.context_fragment_tokens
            && tokens > max_fragment
        {
            return Some(PolicyResult::OversizedFragment {
                actual: tokens,
                max: max_fragment,
            });
        }

        // Cumulative budget check (if context_budget is provided)
        if let (Some(budget), Some(tokens)) = (&ctx.context_budget, ctx.context_fragment_tokens)
            && !budget.can_fit(tokens)
        {
            return Some(PolicyResult::ContextExhausted {
                used: budget.used(),
                budget: budget.budget(),
                remaining: budget.remaining(),
            });
        }

        None
    }
}

impl Default for StaticPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEngine for StaticPolicyEngine {
    fn evaluate(&self, ctx: &PolicyContext) -> PolicyResult {
        StaticPolicyEngine::evaluate(self, ctx)
    }

    fn reload(&mut self, path: &Path) -> Result<(), String> {
        let new_config = PolicyConfig::load(path)?;

        if self.config.differs_from(&new_config) {
            debug!(
                policy_path = %path.display(),
                "Policy engine: reloading configuration"
            );
            self.config = new_config;
        } else {
            debug!("Policy engine: no changes detected, skipping reload");
        }

        Ok(())
    }

    fn source_description(&self) -> String {
        match &self.config.source_path {
            Some(path) => path.to_string_lossy().to_string(),
            None => "static (default)".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::TrustScore;
    use tempfile::tempdir;

    fn make_context(
        command: &str,
        args: Vec<String>,
        trust_layer: u32,
        confidence: f64,
    ) -> PolicyContext {
        PolicyContext::new("test", command, args, trust_layer, TrustScore::new(confidence))
    }

    #[test]
    fn test_static_engine_default_proceeds() {
        let engine = StaticPolicyEngine::new();
        let ctx = make_context("echo", vec!["hello".to_string()], 3, 0.9);
        assert!(engine.evaluate(&ctx).is_proceed());
    }

    #[test]
    fn test_static_engine_dangerous_command_denied() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 2
confidence_floor = 0.6
dangerous_commands = ["rm -rf /", "mkfs"]
"#,
        )
        .unwrap();

        let engine = StaticPolicyEngine::from_file(&config_path).unwrap();
        let ctx = make_context("rm", vec!["-rf".to_string(), "/".to_string()], 3, 0.9);
        let result = engine.evaluate(&ctx);
        assert!(matches!(result, PolicyResult::Interrupted { .. }));
    }

    #[test]
    fn test_static_engine_confidence_floor() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 2
confidence_floor = 0.8
"#,
        )
        .unwrap();

        let engine = StaticPolicyEngine::from_file(&config_path).unwrap();
        let ctx = make_context("echo", vec![], 3, 0.7); // below floor
        let result = engine.evaluate(&ctx);
        assert!(matches!(result, PolicyResult::Interrupted { .. }));

        // Above floor should proceed
        let ctx_high = make_context("echo", vec![], 3, 0.9);
        assert!(engine.evaluate(&ctx_high).is_proceed());
    }

    #[test]
    fn test_static_engine_trust_layer_minimum() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 3
confidence_floor = 0.6
"#,
        )
        .unwrap();

        let engine = StaticPolicyEngine::from_file(&config_path).unwrap();

        // Layer 2 below minimum of 3
        let ctx_low = make_context("echo", vec![], 2, 0.9);
        assert!(matches!(engine.evaluate(&ctx_low), PolicyResult::Interrupted { .. }));

        // Layer 3 meets minimum
        let ctx_high = make_context("echo", vec![], 3, 0.9);
        assert!(engine.evaluate(&ctx_high).is_proceed());
    }

    #[test]
    fn test_static_engine_scope_isolation() {
        let engine = StaticPolicyEngine::new();
        // Use user_project scopes so identity is User (not System), which can't bypass scope checks
        let actor = crate::scope::Scope::user_project("alice", "project-a");
        let target = crate::scope::Scope::user_project("bob", "project-b");

        let ctx = PolicyContext {
            action_type: "test".to_string(),
            command: "echo".to_string(),
            args: vec![],
            trust_layer: 3,
            confidence: TrustScore::new(0.9),
            source_event_id: None,
            can_interrupt: true,
            pid: None,
            scope: Some(actor),
            target_scope: Some(target),
            domains: Vec::new(),
            context_budget: None,
            context_fragment_tokens: None,
        };

        assert!(matches!(engine.evaluate(&ctx), PolicyResult::Interrupted { .. }));
    }

    #[test]
    fn test_static_engine_scope_isolation_disabled() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 2
enforce_scope_isolation = false
"#,
        )
        .unwrap();

        let engine = StaticPolicyEngine::from_file(&config_path).unwrap();
        let actor = crate::scope::Scope::project("project-a");
        let target = crate::scope::Scope::project("project-b");

        let ctx = PolicyContext {
            action_type: "test".to_string(),
            command: "echo".to_string(),
            args: vec![],
            trust_layer: 3,
            confidence: TrustScore::new(0.9),
            source_event_id: None,
            can_interrupt: true,
            pid: None,
            scope: Some(actor),
            target_scope: Some(target),
            domains: Vec::new(),
            context_budget: None,
            context_fragment_tokens: None,
        };

        // Scope isolation disabled — should proceed despite cross-project access
        assert!(engine.evaluate(&ctx).is_proceed());
    }

    #[test]
    fn test_static_engine_domain_strict_mode() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 2
domain_allowlist_mode = "strict"
"#,
        )
        .unwrap();

        let engine = StaticPolicyEngine::from_file(&config_path).unwrap();

        // Evil.com not in default allowlist — should be blocked in strict mode
        let ctx = PolicyContext {
            action_type: "test".to_string(),
            command: "curl".to_string(),
            args: vec!["https://evil.com".to_string()],
            trust_layer: 3,
            confidence: TrustScore::new(0.9),
            source_event_id: None,
            can_interrupt: true,
            pid: None,
            scope: None,
            target_scope: None,
            domains: vec!["evil.com".to_string()],
            context_budget: None,
            context_fragment_tokens: None,
        };

        assert!(matches!(engine.evaluate(&ctx), PolicyResult::Interrupted { .. }));
    }

    #[test]
    fn test_static_engine_domain_permissive_mode() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 2
domain_allowlist_mode = "permissive"
"#,
        )
        .unwrap();

        let engine = StaticPolicyEngine::from_file(&config_path).unwrap();

        // Evil.com not in default allowlist — should be allowed in permissive mode
        let ctx = PolicyContext {
            action_type: "test".to_string(),
            command: "curl".to_string(),
            args: vec!["https://evil.com".to_string()],
            trust_layer: 3,
            confidence: TrustScore::new(0.9),
            source_event_id: None,
            can_interrupt: true,
            pid: None,
            scope: None,
            target_scope: None,
            domains: vec!["evil.com".to_string()],
            context_budget: None,
            context_fragment_tokens: None,
        };

        assert!(engine.evaluate(&ctx).is_proceed());
    }

    #[test]
    fn test_static_engine_hot_reload() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");

        // Write initial config with high confidence floor
        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 2
confidence_floor = 0.9
"#,
        )
        .unwrap();

        let mut engine = StaticPolicyEngine::from_file(&config_path).unwrap();

        // Should fail with low confidence
        let ctx = make_context("echo", vec![], 3, 0.7);
        assert!(matches!(engine.evaluate(&ctx), PolicyResult::Interrupted { .. }));

        // Reload with lower confidence floor
        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 2
confidence_floor = 0.5
"#,
        )
        .unwrap();

        engine.reload(&config_path).unwrap();

        // Should now pass with low confidence
        assert!(engine.evaluate(&ctx).is_proceed());
    }

    #[test]
    fn test_static_engine_no_op_reload() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");

        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 2
confidence_floor = 0.6
"#,
        )
        .unwrap();

        let mut engine = StaticPolicyEngine::from_file(&config_path).unwrap();

        // Reload without changes — should not panic or error
        assert!(engine.reload(&config_path).is_ok());
    }

    #[test]
    fn test_static_engine_source_description() {
        let engine = StaticPolicyEngine::new();
        assert_eq!(engine.source_description(), "static (default)");

        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"min_trust_layer = 2"#,
        )
        .unwrap();

        let engine = StaticPolicyEngine::from_file(&config_path).unwrap();
        assert!(engine.source_description().contains("policy.toml"));
    }

    #[test]
    fn test_policy_config_validation_invalid_trust_layer() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"min_trust_layer = 5"#, // invalid: must be 1-3
        )
        .unwrap();

        assert!(PolicyConfig::load(&config_path).is_err());
    }

    #[test]
    fn test_policy_config_validation_invalid_confidence() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"confidence_floor = 1.5"#, // invalid: must be 0-1
        )
        .unwrap();

        assert!(PolicyConfig::load(&config_path).is_err());
    }

    #[test]
    fn test_policy_result_into_gate_result() {
        let proceed = PolicyResult::Proceed.into_gate_result();
        assert!(matches!(proceed, crate::gate_result::GateResult::Proceed));

        let interrupted = PolicyResult::Interrupted { reason: "test".to_string() }
            .into_gate_result();
        assert!(matches!(interrupted, crate::gate_result::GateResult::Interrupted { .. }));

        let pending = PolicyResult::Pending.into_gate_result();
        assert!(matches!(pending, crate::gate_result::GateResult::Pending));

        let dry_run = PolicyResult::DryRun.into_gate_result();
        assert!(matches!(dry_run, crate::gate_result::GateResult::DryRun));

        let revoked = PolicyResult::Revoked { reason: "test".to_string() }
            .into_gate_result();
        assert!(matches!(revoked, crate::gate_result::GateResult::Revoked { .. }));
    }

    #[test]
    fn test_static_engine_review_required_commands() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 2
confidence_floor = 0.5
review_required_commands = ["dd", "fdisk"]
"#,
        )
        .unwrap();

        let engine = StaticPolicyEngine::from_file(&config_path).unwrap();
        let ctx = make_context("dd", vec!["if=/dev/zero".to_string()], 3, 0.9);

        // Note: review_required_commands is stored in config but not yet evaluated by the static engine.
        // This test documents current behavior — commands in this list are NOT automatically pending.
        // They would need to be checked explicitly (future enhancement or gate integration).
        let result = engine.evaluate(&ctx);
        // Currently proceeds because review_required_commands isn't enforced by StaticPolicyEngine yet
        assert!(matches!(result, PolicyResult::Proceed));
    }

    #[test]
    fn test_static_engine_context_budget_exhausted() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 2
max_context_budget_tokens = 1000
"#,
        )
        .unwrap();

        let engine = StaticPolicyEngine::from_file(&config_path).unwrap();

        // Create a budget that's already full
        let budget = crate::context_budget::ContextBudget::new(500); // 500 token budget
        let ctx = PolicyContext {
            action_type: "test".to_string(),
            command: "echo".to_string(),
            args: vec![],
            trust_layer: 3,
            confidence: TrustScore::new(0.9),
            source_event_id: None,
            can_interrupt: true,
            pid: None,
            scope: None,
            target_scope: None,
            domains: Vec::new(),
            context_budget: Some(budget),
            context_fragment_tokens: Some(600), // exceeds remaining budget
        };

        let result = engine.evaluate(&ctx);
        assert!(matches!(result, PolicyResult::ContextExhausted { .. }));
    }

    #[test]
    fn test_static_engine_oversized_fragment() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("policy.toml");
        std::fs::write(
            &config_path,
            r#"
min_trust_layer = 2
max_context_budget_tokens = 10000
max_fragment_tokens = 5000
"#,
        )
        .unwrap();

        let engine = StaticPolicyEngine::from_file(&config_path).unwrap();

        let ctx = PolicyContext {
            action_type: "test".to_string(),
            command: "echo".to_string(),
            args: vec![],
            trust_layer: 3,
            confidence: TrustScore::new(0.9),
            source_event_id: None,
            can_interrupt: true,
            pid: None,
            scope: None,
            target_scope: None,
            domains: Vec::new(),
            context_budget: None,
            context_fragment_tokens: Some(8000), // exceeds 5K limit
        };

        let result = engine.evaluate(&ctx);
        assert!(matches!(result, PolicyResult::OversizedFragment { .. }));
    }
}
