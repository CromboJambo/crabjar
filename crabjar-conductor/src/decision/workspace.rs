//! Phase 4: Multi-Crate Workspace Analysis
//!
//! Extends structural evaluation to analyze entire Cargo workspaces,
//! evaluating architectural properties across crate boundaries and
//! generating workspace-wide audit trails.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::structural::{build_target, EvaluationTarget};
use super::evaluate::evaluate_structural_conditions;

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

/// Parse Cargo.toml to extract crate name and dependencies.
fn parse_cargo_toml(path: &Path) -> Option<(String, Vec<String>)> {
    let content = fs::read_to_string(path).ok()?;
    let doc: toml::Value = toml::from_str(&content).ok()?;

    let name = doc.get("package")?.get("name")?.as_str()?.to_string();

    let mut deps = Vec::new();
    if let Some(dependencies) = doc.get("dependencies").and_then(|d| d.as_table()) {
        for dep_name in dependencies.keys() {
            deps.push(dep_name.to_string());
        }
    }

    // Also check dev-dependencies and build-dependencies
    for section in ["dev-dependencies", "build-dependencies"] {
        if let Some(deps_table) = doc.get(section).and_then(|d| d.as_table()) {
            for dep_name in deps_table.keys() {
                if !deps.contains(&dep_name.to_string()) {
                    deps.push(dep_name.to_string());
                }
            }
        }
    }

    Some((name, deps))
}

/// Find all crates in a Cargo workspace.
fn discover_crates(workspace_path: &Path) -> Vec<CrateInfo> {
    let mut crates = Vec::new();

    // Check root Cargo.toml
    let root_toml = workspace_path.join("Cargo.toml");
    if root_toml.exists() {
        if let Some((name, deps)) = parse_cargo_toml(&root_toml) {
            crates.push(CrateInfo {
                name,
                path: root_toml.parent().unwrap().to_path_buf(),
                dependencies: deps,
                reverse_dependencies: Vec::new(),
                is_library: workspace_path.join("src").join("lib.rs").exists(),
                is_binary: workspace_path.join("src").join("main.rs").exists(),
                evaluation_result: None,
            });
        }
    }

    // Check Cargo.toml members array for sub-crates
    if let Ok(content) = fs::read_to_string(&root_toml) {
        if let Ok(doc) = toml::from_str::<toml::Value>(&content) {
            if let Some(workspace) = doc.get("workspace").and_then(|w| w.as_table()) {
                if let Some(members) = workspace.get("members").and_then(|m| m.as_array()) {
                    for member in members {
                        if let Some(path_str) = member.as_str() {
                            let member_path = workspace_path.join(path_str);
                            let member_toml = member_path.join("Cargo.toml");
                            if let Some((name, deps)) = parse_cargo_toml(&member_toml) {
                                crates.push(CrateInfo {
                                    name,
                                    path: member_path.clone(),
                                    dependencies: deps,
                                    reverse_dependencies: Vec::new(),
                                    is_library: member_path.join("src").join("lib.rs").exists(),
                                    is_binary: member_path.join("src").join("main.rs").exists(),
                                    evaluation_result: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    crates
}

/// Build the dependency graph and reverse dependencies.
fn build_dependency_graph(crates: &mut [CrateInfo]) {
    let name_to_index: HashMap<String, usize> = crates
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.clone(), i))
        .collect();

    for crate_info in crates.iter_mut() {
        crate_info.reverse_dependencies.clear();
    }

    // Collect edges first to avoid borrow checker issues
    let mut edges: Vec<(usize, String)> = Vec::new();
    for (i, crate_info) in crates.iter().enumerate() {
        for dep in &crate_info.dependencies {
            if let Some(&dep_index) = name_to_index.get(dep) {
                edges.push((dep_index, crate_info.name.clone()));
            }
        }
    }

    // Apply edges
    for (dep_index, dependent_name) in edges {
        if !crates[dep_index].reverse_dependencies.contains(&dependent_name) {
            crates[dep_index].reverse_dependencies.push(dependent_name);
        }
    }
}

/// Calculate maximum dependency depth.
fn calculate_max_depth(crates: &[CrateInfo]) -> usize {
    let name_to_index: HashMap<String, usize> = crates
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.clone(), i))
        .collect();

    fn depth_of(crate_info: &CrateInfo, crates: &[CrateInfo], name_to_index: &HashMap<String, usize>, visited: &mut HashSet<usize>) -> usize {
        if visited.contains(&name_to_index[&crate_info.name]) {
            return 0;
        }
        visited.insert(name_to_index[&crate_info.name]);

        let mut max_depth = 0;
        for dep in &crate_info.dependencies {
            if let Some(&dep_index) = name_to_index.get(dep) {
                let dep_depth = depth_of(&crates[dep_index], crates, name_to_index, visited);
                max_depth = max_depth.max(dep_depth + 1);
            }
        }

        visited.remove(&name_to_index[&crate_info.name]);
        max_depth
    }

    let mut max_depth = 0;
    for crate_info in crates {
        let mut visited = HashSet::new();
        let depth = depth_of(crate_info, crates, &name_to_index, &mut visited);
        max_depth = max_depth.max(depth);
    }

    max_depth
}

/// Detect circular dependencies.
fn detect_circular_dependencies(crates: &[CrateInfo]) -> Vec<Vec<String>> {
    let name_to_index: HashMap<String, usize> = crates
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.clone(), i))
        .collect();

    let mut cycles = Vec::new();
    let mut visited = vec![false; crates.len()];
    let mut rec_stack = vec![false; crates.len()];
    let mut path = Vec::new();

    fn dfs(
        idx: usize,
        crates: &[CrateInfo],
        name_to_index: &HashMap<String, usize>,
        visited: &mut [bool],
        rec_stack: &mut [bool],
        path: &mut Vec<usize>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited[idx] = true;
        rec_stack[idx] = true;
        path.push(idx);

        for dep in &crates[idx].dependencies {
            if let Some(&dep_idx) = name_to_index.get(dep) {
                if !visited[dep_idx] {
                    dfs(dep_idx, crates, name_to_index, visited, rec_stack, path, cycles);
                } else if rec_stack[dep_idx] {
                    // Found a cycle
                    let cycle_start = path.iter().position(|&p| p == dep_idx).unwrap();
                    let cycle: Vec<String> = path[cycle_start..]
                        .iter()
                        .map(|&i| crates[i].name.clone())
                        .collect();
                    cycles.push(cycle);
                }
            }
        }

        path.pop();
        rec_stack[idx] = false;
    }

    for i in 0..crates.len() {
        if !visited[i] {
            dfs(i, crates, &name_to_index, &mut visited, &mut rec_stack, &mut path, &mut cycles);
        }
    }

    cycles
}

/// Find orphaned crates (no dependents and not a binary).
fn find_orphaned_crates(crates: &[CrateInfo]) -> Vec<String> {
    crates
        .iter()
        .filter(|c| c.reverse_dependencies.is_empty() && !c.is_binary)
        .map(|c| c.name.clone())
        .collect()
}

/// Calculate coupling score (higher = more tightly coupled).
fn calculate_coupling_score(crates: &[CrateInfo]) -> f64 {
    if crates.is_empty() {
        return 0.0;
    }

    let total_possible_edges = crates.len() * (crates.len() - 1);
    let actual_edges: usize = crates.iter().map(|c| c.dependencies.len()).sum();

    if total_possible_edges == 0 {
        return 0.0;
    }

    actual_edges as f64 / total_possible_edges as f64
}

/// Calculate cohesion score (higher = more cohesive).
fn calculate_cohesion_score(crates: &[CrateInfo]) -> f64 {
    if crates.is_empty() {
        return 0.0;
    }

    // Simple metric: ratio of internal dependencies to total dependencies
    let name_set: HashSet<String> = crates.iter().map(|c| c.name.clone()).collect();
    let mut internal_deps = 0;
    let mut total_deps = 0;

    for crate_info in crates {
        for dep in &crate_info.dependencies {
            total_deps += 1;
            if name_set.contains(dep) {
                internal_deps += 1;
            }
        }
    }

    if total_deps == 0 {
        return 1.0; // No dependencies = perfectly cohesive (trivially)
    }

    internal_deps as f64 / total_deps as f64
}

/// Evaluate a single crate using structural conditions.
fn evaluate_crate(crate_info: &CrateInfo, criteria_path: &str) -> Option<EvaluationResult> {
    // Build evaluation target from crate's source
    let src_dir = crate_info.path.join("src");
    if !src_dir.exists() {
        return None;
    }

    // For workspace analysis, evaluate the entire crate as a single region
    let lib_path = src_dir.join("lib.rs");
    let main_path = src_dir.join("main.rs");

    let target_path = if lib_path.exists() {
        lib_path
    } else if main_path.exists() {
        main_path
    } else {
        return None;
    };

    let source = match fs::read_to_string(&target_path) {
        Ok(content) => content,
        Err(_) => return None,
    };

    let target = build_target(
        super::structural::SpanKind::Item,
        [0, source.len()],
        &source,
        Some(&crate_info.name),
    );

    // Try structural condition evaluation first
    if let Some(result) = evaluate_structural_conditions(&target, criteria_path) {
        return Some(EvaluationResult {
            option: result.option.clone(),
            confidence: result.confidence,
            rationale: result.rationale.clone(),
            method: "structural-condition".to_string(),
        });
    }

    None
}

/// Analyze an entire Cargo workspace.
pub fn analyze_workspace(
    workspace_path: &Path,
    criteria_path: &str,
    budget: AnalysisBudget,
) -> WorkspaceAnalysis {
    let start_time = std::time::Instant::now();

    // Discover all crates in the workspace
    let mut crates = discover_crates(workspace_path);

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
    build_dependency_graph(&mut crates);

    // Calculate architectural properties
    let max_depth = calculate_max_depth(&crates);
    let circular_deps = detect_circular_dependencies(&crates);
    let orphaned = find_orphaned_crates(&crates);
    let coupling = calculate_coupling_score(&crates);
    let cohesion = calculate_cohesion_score(&crates);

    // Evaluate each crate within budget
    let mut evaluation_summary = EvaluationSummary {
        total_evaluations: 0,
        structural_condition_matches: 0,
        embedding_fallbacks: 0,
        average_confidence: 0.0,
        per_crate_results: HashMap::new(),
    };

    let mut confidence_sum = 0.0;
    for crate_info in crates.iter_mut() {
        if budget.max_time_seconds > 0 {
            let elapsed = start_time.elapsed().as_secs();
            if elapsed >= budget.max_time_seconds {
                break;
            }
        }

        evaluation_summary.total_evaluations += 1;

        match evaluate_crate(crate_info, criteria_path) {
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
            crate_info.evaluation_result.clone().unwrap_or(EvaluationResult {
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
    let budget_exhausted = budget.max_crates > 0 && evaluation_summary.total_evaluations >= budget.max_crates;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_workspace() -> PathBuf {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();

        // Write root Cargo.toml with workspace definition
        fs::write(
            path.join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/core", "crates/api", "crates/cli"]

[package]
name = "test-workspace"
version = "0.1.0"
"#,
        )
        .unwrap();
        
        println!("Wrote root Cargo.toml");

        // crates/core/Cargo.toml
        fs::create_dir_all(path.join("crates/core/src")).unwrap();
        fs::write(
            path.join("crates/core/Cargo.toml"),
            r#"
[package]
name = "test-core"
version = "0.1.0"
"#,
        )
        .unwrap();
        fs::write(
            path.join("crates/core/src/lib.rs"),
            "pub fn core_function() -> i32 { 42 }",
        )
        .unwrap();

        // crates/api/Cargo.toml (depends on core)
        fs::create_dir_all(path.join("crates/api/src")).unwrap();
        fs::write(
            path.join("crates/api/Cargo.toml"),
            r#"
[package]
name = "test-api"
version = "0.1.0"

[dependencies]
test-core = { path = "../core" }
"#,
        )
        .unwrap();
        fs::write(
            path.join("crates/api/src/lib.rs"),
            "pub fn api_function() -> i32 { test_core::core_function() }",
        )
        .unwrap();

        // crates/cli/Cargo.toml (depends on core and api)
        fs::create_dir_all(path.join("crates/cli/src")).unwrap();
        fs::write(
            path.join("crates/cli/Cargo.toml"),
            r#"
[package]
name = "test-cli"
version = "0.1.0"

[dependencies]
test-core = { path = "../core" }
test-api = { path = "../api" }
"#,
        )
        .unwrap();
        fs::write(
            path.join("crates/cli/src/main.rs"),
            "fn main() { println!(\"Hello\"); }",
        )
        .unwrap();

        // Convert to PathBuf without auto-cleanup (into_path prevents tempdir from being dropped)
        dir.into_path()
    }

    #[test]
    fn test_discover_crates() {
        let workspace = create_test_workspace();
        
        // Debug: check if root Cargo.toml exists and can be read
        let root_toml = workspace.join("Cargo.toml");
        println!("Root toml exists: {}", root_toml.exists());
        if root_toml.exists() {
            let content = fs::read_to_string(&root_toml).unwrap();
            println!("Root toml content length: {}", content.len());
        }
        
        let crates = discover_crates(&workspace);
        println!("Discovered {} crates", crates.len());
        for c in &crates {
            println!("  Crate: {} at {:?}", c.name, c.path);
        }

        // Should find root + 3 member crates
        assert_eq!(crates.len(), 4);

        // Check crate names
        let names: HashSet<String> = crates.iter().map(|c| c.name.clone()).collect();
        assert!(names.contains("test-workspace"));
        assert!(names.contains("test-core"));
        assert!(names.contains("test-api"));
        assert!(names.contains("test-cli"));
    }

    #[test]
    fn test_dependency_graph() {
        let workspace = create_test_workspace();
        let mut crates = discover_crates(&workspace);
        build_dependency_graph(&mut crates);

        // Find test-core and verify it has reverse dependencies
        let core = crates.iter().find(|c| c.name == "test-core").unwrap();
        assert!(core.reverse_dependencies.contains(&"test-api".to_string()));
        assert!(core.reverse_dependencies.contains(&"test-cli".to_string()));

        // Find test-api and verify it has one reverse dependency
        let api = crates.iter().find(|c| c.name == "test-api").unwrap();
        assert!(api.reverse_dependencies.contains(&"test-cli".to_string()));
    }

    #[test]
    fn test_max_depth() {
        let workspace = create_test_workspace();
        let mut crates = discover_crates(&workspace);
        build_dependency_graph(&mut crates);

        // Depth should be 2: cli -> api -> core
        let depth = calculate_max_depth(&crates);
        assert_eq!(depth, 2);
    }

    #[test]
    fn test_orphaned_crates() {
        let workspace = create_test_workspace();
        let mut crates = discover_crates(&workspace);
        build_dependency_graph(&mut crates);

        // Root workspace has no dependents and is not a binary
        let orphaned = find_orphaned_crates(&crates);
        assert!(orphaned.contains(&"test-workspace".to_string()));

        // test-cli is a binary, so not orphaned
        assert!(!orphaned.contains(&"test-cli".to_string()));
    }
}
