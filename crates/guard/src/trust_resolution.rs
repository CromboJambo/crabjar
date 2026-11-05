//! Trust resolution types — requested vs effective trust.
//!
//! Distinguishes between what a tool *requests* and what the *effective*
//! trust level is after policy resolution.

use crate::scope::Scope;
use std::fmt;

/// The trust layer a tool or agent is requesting.
/// This is the *claimed* trust level, not necessarily what they get.
#[derive(Debug, Clone, PartialEq)]
pub struct RequestedTrust {
    /// The trust layer being requested
    pub layer: u32,
    /// The confidence backing this request (0.0 to 1.0)
    pub confidence: f64,
    /// The source of the request (tool name, agent name, etc.)
    pub source: String,
    /// Timestamp of the request
    pub requested_at: i64,
}

impl RequestedTrust {
    pub fn new(layer: u32, confidence: f64, source: impl Into<String>) -> Self {
        Self {
            layer,
            confidence: confidence.clamp(0.0, 1.0),
            source: source.into(),
            requested_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a low-trust request (raw/observed layer).
    pub fn raw(source: impl Into<String>) -> Self {
        Self::new(0, 0.1, source)
    }

    /// Create a working-trust request.
    pub fn working(source: impl Into<String>) -> Self {
        Self::new(3, 0.85, source)
    }

    /// Create an annealed-trust request.
    pub fn annealed(source: impl Into<String>) -> Self {
        Self::new(4, 0.95, source)
    }
}

impl fmt::Display for RequestedTrust {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "requested_trust(layer={}, confidence={:.3}, source={})",
            self.layer, self.confidence, self.source
        )
    }
}

/// The effective trust level after policy resolution.
///
/// This is the *actual* trust level granted after applying all policies.
/// It may be lower than the requested level.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveTrust {
    /// The effective trust layer
    pub layer: u32,
    /// The effective confidence (0.0 to 1.0)
    pub confidence: f64,
    /// The policy that determined this level
    pub determined_by: String,
}

impl EffectiveTrust {
    pub fn new(layer: u32, confidence: f64, determined_by: impl Into<String>) -> Self {
        Self {
            layer,
            confidence: confidence.clamp(0.0, 1.0),
            determined_by: determined_by.into(),
        }
    }

    /// Check if this effective trust allows auto-execution.
    pub fn allows_auto_execute(&self) -> bool {
        // Layer 4 (annealed) allows auto-execute
        self.layer >= 4
    }

    /// Check if this effective trust requires review.
    pub fn requires_review(&self) -> bool {
        // Layers 0-3 require review
        self.layer < 4
    }
}

impl fmt::Display for EffectiveTrust {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "effective_trust(layer={}, confidence={:.3}, by={})",
            self.layer, self.confidence, self.determined_by
        )
    }
}

/// Policy source in the resolution chain.
/// Policies are applied in order — earlier policies can override later ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicySource {
    /// No policy — use default trust layer
    None,
    /// Default fallback policy
    Default,
    /// User-level policy (highest priority)
    User(String),
    /// Project-level policy
    Project(String),
    /// Scope-based policy
    Scope,
}

impl fmt::Display for PolicySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicySource::None => write!(f, "none"),
            PolicySource::Default => write!(f, "default"),
            PolicySource::User(name) => write!(f, "user:{}", name),
            PolicySource::Project(name) => write!(f, "project:{}", name),
            PolicySource::Scope => write!(f, "scope"),
        }
    }
}

/// A single policy in the resolution chain.
/// Each policy can cap, boost, or pass through the trust layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// The source of this policy
    pub source: PolicySource,
    /// Maximum trust layer this policy allows
    pub max_layer: u32,
    /// Minimum trust layer this policy requires
    pub min_layer: u32,
    /// Whether this policy capping is active
    pub active: bool,
    /// Description of what this policy does
    pub description: String,
}

impl fmt::Display for Policy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}(max={}, min={}, active={})",
            self.source, self.max_layer, self.min_layer, self.active
        )
    }
}

impl Policy {
    /// Create a capping policy (limits maximum trust layer).
    pub fn cap(source: PolicySource, max_layer: u32, description: impl Into<String>) -> Self {
        Self {
            source,
            max_layer,
            min_layer: 0,
            active: true,
            description: description.into(),
        }
    }

    /// Create a minimum trust policy (requires minimum trust layer).
    pub fn require(source: PolicySource, min_layer: u32, description: impl Into<String>) -> Self {
        Self {
            source,
            max_layer: u32::MAX,
            min_layer,
            active: true,
            description: description.into(),
        }
    }

    /// Apply this policy to a trust layer, returning the capped/raised layer.
    pub fn apply(&self, layer: u32) -> u32 {
        if !self.active {
            return layer;
        }
        let capped = layer.min(self.max_layer);
        capped.max(self.min_layer)
    }

    /// Check if this policy would change the trust layer.
    pub fn would_change(&self, layer: u32) -> bool {
        if !self.active {
            return false;
        }
        layer != self.apply(layer)
    }
}

/// The policy chain used for trust resolution.
/// Policies are applied in order — user > project > scope > default.
#[derive(Debug, Clone)]
pub struct PolicyChain {
    /// User-level policies (highest priority)
    pub user_policies: Vec<Policy>,
    /// Project-level policies
    pub project_policies: Vec<Policy>,
    /// Scope-level policies
    pub scope_policies: Vec<Policy>,
    /// Default policy (lowest priority)
    pub default_policy: Option<Policy>,
}

impl PolicyChain {
    pub fn new() -> Self {
        Self {
            user_policies: Vec::new(),
            project_policies: Vec::new(),
            scope_policies: Vec::new(),
            default_policy: None,
        }
    }

    /// Add a user policy (highest priority).
    pub fn with_user_policy(mut self, policy: Policy) -> Self {
        self.user_policies.push(policy);
        self
    }

    /// Add a project policy.
    pub fn with_project_policy(mut self, policy: Policy) -> Self {
        self.project_policies.push(policy);
        self
    }

    /// Add a scope policy.
    pub fn with_scope_policy(mut self, policy: Policy) -> Self {
        self.scope_policies.push(policy);
        self
    }

    /// Set the default policy (lowest priority).
    pub fn with_default_policy(mut self, policy: Policy) -> Self {
        self.default_policy = Some(policy);
        self
    }

    /// Resolve the effective trust from a requested trust.
    ///
    /// Resolution order: user → project → scope → default → requested
    /// Each policy can cap or raise the trust layer.
    /// The result is the *effective* trust level granted.
    pub fn resolve(&self, requested: &RequestedTrust, scope: &Scope) -> EffectiveTrust {
        let mut effective_layer = requested.layer;
        let mut determined_by = String::from("requested");
        let mut effective_confidence = requested.confidence;

        // Apply user policies (highest priority)
        for policy in &self.user_policies {
            if policy.would_change(effective_layer) {
                effective_layer = policy.apply(effective_layer);
                determined_by = format!("user-policy:{}", policy.source);
            }
        }

        // Apply project policies
        if scope.project.is_some() {
            for policy in &self.project_policies {
                if policy.would_change(effective_layer) {
                    effective_layer = policy.apply(effective_layer);
                    determined_by = format!("project-policy:{}", policy.source);
                }
            }
        }

        // Apply scope policies
        for policy in &self.scope_policies {
            if policy.would_change(effective_layer) {
                effective_layer = policy.apply(effective_layer);
                determined_by = format!("scope-policy:{}", policy.source);
            }
        }

        // Apply default policy (lowest priority)
        if let Some(ref policy) = self.default_policy
            && policy.would_change(effective_layer)
        {
            effective_layer = policy.apply(effective_layer);
            determined_by = format!("default-policy:{policy}");
        }

        // Compute effective confidence based on the gap between requested and effective
        if effective_layer < requested.layer && requested.layer > 0 {
            let ratio = effective_layer as f64 / requested.layer as f64;
            effective_confidence = requested.confidence * ratio;
        }

        EffectiveTrust::new(effective_layer, effective_confidence, determined_by)
    }
}

impl Default for PolicyChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Trust resolution audit log entry.
///
/// Records how the effective trust was derived from the requested trust.
/// This provides an audit trail for authorization decisions.
#[derive(Debug, Clone)]
pub struct TrustResolution {
    /// The requested trust level
    pub requested: RequestedTrust,
    /// The effective trust level after resolution
    pub effective: EffectiveTrust,
    /// Policies that changed the trust level
    pub applied_policies: Vec<String>,
    /// The scope used for resolution
    pub scope: Scope,
    /// Timestamp of resolution
    pub resolved_at: i64,
}

impl TrustResolution {
    pub fn new(
        requested: RequestedTrust,
        effective: EffectiveTrust,
        applied_policies: Vec<String>,
        scope: Scope,
    ) -> Self {
        Self {
            requested,
            effective,
            applied_policies,
            scope,
            resolved_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Check if trust was downgraded during resolution.
    pub fn was_downgraded(&self) -> bool {
        self.effective.layer < self.requested.layer
    }

    /// Format as a human-readable audit log entry.
    pub fn audit_log(&self) -> String {
        let downgrade = if self.was_downgraded() {
            format!(
                " DOWNGRADED from layer {} to {} ({} policies applied)",
                self.requested.layer,
                self.effective.layer,
                self.applied_policies.len()
            )
        } else {
            String::new()
        };

        format!(
            "TRUST RESOLUTION: {} → {}{}",
            self.requested, self.effective, downgrade
        )
    }
}

/// Trust resolver — the main entry point for trust resolution.
pub struct TrustResolver {
    /// The policy chain
    pub policy_chain: PolicyChain,
}

impl TrustResolver {
    pub fn new(policy_chain: PolicyChain) -> Self {
        Self { policy_chain }
    }

    /// Resolve trust with a scope and return the resolution record.
    pub fn resolve_with_scope(&self, requested: &RequestedTrust, scope: &Scope) -> TrustResolution {
        let effective = self.policy_chain.resolve(requested, scope);
        let applied_policies = self.collect_applied_policies(requested.layer, &effective);

        TrustResolution::new(
            requested.clone(),
            effective,
            applied_policies,
            scope.clone(),
        )
    }

    /// Resolve trust with default scope (no project/tenant/thread).
    pub fn resolve(&self, requested: &RequestedTrust) -> TrustResolution {
        let scope = Scope::project("default");
        self.resolve_with_scope(requested, &scope)
    }

    /// Collect the policies that actually changed the trust level.
    fn collect_applied_policies(
        &self,
        original_layer: u32,
        _effective: &EffectiveTrust,
    ) -> Vec<String> {
        let mut applied = Vec::new();

        for policy in &self.policy_chain.user_policies {
            if policy.would_change(original_layer) {
                applied.push(format!("user:{}", policy.source));
            }
        }

        for policy in &self.policy_chain.project_policies {
            if policy.would_change(original_layer) {
                applied.push(format!("project:{}", policy.source));
            }
        }

        for policy in &self.policy_chain.scope_policies {
            if policy.would_change(original_layer) {
                applied.push(format!("scope:{}", policy.source));
            }
        }

        if let Some(ref policy) = self.policy_chain.default_policy
            && policy.would_change(original_layer)
        {
            applied.push(format!("default:{policy}"));
        }

        applied
    }
}
