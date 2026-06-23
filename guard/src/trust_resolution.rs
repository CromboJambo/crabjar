/// Requested-vs-effective trust resolution.
///
/// Distinguishes between what a tool *requests* and what the *effective*
/// trust level is after policy resolution.
///
/// ## Design
///
/// IronClaw's `ironclaw_trust` separates `requested_trust` from
/// `effective_trust`. Crabjar's guard has a simpler deny/pending/proceed
/// model without this resolution layer.
///
/// ## Resolution Chain
///
/// 1. **Requested trust** — what the tool/agent asks for
/// 2. **Scope resolution** — does the scope allow this trust level?
/// 3. **Project policy** — per-project overrides
/// 4. **User policy** — user-level overrides
/// 5. **Default** — fallback trust layer
///
/// ## Example
///
/// ```ignore
/// let requested = RequestedTrust::new(3, 0.85, "agent");
/// let effective = TrustResolver::new(chain).resolve(&requested, &scope);
/// // effective might be lower than requested after policy resolution
/// ```
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
    pub fn new(
        layer: u32,
        confidence: f64,
        determined_by: impl Into<String>,
    ) -> Self {
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

        EffectiveTrust::new(
            effective_layer,
            effective_confidence,
            determined_by,
        )
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
                self.requested.layer, self.effective.layer, self.applied_policies.len()
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
    pub fn resolve_with_scope(
        &self,
        requested: &RequestedTrust,
        scope: &Scope,
    ) -> TrustResolution {
        let effective = self.policy_chain.resolve(requested, scope);
        let applied_policies = self.collect_applied_policies(requested.layer, &effective);

        TrustResolution::new(requested.clone(), effective, applied_policies, scope.clone())
    }

    /// Resolve trust with default scope (no project/tenant/thread).
    pub fn resolve(&self, requested: &RequestedTrust) -> TrustResolution {
        let scope = Scope::project("default");
        self.resolve_with_scope(requested, &scope)
    }

    /// Collect the policies that actually changed the trust level.
    fn collect_applied_policies(&self, original_layer: u32, _effective: &EffectiveTrust) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requested_trust_creation() {
        let rt = RequestedTrust::new(3, 0.85, "test-tool");
        assert_eq!(rt.layer, 3);
        assert_eq!(rt.confidence, 0.85);
        assert_eq!(rt.source, "test-tool");
    }

    #[test]
    fn test_effective_trust_auto_execute() {
        let high = EffectiveTrust::new(4, 0.95, "policy");
        assert!(high.allows_auto_execute());
        assert!(!high.requires_review());

        let low = EffectiveTrust::new(2, 0.6, "policy");
        assert!(!low.allows_auto_execute());
        assert!(low.requires_review());
    }

    #[test]
    fn test_policy_cap() {
        let policy = Policy::cap(
            PolicySource::Project("test-project".into()),
            2,
            "max layer 2",
        );
        assert_eq!(policy.apply(4), 2);
        assert_eq!(policy.apply(1), 1);
        assert!(policy.would_change(4));
        assert!(!policy.would_change(1));
    }

    #[test]
    fn test_policy_require_minimum() {
        let policy = Policy::require(
            PolicySource::Scope,
            2,
            "min layer 2",
        );
        assert_eq!(policy.apply(0), 2);
        assert_eq!(policy.apply(4), 4);
        assert!(policy.would_change(0));
        assert!(!policy.would_change(4));
    }

    #[test]
    fn test_policy_chain_no_change() {
        let chain = PolicyChain::new();
        let requested = RequestedTrust::new(3, 0.85, "test");
        let scope = Scope::project("test-project");
        let effective = chain.resolve(&requested, &scope);
        assert_eq!(effective.layer, 3);
        assert_eq!(effective.confidence, 0.85);
    }

    #[test]
    fn test_policy_chain_caps_trust() {
        let chain = PolicyChain::new()
            .with_project_policy(Policy::cap(
                PolicySource::Project("secure-project".into()),
                2,
                "secure project cap",
            ));

        let requested = RequestedTrust::new(4, 0.95, "agent");
        let scope = Scope::project("secure-project");
        let effective = chain.resolve(&requested, &scope);

        assert_eq!(effective.layer, 2);
        assert!(effective.requires_review());
        assert!(effective.determined_by.contains("project-policy"));
    }

    #[test]
    fn test_policy_chain_minimum() {
        let chain = PolicyChain::new()
            .with_scope_policy(Policy::require(
                PolicySource::Scope,
                2,
                "min trust 2",
            ));

        let requested = RequestedTrust::new(0, 0.1, "raw-tool");
        let scope = Scope::project("test");
        let effective = chain.resolve(&requested, &scope);

        assert_eq!(effective.layer, 2);
    }

    #[test]
    fn test_policy_chain_user_overrides_project() {
        let chain = PolicyChain::new()
            .with_user_policy(Policy::cap(
                PolicySource::User("alice".into()),
                1,
                "alice's personal cap",
            ))
            .with_project_policy(Policy::cap(
                PolicySource::Project("project".into()),
                3,
                "project cap",
            ));

        let requested = RequestedTrust::new(4, 0.95, "agent");
        let scope = Scope::project("project");
        let effective = chain.resolve(&requested, &scope);

        // User policy (layer 1) is more restrictive than project policy (layer 3)
        assert_eq!(effective.layer, 1);
    }

    #[test]
    fn test_trust_resolution_audit_log() {
        let chain = PolicyChain::new()
            .with_project_policy(Policy::cap(
                PolicySource::Project("secure".into()),
                2,
                "secure cap",
            ));

        let requested = RequestedTrust::new(4, 0.95, "agent");
        let scope = Scope::project("secure");
        let resolver = TrustResolver::new(chain);
        let resolution = resolver.resolve_with_scope(&requested, &scope);

        assert!(resolution.was_downgraded());
        let log = resolution.audit_log();
        assert!(log.contains("DOWNGRADED"));
        assert!(log.contains("layer 4"));
        assert!(log.contains("to 2"));
    }

    #[test]
    fn test_trust_resolution_no_downgrade() {
        let chain = PolicyChain::new();
        let requested = RequestedTrust::new(3, 0.85, "tool");
        let scope = Scope::project("test");
        let resolver = TrustResolver::new(chain);
        let resolution = resolver.resolve_with_scope(&requested, &scope);

        assert!(!resolution.was_downgraded());
        let log = resolution.audit_log();
        assert!(!log.contains("DOWNGRADED"));
    }

    #[test]
    fn test_effective_confidence_reduction_on_downgrade() {
        let chain = PolicyChain::new()
            .with_project_policy(Policy::cap(
                PolicySource::Project("project".into()),
                2,
                "cap",
            ));

        let requested = RequestedTrust::new(4, 0.90, "agent");
        let scope = Scope::project("project");
        let effective = chain.resolve(&requested, &scope);

        // Confidence should be reduced: 0.90 * (2/4) = 0.45
        assert!(effective.confidence < 0.90);
        assert!(effective.confidence > 0.0);
    }

    #[test]
    fn test_policy_source_display() {
        assert_eq!(format!("{}", PolicySource::None), "none");
        assert_eq!(format!("{}", PolicySource::Default), "default");
        assert!(format!("{}", PolicySource::User("alice".into())).contains("user:alice"));
        assert!(format!("{}", PolicySource::Project("proj".into())).contains("project:proj"));
        assert_eq!(format!("{}", PolicySource::Scope), "scope");
    }

    #[test]
    fn test_requested_trust_display() {
        let rt = RequestedTrust::new(3, 0.85, "test");
        let display = format!("{}", rt);
        assert!(display.contains("requested_trust"));
        assert!(display.contains("layer=3"));
        assert!(display.contains("confidence=0.850"));
        assert!(display.contains("source=test"));
    }

    #[test]
    fn test_effective_trust_display() {
        let et = EffectiveTrust::new(2, 0.45, "project-policy");
        let display = format!("{}", et);
        assert!(display.contains("effective_trust"));
        assert!(display.contains("layer=2"));
        assert!(display.contains("confidence=0.450"));
        assert!(display.contains("by=project-policy"));
    }
}
