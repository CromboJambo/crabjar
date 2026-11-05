//! Phase 4: Multi-Crate Workspace Analysis
//!
//! Extends structural evaluation to analyze entire Cargo workspaces,
//! evaluating architectural properties across crate boundaries and
//! generating workspace-wide audit trails.

use std::collections::HashMap;
use std::path::Path;

use super::evaluate::evaluate_structural_conditions;
use super::structural::build_target;

pub(crate) mod cargo;
pub(crate) mod dependency_graph;
pub(crate) mod evaluate;
pub(crate) mod types;

// Re-export public types
pub use self::types::{
    AnalysisBudget, ArchitecturalProperties, BudgetUsage, CrateInfo, EvaluationResult,
    EvaluationSummary, WorkspaceAnalysis,
};

/// Analyze an entire Cargo workspace.
#[must_use]
pub fn analyze_workspace(
    workspace_path: &Path,
    criteria_path: &str,
    budget: AnalysisBudget,
) -> WorkspaceAnalysis {
    let start_time = std::time::Instant::now();

    // Discover all crates in the workspace
    let mut crates = cargo::discover_crates(workspace_path);

    // Apply budget constraint on crate count
    if budget.max_crates > 0 && crates.len() > budget.max_crates {
        // Evaluate most depended-upon crates first
        crates.sort_by(|a, b| {
            b.reverse_dependencies
                .len()
                .cmp(&a.reverse_dependencies.len())
        });
        crates.truncate(budget.max_crates);
    }

    // Build dependency graph
    dependency_graph::build_dependency_graph(&mut crates);

    // Calculate architectural properties
    let max_depth = dependency_graph::calculate_max_depth(&crates);
    let circular_deps = dependency_graph::detect_circular_dependencies(&crates);
    let orphaned = dependency_graph::find_orphaned_crates(&crates);
    let coupling = dependency_graph::calculate_coupling_score(&crates);
    let cohesion = dependency_graph::calculate_cohesion_score(&crates);

    // Evaluate each crate within budget
    let mut evaluation_summary = EvaluationSummary {
        total_evaluations: 0,
        structural_condition_matches: 0,
        embedding_fallbacks: 0,
        average_confidence: 0.0,
        per_crate_results: HashMap::new(),
    };

    let mut confidence_sum = 0.0;
    for crate_info in &mut crates {
        if budget.max_time_seconds > 0 {
            let elapsed = start_time.elapsed().as_secs();
            if elapsed >= budget.max_time_seconds {
                break;
            }
        }

        evaluation_summary.total_evaluations += 1;

        match evaluate::evaluate_crate(crate_info, criteria_path) {
            Some(result) => {
                crate_info.evaluation_result = Some(result.clone());
                evaluation_summary.structural_condition_matches += 1;
                confidence_sum += result.confidence;
            }
            None => {
                // No structural conditions matched — would fall back to embedding
                // in a full integration, but for workspace analysis we note it
                evaluation_summary.embedding_fallbacks += 1;
            }
        }

        evaluation_summary.per_crate_results.insert(
            crate_info.name.clone(),
            crate_info
                .evaluation_result
                .clone()
                .unwrap_or(EvaluationResult {
                    option: "unevaluated".to_string(),
                    confidence: 0.0,
                    rationale: "No structural conditions matched".to_string(),
                    method: "none".to_string(),
                }),
        );
    }

    if evaluation_summary.total_evaluations > 0 {
        evaluation_summary.average_confidence =
            confidence_sum / evaluation_summary.total_evaluations as f64;
    }

    // Build dependency graph for output
    let dep_graph: HashMap<String, Vec<String>> = crates
        .iter()
        .map(|c| (c.name.clone(), c.dependencies.clone()))
        .collect();

    let elapsed_seconds = start_time.elapsed().as_secs();
    let budget_exhausted =
        budget.max_crates > 0 && evaluation_summary.total_evaluations >= budget.max_crates;
    let total_crates = crates.len();
    let total_evaluations = evaluation_summary.total_evaluations;

    WorkspaceAnalysis {
        workspace_path: workspace_path.to_path_buf(),
        crates,
        dependency_graph: dep_graph,
        architectural_properties: ArchitecturalProperties {
            total_crates,
            max_dependency_depth: max_depth,
            circular_dependencies: circular_deps,
            orphaned_crates: orphaned,
            coupling_score: coupling,
            cohesion_score: cohesion,
        },
        evaluation_summary,
        budget_used: BudgetUsage {
            crates_evaluated: total_evaluations,
            time_spent_seconds: elapsed_seconds,
            budget_exhausted,
        },
    }
}
