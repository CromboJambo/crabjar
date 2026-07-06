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

## Architecture Overview

Crabjar is a Rust workspace. Discover the current member list from `Cargo.toml` — it's the source of truth.

Key subsystems:
- **`guard/`**: Execution gate, trust layers, scope isolation. The canonical example of the module-splitting convention.
- **`memory/`**: Agent-context SQLite storage + state-docs querier (`memory/src/state_docs/`).
- **`host/`**: 8 host crates (core, system, observe, agent, webview, mqtt, graph, screen).
- **`orchestrator/`**: Axum SSE server + unified inference backend (LM Studio, MistralRs).
- **`zed-acp-bridge/`** + **`zed-acp-server/`**: Zed Agent Protocol bridge (Wasm + stdio).
- **`crabjar-architecture/`**: Mechanical dependency boundary enforcement (8-layer model).
- **`apps/teams/`**: Teams plugin (reference application).
- **`src/skill-*`**: Skill crates (script-runner, reference-store).

For crate-level details, use `ls` on the crate directory or read its `AGENTS.md`.

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

## Coding Style

- `cargo fmt` before changes; `cargo clippy -- -D warnings`
- `snake_case` functions/variables/modules; `PascalCase` types/traits; `SCREAMING_SNAKE_CASE` constants
- `thiserror` for library errors; `?` propagation; no `unwrap()` outside tests

## Module Size Governance

**500 LoC rule**: No single `.rs` module may exceed 500 lines. This is a CI gate, not a suggestion.

Codex-core bloat is the anti-pattern Crabjar must avoid. When a module grows past 500 LoC, split by concern:

- **Types** → separate file (e.g., `types.rs` → `action.rs`, `trust.rs`, `memory_types.rs`)
- **Context structs** → separate file (e.g., `GateContext`)
- **Result types** → separate file (e.g., `GateResult`)
- **Config** → separate file (e.g., `RiskConfig`)
- **Risk lists** → separate file (e.g., `CommandRisk`)

Tooling: `just module-sizes` (report), `just module-sizes-check` (CI gate).

## Testing

- `#[test]` and `#[tokio::test]`; unit tests beside code under `#[cfg(test)]`
- CLI integration tests in `tests/cli.rs` using `std::process::Command`
- Filesystem fixtures: `tempfile::tempdir()`; never write into repository
- Test names: descriptive snake_case stating behaviour, e.g. `state_list_returns_json`

## Drift Governance

`project_map.md` stale after >7 days without modification. Divergence between documented structure and actual filesystem is a structural integrity concern.

### State-Doc Staleness (Three-Tier Thresholds)

State-docs use a graduated staleness model with three tiers:

| Tier | Age | Behavior |
|------|-----|----------|
| **Fresh** | < 7 days | Trusted content, no warnings |
| **Stale** | 7–14 days | Warning — content may have drifted from indexed state; `is_trustworthy` still true |
| **Expired** | 14–30 days | Untrustworthy without re-index; flag for regeneration |
| **Moldy** | > 30 days | Corroded beyond useful provenance; discarded unless additional context (annotations) added since last modification relative to reconstruction cost |

The `StalenessStatus` enum is defined in `memory/src/state_docs/models.rs`. Use `StateDocQuerier::staleness_status()` to compute it — this checks both age and whether annotations were added after the doc was last modified.

CLI: `crabjar state staleness <doc_name>` returns structured JSON with status, days_old, is_trustworthy, and warning fields.

## Zed Agent Protocol

- Zed agent servers require stdin/stdout JSON-RPC
- `zed-acp-bridge` (Wasm extension) + `zed-acp-server` (stdio binary)
- Wasm deps: `zed_extension_api`, `serde`, `uuid(js)` only
- `tokio` pulls `mio` (wasm incompatible); `rusqlite` pulls `libsqlite3-sys` (C compilation fails on wasm); `uuid` requires `js` feature; HTTP (axum) cannot be adapted to stdio

## Navigation

- **`project_map.md`**: Structural map. Has its own freshness tracking (last audit date in Section 10). If >7 days old, treat as potentially stale.
- **`agent_config.md`**: Agent philosophy and behavior guidelines.
- **`ROADMAP.md`**: Development priorities and completed phases.
- **`<crate>/AGENTS.md`**: Per-crate documentation.

### Document Freshness Protocol

These docs decay. Here's how to handle it:

1. **Always verify structure before trusting details**: Run `ls <path>` or `list_directory` before relying on documented file counts, module lists, or LoC numbers. The filesystem is the source of truth.

2. **Treat structural tables as hints, not contracts**: Module tables, workspace member lists, and file inventories are snapshots. They tell you *what existed when written*, not *what exists now*. Use them for patterns and conventions, not exact counts.

3. **When in doubt, discover**: `list_directory` > read a stale file listing. `grep` > rely on documented API surfaces. `cargo check` > trust a documented build command.

4. **Update on divergence**: If you discover the filesystem has diverged from the docs during a task, update the relevant section with a note about what changed. Don't wait for a scheduled audit.

> **Key principle**: The *conventions* in these docs matter more than the *inventory*. A module-splitting rule is useful forever. A specific file count is only useful for the next 7 days.

## LLM Runner

Experimental (in `llm-workspace/`). CPU fallback kernels operational; GPU path stubbed; K-family dequantization unimplemented. See `llmrunner.md` for gap analysis.
