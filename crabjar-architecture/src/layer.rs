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
pub fn crate_to_layer() -> HashMap<&'static str, usize> {
    let mut map = HashMap::new();

    // Layer 0: common (shared types, no workspace deps)
    // No crates in this layer yet — all crates have workspace deps

    // Layer 1: substrate — low-level storage and isolation
    map.insert("agent-context", 1);       // memory
    map.insert("crabjar-guard", 1);       // guard
    map.insert("crabjar-telemetry", 1);   // telemetry
    map.insert("crabjar-sandbox", 1);     // sandbox

    // Layer 2: authority — capability/registry layer
    map.insert("crabjar-tool-registry", 2); // tool_registry

    // Layer 3: runtime — execution runtime
    map.insert("orchestrator", 3);
    map.insert("vm-bridge", 3);           // axum-mux

    // Layer 4: host — host runtime crates
    map.insert("crabjar-host-core", 4);
    map.insert("crabjar-host-system", 4);
    map.insert("crabjar-host-observe", 4);
    map.insert("crabjar-host-agent", 4);
    map.insert("crabjar-host-webview", 4);
    map.insert("host-mqtt", 4);
    map.insert("host-graph", 4);
    map.insert("host-screen", 4);

    // Layer 5: product — product-facing crates
    map.insert("crabjar-host", 5);        // host-binary
    map.insert("crabjar-app-teams", 5);   // apps/teams

    // Layer 6: bridge — external protocol bridges
    map.insert("zed-acp-bridge", 6);
    map.insert("zed-acp-server", 6);

    // Layer 7: skills — agent skills
    map.insert("skill-script-runner", 7);
    map.insert("skill-reference-store", 7);

    // Layer 0: common — the root crabjar crate itself
    map.insert("crabjar_lib", 0);

    map
}

/// Layers that a given layer may depend on (inclusive of itself).
/// Returns the set of allowed layer indices for each layer.
pub fn allowed_dependencies(layer: usize) -> &'static [usize] {
    match layer {
        0 => &[0],                    // common → common only
        1 => &[0, 1],                 // substrate → common + substrate
        2 => &[0, 1, 2],              // authority → common + substrate + authority
        3 => &[0, 1, 2, 3],           // runtime → common + substrate + authority + runtime
        4 => &[0, 1, 2, 3, 4],        // host → all below + host
        5 => &[0, 1, 2, 3, 4, 5],    // product → all below + product
        6 => &[0, 1, 2, 3, 4, 5, 6], // bridge → all below + bridge
        7 => &[0, 1, 2, 3, 4, 5, 6, 7], // skills → all
        _ => &[],
    }
}

/// Layer name for a given layer index.
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
