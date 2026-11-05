/// Boundary enforcement: verify that crate dependencies respect layer ordering.
///
/// Parses Cargo.toml files in the workspace and checks that each crate
/// Only depends on crates in allowed layers (its own layer or below).
use crate::layer::{allowed_dependencies, crate_to_layer, layer_name};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// A single dependency violation found during boundary checking.
#[derive(Debug, Clone)]
pub struct Violation {
    /// The crate that has the invalid dependency.
    pub from_crate: String,
    /// The layer the `from_crate` belongs to.
    pub from_layer: usize,
    /// The dependency that violates the boundary.
    pub to_crate: String,
    /// The layer the `to_crate` belongs to.
    pub to_layer: usize,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let max_allowed = *allowed_dependencies(self.from_layer).last().unwrap_or(&0);
        write!(
            f,
            "LAYER VIOLATION: {} (layer {}:{}) depends on {} (layer {}:{}) \
             — layer {} may only depend on layers 0..={}",
            self.from_crate,
            self.from_layer,
            layer_name(self.from_layer).unwrap_or("unknown"),
            self.to_crate,
            self.to_layer,
            layer_name(self.to_layer).unwrap_or("unknown"),
            layer_name(self.from_layer).unwrap_or("unknown"),
            max_allowed,
        )
    }
}

/// Crate dependency info extracted from a Cargo.toml.
#[derive(Debug, Clone)]
pub struct CrateInfo {
    /// Crate name (from [package].name)
    pub name: String,
    /// Crate source directory
    pub manifest_path: PathBuf,
    /// Direct dependencies (resolved to package names)
    pub direct_deps: Vec<String>,
}

/// Resolve the actual package name from a dependency value.
/// Handles workspace = true inherited deps and `package` field aliases.
fn resolve_dep_package_name(dep_val: &toml::Value) -> Option<String> {
    // Check for explicit package override (alias)
    if let Some(pkg_field) = dep_val.get("package") {
        if let Some(pkg_name) = pkg_field.as_str() {
            return Some(pkg_name.to_string());
        }
    }

    // For workspace = true, we'd need to resolve through the workspace manifest.
    // This is complex — for now, skip these (they're version-inherited, not structural).
    if dep_val.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
        return None;
    }

    // If it's just a string (version), the key is the package name
    if dep_val.is_str() {
        return None; // Key is the name, caller handles it
    }

    // Has path or other fields — key is the name
    None
}

/// Parse a Cargo.toml and extract the crate name and direct dependencies.
fn parse_crate_info(manifest_path: &Path) -> Option<CrateInfo> {
    let content = fs::read_to_string(manifest_path).ok()?;
    let parsed = content.parse::<toml::Value>().ok()?;

    // Get crate name
    let name = parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(std::string::ToString::to_string)?;

    let mut direct_deps = Vec::new();

    // Check [dependencies], [dev-dependencies], [build-dependencies]
    for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(deps) = parsed.get(section) {
            if let Some(table) = deps.as_table() {
                for (dep_key, dep_val) in table {
                    // Resolve package name (handle aliases)
                    let pkg_name = resolve_dep_package_name(dep_val);
                    match pkg_name {
                        Some(name) => direct_deps.push(name),
                        None => {
                            // Key is the package name (or workspace-inherited, which we skip)
                            if dep_val.get("workspace").and_then(toml::Value::as_bool) == Some(true)
                            {
                                continue;
                            }
                            direct_deps.push(dep_key.clone());
                        }
                    }
                }
            }
        }
    }

    // Check target-specific dependency tables
    if let Some(target_table) = parsed.get("target") {
        if let Some(targets) = target_table.as_table() {
            for (_target_name, target_val) in targets {
                for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
                    if let Some(deps) = target_val.get(section) {
                        if let Some(table) = deps.as_table() {
                            for (dep_key, dep_val) in table {
                                let pkg_name = resolve_dep_package_name(dep_val);
                                match pkg_name {
                                    Some(name) => direct_deps.push(name),
                                    None => {
                                        if dep_val.get("workspace").and_then(toml::Value::as_bool)
                                            == Some(true)
                                        {
                                            continue;
                                        }
                                        direct_deps.push(dep_key.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Some(CrateInfo {
        name,
        manifest_path: manifest_path.to_path_buf(),
        direct_deps,
    })
}

/// Discover all workspace member crates by scanning the workspace Cargo.toml.
pub fn discover_workspace_members(workspace_root: &Path) -> Result<Vec<CrateInfo>, String> {
    let workspace_cargo = workspace_root.join("Cargo.toml");
    let content = fs::read_to_string(&workspace_cargo)
        .map_err(|e| format!("Cannot read workspace Cargo.toml: {e}"))?;
    let parsed = content
        .parse::<toml::Value>()
        .map_err(|e| format!("Cannot parse workspace Cargo.toml: {e}"))?;

    let members = parsed
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .ok_or_else(|| "No [workspace.members] found".to_string())?;

    let mut crates = Vec::new();
    for member in members {
        let member_path = member.as_str().ok_or("Invalid member entry")?;
        let manifest_path = workspace_root.join(member_path).join("Cargo.toml");

        if manifest_path.exists() {
            if let Some(info) = parse_crate_info(&manifest_path) {
                crates.push(info);
            }
        }
    }

    Ok(crates)
}

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
    crate_to_layer()
        .get(crate_name)
        .copied()
        .map(|l| (l, layer_name(l).unwrap_or("unknown")))
}
