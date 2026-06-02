**⚠️ Disclaimer: Experimental Project ⚠️**

Please note that CrabJar is an **experimental learning hobby project**. The codebase may contain architectural compromises, undocumented features, and deliberate use of unsafe Rust blocks for educational purposes. This repository is not intended for production use without thorough review and refactoring.

*Unsafe Code Warning:* Currently, the project utilizes approximately `<N>` number of `unsafe` Rust blocks across several libraries (e.g., `<lib1>`, `<lib2>`). These areas require careful security and memory safety auditing before deployment in a critical system. Please treat this codebase with academic curiosity rather than production confidence.

CrabJar is a Rust 2024 workspace centered on the `crabjar` CLI. It includes state-docs management, workspace config loading, knowledge-store bridge, codeburn token tracking, orchestrator (Axum SSE server), guard (execution gate), telemetry (flight recorder), sandbox (agent isolation), safetensors (model weight storage), tool registry, codeburn CLI, skill script runner, skill reference store, and Zed agent protocol bridge.

## Workspace Members

Declared in Cargo.toml `[workspace.members]`:

- `src/crabjar-config`
- `memory`
- `orchestrator`
- `src/codeburn-provider`
- `src/codeburn-config`
- `src/codeburn-classifier`
- `src/codeburn-pricing`
- `src/codeburn`
- `src/skill-script-runner`
- `src/skill-reference-store`
- `guard`
- `telemetry`
- `sandbox`
- `safetensors`
- `tool_registry`
- `src/llm-plug-in`
- `src/llm-runner`
- `zed-acp-bridge`
- `zed-acp-server`

Nested crates under `src/` use `src/<crate>/src/` pattern (not flat `src/<crate>/`).

## Architecture

### Core crabjar crates

| Component | Role |
| :--- | :--- |
| `crabjar` | CLI for state-docs management |
| `crabjar-config` | TOML config crate |
| `memory` | Agent-context SQLite storage |
| `orchestrator` | Axum SSE server (ACP orchestrator) |
| `guard` | Trust layers, annealing, execution gate |
| `telemetry` | Flight recorder, command executor |
| `sandbox` | Agent isolation tooling |
| `safetensors` | Model weight storage |
| `tool_registry` | MCP tool registry |
| `codeburn-provider` | ProviderRegistry |
| `codeburn-classifier` | TaskClassifier |
| `codeburn-pricing` | PricingEngine |
| `codeburn-config` | CodeBurnConfig |
| `codeburn` | codeburn CLI binary |
| `skill-script-runner` | Skill script discovery and execution |
| `skill-reference-store` | Skill reference indexing and staleness |
| `llm-plug-in` | LLM plugin protocol |
| `llm-runner` | LLM runner (Candle/Burn backends) |
| `zed-acp-bridge` | Wasm extension (tool call mapping + gate enforcement) |
| `zed-acp-server` | stdio JSON-RPC server (ACP protocol) |

### File layout

```text
crabjar/
├── Cargo.toml               # Workspace root — 20 members, shared deps
├── Cargo.lock               # Locked dependency graph
├── AGENTS.md               # Repository guidelines
├── README.md               # Project overview
├── agent_config.md         # Agent configuration
├── Justfile               # Task runner shortcuts
├── Containerfile, Dockerfile
│
│  Core crabjar crates
├── src/main.rs              # CLI entry point
├── src/lib.rs               # shared library surface
├── src/state_docs.rs        # state-doc and overlay handling
├── src/crabjar-config/      # workspace config crate
│
│  Supporting crates
├── memory/                  # agent-context crate, SQLite-backed storage
├── orchestrator/            # Axum SSE server
├── guard/                   # Trust layers, execution gate
├── telemetry/               # Flight recorder
├── sandbox/                 # Agent isolation
├── safetensors/             # Model weight storage
├── tool_registry/           # MCP tool registry
├── src/codeburn-provider/   # ProviderRegistry
├── src/codeburn-classifier/ # TaskClassifier
├── src/codeburn-pricing/    # PricingEngine
├── src/codeburn-config/     # CodeBurnConfig
├── src/codeburn/            # codeburn CLI binary
├── src/skill-script-runner/ # Skill script discovery
├── src/skill-reference-store/ # Skill reference indexing
│
│  LLM inference crates
├── src/llm-plug-in/         # LLM plugin protocol
├── src/llm-runner/          # LLM runner (Candle/Burn backends)
│
│  Documentation
├── state-docs/              # Durable Markdown state documentation
├── state-docs/overlay/      # Overlay JSON sidecars
│
│  Non-crate artifacts
├── tests/cli.rs             # CLI integration tests
├── ui-state-copy/           # UI state copy
├── git/                     # git helper scripts
├── gitignore/               # gitignore management
├── testing/configs/         # test configs
├── reference_materials/     # excluded from Git
└── bin/                     # compiled binaries
```

## Build & Test

Use `just` for workflows:

- `just check`: `cargo check --workspace`
- `just build`: `cargo build -p crabjar`
- `just run state list`: `cargo run -p crabjar -- state list` (args replaceable)
- `just test`: `cargo test --workspace`
- `just clean`: removes build artifacts

Narrow scope: `cargo check/clippy/test -p <crate>`

Formatting: `cargo fmt --all`

Lint: `cargo clippy --workspace -- -D warnings`

## CLI Output Contract

All command responses are structured JSON on stdout:

- Success: `"success": true`
- Error: `"success": false`, `"error"` string, `"usage"` array
- `workspace status` returns `"workspace": null` when `.crabjar_config.toml` is missing or malformed
- `knowledge` subcommands return structured fields (`rows`, `events`, `docs`, `ids`) — no plain-text summaries

Every derived output must include a `doubt` block: `assumptions`, `blind_spots`, `last_validation`, `stale_after`.

## Commands

### State docs

```bash
crabjar state list
crabjar state show <doc>
crabjar state annotate <doc> <message>
crabjar state question <doc> <message>
crabjar state resolve <doc> <id>
crabjar workspace status
```

### Knowledge

```bash
crabjar knowledge sync <doc>
crabjar knowledge query --tags=<tag>
crabjar knowledge events --limit=<n>
crabjar knowledge verify
crabjar knowledge deactivate <id> --reason=<reason>
```

### Exec

```bash
crabjar exec <action>
```

## State Docs

Crabjar treats `state-docs/` as a shared project memory surface. Markdown files are the durable source documents; agent/user comments live in `state-docs/overlay/*.overlay.json` so they can be updated without rewriting the base docs.

## Architectural Constraints

### Detection ≠ Authorization

Runtime execution is executor-capable. Execution is opt-in via `.crabjar_config.toml` `tool_execution_enabled`. The single pipeline is: `request → guard → concierge → telemetry → outcome → trust update`. Pending actions persist to GuardDb. All actions require real provenance lookup.

### Doubt Output Requirement

Every derived output must include a `doubt` block with:

- `assumptions` — what it assumed to produce this output
- `blind_spots` — what it couldn't see
- `last_validation` — when this was last checked against raw data
- `stale_after` — when this output should be considered stale

### Naming Collision Pattern

Parameter names must match column names, not semantic intent. Semantic naming drift causes structural bugs. Always verify parameter-to-column alignment before implementing deactivate, filter, or query functions.

### Agent Autonomy Constraints

- Never execute sudo commands — present as user-run actions
- Detection ≠ authorization: observer reports must not trigger execution
- Reversibility gating: destructive actions require user permission
- Commands requiring root access are categorical user-run only

## Zed Agent Server Protocol

- Zed agent servers require stdin/stdout JSON-RPC communication
- Zed sends `{ "method": "...", "params": {...} }` on stdin
- Server responds with `{ "type": "result", "value": {...} }` on stdout
- HTTP orchestrator (axum, TCP port 3000) is incompatible with Zed — requires dedicated stdio server
- Two-layer architecture: `zed-acp-bridge` (Wasm extension) + `zed-acp-server` (stdio binary)
- Wasm deps: `zed_extension_api`, `serde`, `uuid(js)` only
- `tokio` pulls `mio` (wasm incompatible); `rusqlite` pulls `libsqlite3-sys` (C compilation fails on wasm); `uuid` requires `js` feature; HTTP (axum) cannot be adapted to stdio

## Code Quality & Style

- Formatter: rustfmt with default settings
- Linter: Clippy at --deny warnings
- Naming: snake_case for functions/variables/modules, PascalCase for types/traits, SCREAMING_SNAKE_CASE for constants
- Error handling: thiserror for library crates, anyhow for binary/CLI crates
- No unwrap/expect for recoverable errors in library code
- Dependencies: add to workspace root first, then reference with `{ workspace = true }`

## Testing

- `#[test]` and `#[tokio::test]`; unit tests beside code under `#[cfg(test)]`
- CLI integration tests in `tests/cli.rs` using `std::process::Command`
- Filesystem fixtures: `tempfile::tempdir()`; never write into repository
- Test names: descriptive snake_case stating behaviour, e.g. `state_list_returns_json`

## Drift Governance

`project_map.md` stale after >7 days without modification. Divergence between documented structure and actual filesystem is a structural integrity concern.

## Version

0.11.0

## Repository

https://github.com/crombojambo/crabjar
