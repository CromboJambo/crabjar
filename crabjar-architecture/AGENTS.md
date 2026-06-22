# AGENTS.md — crabjar-architecture

> Purpose: Mechanical dependency boundary enforcement for the crabjar workspace.

## Layer Model

This crate defines the dependency layering model and enforces it mechanically.

- Layer 0: common (shared types, no workspace deps)
- Layer 1: substrate (memory, guard, telemetry, sandbox)
- Layer 2: authority (tool_registry)
- Layer 3: runtime (orchestrator, vm-bridge)
- Layer 4: host (host-core, host-system, host-observe, host-agent, etc.)
- Layer 5: product (apps, host-binary)
- Layer 6: bridge (zed-acp-bridge, zed-acp-server)
- Layer 7: skills (skill-script-runner, skill-reference-store)

## Public API

- `layer::ALL_LAYERS` — all defined layer names
- `layer::crate_to_layer()` — crate name → layer index mapping
- `layer::allowed_dependencies(layer)` — which layers a given layer may import
- `layer::crate_layer(name)` — lookup crate's layer
- `layer::crates_in_layer(n)` — list crates in a layer
- `boundary::check_workspace_boundaries(path)` — return violations
- `boundary::enforce_boundaries(path)` — panic on violations (for tests)
- `boundary::Violation` — violation record with Display

## Dependencies

Only base-layer deps (serde, toml, thiserror, anyhow, path-absolutize). No workspace member deps.

## Pitfalls

- When adding a new crate, update `crate_to_layer()` in `layer.rs`
- The integration test `test_workspace_boundaries_are_valid` will fail if a new dependency violates the model
- External crates (not in `crate_to_layer()`) are silently skipped — they are not part of the boundary model
