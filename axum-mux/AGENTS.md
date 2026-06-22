# AGENTS.md — vm-bridge (axum-mux)

> Purpose: Per-VM websocket relay for display protocols — hardened byte transport.

## Layer

Layer 3: runtime — execution runtime, may depend on layers 0, 1, 2, 3.

## Public API

- WebSocket proxy for display protocol bytes
- Per-VM process isolation
- Hardened transport (no protocol parsing, just byte forwarding)

## Key Files

- `src/lib.rs` — crate entry point
- WebSocket proxy logic
- Process supervisor logic

## Dependencies

- tokio, axum, futures-util, serde, toml, anyhow, tracing, tracing-subscriber

## Pitfalls

- vm-bridge has no lib target (WASM-only) — host-screen and apps/teams reference it but it won't compile on native
- Hardened by design: no protocol parsing, just byte transport
- Can be extended with screen sharing in the future
