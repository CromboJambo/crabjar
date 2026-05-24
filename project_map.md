# project_map.md

> Generated: May 15 2026
> Source: Cargo.toml (root + all members), filesystem scan, README.md, AGENTS.md, agent_config.md
> Purpose: Structural alignment reference for agent navigation

---

## 1. Overview

CrabJar is a Rust 2024 workspace centered on the `crabjar` CLI. It includes state-docs management, workspace config loading, knowledge-store bridge, codeburn token tracking, orchestrator (Axum SSE server), guard (execution gate), telemetry (flight recorder), sandbox (agent isolation), safetensors (model weight storage), and tool registry.

---

## 2. Architecture

### 2.1 Workspace Layout

```text
crabjar/
├── Cargo.toml               # Workspace root — 13 members, shared deps
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
├── src/project_loader.rs    # config loading
├── src/state_docs.rs        # state-doc and overlay handling
├── src/crabjar-config/      # workspace config crate
├── src/knowledge_store/     # knowledge-store command bridge
├── src/skill-script-runner/ # skill script discovery and execution
├── src/skill-reference-store/ # skill reference indexing and staleness
│
│  Supporting crates
├── memory/                  # agent-context crate, SQLite-backed storage
├── orchestrator/            # Axum SSE server (ACP-compliant orchestrator)
├── guard/                   # Trust layers, annealing, execution gate
├── telemetry/               # Flight recorder, command executor
├── sandbox/                 # Agent isolation (Unix user, systemd-nspawn / dinit-container, cgroup)
├── safetensors/             # Model weight storage (SQLite, checksum verification)
├── tool_registry/           # MCP tool registry (rig/aur patterns)
├── src/codeburn-provider/   # ProviderRegistry (Claude/Cursor/OpenCode etc.)
├── src/codeburn-classifier/ # TaskClassifier
├── src/codeburn-pricing/    # PricingEngine with LiteLLM fetch
├── src/codeburn-config/     # CodeBurnConfig (TOML parsing)
├── src/codeburn/            # codeburn CLI binary
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
| `zed-acp-server` | stdio JSON-RPC server (ACP protocol execution) | Bridge | Active |
| `zed-acp-bridge` | Wasm extension (tool call mapping + gate enforcement) | Bridge | Active |

### 2.3 Workspace Members

Declared in Cargo.toml `[workspace.members]`:
- `src/crabjar-config`
- `memory`
- `orchestrator`
- `src/codeburn-provider`
- `src/skill-script-runner`
- `src/skill-reference-store`
- `guard`
- `telemetry`
- `sandbox`
- `safetensors`
- `tool_registry`

### 2.4 Shared Dependencies

Declared in Cargo.toml `[workspace.dependencies]`:
- tokio (1.35, full)
- tokio-stream (0.1, time)
- serde (1.0, derive)
- serde_json (1.0)
- toml (0.8)
- thiserror (2.0)
- anyhow (1.0)
- rusqlite (0.32, bundled)
- clap (4.5, derive, env)
- clap_mangen (0.2)
- crossterm (0.28)
- tempfile (3.14)
- uuid (1.10, v4)
- chrono (0.4, serde)
- reqwest (0.12, json)
- axum (0.7)
- tower-http (0.5, cors)
- futures-util (0.3)
- tracing (0.1)
- tracing-subscriber (0.3, env-filter)
- cargo-declared (0.1.3)
- ignore (0.4)
- path-absolutize (3.1)
- sha2 (0.10)
- hex (0.4)

### 2.5 Release Profile

opt-level = 3, lto = true

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
- src/state-docs/ (state-docs source)
- src/dotfile_manager.rs (dotfile management)
- *.manifest.json (file manifests)
- human_reference.md (human reference documentation)
- environment_manifest.json (environment manifest)

### 8.2 Active Rust Surface

crabjar (binary) + crabjar-config (library) + agent-context (library) + orchestrator + guard + telemetry + sandbox + safetensors + tool_registry + codeburn-provider + codeburn-classifier + codeburn-pricing + codeburn-config + codeburn + skill-script-runner + skill-reference-store

### 8.3 Build Commands

- just check: cargo check --workspace
- just build: cargo build -p crabjar
- just test: cargo test --workspace
- just clean: remove build artifacts

---

## 9. Drift Report

### Last Audit

2026-05-15 — workspace consolidation updated. `project_map.md` regenerated with current structure. `src/models/` confirmed empty. `clippy -- -D warnings` passes across all 13 crates. `skill-script-runner` and `skill-reference-store` confirmed as workspace members with source files. `state-docs/checkpoint file` noted.

### Current Audit (2026-05-21)

`project_map.md` generated May 15. Today is May 21 — 6 days approaching >7 day stale threshold. Divergence detected between documented structure and actual filesystem.

### Divergence Items

| Type | project_map.md | Actual |
|---|---|---|
| **Path mismatch** | `src/codeburn-provider/` | `src/codeburn-provider/src/` (nested src/) |
| **Path mismatch** | `src/codeburn-classifier/` | `src/codeburn-classifier/src/` (nested src/) |
| **Path mismatch** | `src/codeburn-pricing/` | `src/codeburn-pricing/src/` (nested src/) |
| **Path mismatch** | `src/codeburn-config/` | `src/codeburn-config/src/` (nested src/) |
| **Path mismatch** | `src/codeburn/` | `src/codeburn/src/` (nested src/) |
| **Path mismatch** | `src/skill-script-runner/` | `src/skill-script-runner/src/` (nested src/) |
| **Path mismatch** | `src/skill-reference-store/` | `src/skill-reference-store/src/` (nested src/) |
| **Missing** | `src/models/` | not found |
| **Missing** | `src/skill-script-runner/src/lib.rs` | not found |
| **Naming mismatch** | `src/state-docs/` | `src/state_docs/` (underscore) |
| **Unexpected** | `src/codeburn/tests/cli.rs` | present (not in map as `tests/cli.rs`) |
| **Unexpected** | `src/codeburn/build.rs` | present (not in map) |
| **Removed** | `orchestrator/src/concierge.rs` | removed — duplicate of guard's gate concierge |

### Structural Correction Required

All `src/`-nested crates must be documented with `src/<crate>/src/` pattern, not `src/<crate>/` flat pattern. The map declares 13 workspace members with flat paths but actual structure uses nested `src/` inside each crate under `src/`.

### Known Items

- `state-docs/` checkpoint file — user checkpoint artifact awaiting resolution
- Single Git repo — all nested `.git/` removed
- Single `Cargo.lock` at workspace root
- `reference_materials/` — excluded from Git (cloned reference repos, not authored code)
- `pipelines/` — empty, reserved for future pipeline definitions
- `ui-state-copy/` — UI state copy artifacts
- `orchestrator/src/concierge.rs` — removed; guard's GateConcierge is the sole gate enforcement layer

### Provenance Entries

| UUID | Item | Set At | Reason | Source |
|---|---|---|---|---|
| `prov-map-drift-2026-05-15` | project_map.md regenerated with current structure | 2026-05-15 | Phase 1 structural alignment | crabjar/project_map.md |
| `prov-clippy-fix-2026-05-15` | clippy fixes across sandbox, safetensors, tool_registry, telemetry, guard | 2026-05-15 | Phase 1 lint enforcement | crabjar |
| `prov-map-drift-2026-05-21` | project_map.md stale — 6 days; structural divergence detected | 2026-05-21 | Phase 1 structural alignment refresh required | crabjar/project_map.md |
| `prov-concierge-consolidate` | orchestrator/src/concierge.rs removed; guard's GateConcierge is sole gate layer | 2026-05-21 | pipeline collapse prevention | crabjar |
| `prov-reversibility-bounded` | guard/src/reversibility.rs: ReversibilityScore → PerturbationSet | 2026-05-21 | bounded perturbations over single-point worst-case | crabjar |
| `prov-querier-drift` | memory/src/state_docs/querier.rs: drift_status() added | 2026-05-21 | coasting/resisting checksum comparison | crabjar |

---

*End of review.*
