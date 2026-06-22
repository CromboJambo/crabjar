# AGENTS.md — zed-acp-bridge (zed-acp-bridge)

> Purpose: Wasm extension for Zed Agent Protocol — tool call mapping and gate enforcement.

## Layer

Layer 6: bridge — external protocol bridges, may depend on all layers.

## Public API

- Tool call mapping (map Zed ACP tool calls to crabjar actions)
- Gate enforcement (check guard before executing tool calls)
- WASM extension for Zed editor

## Key Files

- `src/lib.rs` — WASM extension entry point
- `src/tool_mapping.rs` — tool call mapping logic
- `src/gate.rs` — gate enforcement

## Dependencies

- zed_extension_api, serde, serde_json, thiserror, uuid, chrono, zed-acp-server, agent-context, crabjar-guard, tempfile

## Pitfalls

- WASM deps are strictly limited: zed_extension_api, serde, uuid(js) only
- tokio pulls mio (wasm incompatible) — cannot use tokio in WASM
- rusqlite pulls libsqlite3-sys (C compilation fails on wasm) — cannot use rusqlite in WASM
- HTTP (axum) cannot be adapted to stdio — cannot use axum in WASM
- uuid requires `js` feature for WASM compatibility
- zed-acp-server is a stdio binary, not WASM — bridge communicates via stdio
