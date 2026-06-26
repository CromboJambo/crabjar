# Repository Guidelines

## Build & Test

Use `just` for workflows:

- `just check`: `cargo check --workspace`
- `just build`: `cargo build -p crabjar`
- `just run state list`: `cargo run -p crabjar -- state list` (args replaceable)
- `just test`: `cargo test --workspace`
- `just clean`: removes build artifacts

Narrow scope: `cargo check/clippy/test -p <crate>`

## CLI Output Contract

All command responses are structured JSON on stdout:

- Success: `"success": true`
- Error: `"success": false`, `"error"` string, `"usage"` array
- `workspace status` returns `"workspace": null` when `.crabjar_config.toml` is missing or malformed
- `knowledge` subcommands return structured fields (`rows`, `events`, `docs`, `ids`) — no plain-text summaries

Every derived output must include a `doubt` block: `assumptions`, `blind_spots`, `last_validation`, `stale_after`.

## Architecture

### Core (state-docs & execution)
- `src/main.rs`: CLI entry point
- `src/lib.rs`: shared library surface
- `memory/`: agent-context SQLite storage (knowledge.db)
- `guard/`: execution gate (guard.db)
- `telemetry/`: flight recorder
- `orchestrator/`: Axum SSE server
- `sandbox/`: execution sandbox
- `tool_registry/`: tool registry
- `axum-mux/`: vm-bridge (per-VM websocket relay)
- `zed-acp-bridge/` + `zed-acp-server/`: Zed Agent Protocol bridge

### Host runtime
- `host/host-core/`: Event bus, plugin API, WorkItem model, config
- `host/host-system/`: System tray, notifications, clipboard, secrets
- `host/host-observe/`: Metrics, tracing, health reporting
- `host/host-agent/`: Agent loop (observe→understand→plan→execute→verify→reflect)
- `host/host-webview/`: WebView session management, OAuth2, token cache
- `host/host-mqtt/`: MQTT client + Home Assistant discovery
- `host/host-graph/`: Microsoft Graph API client
- `host/host-screen/`: Screen capture + display protocol integration

### Host apps
- `apps/teams/`: Teams plugin (reference application)

### Skill crates
- `src/skill-script-runner/`: Skill script runner
- `src/skill-reference-store/`: Skill reference store

### State-docs
- `state-docs/`: Durable Markdown docs; overlays in `state-docs/overlay/*.overlay.json`

Workspace config from `.crabjar_config.toml`; missing/malformed → `workspace: null`.

## Execution Pipeline

`crabjar exec` passes through: `request → guard → concierge → telemetry → outcome → trust update`.
Execution is opt-in via `.crabjar_config.toml` `tool_execution_enabled`. All actions require real provenance lookup. Pending actions persist to GuardDb.

## Agent Autonomy Constraints

- Never execute sudo commands — present as user-run actions
- Detection ≠ authorization: observer reports must not trigger execution
- Reversibility gating: destructive actions require user permission
- Commands requiring root access are categorical user-run only

## Naming Collision Pattern

Parameter names must match column names, not semantic intent. Semantic naming drift causes structural bugs: `provenance_id` parameter querying `source_id` column is a known bug that makes every downstream caller operate on the wrong column. Always verify parameter-to-column alignment before implementing deactivate, filter, or query functions.

## Workspace Members

Declared in Cargo.toml `[workspace.members]`:
- **Core**: `memory`, `guard`, `telemetry`, `orchestrator`, `sandbox`, `tool_registry`, `axum-mux`
- **Host**: `host/host-core`, `host/host-system`, `host/host-observe`, `host/host-agent`, `host/host-webview`, `host/host-mqtt`, `host/host-graph`, `host/host-screen`
- **Apps**: `apps/teams`
- **Binary**: `src/host-binary`
- **Skills**: `src/skill-script-runner`, `src/skill-reference-store`
- **Zed ACP**: `zed-acp-bridge`, `zed-acp-server`

Nested crates use `src/<crate>/src/` pattern (not flat `src/<crate>/`).

## Coding Style

- `cargo fmt` before changes; `cargo clippy -- -D warnings`
- `snake_case` functions/variables/modules; `PascalCase` types/traits; `SCREAMING_SNAKE_CASE` constants
- `thiserror` for library errors; `?` propagation; no `unwrap()` outside tests

## Module Size Governance

**500 LoC rule**: No single `.rs` module may exceed 500 lines. This is a CI gate, not a suggestion.

### Rationale

Codex-core bloat is the anti-pattern Crabjar must avoid. The 500 LoC rule is cognitive load management, not bureaucracy. When a module grows past 500 LoC, it's a signal to split by concern:

- **Types** → separate file (e.g., `types.rs` → `action.rs`, `trust.rs`, `memory_types.rs`)
- **Context structs** → separate file (e.g., `GateContext`)
- **Result types** → separate file (e.g., `GateResult`)
- **Config** → separate file (e.g., `RiskConfig`)
- **Risk lists** → separate file (e.g., `CommandRisk`)

### How to split

1. Identify the concern (types, context, config, risk, etc.)
2. Create the new file with proper module doc comment
3. Move the relevant types/impls
4. Update `lib.rs` to `pub mod` the new module and re-export types
5. Update all `use crate::` imports in other modules
6. Run `cargo check --workspace` to verify

### Tooling

- `just module-sizes` — report all modules exceeding threshold (default: 500)
- `just module-sizes-check` — CI gate (fails if any module exceeds threshold)
- CI job: `.github/workflows/rust.yml` → `module-sizes` job

### Current guard crate structure (post-split)

| Module | LoC | Concern |
|--------|-----|---------|
| `trust.rs` | 406 | TrustScore, TrustLayer, TrustManager, ReviewAction, AnnealConfig, RetrievalBand |
| `memory_types.rs` | 193 | NodeKind, MemoryNode, EdgeRelation, MemoryEdge |
| `memory.rs` | 380 | MemoryGraph (DB-backed impl) |
| `action.rs` | 318 | ActionStatus, OutcomeStatus, ActionRequest, ActionOutcome |
| `inference.rs` | 298 | ModelInferenceKind, ModelInferenceRequest, ModelInferenceOutcome |
| `gate.rs` | 480 | ExecutionGate impl |
| `gate_context.rs` | 108 | GateContext struct |
| `gate_result.rs` | 88 | GateResult enum |
| `command_risk.rs` | 130 | CommandRisk, HIGH/MEDIUM_RISK_COMMANDS |
| `risk_config.rs` | 56 | RiskConfig |

## Testing

- `#[test]` and `#[tokio::test]`; unit tests beside code under `#[cfg(test)]`
- CLI integration tests in `tests/cli.rs` using `std::process::Command`
- Filesystem fixtures: `tempfile::tempdir()`; never write into repository
- Test names: descriptive snake_case stating behaviour, e.g. `state_list_returns_json`

## Drift Governance

`project_map.md` stale after >7 days without modification. Divergence between documented structure and actual filesystem is a structural integrity concern.

## Zed Agent Protocol

- Zed agent servers require stdin/stdout JSON-RPC
- `zed-acp-bridge` (Wasm extension) + `zed-acp-server` (stdio binary)
- Wasm deps: `zed_extension_api`, `serde`, `uuid(js)` only
- `tokio` pulls `mio` (wasm incompatible); `rusqlite` pulls `libsqlite3-sys` (C compilation fails on wasm); `uuid` requires `js` feature; HTTP (axum) cannot be adapted to stdio

## Navigation

Use `project_map.md` and `AGENTS.md` as primary navigation tools. Verify paths before assuming they exist.

## LLM Runner Status

`llm-runner` (in `llm-workspace/`) is experimental: CPU fallback kernels (`CpuGemmKernel`, `CpuAttentionKernel`) are operational for verification. GPU path is stubbed (`GemmBuilder::build` returns `KernelFromPtx` with no-op matmul). Weight loading → inference pipeline bridge, RoPE/RMSNorm/activations/LM head/sampling are unimplemented. K-family quantization dequantization is unimplemented. See `llmrunner.md` for gap analysis.
