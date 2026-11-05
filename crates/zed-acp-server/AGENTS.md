# AGENTS.md — zed-acp-server (zed-acp-server)

> Purpose: stdio JSON-RPC server for Zed Agent Protocol — ACP protocol execution.

## Layer

Layer 6: bridge — external protocol bridges, may depend on all layers.

## Public API

- stdio JSON-RPC server (ACP protocol execution)
- Tool call execution (via guard authorization)
- Telemetry recording (via crabjar-telemetry)

## Key Files

- `src/lib.rs` — crate entry point
- `src/main.rs` — stdio server entry point
- `src/server.rs` — JSON-RPC server implementation

## Dependencies

- tokio, tokio-stream, serde, serde_json, thiserror, anyhow, crabjar-guard, crabjar-telemetry, tracing, tracing-subscriber, uuid, chrono, crabjar_lib, tempfile

## Pitfalls

- stdio JSON-RPC requires stdin/stdout — cannot use TUI or interactive prompts
- Tool calls must pass through guard authorization before execution
- Telemetry records all tool invocations for audit trail
- crabjar_lib dependency means this crate is part of the crabjar workspace
