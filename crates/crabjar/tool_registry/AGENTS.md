# AGENTS.md — crabjar-tool-registry (tool_registry)

> Purpose: MCP tool registry — dynamic capability discovery, tool metadata, and versioned interfaces.

## Layer

Layer 2: authority — capability/registry layer, may depend on layers 0, 1, 2.

## Public API

- Tool discovery (MCP scanning, state-based discovery, auto-registration, binary validation)
- Tool metadata (description, params, return types)
- Fallback chains for tool availability
- Versioned tool interfaces

## Key Files

- `src/lib.rs` — crate entry point
- `src/tool_registry.rs` — core registry logic
- `src/schema.rs` — SQLite schema
- `src/error.rs` — error types

## Dependencies

- tokio, serde, serde_json, chrono, uuid, rusqlite, tracing, thiserror, path-absolutize, reqwest, which, tempfile

## Pitfalls

- Tool discovery must validate binaries before registration
- Fallback chains should handle missing tools gracefully
- Versioned interfaces prevent breaking changes in tool contracts
