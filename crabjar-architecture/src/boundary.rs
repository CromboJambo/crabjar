/// Boundary enforcement: verify that crate dependencies respect layer ordering.
///
/// Parses Cargo.toml files in the workspace and checks that each crate
/// Only depends on crates in allowed layers (its own layer or below).
use crate::layer::{allowed_dependencies, crate_to_layer, layer_name};
use std::fs;
use std::path::{Path, PathBuf};

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
    /// Direct dependencies (excluding workspace = true deps, which are shared deps not structural)
    pub direct_deps: Vec<String>,
}

/// Parse a Cargo.toml and extract the crate name and direct (non-workspace) dependencies.
fn parse_crate_info(manifest_path: &Path) -> Option<CrateInfo> {
    let content = fs::read_to_string(manifest_path).ok()?;
    let parsed = content.parse::<toml::Value>().ok()?;

    // Get crate name
    let name = parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(std::string::ToString::to_string)?;

    // Extract direct (non-workspace) dependencies from [dependencies], [dev-dependencies], [build-dependencies]
    let mut direct_deps = Vec::new();

    for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(deps) = parsed.get(section)
            && let Some(table) = deps.as_table()
        {
            for (dep_name, dep_val) in table {
                // Skip workspace = true deps — they are shared dependency versions, not structural deps
                if dep_val.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
                    continue;
                }
                direct_deps.push(dep_name.clone());
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
fn discover_workspace_members(workspace_root: &Path) -> Result<Vec<CrateInfo>, String> {
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

        if manifest_path.exists()
            && let Some(info) = parse_crate_info(&manifest_path)
        {
            crates.push(info);
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

    for crate_info in &crates {
        let Some(&layer) = layer_map.get(crate_info.name.as_str()) else {
            // Crate not in the architecture model — skip it (could be an external dep)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_layer_allowed_dependencies() {
        // Layer 0 (common) can only depend on layer 0
        assert_eq!(allowed_dependencies(0), &[0]);

        // Layer 7 (skills) can depend on everything
        assert_eq!(allowed_dependencies(7), &[0, 1, 2, 3, 4, 5, 6, 7]);

        // Layer 3 (runtime) can depend on 0, 1, 2, 3
        assert_eq!(allowed_dependencies(3), &[0, 1, 2, 3]);
    }

    #[test]
    fn test_layer_name() {
        assert_eq!(layer_name(0), Some("common"));
        assert_eq!(layer_name(1), Some("substrate"));
        assert_eq!(layer_name(3), Some("runtime"));
        assert_eq!(layer_name(7), Some("skills"));
        assert_eq!(layer_name(99), None);
    }

    #[test]
    fn test_crate_to_layer_completeness() {
        // All 22 workspace members should be mapped
        let layer_map = crate_to_layer();
        assert!(
            layer_map.len() >= 22,
            "Not all workspace members are mapped"
        );
    }

    #[test]
    fn test_crabjar_lib_in_common() {
        let layer_map = crate_to_layer();
        assert_eq!(layer_map.get("crabjar_lib"), Some(&0));
    }

    #[test]
    fn test_guard_in_substrage() {
        let layer_map = crate_to_layer();
        assert_eq!(layer_map.get("crabjar-guard"), Some(&1));
    }

    #[test]
    fn test_orchestrator_in_runtime() {
        let layer_map = crate_to_layer();
        assert_eq!(layer_map.get("orchestrator"), Some(&3));
    }

    #[test]
    fn test_host_in_host_layer() {
        let layer_map = crate_to_layer();
        assert_eq!(layer_map.get("crabjar-host-core"), Some(&4));
        assert_eq!(layer_map.get("crabjar-host-system"), Some(&4));
        assert_eq!(layer_map.get("crabjar-host-agent"), Some(&4));
    }

    #[test]
    fn test_product_in_product_layer() {
        let layer_map = crate_to_layer();
        assert_eq!(layer_map.get("crabjar-host"), Some(&5));
        assert_eq!(layer_map.get("crabjar-app-teams"), Some(&5));
    }

    #[test]
    fn test_bridge_in_bridge_layer() {
        let layer_map = crate_to_layer();
        assert_eq!(layer_map.get("zed-acp-bridge"), Some(&6));
        assert_eq!(layer_map.get("zed-acp-server"), Some(&6));
    }

    #[test]
    fn test_skills_in_skills_layer() {
        let layer_map = crate_to_layer();
        assert_eq!(layer_map.get("skill-script-runner"), Some(&7));
        assert_eq!(layer_map.get("skill-reference-store"), Some(&7));
    }

    #[test]
    fn test_violation_display() {
        let v = Violation {
            from_crate: "bad-crate".to_string(),
            from_layer: 5,
            to_crate: "higher-crate".to_string(),
            to_layer: 7,
        };
        let msg = v.to_string();
        assert!(msg.contains("LAYER VIOLATION"));
        assert!(msg.contains("bad-crate"));
        assert!(msg.contains("higher-crate"));
    }

    #[test]
    fn test_crabjar_guard_can_depend_on_agent_context() {
        // guard (layer 1) can depend on agent-context (layer 1) — same layer is OK
        let layer_map = crate_to_layer();
        let guard_layer = layer_map["crabjar-guard"];
        let context_layer = layer_map["agent-context"];
        let allowed = allowed_dependencies(guard_layer);
        assert!(
            allowed.contains(&context_layer),
            "guard (layer {guard_layer}) should be able to depend on agent-context (layer {context_layer})"
        );
    }

    #[test]
    fn test_orchestrator_can_depend_on_guard() {
        // orchestrator (layer 3) can depend on guard (layer 1) — lower layer is OK
        let layer_map = crate_to_layer();
        let orch_layer = layer_map["orchestrator"];
        let guard_layer = layer_map["crabjar-guard"];
        let allowed = allowed_dependencies(orch_layer);
        assert!(
            allowed.contains(&guard_layer),
            "orchestrator (layer {orch_layer}) should be able to depend on guard (layer {guard_layer})"
        );
    }

    #[test]
    fn test_zed_acp_bridge_can_depend_on_guard() {
        // zed-acp-bridge (layer 6) can depend on guard (layer 1) — lower layer is OK
        let layer_map = crate_to_layer();
        let bridge_layer = layer_map["zed-acp-bridge"];
        let guard_layer = layer_map["crabjar-guard"];
        let allowed = allowed_dependencies(bridge_layer);
        assert!(allowed.contains(&guard_layer));
    }

    #[test]
    fn test_host_agent_can_depend_on_host_core() {
        // host-agent (layer 4) can depend on host-core (layer 4) — same layer is OK
        let layer_map = crate_to_layer();
        let agent_layer = layer_map["crabjar-host-agent"];
        let core_layer = layer_map["crabjar-host-core"];
        let allowed = allowed_dependencies(agent_layer);
        assert!(allowed.contains(&core_layer));
    }

    #[test]
    fn test_teams_can_depend_on_host_system() {
        // crabjar-app-teams (layer 5) can depend on host-system (layer 4) — lower layer is OK
        let layer_map = crate_to_layer();
        let teams_layer = layer_map["crabjar-app-teams"];
        let system_layer = layer_map["crabjar-host-system"];
        let allowed = allowed_dependencies(teams_layer);
        assert!(allowed.contains(&system_layer));
    }

    #[test]
    fn test_screen_can_depend_on_vm_bridge() {
        // host-screen (layer 4) can depend on vm-bridge (layer 3) — lower layer is OK
        let layer_map = crate_to_layer();
        let screen_layer = layer_map["host-screen"];
        let bridge_layer = layer_map["vm-bridge"];
        let allowed = allowed_dependencies(screen_layer);
        assert!(allowed.contains(&bridge_layer));
    }

    #[test]
    fn test_zed_acp_server_can_depend_on_guard() {
        // zed-acp-server (layer 6) can depend on guard (layer 1)
        let layer_map = crate_to_layer();
        let server_layer = layer_map["zed-acp-server"];
        let guard_layer = layer_map["crabjar-guard"];
        let allowed = allowed_dependencies(server_layer);
        assert!(allowed.contains(&guard_layer));
    }

    #[test]
    fn test_zed_acp_server_can_depend_on_telemetry() {
        // zed-acp-server (layer 6) can depend on crabjar-telemetry (layer 1)
        let layer_map = crate_to_layer();
        let server_layer = layer_map["zed-acp-server"];
        let telem_layer = layer_map["crabjar-telemetry"];
        let allowed = allowed_dependencies(server_layer);
        assert!(allowed.contains(&telem_layer));
    }

    #[test]
    fn test_zed_acp_bridge_can_depend_on_guard_and_context() {
        // zed-acp-bridge (layer 6) can depend on guard (layer 1) and agent-context (layer 1)
        let layer_map = crate_to_layer();
        let bridge_layer = layer_map["zed-acp-bridge"];
        let guard_layer = layer_map["crabjar-guard"];
        let context_layer = layer_map["agent-context"];
        let allowed = allowed_dependencies(bridge_layer);
        assert!(allowed.contains(&guard_layer));
        assert!(allowed.contains(&context_layer));
    }

    #[test]
    fn test_orchestrator_can_depend_on_guard_telemetry_and_context() {
        // orchestrator (layer 3) can depend on guard (1), telemetry (1), agent-context (1)
        let layer_map = crate_to_layer();
        let orch_layer = layer_map["orchestrator"];
        let allowed = allowed_dependencies(orch_layer);
        assert!(allowed.contains(&layer_map["crabjar-guard"]));
        assert!(allowed.contains(&layer_map["crabjar-telemetry"]));
        assert!(allowed.contains(&layer_map["agent-context"]));
    }

    #[test]
    fn test_host_binary_can_depend_on_all_host_crates() {
        // crabjar-host (layer 5) can depend on all host crates (layer 4)
        let layer_map = crate_to_layer();
        let host_layer = layer_map["crabjar-host"];
        let allowed = allowed_dependencies(host_layer);
        for host_crate in &[
            "crabjar-host-core",
            "crabjar-host-system",
            "crabjar-host-observe",
            "crabjar-host-agent",
            "crabjar-host-webview",
        ] {
            let crate_layer = layer_map[host_crate];
            assert!(
                allowed.contains(&crate_layer),
                "crabjar-host (layer {host_layer}) should depend on {host_crate}"
            );
        }
    }

    #[test]
    fn test_host_binary_can_depend_on_teams() {
        // crabjar-host (layer 5) can depend on crabjar-app-teams (layer 5) — same layer
        let layer_map = crate_to_layer();
        let host_layer = layer_map["crabjar-host"];
        let teams_layer = layer_map["crabjar-app-teams"];
        let allowed = allowed_dependencies(host_layer);
        assert!(allowed.contains(&teams_layer));
    }

    #[test]
    fn test_tools_registry_cannot_depend_on_host() {
        // tool_registry (layer 2) should NOT be able to depend on host (layer 4)
        let layer_map = crate_to_layer();
        let reg_layer = layer_map["crabjar-tool-registry"];
        let host_layer = layer_map["crabjar-host-core"];
        let allowed = allowed_dependencies(reg_layer);
        assert!(!allowed.contains(&host_layer));
    }

    #[test]
    fn test_sandbox_cannot_depend_on_orchestrator() {
        // sandbox (layer 1) should NOT be able to depend on orchestrator (layer 3)
        let layer_map = crate_to_layer();
        let sandbox_layer = layer_map["crabjar-sandbox"];
        let orch_layer = layer_map["orchestrator"];
        let allowed = allowed_dependencies(sandbox_layer);
        assert!(!allowed.contains(&orch_layer));
    }

    #[test]
    fn test_guard_cannot_depend_on_orchestrator() {
        // guard (layer 1) should NOT be able to depend on orchestrator (layer 3)
        let layer_map = crate_to_layer();
        let guard_layer = layer_map["crabjar-guard"];
        let orch_layer = layer_map["orchestrator"];
        let allowed = allowed_dependencies(guard_layer);
        assert!(!allowed.contains(&orch_layer));
    }

    #[test]
    fn test_crates_in_layer() {
        let substrate_crates = crates_in_layer(1);
        assert!(substrate_crates.contains(&"agent-context".to_string()));
        assert!(substrate_crates.contains(&"crabjar-guard".to_string()));
        assert!(substrate_crates.contains(&"crabjar-telemetry".to_string()));
        assert!(substrate_crates.contains(&"crabjar-sandbox".to_string()));
        assert_eq!(substrate_crates.len(), 4);
    }

    #[test]
    fn test_crate_layer_lookup() {
        assert_eq!(crate_layer("crabjar-guard"), Some((1, "substrate")));
        assert_eq!(crate_layer("orchestrator"), Some((3, "runtime")));
        assert_eq!(crate_layer("crabjar-host-agent"), Some((4, "host")));
        assert_eq!(crate_layer("nonexistent"), None);
    }

    // Integration test: check the actual workspace
    #[test]
    fn test_workspace_boundaries_are_valid() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let result = enforce_boundaries(&workspace_root);
        assert!(
            result.is_ok(),
            "Workspace boundaries violated:\n{}",
            result
                .unwrap_err()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
