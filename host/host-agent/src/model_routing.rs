/// Model routing for the agent loop — phase-specific model selection.
///
/// Different phases of the ReAct loop benefit from different models:
/// - `plan` / `reflect` → reasoning model (stronger, slower)
/// - `observe` / `verify` → fast model (lighter, quicker)
/// - `execute` / `understand` → heuristic or fast model
///
/// This module provides `ModelRouter` which holds per-phase backend configs
/// and dispatches inference calls to the appropriate backend.
use std::collections::HashMap;
use std::fmt;

use crate::inference::InferenceBackend;
use crate::inference::InferenceError;

// ---------------------------------------------------------------------------
// Phase enum
// ---------------------------------------------------------------------------

/// Phase of the agent loop — determines which model to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoopPhase {
    Observe,
    Understand,
    Plan,
    Execute,
    Verify,
    Reflect,
}

impl LoopPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Understand => "understand",
            Self::Plan => "plan",
            Self::Execute => "execute",
            Self::Verify => "verify",
            Self::Reflect => "reflect",
        }
    }

    /// Return the list of all phases in execution order.
    pub fn all() -> [LoopPhase; 6] {
        [
            LoopPhase::Observe,
            LoopPhase::Understand,
            LoopPhase::Plan,
            LoopPhase::Execute,
            LoopPhase::Verify,
            LoopPhase::Reflect,
        ]
    }
}

impl fmt::Display for LoopPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Phase backend kind
// ---------------------------------------------------------------------------

/// Which backend to use for a phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseBackendKind {
    /// Deterministic heuristic (no model).
    Heuristic,
    /// HTTP backend with specific endpoint/model.
    Http {
        endpoint: String,
        model: String,
        api_key: Option<String>,
    },
}

impl PhaseBackendKind {
    fn is_http(&self) -> bool {
        matches!(self, PhaseBackendKind::Http { .. })
    }
}

// ---------------------------------------------------------------------------
// Phase config
// ---------------------------------------------------------------------------

/// Configuration for a single phase's model routing.
#[derive(Debug, Clone)]
pub struct PhaseConfig {
    /// Which backend to use for this phase.
    pub backend: PhaseBackendKind,
}

impl PhaseConfig {
    /// Use heuristic (deterministic stub) for this phase.
    pub fn heuristic() -> Self {
        Self {
            backend: PhaseBackendKind::Heuristic,
        }
    }

    /// Use HTTP backend with the given model.
    pub fn http(endpoint: String, model: String, api_key: Option<String>) -> Self {
        Self {
            backend: PhaseBackendKind::Http {
                endpoint,
                model,
                api_key,
            },
        }
    }
}

impl Default for PhaseConfig {
    fn default() -> Self {
        Self::heuristic()
    }
}

// ---------------------------------------------------------------------------
// Model router
// ---------------------------------------------------------------------------

/// Routes inference requests to phase-specific backends.
///
/// Holds a map of `LoopPhase → PhaseConfig` and a default config.
/// When a phase has no explicit config, falls back to the default.
#[derive(Debug, Clone)]
pub struct ModelRouter {
    phases: HashMap<LoopPhase, PhaseConfig>,
    default: PhaseConfig,
}

impl ModelRouter {
    /// Create a new router with only the default config.
    pub fn new(default: PhaseConfig) -> Self {
        Self {
            phases: HashMap::new(),
            default,
        }
    }

    /// Set phase-specific config.
    pub fn with_phase(mut self, phase: LoopPhase, config: PhaseConfig) -> Self {
        self.phases.insert(phase, config);
        self
    }

    /// Set phase-specific config (builder style).
    pub fn set_phase(&mut self, phase: LoopPhase, config: PhaseConfig) {
        self.phases.insert(phase, config);
    }

    /// Get the config for a phase (resolves to default if not set).
    pub fn config_for(&self, phase: LoopPhase) -> &PhaseConfig {
        self.phases
            .get(&phase)
            .unwrap_or(&self.default)
    }

    /// Get the backend kind for a phase.
    pub fn backend_for(&self, phase: LoopPhase) -> &PhaseBackendKind {
        &self.config_for(phase).backend
    }

    /// Check if a phase uses an HTTP backend.
    pub fn is_http(&self, phase: LoopPhase) -> bool {
        self.backend_for(phase).is_http()
    }

    /// Get all configured phases (excluding default).
    pub fn configured_phases(&self) -> Vec<LoopPhase> {
        self.phases.keys().copied().collect()
    }

    /// Create a default router optimized for the agent loop:
    /// - plan/reflect → http with reasoning model
    /// - others → heuristic
    pub fn default_for_loop(http_endpoint: String, http_model: String, http_api_key: Option<String>) -> Self {
        let mut router = Self::new(PhaseConfig::heuristic());

        // Plan and reflect benefit from a stronger reasoning model
        let reasoning = PhaseConfig::http(http_endpoint, http_model, http_api_key);
        router.set_phase(LoopPhase::Plan, reasoning.clone());
        router.set_phase(LoopPhase::Reflect, reasoning);

        router
    }

    /// Build a backend for the given phase.
    ///
    /// Returns `None` if the phase uses heuristic backend.
    pub fn build_backend(&self, phase: LoopPhase) -> Option<Box<dyn InferenceBackend>> {
        match self.backend_for(phase) {
            PhaseBackendKind::Heuristic => None,
            PhaseBackendKind::Http {
                endpoint,
                model,
                api_key,
            } => {
                if endpoint.is_empty() {
                    tracing::warn!(phase = %phase, "HTTP backend configured but endpoint is empty; falling back to heuristic");
                    return None;
                }
                Some(Box::new(
                    crate::inference::HttpBackend::new(endpoint, model, api_key.clone()),
                ))
            }
        }
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new(PhaseConfig::heuristic())
    }
}

// ---------------------------------------------------------------------------
// Inference with phase routing
// ---------------------------------------------------------------------------

/// Run inference for a specific phase using the router.
///
/// If the phase has an HTTP backend configured and the endpoint is valid,
/// uses that backend. Otherwise falls back to heuristic.
pub async fn phase_infer(
    router: &ModelRouter,
    phase: LoopPhase,
    prompt: &str,
) -> Result<String, InferenceError> {
    if let Some(backend) = router.build_backend(phase) {
        return backend.infer(prompt).await;
    }

    // Fall back to heuristic
    crate::inference::HeuristicBackend.infer(prompt).await
}

// ---------------------------------------------------------------------------
// Phase-specific config helpers
// ---------------------------------------------------------------------------

/// Builder for common phase config patterns.
#[derive(Debug, Clone)]
pub struct PhaseBuilder {
    plan_model: Option<String>,
    plan_endpoint: Option<String>,
    plan_api_key: Option<String>,
    reflect_model: Option<String>,
    reflect_endpoint: Option<String>,
    reflect_api_key: Option<String>,
}

impl PhaseBuilder {
    /// Create a builder with reasoning models for plan/reflect phases.
    pub fn reasoning_for_planning(plan_endpoint: String, plan_model: String, plan_api_key: Option<String>) -> Self {
        Self {
            plan_model: Some(plan_model),
            plan_endpoint: Some(plan_endpoint),
            plan_api_key: plan_api_key.clone(),
            reflect_model: None,
            reflect_endpoint: None,
            reflect_api_key: None,
        }
    }

    /// Set explicit reflect phase config (separate from plan).
    pub fn with_reflect(mut self, endpoint: String, model: String, api_key: Option<String>) -> Self {
        self.reflect_endpoint = Some(endpoint);
        self.reflect_model = Some(model);
        self.reflect_api_key = api_key;
        self
    }

    /// Build the ModelRouter.
    pub fn build(self) -> ModelRouter {
        let mut router = ModelRouter::new(PhaseConfig::heuristic());

        // Plan phase
        if let Some(ref endpoint) = self.plan_endpoint {
            let model = self
                .plan_model
                .clone()
                .unwrap_or_else(|| "gpt-4o-mini".to_string());
            let config = PhaseConfig::http(endpoint.clone(), model, self.plan_api_key.clone());
            router.set_phase(LoopPhase::Plan, config);
        }

        // Reflect phase
        if let Some(ref endpoint) = self.reflect_endpoint {
            let model = self
                .reflect_model
                .clone()
                .unwrap_or_else(|| "gpt-4o-mini".to_string());
            let config = PhaseConfig::http(endpoint.clone(), model, self.reflect_api_key);
            router.set_phase(LoopPhase::Reflect, config);
        } else if let Some(ref endpoint) = self.plan_endpoint {
            // Fall back to plan config for reflect
            let model = self
                .plan_model
                .clone()
                .unwrap_or_else(|| "gpt-4o-mini".to_string());
            let config = PhaseConfig::http(endpoint.clone(), model, self.plan_api_key);
            router.set_phase(LoopPhase::Reflect, config);
        }

        router
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_display() {
        assert_eq!(format!("{}", LoopPhase::Observe), "observe");
        assert_eq!(format!("{}", LoopPhase::Plan), "plan");
        assert_eq!(format!("{}", LoopPhase::Reflect), "reflect");
    }

    #[test]
    fn test_phase_all() {
        let phases = LoopPhase::all();
        assert_eq!(phases.len(), 6);
        assert_eq!(phases[0], LoopPhase::Observe);
        assert_eq!(phases[5], LoopPhase::Reflect);
    }

    #[test]
    fn test_phase_config_default_is_heuristic() {
        let config = PhaseConfig::default();
        assert!(matches!(config.backend, PhaseBackendKind::Heuristic));
    }

    #[test]
    fn test_phase_config_http() {
        let config = PhaseConfig::http(
            "http://localhost:1234".into(),
            "gpt-4".into(),
            Some("key123".into()),
        );
        assert!(config.backend.is_http());
    }

    #[test]
    fn test_router_default_config() {
        let router = ModelRouter::new(PhaseConfig::heuristic());
        assert!(!router.is_http(LoopPhase::Observe));
        assert!(!router.is_http(LoopPhase::Plan));
    }

    #[test]
    fn test_router_phase_override() {
        let http_config = PhaseConfig::http(
            "http://localhost:1234".into(),
            "gpt-4".into(),
            Some("key".into()),
        );
        let mut router = ModelRouter::new(PhaseConfig::heuristic());
        router.set_phase(LoopPhase::Plan, http_config);

        assert!(!router.is_http(LoopPhase::Observe)); // default
        assert!(router.is_http(LoopPhase::Plan));
    }

    #[test]
    fn test_router_build_backend_http() {
        let config = PhaseConfig::http(
            "http://localhost:1234".into(),
            "gpt-4".into(),
            None,
        );
        let mut router = ModelRouter::new(PhaseConfig::heuristic());
        router.set_phase(LoopPhase::Plan, config);

        let backend = router.build_backend(LoopPhase::Plan);
        assert!(backend.is_some());
    }

    #[test]
    fn test_router_build_backend_heuristic() {
        let router = ModelRouter::new(PhaseConfig::heuristic());
        let backend = router.build_backend(LoopPhase::Observe);
        assert!(backend.is_none());
    }

    #[test]
    fn test_router_build_backend_empty_endpoint() {
        let config = PhaseConfig::http(
            "".into(),
            "gpt-4".into(),
            None,
        );
        let mut router = ModelRouter::new(PhaseConfig::heuristic());
        router.set_phase(LoopPhase::Plan, config);

        // Empty endpoint should return None (fallback to heuristic)
        let backend = router.build_backend(LoopPhase::Plan);
        assert!(backend.is_none());
    }

    #[test]
    fn test_default_for_loop() {
        let router = ModelRouter::default_for_loop(
            "http://localhost:1234".into(),
            "gpt-4".into(),
            Some("key".into()),
        );

        assert!(!router.is_http(LoopPhase::Observe));
        assert!(!router.is_http(LoopPhase::Execute));
        assert!(router.is_http(LoopPhase::Plan));
        assert!(router.is_http(LoopPhase::Reflect));
    }

    #[test]
    fn test_phase_builder_default() {
        let builder = PhaseBuilder::reasoning_for_planning(
            "http://localhost:1234".into(),
            "gpt-4".into(),
            Some("key".into()),
        );
        let router = builder.build();

        assert!(router.is_http(LoopPhase::Plan));
        assert!(router.is_http(LoopPhase::Reflect));
        assert!(!router.is_http(LoopPhase::Observe));
    }
}
