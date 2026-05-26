# project_map.md

> Generated: May 26 2026
> Source: Cargo.toml (root + all members), filesystem scan, README.md, AGENTS.md, agent_config.md
> Purpose: Structural alignment reference for agent navigation

---

## 1. Overview

CrabJar is a Rust 2024 workspace centered on the `crabjar` CLI. It includes state-docs management, workspace config loading, knowledge-store bridge, codeburn token tracking, orchestrator (Axum SSE server), guard (execution gate), telemetry (flight recorder), sandbox (agent isolation), safetensors (model weight storage), tool registry, LLM inference crates, and Zed ACP bridge.

---

## 2. Architecture

### 2.1 Workspace Layout

```text
crabjar/
├── Cargo.toml               # Workspace root — 20 members, shared deps
├── Cargo.lock               # Locked dependency graph
├── build.rs                 # Root build script
├── AGENTS.md               # Repository guidelines
├── README.md               # Project overview
├── agent_config.md         # Agent configuration
├── Justfile               # Task runner shortcuts
├── Containerfile, Dockerfile
├── index.md               # Root index
├── REPRO.md               # Reproduction guide
│
│  Core crabjar crates
├── src/main.rs              # CLI entry point
├── src/lib.rs               # shared library surface
├── src/project_loader.rs    # config loading
├── src/dotfile_manager.rs   # dotfile management
├── src/state_docs.rs        # state-docs source (top-level)
├── src/state_docs/          # state-docs source (module)
│   └── commands.rs
├── src/knowledge_store/     # knowledge-store commands
│   ├── mod.rs
│   └── commands.rs
├── src/crabjar-config/      # workspace config crate
├── src/models/              # empty directory
│
│  Codeburn crates (nested src/)
├── src/codeburn-provider/   # ProviderRegistry (Claude/Cursor/OpenCode etc.)
│   └── src/lib.rs
├── src/codeburn-config/     # CodeBurnConfig (TOML parsing)
│   └── src/lib.rs
├── src/codeburn-classifier/ # TaskClassifier
│   └── src/lib.rs
├── src/codeburn-pricing/    # PricingEngine with LiteLLM fetch
│   └── src/lib.rs
├── src/codeburn/            # codeburn CLI binary
│   ├── Cargo.toml
│   ├── build.rs
│   ├── src/lib.rs
│   ├── src/main.rs
│   ├── src/tui.rs
│   └── tests/cli.rs
│
│  LLM inference crates
├── src/llm-plug-in/         # LLM plugin protocol
│   └── src/lib.rs
├── src/llm-runner/          # LLM runner (Candle/Burn backends)
│   └── src/lib.rs
│
│  Skill crates (nested src/)
├── src/skill-script-runner/ # skill script discovery and execution
│   └── src/lib.rs
├── src/skill-reference-store/ # skill reference indexing and staleness
│   └── src/lib.rs
│
│  Supporting crates
├── memory/                  # agent-context crate, SQLite-backed storage
│   └── src/lib.rs
├── orchestrator/            # Axum SSE server (ACP-compliant orchestrator)
│   └── src/main.rs
├── guard/                   # Trust layers, annealing, execution gate
│   └── src/lib.rs
├── telemetry/               # Flight recorder, command executor
│   └── src/lib.rs
├── sandbox/                 # Agent isolation (Unix user, systemd-nspawn / dinit-container, cgroup)
│   └── src/lib.rs
├── safetensors/             # Model weight storage (SQLite, checksum verification)
│   └── src/lib.rs
├── tool_registry/           # MCP tool registry (rig/aur patterns)
│   └── src/lib.rs
│
│  Zed ACP bridge
├── zed-acp-bridge/          # Wasm extension (tool call mapping + gate enforcement)
│   └── pkg/
├── zed-acp-server/          # stdio JSON-RPC server (ACP protocol execution)
│   └── src/lib.rs
│
│  Documentation
├── state-docs/              # Durable Markdown state documentation
│   ├── zeroclaw/
│   ├── GitNexus/
│   ├── oxc/
│   ├── vllm.rs/
│   ├── pi-subagents/
│   ├── rakers/
│   ├── rusty-buns/
│   └── Crane/
│
│  Non-crate artifacts
├── tests/cli.rs             # CLI integration tests
├── tests/cli/               # CLI test fixtures
├── testing/configs/         # test configs
├── ui-state-copy/           # UI state copy
├── git/                     # git helper scripts
├── gitignore/               # gitignore management
├── workspace/               # workspace config
├── bin/                     # compiled binaries
├── assets/                  # assets
├── reference_materials/     # excluded from Git
└── browser-tools-mcp/       # external tool (has .git/)
```

### 2.2 Core Components

| Component | Role | Layer | Status |
| :--- | :--- | :--- | :--- |
| `crabjar` | CLI for state-docs management | Observer (Active) | Core |
| `crabjar-config` | TOML config crate | Config | Active |
| `memory` | Agent-context SQLite storage | Storage | Active |
| `orchestrator` | Axum SSE server (ACP orchestrator) | Orchestrator | Active |
| `guard` | Trust layers, annealing, execution gate | Authorization | Active |
| `telemetry` | Flight recorder, command executor | Telemetry | Active |
| `sandbox` | Agent isolation tooling (Unix user, systemd-nspawn / dinit-container, cgroup) | Isolation | Active |
| `safetensors` | Model weight storage | Storage | Active |
| `tool_registry` | MCP tool registry | Registry | Active |
| `codeburn-provider` | ProviderRegistry (Claude/Cursor/OpenCode etc.) | Provider | Active |
| `codeburn-classifier` | TaskClassifier | Classification | Active |
| `codeburn-pricing` | PricingEngine with LiteLLM fetch | Pricing | Active |
| `codeburn-config` | CodeBurnConfig | Config | Active |
| `codeburn` | codeburn CLI binary | CLI | Active |
| `skill-script-runner` | Skill script discovery and execution | Skill | Active |
| `skill-reference-store` | Skill reference indexing and staleness | Reference | Active |
| `llm-plug-in` | LLM plugin protocol | Plugin | Active |
| `llm-runner` | LLM runner (Candle/Burn backends) | Inference | Active |
| `zed-acp-bridge` | Wasm extension (tool call mapping + gate enforcement) | Bridge | Active |
| `zed-acp-server` | stdio JSON-RPC server (ACP protocol execution) | Bridge | Active |

### 2.3 Workspace Members

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

### 2.4 Shared Dependencies

Declared in Cargo.toml `[workspace.dependencies]`:
- tokio (1.51, full)
- tokio-stream (0.1, time)
- serde (1.0, derive)
- serde_json (1.0)
- toml (0.8)
- thiserror (2.0)
- anyhow (1.0)
- rusqlite (0.37, bundled)
- clap (4.5, derive, env)
- clap_mangen (0.2)
- crossterm (0.28)
- ratatui (0.30.0)
- tempfile (3.24.0)
- uuid (1.23.0, v4)
- chrono (0.4.38, serde, now)
- reqwest (0.13.2, json, stream)
- axum (0.8, http1, tokio)
- tower-http (0.5, cors)
- futures-util (0.3)
- tracing (0.1)
- tracing-subscriber (0.3, env-filter)
- cargo-declared (0.1.3)
- ignore (0.4)
- path-absolutize (3.1)
- sha2 (0.10)
- hex (0.4)
- zed_extension_api (0.2)
- candle-core (0.10.2)
- candle-nn (0.10.2)
- candle-transformers (0.10.2)
- candle-datasets (0.10.2)
- burn (0.21.0, default-features=false)
- burn-core (0.21.0)
- burn-nn (0.21.0)
- burn-autodiff (0.21.0)
- burn-train (0.21.0, default-features=false)
- burn-cpu (0.21.0)
- rmcp (1.7.0, server, macros)
- rmcp-macros (1.7.0)
- schemars (1.0, chrono04)
- pastey (0.2.0)
- tiktoken-rs (0.11.0)
- safetensors (0.7.0, default-features=false)
- cudarc (0.19.4, cuda-12050)
- gemm (0.19.0)
- half (2.7.1)
- float8 (0.7.0)
- intel-mkl-src (0.8.1)
- tokenizers (0.22.0, default-features=false)
- yoke (0.8.1, derive)
- zip (8.6.0, default-features=false)
- hf-hub (0.4.1)
- parquet (58)
- image (0.25.9, jpeg, png)
- criterion (0.8, default-features=false)
- rand (0.10.1, std_rng)
- rand_distr (0.6.0, default-features=false)
- dirs (6.0.0)
- tracing-appender (0.2.3)
- nvml-wrapper (0.12.0)
- sysinfo (0.38.0)
- async-channel (2.5)

### 2.5 Release Profile

inherits = "release", debug = 2

### 2.6 Dev Profile

debug = true

---

## 3. Build & Test

| Command | Purpose |
|---|---|
| `just check` | cargo check --workspace |
| `just build` | cargo build -p crabjar |
| `just run state list` | runs binary with replaceable arguments |
| `just test` | cargo test --workspace |
| `just clean` | removes build artifacts |
| `cargo clippy --workspace -- -D warnings` | lint; warnings treated as errors |
| `cargo fmt --all` | auto-format every crate |
| `cargo fmt --all -- --check` | CI formatting gate |

---

## 4. Code Quality & Style

- Formatter: rustfmt with default settings
- Linter: Clippy at --deny warnings
- Naming: snake_case for functions/variables/modules, PascalCase for types/traits, SCREAMING_SNAKE_CASE for constants
- Error handling: thiserror for library crates, anyhow for binary/CLI crates
- No unwrap/expect for recoverable errors in library code
- Dependencies: add to workspace root first, then reference with { workspace = true }
- CLI commands emit structured JSON to stdout; no plain-text success paths

---

## 5. Testing Guidelines

- Framework: Rust built-in #[test] and #[cfg(test)]
- SQLite tests: in-memory database (:memory:) or tempfile managed path
- Filesystem tests: use tempfile
- Test naming: descriptive snake_case stating behaviour under test
- Coverage: aim to cover full public API surface of each crate
- CLI integration tests in tests/cli.rs using std::process::Command

---

## 6. Integration Roadmap

### Phase 1 — Standardization

clippy passes across all members; unified error-handling patterns; CI passes across all members

### Phase 2 — Feature Integration

wire orchestrator → guard → telemetry into a `crabjar exec` CLI command; implement actual safetensors weight parsing; implement tool discovery in tool_registry; implement optimize_engine in codeburn

### Phase 3 — Consolidation

move completed experiments to archive/ directory; produce clean pre-optimized workspace

---

## 7. Architectural Constraints

### Detection ≠ Authorization

Crabjar is a pure observer. It knows what happened but cannot change what happens. This is enforced by design.

### Executor Layer Status

Runtime execution is executor-capable. Execution is opt-in via `.crabjar_config.toml` `tool_execution_enabled`. The single pipeline is: `request → guard → concierge → telemetry → outcome → trust update`. Pending actions persist to GuardDb. All actions require real provenance lookup.

### Doubt Output Requirement

Every derived output must include a `doubt` block with:
- `assumptions` — what it assumed to produce this output
- `blind_spots` — what it couldn't see
- `last_validation` — when this was last checked against raw data
- `stale_after` — when this output should be considered stale

---

## 8. Crabjar Context

### 8.1 Structure

crabjar contains:
- agent_config.md
- AGENTS.md
- Cargo.toml (workspace root + crabjar binary manifest)
- Justfile
- Containerfile, Dockerfile
- orchestrator (Axum SSE server)
- guard (SecurityGuard)
- codeburn-config (config struct, TOML parsing)
- codeburn-provider (ProviderRegistry)
- codeburn-classifier (TaskClassifier)
- codeburn-pricing (PricingEngine)
- codeburn (CLI binary)
- memory/files (agent-context crate)
- tests/cli.rs
- ui-state-copy
- reference_materials (excluded from Git)
- bin/ (compiled binaries)
- git/ (git helper scripts)
- gitignore/ (gitignore management)
- workspace/ (workspace config)
- state-docs/ (local state-docs)
- src/models/ (empty directory)
- src/state_docs/ (state-docs source)
- src/dotfile_manager.rs (dotfile management)
- src/knowledge_store/ (knowledge store commands)
- src/llm-plug-in/ (LLM plugin protocol)
- src/llm-runner/ (LLM runner)
- *.manifest.json (file manifests)
- human_reference.md (human reference documentation)
- environment_manifest.json (environment manifest)
- index.md (root index)
- REPRO.md (reproduction guide)
- build.rs (root build script)

### 8.2 Active Rust Surface

crabjar (binary) + crabjar-config (library) + agent-context (library) + orchestrator + guard + telemetry + sandbox + safetensors + tool_registry + codeburn-provider + codeburn-classifier + codeburn-pricing + codeburn-config + codeburn + skill-script-runner + skill-reference-store + llm-plug-in + llm-runner + zed-acp-bridge + zed-acp-server

### 8.3 Build Commands

- just check: cargo check --workspace
- just build: cargo build -p crabjar
- just test: cargo test --workspace
- just clean: remove build artifacts

---

## 9. Drift Report

### Last Audit

2026-05-26 — crates.io prep. Version bumped to 0.11.0. `publish = true` on root package, `publish = false` on all 20 workspace members. `src/models/` removed (empty dir). `.manifest.json` files removed from root. gitignore updated for `*.manifest.json`. All paths verified against filesystem.

### Known Items

- `state-docs/` overlays in `state-docs/*/` subdirectories (zeroclaw, GitNexus, oxc, vllm.rs, pi-subagents, rakers, rusty-buns, Crane)
- Single Git repo — `browser-tools-mcp/` is an external submodule with its own `.git/`
- Single `Cargo.lock` at workspace root
- `reference_materials/` — excluded from Git (cloned reference repos, not authored code)
- `src/models/` — empty directory, no files
- `src/knowledge_store/` — contains `mod.rs` and `commands.rs` (not a workspace crate)
- `src/state_docs/` — contains `commands.rs` (not a workspace crate)
- `guard/src/concierge.rs` — present (guard's GateConcierge is sole gate enforcement layer)
- `guard/src/reversibility.rs` — ReversibilityScore → PerturbationSet
- `memory/src/state_docs/querier.rs` — drift_status() added
- `orchestrator/src/concierge.rs` — removed (not present)

### Provenance Entries

| UUID | Item | Set At | Reason | Source |
|---|---|---|---|---|
| `prov-map-drift-2026-05-15` | project_map.md regenerated with current structure | 2026-05-15 | Phase 1 structural alignment | crabjar/project_map.md |
| `prov-clippy-fix-2026-05-15` | clippy fixes across sandbox, safetensors, tool_registry, telemetry, guard | 2026-05-15 | Phase 1 lint enforcement | crabjar |
| `prov-map-drift-2026-05-24` | project_map.md regenerated — 20 workspace members, llm/zed crates added | 2026-05-24 | Structural alignment refresh | crabjar/project_map.md |
| `prov-concierge-consolidate` | orchestrator/src/concierge.rs removed; guard's GateConcierge is sole gate layer | 2026-05-21 | pipeline collapse prevention | crabjar |
| `prov-reversibility-bounded` | guard/src/reversibility.rs: ReversibilityScore → PerturbationSet | 2026-05-21 | bounded perturbations over single-point worst-case | crabjar |
| `prov-querier-drift` | memory/src/state_docs/querier.rs: drift_status() added | 2026-05-21 | coasting/resisting checksum comparison | crabjar |
| `prov-crates-io-0110` | version 0.10.2 → 0.11.0, publish config added to all crates, stale artifacts removed | 2026-05-26 | crates.io publication prep | crabjar |

---

*End of review.*
