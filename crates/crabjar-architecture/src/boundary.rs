/// Boundary enforcement: verify that crate dependencies respect layer ordering.
///
/// Parses Cargo.toml files in the workspace and checks that each crate
/// Only depends on crates in allowed layers (its own layer or below).

use std::collections::HashSet;
use std::path::Path;

use crate::layer::{allowed_dependencies, crate_to_layer};

pub use self::parsing::{discover_workspace_members, parse_crate_info, CrateInfo};
pub use self::violation::Violation;

mod parsing;
mod violation;

#[cfg(test)]
mod tests;

/// Check all workspace members for layer boundary violations.
///
/// Returns a list of violations found. An empty list means all boundaries are respected.
pub fn check_workspace_boundaries(workspace_root: &Path) -> Result<Vec<Violation>, String> {
    let crates = discover_workspace_members(workspace_root)?;
    let layer_map = crate_to_layer();
    let mut violations = Vec::new();

    // Build set of known workspace crate names for unmapped detection
    let discovered_names: HashSet<&str> = crates.iter().map(|c| c.name.as_str()).collect();
    let mapped_names: HashSet<&str> = layer_map.keys().copied().collect();

    // Fail if any discovered workspace member is not mapped in crate_to_layer()
    for name in &discovered_names {
        if !mapped_names.contains(name) {
            violations.push(Violation {
                from_crate: name.to_string(),
                from_layer: 0,
                to_crate: "(unmapped)".to_string(),
                to_layer: 0,
            });
        }
    }

    for crate_info in &crates {
        let Some(&layer) = layer_map.get(crate_info.name.as_str()) else {
            // Crate not in the architecture model — already reported above
            continue;
        };

        let allowed = allowed_dependencies(layer);

        for dep in &crate_info.direct_deps {
            // Only check deps that are workspace members (not external crates)
            let Some(&to_layer) = layer_map.get(dep.as_str()) else {
                continue; // External crate, not our concern
            };

            // Check if to_layer is in the allowed set
            if !allowed.contains(&to_layer) {
                violations.push(Violation {
                    from_crate: crate_info.name.clone(),
                    from_layer: layer,
                    to_crate: dep.clone(),
                    to_layer,
                });
            }
        }
    }

    Ok(violations)
}

/// Check that the workspace boundaries are valid and panic on failure.
/// This is designed to be called from tests.
pub fn enforce_boundaries(workspace_root: &Path) -> Result<(), Vec<Violation>> {
    let violations = check_workspace_boundaries(workspace_root).expect("Boundary check failed");

    if !violations.is_empty() {
        return Err(violations);
    }

    Ok(())
}

/// Get all crates in a specific layer.
#[must_use]
pub fn crates_in_layer(layer: usize) -> Vec<String> {
    crate_to_layer()
        .into_iter()
        .filter(|(_, l)| *l == layer)
        .map(|(name, _)| name.to_string())
        .collect()
}

/// Get the layer name for a crate.
#[must_use]
pub fn crate_layer(crate_name: &str) -> Option<(usize, &'static str)> {
    use crate::layer::layer_name;
    crate_to_layer()
        .get(crate_name)
        .copied()
        .map(|l| (l, layer_name(l).unwrap_or("unknown")))
}
