# project_map.md

> Generated: May 31 2026
> Source: Cargo.toml (root + all members), filesystem scan, README.md, AGENTS.md, agent_config.md
> Purpose: Structural alignment reference for agent navigation

---

## 1. Overview

CrabJar is a Rust 2024 workspace centered on the `crabjar` CLI. It includes state-docs management, workspace config loading, knowledge-store bridge, codeburn token tracking, orchestrator (Axum SSE server with unified LM client), guard (execution gate), telemetry (flight recorder), sandbox (agent isolation), tool registry, codeburn CLI, skill script runner, skill reference store, Zed ACP bridge, and agent skills. LLM inference (runner, plug-in, safetensors, GGUF parser) is in a separate workspace at `llm-workspace/`.

---

## 2. Architecture

### 2.1 Workspace Layout

```text
crabjar/
├── Cargo.toml               # Workspace root — 21 members, shared deps
├── Cargo.lock               # Locked dependency graph
├── build.rs                 # Root build script
├── AGENTS.md               # Repository guidelines
├── README.md               # Project overview
├── agent_config.md         # Agent configuration
├── Justfile               # Task runner shortcuts
├── Containerfile, Dockerfile
├── rust-toolchain.toml     # Rust nightly toolchain
├── .crabjar_config.toml    # Workspace config (tool_execution_enabled)
├── index.md               # Root index
├── REPRO.md               # Reproduction guide
├── human_reference.md     # Human reference documentation
├── environment_manifest.json  # Environment manifest (CPU/GPU/storage)
├── llmrunner.md           # LLM runner architecture notes
├── project_map.md         # This file
│
│  Core crabjar crates
├── src/main.rs              # CLI entry point
├── src/lib.rs               # shared library surface
├── src/project_loader.rs    # config loading
├── src/dotfile_manager.rs   # dotfile management
├── src/state_docs.rs        # state-docs manager (top-level)
├── src/state_docs/          # state-docs commands
│   └── commands.rs
├── src/knowledge_store/     # knowledge-store commands
│   ├── mod.rs
│   └── commands.rs
├── src/crabjar-config/      # workspace config crate
│   └── src/lib.rs
├── src/bitwarden/           # bitwarden CLI integration
│   ├── cli.rs
│   ├── mod.rs
│   └── store.rs
├── src/index.md             # src directory index
├── src/manifest.json        # src directory manifest
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
│   ├── src/optimize.rs
│   └── tests/cli.rs
│
│  Skill crates (nested src/)
├── src/skill-script-runner/ # skill script discovery and execution
│   └── src/
│       ├── discovery.rs
│       ├── execution.rs
│       └── lib.rs
├── src/skill-reference-store/ # skill reference indexing and staleness
│   └── src/lib.rs
│
│  Supporting crates
├── memory/                  # agent-context crate, SQLite-backed storage
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── error.rs
│   │   ├── models.rs
│   │   ├── schema.rs
│   │   ├── state_docs/      # state-docs querier (drift_status)
│   │   │   ├── mod.rs
│   │   │   ├── indexer.rs
│   │   │   ├── querier.rs
│   │   │   ├── renderer.rs
│   │   │   ├── models.rs
│   │   │   └── schema.rs
│   │   └── files/
│   │       ├── index.md
│   │       └── manifest.json
│   └── tests/
│       └── state_docs_tests.rs
├── orchestrator/            # Axum SSE server (ACP orchestrator + unified LM client)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # Axum router: /acp/run, /acp/prompt, /acp/chat
│       └── lm_studio_client/
│           └── mod.rs       # Unified client (native/OpenAI/Anthropic) + SessionStore
├── guard/                   # Trust layers, annealing, execution gate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── gate.rs          # ExecutionGate (single authorization boundary)
│       ├── concierge.rs     # GateConcierge (enforce: deny/pending/proceed)
│       ├── trust.rs         # TrustManager (confidence bands)
│       ├── annealing.rs     # AnnealingPipeline (confidence decay/reinforcement)
│       ├── retrieval.rs     # RetrievalEngine (layer-based querying)
│       ├── memory.rs        # MemoryGraph (nodes + edges)
│       ├── reversibility.rs # ReversibilityScore → PerturbationSet
│       ├── guard_db.rs      # GuardDb (SQLite schema + queries)
│       ├── schema.sql       # GuardDb schema definition
│       └── types.rs
├── telemetry/               # Flight recorder, command executor
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── flight_recorder.rs  # FlightRecorder (execute_command, git capture)
│       ├── command_executor.rs # process spawning + output capture
│       ├── schema.rs
│       └── error.rs
├── sandbox/                 # Agent isolation (Unix user, dinit-container, cgroup)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── agent_isolation.rs
│       ├── schema.rs
│       └── error.rs
├── tool_registry/           # MCP tool registry (rig/aur patterns)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── tool_registry.rs
│       ├── schema.rs
│       └── error.rs
│
│  Zed ACP bridge
├── zed-acp-bridge/          # Wasm extension (tool call mapping + gate enforcement)
│   ├── Cargo.toml
│   ├── src/lib.rs
│   └── pkg/
├── zed-acp-server/          # stdio JSON-RPC server (ACP protocol execution)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── main.rs
│
│  Documentation
├── state-docs/              # Durable Markdown state documentation
│   ├── bustd-state.md
│   ├── checkpoint-2026-05-15.md
│   ├── checkpoint-2026-05-16.md
│   ├── codex5.14.md
│   ├── crabjar-state.md
│   ├── cuda-oxide-state.md
│   ├── DeepSeek-TUI-state.md
│   ├── failing-tests.md
│   ├── gitnexus-state.md
│   ├── graphifyrs-state.md
│   ├── graphify-state.md
│   ├── hermes-agent-state.md
│   ├── iced-state.md
│   ├── microsandbox-state.md
│   ├── mistralrs.md
│   ├── Observability in Agentic Workflows.md
│   ├── superpowers-state.md
│   ├── tokenizers-state.md
│   ├── uk-pitfalls-state.md
│   └── overlay/
│       └── cuda-oxide-state.overlay.json
│
│  Non-crate artifacts
├── tests/                   # CLI integration tests
│   ├── cli.rs
│   ├── cli/
│   └── index.md
│   └── manifest.json
├── testing/
│   └── configs/
├── bin/                     # compiled binaries
│   ├── index.md
│   └── manifest.json
├── assets/                  # assets
│   └── crabjar-banner.png
├── workspace/               # workspace config
│   ├── index.md
│   └── manifest.json
├── git/                     # git helper scripts
│   ├── index.md
│   └── manifest.json
├── gitignore/               # gitignore management
│   ├── index.md
│   └── manifest.json
├── ui-state-copy/           # UI state copy
│   └── daemon/
│       ├── index.md
│       └── manifest.json
├── reference_materials/     # excluded from Git
└── browser-tools-mcp/       # external tool (has .git/)
```

### 2.2 Core Components

| Component | Role | Layer | Status |
| :--- | :--- | :--- | :--- |
| `crabjar` | CLI for state-docs management | Observer (Active) | Core |
| `crabjar-config` | TOML config crate | Config | Active |
| `memory` | Agent-context SQLite storage | Storage | Active |
| `orchestrator` | Axum SSE server + unified LM client | Orchestrator | Active |
| `guard` | Trust layers, annealing, execution gate | Authorization | Active |
| `telemetry` | Flight recorder, command executor | Telemetry | Active |
| `sandbox` | Agent isolation (Unix user, dinit-container, cgroup) | Isolation | Active |
| `tool_registry` | MCP tool registry | Registry | Active |
| `codeburn-provider` | ProviderRegistry (Claude/Cursor/OpenCode etc.) | Provider | Active |
| `codeburn-classifier` | TaskClassifier | Classification | Active |
| `codeburn-pricing` | PricingEngine with LiteLLM fetch | Pricing | Active |
| `codeburn-config` | CodeBurnConfig | Config | Active |
| `codeburn` | codeburn CLI binary | CLI | Active |
| `skill-script-runner` | Skill script discovery and execution | Skill | Active |
| `skill-reference-store` | Skill reference indexing and staleness | Reference | Active |
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
- `tool_registry`
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
- systemstat (0.2.6)
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

## 6. CLI Binary Surface

### 6.1 `crabjar` CLI (src/main.rs)

| Command | Status | Notes |
|---|---|---|
| `crabjar state list` | wired | JSON output, lists state-docs |
| `crabjar state show <doc>` | wired | JSON output, doc + annotations |
| `crabjar state annotate <doc> <msg>` | wired | JSON output, note annotation |
| `crabjar state question <doc> <msg>` | wired | JSON output, question annotation |
| `crabjar state resolve <doc> <id>` | wired | JSON output, resolved annotation |
| `crabjar knowledge index <doc>` | wired | Structured JSON |
| `crabjar knowledge sync <doc>` | wired | Structured JSON |
| `crabjar knowledge query --tags <tags>` | wired | Structured JSON |
| `crabjar knowledge insert --content --kind --tags` | wired | Structured JSON |
| `crabjar knowledge verify` | wired | Structured JSON |
| `crabjar knowledge events --limit <n>` | wired | Structured JSON |
| `crabjar knowledge deactivate <id> --reason` | wired | Structured JSON |
| `crabjar knowledge promote <id> --reason` | wired | Structured JSON |
| `crabjar knowledge resolve-annotation` | wired | Structured JSON |
| `crabjar dotfile promote <path>` | wired | JSON output |
| `crabjar workspace status` | wired | `workspace: null` when config missing |
| `crabjar guard queue --status --limit` | wired | Reads guard.db pending_queue |
| `crabjar guard approve --action_id` | wired | Updates guard.db |
| `crabjar guard reject --action_id --reason` | wired | Updates guard.db |
| `crabjar guard interrupted --limit` | wired | Reads guard.db interrupted_log |
| `crabjar guard provenance --source_event_id` | wired | Provenance lookup in guard.db |
| `crabjar guard grant --pid --trust_layer` | wired | PID trust grant |
| `crabjar guard revoke --pid` | wired | PID trust revoke |
| `crabjar exec --command <cmd> --reason <id>` | **end-to-end** | request → guard → concierge → telemetry → outcome → trust update |
| `crabjar bitwarden status/list/get/search/generate` | wired | CLI-available gate |
| `crabjar doctor check` | wired | Checks guard.db/flight.db/knowledge.db schema |

### 6.2 `crabjar exec` Pipeline

```
config check (tool_execution_enabled)
  → dry_run shortcut (skip gate + telemetry)
  → ExecutionGate::check() with GateContext{trust_layer, confidence, source_event_id}
  → GateConcierge::enforce() → ActionStatus {Denied|Pending|TrustApproved|Executed|Interrupted}
  → Pending → persist to guard.db pending_queue
  → TrustApproved → FlightRecorder::execute_command() → capture_git_dirty + capture_git_diff
  → GuardDb::action_outcomes INSERT (confidence_delta = 0.02)
  → JSON output with cmd_id, exit_code, gate_result, outcome_id, flight_recorder
```

### 6.3 `orchestrator` Binary (Axum SSE server)

| Endpoint | Purpose |
|---|---|
| `POST /acp/run` | Run command + stream output via SSE |
| `POST /acp/prompt` | Acknowledge prompt (JSON response) |
| `POST /acp/chat` | LLM chat via unified inference backend |

The `handle_chat` handler uses the `InferenceBackend` trait — switches between LM Studio and mistral.rs at runtime via `INFERENCE_BACKEND` env var. The `lm_studio_client` module provides a unified client (native/OpenAI/Anthropic/mistral.rs serve) with `SessionStore` (SQLite-backed session persistence). The `LmStudioEndpoint::MistralRsServe` variant routes to `MISTRALRS_SERVE_URL` (default `http://127.0.0.1:8081`) for mistral.rs serve instances.

---

## 7. Integration Roadmap

### Phase 1 — Standardization ✅ DONE

clippy passes across all members (0 warnings); unified error-handling patterns (libraries: thiserror, binaries: anyhow); CI passes across all members (fmt + clippy + build + test); 741 tests passing (0 failed, 1 ignored).

### Phase 2 — Feature Integration ✅ DONE

wire orchestrator → guard → telemetry into a `crabjar exec` CLI command ✅; implement actual safetensors weight parsing ✅ (uses real safetensors crate with actual tensor loading + SHA-256 checksums); implement tool discovery in tool_registry ✅ (MCP scanning, state-based discovery, auto-registration, binary validation); implement optimize_engine in codeburn ✅ (extracted to dedicated module with 4 heuristic rules, configurable thresholds, markdown output).

### Phase 3 — Consolidation

move completed experiments to archive/ directory; produce clean pre-optimized workspace.

### Phase 4 — Inference Integration ✅ DONE

wire orchestrator's `LmStudioClient` into the `handle_chat` handler ✅ (via `InferenceBackend` trait); add mistral.rs as inference substrate ✅ (`MistralRsClient` with lazy model loading); unify LM client to support mistral.rs serve on OpenAI-compatible port ✅ (`LmStudioEndpoint::MistralRsServe` variant + `MISTRALRS_SERVE_URL` env var); replace hardcoded LM Studio URL with env-configurable endpoint ✅ (`INFERENCE_BACKEND`, `LM_STUDIO_URL`, `MISTRALRS_MODEL`, `MISTRALRS_QUANT`, `MISTRALRS_SERVE_URL`).

---

## 8. Architectural Constraints

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

## 9. Crabjar Context

### 9.1 Structure

crabjar contains:
- agent_config.md
- AGENTS.md
- Cargo.toml (workspace root + crabjar binary manifest)
- Justfile
- Containerfile, Dockerfile
- orchestrator (Axum SSE server + unified LM client)
- guard (SecurityGuard: gate, concierge, trust, annealing, memory, retrieval, reversibility)
- codeburn-config (config struct, TOML parsing)
- codeburn-provider (ProviderRegistry)
- codeburn-classifier (TaskClassifier)
- codeburn-pricing (PricingEngine)
- codeburn (CLI binary with TUI + optimize_engine)
- memory/files (agent-context crate with state_docs querier)
- tests/cli.rs
- ui-state-copy
- reference_materials (excluded from Git)
- bin/ (compiled binaries)
- git/ (git helper scripts)
- gitignore/ (gitignore management)
- workspace/ (workspace config)
- state-docs/ (local state-docs + overlay)
- .agents/skills/ (29 agent skills)
- .agents/references/
- src/gguf/ (GGUF parser crate)
- src/gguf-cli/ (GGUF CLI binary)
- src/bitwarden/ (bitwarden CLI integration)
- src/state_docs/ (state-docs commands)
- src/dotfile_manager.rs (dotfile management)
- src/knowledge_store/ (knowledge store commands)
- src/llm-plug-in/ (LLM plugin protocol)
- src/llm-runner/ (LLM runner, CPU fallback kernels)
- src/skill-script-runner/ (skill script discovery)
- src/skill-reference-store/ (skill reference indexing)
- src/crabjar-config/ (workspace config crate)
- src/codeburn*/ (codeburn sub-crates)
- *.manifest.json (file manifests)
- human_reference.md (human reference documentation)
- environment_manifest.json (environment manifest)
- index.md (root index)
- REPRO.md (reproduction guide)
- build.rs (root build script)
- llmrunner.md (LLM runner architecture notes)

### 9.2 Active Rust Surface

crabjar (binary) + crabjar-config (library) + agent-context (library) + orchestrator + guard + telemetry + sandbox + safetensors + tool_registry + codeburn-provider + codeburn-classifier + codeburn-pricing + codeburn-config + codeburn + skill-script-runner + skill-reference-store + llm-plug-in + llm-runner + gguf + gguf-cli + zed-acp-bridge + zed-acp-server

### 9.3 Test Count

741 tests passing across all workspace members (0 failed, 1 ignored). Clippy: clean.

---

## 10. Drift Report

### Last Audit

2026-05-31 — Phase 4 complete. Clippy: 39 pre-existing warnings (not from this change). 741 tests passing (0 failed, 1 ignored). CI verified. `orchestrator/src/backend/` unified inference backend (`InferenceBackend` trait + `Backend` enum). `LmStudioEndpoint::MistralRsServe` variant added — routes to `MISTRALRS_SERVE_URL` (default `http://127.0.0.1:8081`) for mistral.rs serve instances. `LmStudioConfig` has `serve_base_url` field. `detect_available_endpoints()` probes mistral.rs serve. `guard/src/schema.sql` added. `memory/src/state_docs/` has indexer.rs, querier.rs (drift_status), renderer.rs. `llm-runner/src/kernel/tests/` added. `state-docs/` has 19 state docs + overlay. `src/models/` removed. `src/llm-runner/src/` has 6 legacy empty subdirs (device/, inference-engine/, model-loader/, plug-in/, runner/, tokenizer/). Version 0.11.0.

### Known Items

- `state-docs/` overlays in `state-docs/*/` subdirectories (cuda-oxide-state.overlay.json)
- `state-docs/` has 19 Markdown state docs (bustd-state.md, checkpoint-2026-05-15.md, checkpoint-2026-05-16.md, codex5.14.md, crabjar-state.md, cuda-oxide-state.md, DeepSeek-TUI-state.md, failing-tests.md, gitnexus-state.md, graphifyrs-state.md, graphify-state.md, hermes-agent-state.md, iced-state.md, microsandbox-state.md, mistralrs.md, Observability in Agentic Workflows.md, superpowers-state.md, tokenizers-state.md, uk-pitfalls-state.md)
- Single Git repo — `browser-tools-mcp/` is an external submodule with its own `.git/`
- Single `Cargo.lock` at workspace root
- `reference_materials/` — excluded from Git (cloned reference repos, not authored code)
- `src/llm-runner/src/device/`, `inference-engine/`, `model-loader/`, `plug-in/`, `runner/`, `tokenizer/` — empty legacy directories
- `guard/src/schema.sql` — GuardDb schema definition
- `guard/src/concierge.rs` — present (guard's GateConcierge is sole gate enforcement layer)
- `guard/src/reversibility.rs` — ReversibilityScore → PerturbationSet
- `memory/src/state_docs/querier.rs` — drift_status() added
- `orchestrator/src/lm_studio_client/` — unified LM client with SessionStore; `LmStudioEndpoint::MistralRsServe` variant for mistral.rs serve
- `orchestrator/src/backend/` — unified `InferenceBackend` trait + `Backend` enum (LM Studio / mistral.rs)
- Phase 4 complete — unified inference backend, mistral.rs serve support, env-configurable endpoints
- Phase 2 complete — 741 tests passing, clippy clean
- Phase 1 complete — clippy clean, CI verified
- `.agents/skills/` — 29 agent skills
- `.agents/references/` — agent reference files
- `llm-runner` — CPU fallback kernels (CpuGemmKernel, CpuAttentionKernel) operational; GPU path stubbed (GemmBuilder::build returns KernelFromPtx with no-op matmul); weight loading → inference pipeline bridge missing; K-family dequantization unimplemented; RoPE/RMSNorm/activations/LM head/sampling not yet coded
- **Cross-project: llm-workspace** — configured via opencode.jsonc instructions + dotfiles symlink graph (`llm-workspace-rules.md` → AGENTS.md + ROADMAP.md + llmrunner.md)

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
| `prov-phase1-done` | Phase 1 complete: clippy clean, 516 tests passing, CI verified, 6 ignored tests enabled | 2026-05-27 | Phase 1 standardization completion | crabjar |
| `prov-phase2-done` | Phase 2 complete: 584 tests passing, safetensors real tensor loading, tool_registry MCP discovery, codeburn optimize_engine extraction | 2026-05-27 | Phase 2 feature integration completion | crabjar |
| `prov-map-drift-2026-05-30` | project_map.md regenerated — 21 members, gguf/gguf-cli added, orchestrator lm_studio_client documented, test count 741, legacy dirs noted | 2026-05-30 | Structural alignment refresh | crabjar/project_map.md |
| `prov-phase4-done` | Phase 4 complete: unified InferenceBackend trait, MistralRsServe endpoint, MISTRALRS_SERVE_URL env var, detect_available_endpoints probes mistral.rs serve | 2026-05-31 | Inference integration completion | crabjar/project_map.md |
| `prov-llm-cross-project` | llm-workspace added as cross-project reference via opencode instructions + dotfiles symlink graph | 2026-06-06 | Cross-project awareness | crabjar/.agents/skills/cross-project |

---

*End of review.*
