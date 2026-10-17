//! Decision Policy Configuration
//!
//! Defines default criteria files and fallback behavior for each decision gate type.

use super::GateType;

/// Decision policy that maps gate types to default criteria files
#[derive(Debug, Clone)]
pub struct DecisionPolicy {
    /// Default criteria file for pre-execution readiness checks
    pub pre_execution_criteria: Option<String>,

    /// Default criteria file for annealing loop decisions
    pub annealing_criteria: Option<String>,

    /// Default criteria file for resource contention priority
    pub resource_contention_criteria: Option<String>,

    /// Whether to block goal execution on decision failure (vs. continue with warning)
    pub block_on_failure: bool,

    /// Timeout in seconds for JEV evaluation requests
    pub evaluation_timeout_seconds: u64,
}

impl Default for DecisionPolicy {
    fn default() -> Self {
        Self {
            pre_execution_criteria: Some("/opt/crabjar/criteria/deployment-gate.json".to_string()),
            annealing_criteria: Some("/opt/crabjar/criteria/service-triage.json".to_string()),
            resource_contention_criteria: None, // Requires explicit specification
            block_on_failure: true,
            evaluation_timeout_seconds: 30,
        }
    }
}

impl DecisionPolicy {
    /// Get the default criteria file for a given gate type
    pub fn default_criteria_for(&self, gate: GateType) -> Option<String> {
        match gate {
            GateType::PreExecution => self.pre_execution_criteria.clone(),
            GateType::Annealing => self.annealing_criteria.clone(),
            GateType::ResourceContention => self.resource_contention_criteria.clone(),
        }
    }

    /// Set the pre-execution criteria file
    pub fn with_pre_execution_criteria(mut self, path: &str) -> Self {
        self.pre_execution_criteria = Some(path.to_string());
        self
    }

    /// Set the annealing loop criteria file
    pub fn with_annealing_criteria(mut self, path: &str) -> Self {
        self.annealing_criteria = Some(path.to_string());
        self
    }

    /// Set the resource contention criteria file
    pub fn with_resource_contention_criteria(mut self, path: &str) -> Self {
        self.resource_contention_criteria = Some(path.to_string());
        self
    }

    /// Configure whether to block goal execution on decision failure
    pub fn with_block_on_failure(mut self, block: bool) -> Self {
        self.block_on_failure = block;
        self
    }

    /// Configure the evaluation timeout in seconds
    pub fn with_timeout_seconds(mut self, seconds: u64) -> Self {
        self.evaluation_timeout_seconds = seconds;
        self
    }
}