//! Dependency graph analysis for workspace crates.

use std::collections::{HashMap, HashSet};

use super::types::CrateInfo;

/// Build the dependency graph and reverse dependencies.
pub fn build_dependency_graph(crates: &mut [CrateInfo]) {
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
    for crate_info in crates.iter() {
        for dep in &crate_info.dependencies {
            if let Some(&dep_index) = name_to_index.get(dep) {
                edges.push((dep_index, crate_info.name.clone()));
            }
        }
    }

    // Apply edges
    for (dep_index, dependent_name) in edges {
        if !crates[dep_index]
            .reverse_dependencies
            .contains(&dependent_name)
        {
            crates[dep_index].reverse_dependencies.push(dependent_name);
        }
    }
}

/// Calculate maximum dependency depth.
pub fn calculate_max_depth(crates: &[CrateInfo]) -> usize {
    let name_to_index: HashMap<String, usize> = crates
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.clone(), i))
        .collect();

    fn depth_of(
        crate_info: &CrateInfo,
        crates: &[CrateInfo],
        name_to_index: &HashMap<String, usize>,
        visited: &mut HashSet<usize>,
    ) -> usize {
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
pub fn detect_circular_dependencies(crates: &[CrateInfo]) -> Vec<Vec<String>> {
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
                    dfs(
                        dep_idx,
                        crates,
                        name_to_index,
                        visited,
                        rec_stack,
                        path,
                        cycles,
                    );
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
            dfs(
                i,
                crates,
                &name_to_index,
                &mut visited,
                &mut rec_stack,
                &mut path,
                &mut cycles,
            );
        }
    }

    cycles
}

/// Find orphaned crates (no dependents and not a binary).
pub fn find_orphaned_crates(crates: &[CrateInfo]) -> Vec<String> {
    crates
        .iter()
        .filter(|c| c.reverse_dependencies.is_empty() && !c.is_binary)
        .map(|c| c.name.clone())
        .collect()
}

/// Calculate coupling score (higher = more tightly coupled).
pub fn calculate_coupling_score(crates: &[CrateInfo]) -> f64 {
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
pub fn calculate_cohesion_score(crates: &[CrateInfo]) -> f64 {
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

    f64::from(internal_deps) / f64::from(total_deps)
}
