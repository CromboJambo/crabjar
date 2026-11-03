//! JEV Decision Gate Orchestration
//!
//! Integrates Jev semantic decision-making at key points in the conductor's goal lifecycle:
//! - Pre-execution readiness assessment
//! - Annealing/trust-update loop decisions
//! - Cross-goal resource contention coordination

pub mod cache;
pub mod condition;
pub mod evaluate;
pub mod policy;
pub mod postprocessor;
pub mod registry;
pub mod rule_extraction;
pub mod structural;
pub mod workspace;

#[cfg(test)]
mod condition_integration_test;

pub use workspace::{AnalysisBudget, CrateInfo, EvaluationResult, WorkspaceAnalysis};

use crate::state_store::StateStore;

pub use cache::{CacheStats, DecisionCache};
pub use evaluate::{BackendConfig, DecisionResult, JevEvaluator};
pub use policy::DecisionPolicy;
pub use registry::{DecisionCapability, DecisionRegistry};

/// Decision gate types in the goal lifecycle
#[derive(Debug, Clone, PartialEq)]
pub enum GateType {
    PreExecution,
    Annealing,
    ResourceContention,
}

/// Result of a JEV decision gate evaluation
#[derive(Debug, Clone)]
pub struct GateResult {
    pub gate: GateType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub criteria_file: String,
    pub observation: String,
    pub result: evaluate::DecisionResult,
}

/// Decision gate orchestrator that manages JEV evaluations at lifecycle points
pub struct DecisionGate {
    evaluator: JevEvaluator,
    policy: Option<DecisionPolicy>,
    registry: DecisionRegistry,
}

impl DecisionGate {
    /// Create a new decision gate with optional policy configuration
    pub fn new(evaluator: JevEvaluator, policy: Option<DecisionPolicy>) -> Self {
        Self {
            evaluator,
            policy,
            registry: DecisionRegistry::new(),
        }
    }

    /// Load decision capabilities from a directory of JSON manifests
    pub fn load_decision_capabilities<P: AsRef<std::path::Path>>(&mut self, dir: P) -> Result<usize, String> {
        self.registry.load_from_directory(dir)
    }

    /// Register a single decision capability
    pub fn register_decision_capability(&mut self, capability: DecisionCapability) {
        self.registry.register(capability);
    }

    /// Get the decision registry for inspection
    pub fn registry(&self) -> &DecisionRegistry {
        &self.registry
    }

    /// Get cache statistics from the underlying JEV evaluator
    pub fn cache_stats(&self) -> CacheStats {
        self.evaluator.cache_stats()
    }

    /// Find a decision capability that matches the observation's triggers.
    /// Returns the first matching capability, or None if no triggers match.
    fn find_matching_capability(&self, observation: &str) -> Option<&DecisionCapability> {
        let matches = self.registry.find_matches(observation);
        matches.first().copied()
    }

    /// Evaluate an observation against decision criteria (generic, for external callers).
    pub async fn evaluate(
        &self,
        observation: &str,
        criteria_file: &str,
        state_store: Option<&StateStore>,
    ) -> Result<DecisionResult, Box<dyn std::error::Error>> {
        self.evaluator
            .evaluate(observation, criteria_file, state_store)
            .await
            .map_err(std::convert::Into::into)
    }

    /// Evaluate pre-execution readiness for a goal
    pub async fn evaluate_pre_execution(
        &self,
        goal_description: &str,
        fleet_state: &str,
        criteria_file: Option<&str>,
        state_store: Option<&StateStore>,
    ) -> Result<GateResult, Box<dyn std::error::Error>> {
        let observation = format!(
            "Goal: {goal_description}. Current fleet state: {fleet_state}"
        );

        // First check for trigger-based decision capabilities
        let criteria = if let Some(cf) = criteria_file {
            cf.to_string()
        } else if let Some(capability) = self.find_matching_capability(&observation) {
            tracing::info!(
                "Trigger matched decision capability '{}'",
                capability.name
            );
            capability.criteria_file.to_string_lossy().to_string()
        } else {
            // Fall back to policy-based selection
            let policy = self.policy.as_ref();
            policy
                .and_then(|p| p.default_criteria_for(GateType::PreExecution))
                .ok_or("No pre-execution criteria specified, triggered, or configured")?
        };

        let result = self
            .evaluator
            .evaluate(&observation, &criteria, state_store)
            .await?;

        Ok(GateResult {
            gate: GateType::PreExecution,
            timestamp: chrono::Utc::now(),
            criteria_file: criteria,
            observation,
            result,
        })
    }

    /// Evaluate annealing loop continuation
    pub async fn evaluate_annealing(
        &self,
        iteration: usize,
        success_count: usize,
        failure_count: usize,
        failure_summary: &str,
        fleet_load_percent: f64,
        criteria_file: Option<&str>,
        state_store: Option<&StateStore>,
    ) -> Result<GateResult, Box<dyn std::error::Error>> {
        let policy = self.policy.as_ref();
        let criteria = if let Some(cf) = criteria_file {
            cf.to_string()
        } else {
            policy
                .and_then(|p| p.default_criteria_for(GateType::Annealing))
                .ok_or("No annealing criteria specified or configured")?
        };

        let observation = format!(
            "Annealing iteration {iteration}. Task batch: {success_count} succeeded, {failure_count} failed. \
             Failure patterns: {failure_summary}. Fleet load: {fleet_load_percent:.0}%."
        );

        let result = self
            .evaluator
            .evaluate(&observation, &criteria, state_store)
            .await?;

        Ok(GateResult {
            gate: GateType::Annealing,
            timestamp: chrono::Utc::now(),
            criteria_file: criteria,
            observation,
            result,
        })
    }

    /// Evaluate resource contention priority
    pub async fn evaluate_resource_contention(
        &self,
        competing_goals: &[String],
        queue_depth: usize,
        criteria_file: Option<&str>,
        state_store: Option<&StateStore>,
    ) -> Result<GateResult, Box<dyn std::error::Error>> {
        let policy = self.policy.as_ref();
        let criteria = if let Some(cf) = criteria_file {
            cf.to_string()
        } else {
            policy
                .and_then(|p| p.default_criteria_for(GateType::ResourceContention))
                .ok_or("No resource contention criteria specified or configured")?
        };

        let observation = format!(
            "Resource contention: {} goals competing, queue depth {}. Competing goals: {}",
            competing_goals.len(),
            queue_depth,
            competing_goals.join("; ")
        );

        let result = self
            .evaluator
            .evaluate(&observation, &criteria, state_store)
            .await?;

        Ok(GateResult {
            gate: GateType::ResourceContention,
            timestamp: chrono::Utc::now(),
            criteria_file: criteria,
            observation,
            result,
        })
    }
}
