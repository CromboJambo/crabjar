# AGENTS.md — crabjar-app-teams (apps/teams)

> Purpose: Teams plugin — reference application for the JSON-RPC plugin protocol.

## Layer

Layer 5: product — product-facing crates, may depend on layers 0, 1, 2, 3, 4, 5.

## Public API

- Teams plugin implementation (Teams-specific adapter)
- JSON-RPC plugin protocol compliance
- Session lifecycle management

## Key Files

- `src/lib.rs` — crate entry point
- `src/teams_plugin.rs` — Teams plugin implementation
- `src/protocol.rs` — JSON-RPC protocol handling

## Dependencies

- tokio, serde, serde_json, tracing, uuid, chrono, async-trait, crabjar-host-core, crabjar-host-system, crabjar-host-webview, tempfile

## Pitfalls

- vm-bridge (axum-mux) is a binary-only native crate (no lib target) — do not add it as a library dependency; it is not used by this crate
- Teams plugin is the reference application — follow its patterns for new plugins
- JSON-RPC messages must conform to the plugin protocol schema
- Session lifecycle must handle disconnect/reconnect gracefully
