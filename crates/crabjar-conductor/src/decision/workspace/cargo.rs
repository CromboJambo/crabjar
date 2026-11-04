//! Cargo.toml parsing and crate discovery.

use std::fs;
use std::path::{Path, PathBuf};

use super::types::CrateInfo;

/// Parse Cargo.toml to extract crate name and dependencies.
fn parse_cargo_toml(path: &Path) -> Option<(String, Vec<String>)> {
    let content = fs::read_to_string(path).ok()?;
    let doc: toml::Value = toml::from_str(&content).ok()?;

    let name = doc.get("package")?.get("name")?.as_str()?.to_string();

    let mut deps = Vec::new();
    if let Some(dependencies) = doc.get("dependencies").and_then(|d| d.as_table()) {
        for dep_name in dependencies.keys() {
            deps.push(dep_name.clone());
        }
    }

    // Also check dev-dependencies and build-dependencies
    for section in ["dev-dependencies", "build-dependencies"] {
        if let Some(deps_table) = doc.get(section).and_then(|d| d.as_table()) {
            for dep_name in deps_table.keys() {
                if !deps.contains(&dep_name.clone()) {
                    deps.push(dep_name.clone());
                }
            }
        }
    }

    Some((name, deps))
}

/// Find all crates in a Cargo workspace.
#[must_use]
pub fn discover_crates(workspace_path: &Path) -> Vec<CrateInfo> {
    let mut crates = Vec::new();

    // Check root Cargo.toml
    let root_toml = workspace_path.join("Cargo.toml");
    if root_toml.exists()
        && let Some((name, deps)) = parse_cargo_toml(&root_toml) {
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

    // Check Cargo.toml members array for sub-crates
    if let Ok(content) = fs::read_to_string(&root_toml)
        && let Ok(doc) = toml::from_str::<toml::Value>(&content)
            && let Some(workspace) = doc.get("workspace").and_then(|w| w.as_table())
                && let Some(members) = workspace.get("members").and_then(|m| m.as_array()) {
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

    crates
}
