//! Decision workspace analysis types.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Information about a single crate in the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateInfo {
    pub name: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
    pub reverse_dependencies: Vec<String>,
    pub is_library: bool,
    pub is_binary: bool,
    pub evaluation_result: Option<EvaluationResult>,
}

/// Result of evaluating a single crate or region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub option: String,
    pub confidence: f64,
    pub rationale: String,
    pub method: String, // "structural-condition" or "embedding-evaluation"
}

/// Budget constraints for workspace analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisBudget {
    /// Maximum number of crates to evaluate (0 = all).
    pub max_crates: usize,
    /// Maximum time in seconds (0 = unlimited).
    pub max_time_seconds: u64,
    /// Whether to collapse large regions for budget efficiency.
    pub collapse_large_regions: bool,
}

impl Default for AnalysisBudget {
    fn default() -> Self {
        Self {
            max_crates: 0,
            max_time_seconds: 0,
            collapse_large_regions: true,
        }
    }
}

/// Complete workspace analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceAnalysis {
    pub workspace_path: PathBuf,
    pub crates: Vec<CrateInfo>,
    pub dependency_graph: HashMap<String, Vec<String>>,
    pub architectural_properties: ArchitecturalProperties,
    pub evaluation_summary: EvaluationSummary,
    pub budget_used: BudgetUsage,
}

/// Workspace-level architectural properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalProperties {
    pub total_crates: usize,
    pub max_dependency_depth: usize,
    pub circular_dependencies: Vec<Vec<String>>,
    pub orphaned_crates: Vec<String>,
    pub coupling_score: f64,
    pub cohesion_score: f64,
}

/// Summary of evaluations performed across the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationSummary {
    pub total_evaluations: usize,
    pub structural_condition_matches: usize,
    pub embedding_fallbacks: usize,
    pub average_confidence: f64,
    pub per_crate_results: HashMap<String, EvaluationResult>,
}

/// Budget consumption tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetUsage {
    pub crates_evaluated: usize,
    pub time_spent_seconds: u64,
    pub budget_exhausted: bool,
}
