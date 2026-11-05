//! Policy engine types — abstracts static vs. scriptable policy evaluation.
//!
//! Inspired by Go-Sanitized Starlark's sandboxed execution model, but implemented
//! in pure Rust for zero startup cost and compile-time type safety. The `PolicyEngine`
//! trait provides a forward-compatible abstraction: if/when Starlark scripting becomes
//! necessary (e.g., non-Rust policy authors, complex conditional logic), the backend
//! can be swapped without breaking callers.
//!
//! ## Design Rationale
//!
//! Starlark was considered for 9.5 but deferred in favor of Rust-native declarative
//! policies because:
//! - Zero new dependencies (Starlark adds ~15 transitive deps, ~200KB compiled)
//! - No runtime overhead (interpreted execution vs. inlined Rust branches)
//! - Compile-time type safety (policy validation at load time, not runtime errors)
//! - Team fit (Rust-first team has zero Starlark expertise)
//!
//! The `PolicyEngine` trait preserves the option to add a Starlark backend later via
//! a feature flag (`#[cfg(feature = "starlark")]`) without changing the public API.
//!
//! ## Inspiration from Starlark
//!
//! While we don't use Starlark, its design patterns inform our approach:
//! - **Sandboxed execution**: Policies run in an isolated context with limited built-ins
//! - **Deterministic evaluation**: Same inputs always produce same outputs (no randomness)
//! - **Hot-reload**: Policy changes take effect without binary restart
//! - **Declarative configuration**: Policies expressed as data, not code (TOML format)
//!
//! Future Starlark integration would add:
//! - Arbitrary conditionals (`if trust_layer >= 2 and time_between(2am, 5am):`)
//! - Rate limiting and multi-factor approval chains
//! - Non-Rust policy authoring (Python-like syntax for ops teams)

use std::path::{Path, PathBuf};

use crate::context_budget::ContextBudget;
use crate::scope::Scope;
use crate::trust::TrustScore;
use serde::{Deserialize, Serialize};

/// Trait for policy evaluation engines.
///
/// This abstraction allows swapping between static (Rust-native) and scriptable
/// (future Starlark) backends without changing callers. The default implementation
/// is `StaticPolicyEngine` which parameterizes the existing gate logic via TOML config.
pub trait PolicyEngine: Send + Sync {
    /// Evaluate whether an action should proceed, be interrupted, or deferred.
    fn evaluate(&self, ctx: &PolicyContext) -> PolicyResult;

    /// Reload policy from a file path (for hot-reload support).
    /// Returns `Ok(())` on success, `Err(String)` if the new config is invalid.
    fn reload(&mut self, path: &Path) -> Result<(), String>;

    /// Return the current policy source description (e.g., "static", "policy.toml").
    fn source_description(&self) -> String;
}

/// Configuration for static policy evaluation.
///
/// Loaded from TOML format. Mirrors Starlark's declarative approach but uses
/// Rust types validated at load time rather than runtime interpretation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Minimum trust layer required for auto-execute (1-3).
    #[serde(default = "default_min_trust_layer")]
    pub min_trust_layer: u32,

    /// Confidence floor below which actions are interrupted.
    #[serde(default = "default_confidence_floor")]
    pub confidence_floor: f64,

    /// Maximum context budget in tokens (0 = unlimited).
    #[serde(default)]
    pub max_context_budget_tokens: usize,

    /// Maximum per-fragment token count (0 = use default 10K).
    #[serde(default)]
    pub max_fragment_tokens: usize,

    /// Whether to enforce scope isolation.
    #[serde(default = "default_true")]
    pub enforce_scope_isolation: bool,

    /// Cross-scope auth TTL in seconds (0 = no expiry check).
    #[serde(default = "default_cross_scope_ttl")]
    pub cross_scope_auth_ttl_seconds: u64,

    /// Dangerous commands that are always denied.
    #[serde(default)]
    pub dangerous_commands: Vec<String>,

    /// Commands requiring review (pending) instead of auto-approve.
    #[serde(default)]
    pub review_required_commands: Vec<String>,

    /// Domain allowlist mode: "strict" (deny unknown), "permissive" (allow all).
    #[serde(default = "default_domain_mode")]
    pub domain_allowlist_mode: DomainAllowlistMode,

    /// Path to the policy config file (for hot-reload tracking).
    #[serde(skip, default)]
    pub source_path: Option<PathBuf>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            min_trust_layer: default_min_trust_layer(),
            confidence_floor: default_confidence_floor(),
            max_context_budget_tokens: 0, // unlimited by default
            max_fragment_tokens: 0,       // use default 10K
            enforce_scope_isolation: true,
            cross_scope_auth_ttl_seconds: default_cross_scope_ttl(),
            dangerous_commands: Vec::new(),
            review_required_commands: Vec::new(),
            domain_allowlist_mode: default_domain_mode(),
            source_path: None,
        }
    }
}

impl PolicyConfig {
    /// Load policy config from a TOML file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let mut config: Self = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse policy config at {}: {}", path.display(), e))?;
        config.source_path = Some(path.to_path_buf());

        // Validate constraints
        if config.min_trust_layer < 1 || config.min_trust_layer > 3 {
            return Err("min_trust_layer must be between 1 and 3".to_string());
        }
        if config.confidence_floor < 0.0 || config.confidence_floor > 1.0 {
            return Err("confidence_floor must be between 0.0 and 1.0".to_string());
        }

        Ok(config)
    }

    /// Save policy config to a TOML file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize policy config: {}", e))?;
        std::fs::write(path, content)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))
    }

    /// Check if this config differs from another (for hot-reload detection).
    pub fn differs_from(&self, other: &Self) -> bool {
        self.min_trust_layer != other.min_trust_layer
            || (self.confidence_floor - other.confidence_floor).abs() > f64::EPSILON
            || self.max_context_budget_tokens != other.max_context_budget_tokens
            || self.enforce_scope_isolation != other.enforce_scope_isolation
            || self.dangerous_commands != other.dangerous_commands
            || self.review_required_commands != other.review_required_commands
    }
}

/// Domain allowlist enforcement mode.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DomainAllowlistMode {
    /// Deny all domains not explicitly allowed (deny-by-default).
    #[default]
    Strict,
    /// Allow all domains but log unlisted ones (allow-by-default with audit).
    Permissive,
}

/// Context passed to policy evaluation.
///
/// Mirrors `GateContext` but is policy-engine-agnostic — it doesn't depend on
/// the gate's internal types, making it easy to swap in a Starlark backend later.
#[derive(Debug, Clone)]
pub struct PolicyContext {
    /// Type of action being evaluated (e.g., "exec", "write", "delete").
    pub action_type: String,

    /// Command name being executed.
    pub command: String,

    /// Arguments to the command.
    pub args: Vec<String>,

    /// Trust layer of the calling context (1-3).
    pub trust_layer: u32,

    /// Confidence score for this action (0.0 - 1.0).
    pub confidence: TrustScore,

    /// Source event ID for provenance tracking.
    pub source_event_id: Option<String>,

    /// Whether the action can be interrupted mid-execution.
    pub can_interrupt: bool,

    /// PID of the calling process (for per-process trust).
    pub pid: Option<i32>,

    /// Scope of the actor performing the action.
    pub scope: Option<Scope>,

    /// Scope of the target resource being accessed.
    pub target_scope: Option<Scope>,

    /// Known domains/URLs associated with this action.
    pub domains: Vec<String>,

    /// Cumulative context budget for this action's scope.
    pub context_budget: Option<ContextBudget>,

    /// Token count of the context that would be injected by this action.
    pub context_fragment_tokens: Option<usize>,
}

impl PolicyContext {
    /// Create a new policy context from gate-compatible inputs.
    pub fn new(
        action_type: &str,
        command: &str,
        args: Vec<String>,
        trust_layer: u32,
        confidence: TrustScore,
    ) -> Self {
        Self {
            action_type: action_type.to_string(),
            command: command.to_string(),
            args,
            trust_layer,
            confidence,
            source_event_id: None,
            can_interrupt: true,
            pid: None,
            scope: None,
            target_scope: None,
            domains: Vec::new(),
            context_budget: None,
            context_fragment_tokens: None,
        }
    }
}

/// Result of policy evaluation.
///
/// Maps to `GateResult` but is defined separately so the policy engine trait
/// doesn't couple to gate internals (enabling future Starlark backend).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyResult {
    /// Action is approved — proceed with execution.
    Proceed,

    /// Action was interrupted — do not execute.
    Interrupted { reason: String },

    /// Action requires human review before proceeding.
    Pending,

    /// Dry-run mode — would execute but don't actually run.
    DryRun,

    /// Process trust has been revoked.
    Revoked { reason: String },

    /// Context budget exhausted.
    ContextExhausted {
        used: usize,
        budget: usize,
        remaining: usize,
    },

    /// Context fragment exceeds per-fragment hard cap.
    OversizedFragment { actual: usize, max: usize },
}

impl PolicyResult {
    /// Returns `true` if the policy allows execution to proceed.
    pub fn is_proceed(&self) -> bool {
        matches!(self, PolicyResult::Proceed)
    }

    /// Convert a `PolicyResult` into a `GateResult`.
    ///
    /// This bridges the policy engine abstraction back to gate internals.
    /// A future Starlark backend would implement its own conversion or return
    /// `GateResult` directly if it lives in the same crate.
    pub fn into_gate_result(self) -> crate::gate_result::GateResult {
        match self {
            PolicyResult::Proceed => crate::gate_result::GateResult::Proceed,
            PolicyResult::Interrupted { reason } => {
                crate::gate_result::GateResult::Interrupted { reason }
            }
            PolicyResult::Pending => crate::gate_result::GateResult::Pending,
            PolicyResult::DryRun => crate::gate_result::GateResult::DryRun,
            PolicyResult::Revoked { reason } => crate::gate_result::GateResult::Revoked { reason },
            PolicyResult::ContextExhausted {
                used,
                budget,
                remaining,
            } => crate::gate_result::GateResult::ContextExhausted {
                used,
                budget,
                remaining,
            },
            PolicyResult::OversizedFragment { actual, max } => {
                crate::gate_result::GateResult::OversizedFragment { actual, max }
            }
        }
    }
}

/// Helper functions for default values.
fn default_min_trust_layer() -> u32 {
    2
}
fn default_confidence_floor() -> f64 {
    0.6
}
fn default_cross_scope_ttl() -> u64 {
    3600
}
fn default_domain_mode() -> DomainAllowlistMode {
    DomainAllowlistMode::Strict
}
fn default_true() -> bool {
    true
}
