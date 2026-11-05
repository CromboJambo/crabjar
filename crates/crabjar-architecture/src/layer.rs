/// Dependency layer definitions for the crabjar workspace.
///
/// Each layer declares which lower layers it may depend on.
/// A crate in layer N may only depend on crates in layers 0..=N.
use std::collections::HashMap;

/// All defined layers in the crabjar workspace.
pub const ALL_LAYERS: &[&str] = &[
    "common",
    "substrate",
    "authority",
    "runtime",
    "host",
    "product",
    "bridge",
    "skills",
];

/// Map of crate name → layer it belongs to.
/// This is the authoritative mapping — if a crate is not here,
/// it is not part of the workspace architecture model.
#[must_use]
pub fn crate_to_layer() -> HashMap<&'static str, usize> {
    let mut map = HashMap::new();

    // Layer 0: common (shared types, no workspace deps)
    map.insert("crabjar_lib", 0);

    // Layer 1: substrate — low-level storage and isolation primitives
    map.insert("agent-context", 1); // memory
    map.insert("analyze-common", 1);
    map.insert("crabjar-architecture", 1);
    map.insert("crabjar-browser", 1);
    map.insert("crabjar-compress", 1);
    map.insert("crabjar-cron", 1);
    map.insert("crabjar-delegation", 1);
    map.insert("crabjar-deploy", 1);
    map.insert("crabjar-durable-objects", 1);
    map.insert("crabjar-git", 1);
    map.insert("crabjar-github-client", 1);
    map.insert("crabjar-guard", 1); // guard
    map.insert("crabjar-gpui", 1);
    map.insert("crabjar-homeassistant", 1);
    map.insert("crabjar-lab-topology", 1);
    map.insert("crabjar-memory", 1);
    map.insert("crabjar-podman", 1);
    map.insert("crabjar-review", 1);
    map.insert("crabjar-sandbox", 1); // sandbox
    map.insert("crabjar-skill-scanner", 1);
    map.insert("crabjar-skills", 1);
    map.insert("crabjar-telemetry", 1); // telemetry
    map.insert("crabjar-train-dedup", 1);
    map.insert("crabjar-train-secret-stripper", 1);
    map.insert("crabjar-web-extract", 1);
    map.insert("crabjar-web-search", 1);
    map.insert("hermes-struct-tok-pipeline", 1);
    map.insert("slow-friend", 1);
    map.insert("universal-structure", 1);
    map.insert("vm-core", 1);

    // Layer 2: authority — capability/registry layer + tools depending on substrate
    map.insert("analyze-apk", 2);
    map.insert("analyze-dotnet", 2);
    map.insert("analyze-firmware", 2);
    map.insert("analyze-har", 2);
    map.insert("analyze-js", 2);
    map.insert("analyze-native", 2);
    map.insert("crabjar-benchmarks", 2);
    map.insert("crabjar-clawhub-client", 2);
    map.insert("crabjar-commit", 2);
    map.insert("crabjar-compress-mcp", 2);
    map.insert("crabjar-conductor", 2);
    map.insert("crabjar-scheduler", 2);
    map.insert("crabjar-ssh-bastion", 2);
    map.insert("crabjar-tool-binding", 2);
    map.insert("crabjar-tool-registry", 2); // tool_registry
    map.insert("crabjar-train-extract", 2);
    map.insert("crabjar-train-filewatcher", 2);
    map.insert("crabjar-trust-manager", 2);

    // Layer 3: runtime — execution runtime
    map.insert("analyze-mcp-server", 3);
    map.insert("crabjar-analyze", 3);
    map.insert("crabjar-herdr-mcp", 3);
    map.insert("crabjar-mcp-tool-registry", 3);
    map.insert("crabjar-plugin", 3);
    map.insert("crabjar-terminal", 3); // crates/terminal — typed terminal stream (ADR-005)
    map.insert("crabjar-unsloth-mcp", 3);
    map.insert("crabjar-worker", 3);
    map.insert("orchestrator", 3);
    map.insert("vm-bridge", 3); // axum-mux

    // Layer 4: host — host runtime crates
    map.insert("crabjar-host-agent", 4);
    map.insert("crabjar-host-core", 4);
    map.insert("crabjar-host-observe", 4);
    map.insert("crabjar-host-system", 4);
    map.insert("crabjar-host-webview", 4);
    map.insert("crabjar-kanban-orchestrator", 4);
    map.insert("host-graph", 4);
    map.insert("host-mqtt", 4);
    map.insert("host-screen", 4);

    // Layer 5: product — the crabjar binary and apps
    map.insert("crabjar", 5);
    map.insert("crabjar-app-teams", 5); // apps/teams
    map.insert("crabjar-app-terrarium", 5); // apps/terrarium
    map.insert("crabjar-commit-cli", 5);
    map.insert("crabjar-email-smtp", 5);
    map.insert("crabjar-host", 5); // host-binary
    map.insert("crabjar-structured-aux", 5);
    map.insert("terrarium", 5);

    // Layer 6: bridge — external protocol bridges
    map.insert("zed-acp-bridge", 6);
    map.insert("zed-acp-server", 6);

    // Layer 7: skills — agent-facing capabilities
    map.insert("skill-process-management", 7);
    map.insert("skill-reference-store", 7);
    map.insert("skill-script-runner", 7);

    map
}

/// Layers that a given layer may depend on (inclusive of itself).
/// Returns the set of allowed layer indices for each layer.
#[must_use]
pub fn allowed_dependencies(layer: usize) -> &'static [usize] {
    match layer {
        0 => &[0],                      // common → common only
        1 => &[0, 1],                   // substrate → common + substrate
        2 => &[0, 1, 2],                // authority → common + substrate + authority
        3 => &[0, 1, 2, 3],             // runtime → common + substrate + authority + runtime
        4 => &[0, 1, 2, 3, 4],          // host → all below + host
        5 => &[0, 1, 2, 3, 4, 5],       // product → all below + product
        6 => &[0, 1, 2, 3, 4, 5, 6],    // bridge → all below + bridge
        7 => &[0, 1, 2, 3, 4, 5, 6, 7], // skills → all
        _ => &[],
    }
}

/// Layer name for a given layer index.
#[must_use]
pub fn layer_name(layer: usize) -> Option<&'static str> {
    match layer {
        0 => Some("common"),
        1 => Some("substrate"),
        2 => Some("authority"),
        3 => Some("runtime"),
        4 => Some("host"),
        5 => Some("product"),
        6 => Some("bridge"),
        7 => Some("skills"),
        _ => None,
    }
}
