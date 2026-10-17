//! JEV Decision Gate Orchestration
//!
//! Integrates Jev semantic decision-making at key points in the conductor's goal lifecycle:
//! - Pre-execution readiness assessment
//! - Annealing/trust-update loop decisions
//! - Cross-goal resource contention coordination

pub mod cache;
pub mod evaluate;
pub mod policy;
pub mod rule_extraction;

use crate::state_store::StateStore;

pub use cache::{CacheStats, DecisionCache};
pub use evaluate::{BackendConfig, DecisionResult, JevEvaluator};
pub use policy::DecisionPolicy;

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
}

impl DecisionGate {
    /// Create a new decision gate with optional policy configuration
    pub fn new(evaluator: JevEvaluator, policy: Option<DecisionPolicy>) -> Self {
        Self { evaluator, policy }
    }

    /// Get cache statistics from the underlying JEV evaluator
    pub fn cache_stats(&self) -> CacheStats {
        self.evaluator.cache_stats()
    }

    /// Evaluate an observation against decision criteria (generic, for external callers).
    pub async fn evaluate(
        &self,
        observation: &str,
        criteria_file: &str,
        state_store: Option<&StateStore>,
    ) -> Result<DecisionResult, Box<dyn std::error::Error>> {
        self.evaluator.evaluate(observation, criteria_file, state_store).await.map_err(|e| e.into())
    }

    /// Evaluate pre-execution readiness for a goal
    pub async fn evaluate_pre_execution(
        &self,
        goal_description: &str,
        fleet_state: &str,
        criteria_file: Option<&str>,
        state_store: Option<&StateStore>,
    ) -> Result<GateResult, Box<dyn std::error::Error>> {
        let policy = self.policy.as_ref();
        let criteria = if let Some(cf) = criteria_file {
            cf.to_string()
        } else {
            policy
                .and_then(|p| p.default_criteria_for(GateType::PreExecution))
                .ok_or("No pre-execution criteria specified or configured")?
        };

        let observation = format!(
            "Goal: {}. Current fleet state: {}",
            goal_description, fleet_state
        );

        let result = self.evaluator.evaluate(&observation, &criteria, state_store).await?;

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
            "Annealing iteration {}. Task batch: {} succeeded, {} failed. \
             Failure patterns: {}. Fleet load: {:.0}%.",
            iteration, success_count, failure_count, failure_summary, fleet_load_percent
        );

        let result = self.evaluator.evaluate(&observation, &criteria, state_store).await?;

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

        let result = self.evaluator.evaluate(&observation, &criteria, state_store).await?;

        Ok(GateResult {
            gate: GateType::ResourceContention,
            timestamp: chrono::Utc::now(),
            criteria_file: criteria,
            observation,
            result,
        })
    }
}