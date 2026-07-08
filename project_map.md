# project_map.md

> Generated: July 8 2026
> Source: Cargo.toml (root + all members), filesystem scan, README.md, AGENTS.md, agent_config.md
> Purpose: Structural alignment reference for agent navigation

---

## 1. Overview

CrabJar is a Rust 2024 workspace centered on the `crabjar` CLI. It includes state-docs management, workspace config loading, knowledge-store bridge, orchestrator (Axum SSE server with unified LM client), guard (execution gate with trust layers, annealing, scope isolation, fingerprint approvals), telemetry (flight recorder), sandbox (agent isolation), tool registry, skill script runner, skill reference store, file search engine (BM25/Tantivy 0.22), plugin system (WASM runtime), Zed ACP bridge, and agent skills. LLM inference (runner, plug-in, safetensors, GGUF parser) is in a separate workspace at `llm-workspace/`.

---

## 2. Architecture

### 2.1 Workspace Layout

```text
crabjar/
├── Cargo.toml               # Workspace root — 24 members, shared deps
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

├── project_map.md         # This file
├── ROADMAP.md             # Development roadmap
│
│  Core crabjar crates
├── src/main.rs              # CLI entry point
├── src/lib.rs               # shared library surface
├── src/project_loader.rs    # config loading
├── src/dotfile_manager.rs   # dotfile management
├── src/doctor.rs            # doctor check command
├── src/bitwarden/           # bitwarden CLI integration
│   ├── cli.rs
│   ├── commands.rs
│   ├── mod.rs
│   └── store.rs
├── src/knowledge_store/     # knowledge-store commands
│   ├── mod.rs
│   └── commands.rs
├── src/crabjar_config/      # workspace config crate
│   └── mod.rs
├── src/vm_bridge/           # per-VM websocket relay (screen/terminal)
│   ├── lib.rs
│   ├── relay.rs
│   ├── screen.rs
│   └── terminal.rs
├── src/host-binary/         # host binary crate
│   ├── AGENTS.md
│   ├── Cargo.toml
│   ├── cli.rs
│   ├── dashboard.rs
│   └── main.rs
├── src/skill-script-runner/ # skill script discovery and execution
│   └── src/
├── src/skill-reference-store/ # skill reference indexing and staleness
│   └── src/
├── src/index.md             # src directory index
├── src/manifest.json        # src directory manifest
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
│   │   └── schema.rs
│   └── tests/
│       └── state_docs_tests.rs
├── memory/files/            # memory crate helper files
│   ├── index.md
│   └── manifest.json
├── guard/                   # Trust layers, annealing, execution gate, scope isolation
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── gate.rs          # ExecutionGate (single authorization boundary)
│       ├── gate_tests.rs    # ExecutionGate unit tests
│       ├── gate_context.rs  # GateContext struct
│       ├── gate_result.rs   # GateResult enum
│       ├── concierge.rs     # GateConcierge (enforce: deny/pending/proceed)
│       ├── concierge_types.rs # GateConcierge types
│       ├── trust.rs         # TrustManager (confidence bands)
│       ├── trust_types.rs   # TrustScore, TrustLayer, RetrievalBand types
│       ├── trust_resolution.rs # Requested-vs-effective trust resolution
│       ├── action.rs        # ActionStatus, OutcomeStatus, ActionRequest, ActionOutcome
│       ├── inference.rs     # ModelInferenceKind, ModelInferenceRequest, ModelInferenceOutcome
│       ├── guard_db.rs        # GuardDb (SQLite schema + queries)
│       ├── guard_db_impl.rs   # GuardDb impl (anneal + concierge + PID trust)
│       ├── guard_db_queries.rs # Action requests + trust resolution queries
│       ├── guard_db_types.rs   # TrustResolutionEntry type
│       ├── db_error.rs        # GuardDb error types
│       ├── schema.sql       # GuardDb schema definition
│       ├── scope.rs         # Scope isolation model (identity, project, tenant, thread)
│       ├── fingerprint.rs   # InvocationFingerprint + SHA-256
│       ├── fingerprint_types.rs # ApprovalLease, ApprovalScope types
│       ├── memory.rs        # MemoryGraph (nodes + edges)
│       ├── memory_types.rs  # NodeKind, MemoryNode, EdgeRelation, MemoryEdge
│       ├── command_risk.rs  # CommandRisk, HIGH/MEDIUM_RISK_COMMANDS
│       ├── risk_config.rs   # RiskConfig
│       ├── domain_allowlist.rs  # Domain allowlist for web fetch scope gating
│       ├── policy.rs        # StaticPolicyEngine (TOML-based declarative policies)
│       ├── policy_types.rs  # PolicyRule, PolicyCheck types
│       └── context_budget.rs # ContextBudget + MAX_TOKENS_PER_FRAGMENT
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
├── axum-mux/                # vm-bridge (per-VM websocket relay)
│   ├── Cargo.toml
│   ├── README.md
│   ├── ROADMAP.md
│   └── AGENTS.md
├── crabjar-architecture/    # Mechanical dependency boundary enforcement
│   ├── Cargo.toml
│   ├── AGENTS.md
│   └── src/
│       ├── lib.rs
│       ├── layer.rs         # 8-layer model (0-7)
│       └── boundary.rs      # boundary::check_workspace_boundaries()
├── crabjar-plugin/          # Plugin system (WASM runtime, lifecycle management)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── crates/terminal/         # Terminal multiplexer integration (wezterm/zellij + asciinema v2)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs           # TerminalSession, TerminalManager, Snapshot types
│       ├── backend.rs       # TerminalBackend trait + detection utilities
│       ├── wezterm.rs       # Wezterm mux backend (spawn/send-text/get-text)
│       ├── zellij.rs        # Zellij action protocol backend
│       └── recording.rs     # Asciinema v2 session recorder

│  File search engine
├── file_search/             # BM25-based file indexing and search (Tantivy 0.22)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs           # Public API surface
│       ├── indexer.rs       # BM25 document indexing with LowerCaser tokenizer
│       └── storage.rs       # SearchStorage (index CRUD, search, reload)

│  Host runtime
├── host/host-core/          # Event bus, plugin API, WorkItem model, config
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── adapter.rs       # ProductAdapter trait + AdapterRegistry
│       ├── config.rs
│       ├── event_bus.rs
│       ├── plugin.rs
│       └── work_item.rs
├── host/host-system/        # Notifications, clipboard, secrets, tray
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── clipboard.rs
│       ├── notifications.rs
│       ├── secrets.rs
│       └── tray.rs
├── host/host-observe/       # Metrics, tracing, health reporting
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── health.rs
│       ├── metrics.rs
│       └── tracing_setup.rs
├── host/host-agent/         # Agent loop (ReAct: observe→understand→plan→execute→verify→reflect)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── loop_engine.rs   # ReAct loop engine
│       ├── executor.rs
│       ├── planner.rs
│       ├── verifier.rs
│       ├── reflector.rs
│       ├── work_item_store.rs
│       ├── model_routing.rs  # ModelRouter with LoopPhase enum, phase-specific backends
│       ├── context_compression.rs # ContextCompressor with token budget enforcement
│       ├── decision_gate.rs  # DecisionGate (ToolCall/RespondDirectly/Defer)
│       └── inference/
│           ├── mod.rs
│           ├── backend.rs
│           └── http_backend.rs
├── host/host-webview/       # WebView session management, OAuth2, token cache
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── auth.rs
│       ├── controller.rs
│       ├── cookie_store.rs
│       ├── partition.rs
│       ├── session.rs
│       └── token_cache.rs
├── host/host-mqtt/          # MQTT client + Home Assistant discovery
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── client.rs
│       ├── config.rs
│       ├── discovery.rs
│       ├── handler.rs
│       └── media_bridge.rs
├── host/host-graph/         # Microsoft Graph API client
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── client.rs
│       ├── config.rs
│       └── types.rs
├── host/host-screen/        # Screen capture + display protocol integration
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── display.rs
│       └── terminal.rs
│
│  Host apps
├── apps/teams/              # Teams plugin (reference application)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   └── teams_plugin.rs
│   └── teams-for-linux/     # additional teams-for-linux subdirectory
│
│  Orchestrator (Axum SSE server)
├── orchestrator/            # Axum SSE server + unified LM client
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # Axum router: /acp/run, /acp/prompt, /acp/chat
│       ├── backend/
│       │   └── mod.rs       # InferenceBackend trait + BackendKind enum
│       └── lm_studio_client/
│           ├── mod.rs       # LM Studio client module root
│           ├── types.rs     # Unified message/request/response types, LmStudioEndpoint
│           ├── client.rs    # LmStudioClient::chat() + chat_with_system()
│           ├── session.rs   # SessionState + SessionStore (SQLite)
│           ├── error.rs     # LmStudioError + ToolCallInfo
│           ├── endpoints.rs # Endpoint converters (native, OpenAI, Anthropic)
│           ├── prompt_envelope.rs # PromptEnvelope + PromptValidator (instruction-hijack defense)
│           └── tests.rs     # Unit tests (40+)
│   └── prompts/
│       └── default_system.md # Default system prompt template
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
│
│  Non-crate artifacts
├── tests/                   # CLI integration tests
│   ├── cli.rs
│   ├── cli/
│   └── index.md
├── testing/
│   └── configs/
├── assets/                  # assets
│   └── crabjar-banner.png
├── workspace/               # workspace config
│   ├── index.md
│   └── manifest.json
├── ui-state-copy/           # UI state copy
│   └── daemon/
│       ├── index.md
│       └── manifest.json
├── crabjar-skills/          # reusable crabjar skills (README, install.sh, skill templates)
├── scripts/                 # build/dev scripts
│   └── module-sizes.py
├── archive/                 # empty (was for experiment consolidation)
├── .agents/skills/          # 32 agent skills (Hermes skill ecosystem)
└── .agents/references/      # agent reference files (may be empty in some checkouts)
```

### 2.2 Core Components

| Component | Role | Layer | Status |
| :--- | :--- | :--- | :--- |
| `crabjar` | CLI for state-docs management | Observer (Active) | Core |
| `crabjar_config` | TOML config crate | Config | Active |
| `memory` | Agent-context SQLite storage | Storage | Active |
| `orchestrator` | Axum SSE server + unified LM client | Orchestrator | Active |
| `guard` | Trust layers, annealing, execution gate, scope isolation | Authorization | Active |
| `telemetry` | Flight recorder, command executor | Telemetry | Active |
| `sandbox` | Agent isolation (Unix user, dinit-container, cgroup) | Isolation | Active |
| `tool_registry` | MCP tool registry | Registry | Active |
| `axum-mux` | vm-bridge (per-VM websocket relay) | Bridge | Active |
| `crabjar-architecture` | Mechanical dependency boundary enforcement (8-layer model) | Governance | Active |
| `file_search` | BM25-based file indexing and search (Tantivy 0.22) | Search | Active |
| `crabjar-plugin` | Plugin system (WASM runtime, lifecycle management) | Plugin | Active |
| `crates/terminal` | Terminal multiplexer integration (wezterm/zellij + asciinema v2 recording) | Integration | Active |
| `host/host-core` | Event bus, plugin API, WorkItem model, adapter pattern | Core | Active |
| `host/host-system` | Notifications, clipboard, secrets, tray | System | Active |
| `host/host-observe` | Metrics, tracing, health reporting | Observation | Active |
| `host/host-agent` | Agent loop (ReAct: observe→understand→plan→execute→verify→reflect) | Agent | Active |
| `host/host-webview` | WebView session management, OAuth2, token cache | Integration | Active |
| `host/host-mqtt` | MQTT client + Home Assistant discovery | Integration | Active |
| `host/host-graph` | Microsoft Graph API client | Integration | Active |
| `host/host-screen` | Screen capture + display protocol integration | Integration | Active |
| `apps/teams` | Teams plugin (reference application) | App | Active |
| `src/host-binary` | Host binary crate | Binary | Active |
| `src/skill-script-runner` | Skill script discovery and execution | Skill | Active |
| `src/skill-reference-store` | Skill reference indexing and staleness | Reference | Active |
| `zed-acp-bridge` | Wasm extension (tool call mapping + gate enforcement) | Bridge | Active |
| `zed-acp-server` | stdio JSON-RPC server (ACP protocol execution) | Bridge | Active |

### 2.3 Workspace Members

Declared in Cargo.toml `[workspace.members]`: 24 crates total.
- **Core**: `memory`, `guard`, `telemetry`, `orchestrator`, `sandbox`, `tool_registry`, `axum-mux`, `crabjar-architecture`
- **Plugin system**: `crabjar-plugin` (WASM runtime, lifecycle management)
- **File search**: `file_search` (BM25 indexing with Tantivy 0.22)
- **Terminal**: `crates/terminal` (wezterm/zellij backends + asciinema v2 recording)
- **Host**: `host/host-core`, `host/host-system`, `host/host-observe`, `host/host-agent`, `host/host-webview`, `host/host-mqtt`, `host/host-graph`, `host/host-screen`
- **Apps**: `apps/teams`
- **Binary**: `src/host-binary`
- **Skills**: `src/skill-script-runner`, `src/skill-reference-store`
- **Zed ACP**: `zed-acp-bridge`, `zed-acp-server`
- **ADR process**: `specs/` (Architecture Decision Records, Nygard-style templates)

### 2.4 Shared Dependencies

Declared in Cargo.toml `[workspace.dependencies]`:
- async-trait (0.1), futures (0.3), tokio (1.51.1, full), tokio-stream (0.1, time)
- serde (1.0, derive), serde_json (1.0), toml (0.8)
- thiserror (2.0), anyhow (1.0)
- rusqlite (0.37, bundled)
- clap (4.5, derive, env), clap_mangen (0.2)
- crossterm (0.28), ratatui (0.30)
- tauri (2) + plugins: notification, shell, tray
- libnotify (1.0), arboard (3), keyring (3)
- rumqttc (0.24)
- uuid (1.23.0, v4, serde), chrono (0.4.38, serde, now), tempfile (3.24.0)
- reqwest (0.13.2, json, stream), axum (0.8, http1, tokio), tower-http (0.5, cors)
- tracing (0.1) + subscriber (env-filter), appender (0.2), error (0.2)
- cargo-declared (0.1.3)
- ignore (0.4), path-absolutize (3.1), sha2 (0.10), hex (0.4)
- which (7), dirs (6.0), base64 (0.22)
- zed_extension_api (0.2), tiktoken-rs (0.11.0)
- rstest (0.26.1), serial_test (3.2.0)

### 2.5 Profiles

**Release**: inherits release, opt-level=3, lto=true, strip=true
**Profiling**: inherits release, debug=2
**Dev**: debug=true

---

## 3. Build & Test

| Command | Purpose |
|---|---|
| `just check` | cargo check --workspace |
| `just build` | cargo build -p crabjar |
| `just run state list` | runs binary with replaceable arguments |
| `just test` | cargo test --workspace (691 tests passing) |
| `just clean` | removes build artifacts |
| `cargo clippy --workspace -- -D warnings` | lint; warnings treated as errors |
| `cargo fmt --all` | auto-format every crate |
| `cargo fmt --all -- --check` | CI formatting gate |
| `just module-sizes` | report modules exceeding 500 LoC threshold |
| `just module-sizes-check` | CI gate — fails on >500 LoC |
| `just reproducible-build` | locked deps + deterministic flags |

**Version:** 0.12.0 (Rust 2024 edition)
**Total tests:** 691 passing across workspace, 0 failing

---

## 4. Code Quality & Style

- Formatter: rustfmt with default settings
- Linter: Clippy at --deny warnings
- Naming: snake_case for functions/variables/modules, PascalCase for types/traits, SCREAMING_SNAKE_CASE for constants
- Error handling: thiserror for library crates, anyhow for binary/CLI crates
- No unwrap/expect for recoverable errors in library code
- Dependencies: add to workspace root first, then reference with { workspace = true }
- CLI commands emit structured JSON to stdout; no plain-text success paths
- 500 LoC rule: no single .rs module may exceed 500 lines (CI gate)

---

## 5. Testing Guidelines

- Framework: Rust built-in #[test] and #[cfg(test)]
- SQLite tests: in-memory database (:memory:) or tempfile managed path
- Filesystem tests: use tempfile
- Test naming: descriptive snake_case stating behaviour under test
- Coverage: aim to cover full public API surface of each crate
- CLI integration tests in tests/cli.rs using std::process::Command
- `guard/` crate: 103+ passing tests (scope isolation + trust)

---

## 6. CLI Binary Surface

### 6.1 `crabjar` CLI (src/main.rs)

> Note: CLI surface is the source of truth. If a command is missing from the table, run `crabjar --help` or check `src/main.rs` directly.

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
| `crabjar guard resolution <doc> <id>` | wired | Trust chain resolution view |
| `crabjar exec --command <cmd> --reason <id>` | **end-to-end** | request → guard → concierge → telemetry → outcome → trust update |
| `crabjar bitwarden status/list/get/search/generate` | wired | CLI-available gate |
| `crabjar doctor check` | wired | Checks guard.db/flight.db/knowledge.db schema |
| `crabjar metrics` | wired | Test count, LoC per crate, total modules, workspace member count |

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

> See `ROADMAP.md` for the full ironclaw-informed priority structure. This section summarizes completed work and maps old phases to the new framework.

### Completed Phases

**Phase 1 — Standardization ✅** (clippy clean, CI verified, 741 tests passing)
→ Now maps to **Priority 2: Codex Quality Constraints** in ROADMAP.md

**Phase 2 — Feature Integration ✅** (safetensors, tool_registry, codeburn optimize_engine, crabjar exec pipeline)
→ Now maps to **Priority 1: EdgeCrab Architecture** + **Priority 3: Claw Code Patterns** in ROADMAP.md

**Phase 3 — Consolidation** (move experiments to archive/)
→ Deferred. Focus is on structural patterns from ironclaw.

**Phase 4 — Inference Integration ✅** (unified InferenceBackend, mistral.rs, env-configurable endpoints)
→ Completed. LLM runner remains in separate `llm-workspace/` repo.

### Next Priorities (from ROADMAP.md)

1. **Mechanical dependency boundary enforcement** (`crabjar-architecture` crate) — ✅ done, 8-layer model with CI gate candidate
2. **Scope isolation model** — ✅ done (Scope type + CrossScopeAuth + wired into ExecutionGate)
3. **Requested-vs-effective trust resolution** — ✅ done (with audit trail)
4. **Exact-invocation fingerprint approvals** — ✅ done (InvocationFingerprint + ApprovalLease)
5. **Prompt Envelope** (instruction-hijack defense) — ✅ done (40+ tests)
6. **Product adapter pattern** — ✅ done (ProductAdapter trait + AdapterRegistry)
7. **Per-crate AGENTS.md** — ✅ done (all 23 crates documented)
8. **Dual-backend persistence** — PostgreSQL + SQLite abstraction layer
9. **E2E slice testing** — smoke vs full test matrix
10. **Replay snapshots** — LLM response trace fixtures

**Completed from ROADMAP.md:** 2.5 Prompt Envelope (instruction-hijack defense) — `orchestrator/src/lm_studio_client/prompt_envelope.rs`, 40+ tests, integrated into `chat()` and `chat_with_system()`.

---

### Phase 5: vm-bridge Integration

**Goal:** Integrate vm-bridge as the display/screen sharing layer for crabjar's agent orchestration.

- [x] vm-bridge exists at `src/vm_bridge/` with lib.rs, relay.rs, screen.rs, terminal.rs
- [ ] Wire into crabjar-host for Teams plugin integration
- [ ] Add `crabjar-screen` crate for screen capture
  - [ ] PipeWire integration for screen share sources
  - [ ] XDG-Portal integration for Wayland screen capture
  - [ ] Preview thumbnail generation (320x180 like Electron)
  - [ ] Audio capture (microphone + system audio)
- [ ] Add `crabjar-terminal` crate for shared terminal
  - [ ] Terminal multiplexer integration (wezterm/zellij)
  - [ ] Shared terminal protocol over websocket
  - [ ] Terminal state sync across multiple clients

**Why vm-bridge?**
- Already has WebSocket relay for display protocols
- Process-isolated per-VM architecture
- Hardened (no protocol parsing, just byte transport)
- Can be extended with screen sharing in future

**Integration Points:**
1. `crabjar` → vm-bridge (VM lifecycle management)
2. `crabjar` → `crabjar-host` (screen sharing API)
3. `crabjar-host` → Teams plugin (display protocol routing)

---

## 8. Architectural Constraints

### Detection ≠ Authorization

Crabjar is a pure observer. It knows what happened but cannot change what happens. This is enforced by design.

### Executor Layer Status

Runtime execution is executor-capable. Execution is opt-in via `.crabjar_config.toml` `tool_execution_enabled`. The single pipeline is: `request → guard → concierge → telemetry → outcome → trust update`. Pending actions persist to GuardDb. All actions require real provenance lookup.

*Security model inspired by [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw) docs: six-layer security (channel pairing → autonomy → workspace → command policy → OS sandbox → tool receipts), three-level autonomy (ReadOnly/Supervised/Full), per-tool overrides, HMAC tool receipts. Crabjar extends with dynamic confidence bands + annealing (decay/reinforcement) — ZeroClaw uses static risk profiles.*

### Doubt Output Requirement

Every derived output must include a `doubt` block with:
- `assumptions` — what it assumed to produce this output
- `blind_spots` — what it couldn't see
- `last_validation` — when this was last checked against raw data

---

## 9. Crabjar Context

### 9.1 Structure

crabjar contains:
- agent_config.md
- AGENTS.md
- Cargo.toml (workspace root + crabjar binary manifest, version 0.12.0)
- Justfile
- Containerfile, Dockerfile
- orchestrator (Axum SSE server + unified LM client with prompt envelope defense)
- guard (ExecutionGate + GateConcierge + TrustManager + annealing + scope isolation + fingerprint approvals + trust resolution + static policy engine + context budgeting + domain allowlist)
- memory (agent-context crate with state_docs querier, context fragments with token budget)
- telemetry (flight recorder + command executor)
- sandbox (agent isolation)
- tool_registry (MCP tool registry with 4-layer discovery)
- crabjar-architecture (mechanical dependency boundary enforcement, 8-layer model)
- file_search (BM25-based file indexing with Tantivy 0.22)
- crabjar-plugin (WASM runtime, lifecycle management — stub)
- host/ (8 host crates: core, system, observe, agent, webview, mqtt, graph, screen)
- apps/teams (Teams plugin)
- zed-acp-bridge (Wasm extension)
- zed-acp-server (stdio JSON-RPC server)
- axum-mux (vm-bridge)
- crates/terminal (wezterm/zellij backends + asciinema v2 recording)
- .agents/skills/ (32 agent skills)
- .agents/references/ (may be empty)
- src/vm_bridge/ (per-VM websocket relay)
- src/bitwarden/ (bitwarden CLI integration)
- src/knowledge_store/ (knowledge store commands with bridge + confidence)
- src/crabjar_config/ (workspace config crate)
- src/skill-script-runner/ (skill script discovery)
- src/skill-reference-store/ (skill reference indexing)
- src/host-binary/ (host binary crate with TUI, guard approval flow)
- tests/cli.rs
- tests/e2e/ (smoke + full E2E test slices)
- tests/snapshots/ (insta snapshot baselines)
- ui-state-copy
- workspace/ (workspace config)
- `crabjar-skills/` (reusable crabjar skills)
- `human_reference.md` (human reference documentation)
- `environment_manifest.json` (environment manifest)
- `index.md` (root index)
- `REPRO.md` (reproduction guide)
- `build.rs` (root build script)
- `ROADMAP.md` (development roadmap)
- `specs/` (Architecture Decision Records — Nygard-style templates, see specs/README.md)

### 9.2 Active Rust Surface

crabjar (binary) + crabjar_config (library) + agent-context (library) + orchestrator + guard + telemetry + sandbox + tool_registry + crabjar-architecture + file_search + crabjar-plugin + host/host-core + host/host-system + host/host-observe + host/host-agent + host/host-webview + host/host-mqtt + host/host-graph + host/host-screen + apps/teams + src/host-binary + src/skill-script-runner + src/skill-reference-store + zed-acp-bridge + zed-acp-server + axum-mux + crates/terminal

### Test Count

**691 passing across workspace, 0 failing.**
Guard: ~240+ tests (scope isolation, trust resolution, annealing, policy engine, domain allowlist, context budgeting). Host-agent: ~35+ tests (model routing, context compression, decision gate, loop integration). File search: 6 passing. E2E: 32 total (6 smoke + 26 full). Snapshot: 6 tests.

---

## 10. Drift Report

### Last Audit

2026-07-08 — Fresh filesystem scan. Workspace: 24 members + `specs/` (ADR process). Guard: 24 source files (added policy.rs, policy_types.rs, context_budget.rs, command_risk.rs, risk_config.rs, db_error.rs; split guard_db_impl into impl/queries/types). Memory/state_docs: 9 files (split indexer into extract.rs + insert.rs). Telemetry: 5 files (added command_executor.rs). Host/host-agent: 8 source files + inference/ subdir (added model_routing.rs, context_compression.rs, decision_gate.rs — ReAct loop with phase-aware routing, token budget compression, and decision gating). File search: 3 source files. crabjar-plugin: stub crate. crabjar-architecture: 3 source files. crates/terminal: 4 source files (wezterm/zellij backends + recording). Skills: 32 agent skills. Version: 0.12.0, Rust 2024 edition. Total tests: 691 passing, 0 failing. ADR process established in specs/. Known phantom items removed: state-docs/, bin/, git/, gitignore/, reference_materials/, browser-tools-mcp/, llmrunner.md.

### Known Items

- `guard/src/schema.sql` — GuardDb schema definition
- `guard/src/concierge.rs` — sole gate enforcement layer (~466 LoC)
- `guard/src/scope.rs` — scope isolation model (identity, project, tenant, thread dimensions) + 16 tests
- `guard/src/trust_resolution.rs` — requested-vs-effective trust resolution (~431 LoC)
- `guard/src/fingerprint.rs` — InvocationFingerprint + SHA-256
- `guard/src/domain_allowlist.rs` — Domain allowlist for web fetch scope gating (10 tests)
- `guard/src/policy.rs` — StaticPolicyEngine with TOML-based declarative policies (17 tests)
- `guard/src/context_budget.rs` — ContextBudget + MAX_TOKENS_PER_FRAGMENT (6 tests)
- `guard/src/command_risk.rs` — CommandRisk, HIGH/MEDIUM_RISK_COMMANDS (6 tests)
- `guard/src/guard_db_impl.rs` — 368 LoC (anneal + concierge + PID trust) — split from monolithic impl
- `guard/src/guard_db_queries.rs` — 352 LoC (action requests + trust resolution) — split from monolithic impl
- `guard/src/guard_db_types.rs` — 16 LoC (TrustResolutionEntry) — split from monolithic impl
- `memory/src/state_docs/extract.rs` — markdown parsing (~400 LoC)
- `memory/src/state_docs/insert.rs` — SQLite writes (~144 LoC)
- `memory/src/context/mod.rs` — ContextFragmentBuilder with token budget (24 tests)
- `orchestrator/src/lm_studio_client/` — unified LM client with SessionStore; `LmStudioEndpoint::MistralRsServe` variant for mistral.rs serve
- `orchestrator/src/backend/mod.rs` — unified `InferenceBackend` trait + `BackendKind` enum
- `orchestrator/prompts/default_system.md` — default system prompt template
- `.agents/skills/` — 32 agent skills (added: format-version-drift, session-handoff)
- `.agents/references/` — agent reference files (may be empty)
- `crabjar-architecture/` — mechanical dependency boundary enforcement (8-layer model, CI gate candidate)
- `file_search/` — BM25-based file indexing and search with Tantivy 0.22 (lib.rs, indexer.rs, storage.rs) — 6 tests
- `crabjar-plugin/` — WASM runtime + lifecycle management (stub crate)
- `axum-mux/` — vm-bridge (per-VM websocket relay, screen capture, terminal multiplexer)
- `src/vm_bridge/` — per-VM websocket relay (lib.rs, relay.rs, screen.rs, terminal.rs)
- `src/crabjar_config/` — workspace config crate (underscore, not hyphen)
- `src/bitwarden/commands.rs` — additional bitwarden command handler
- `src/host-binary/` — host binary crate with TUI (cli.rs, dashboard.rs, main.rs, Cargo.toml, AGENTS.md)
- `src/metrics/` — metrics reporting subcommand
- `apps/teams/teams-for-linux/` — additional teams-for-linux subdirectory
- `crabjar-skills/` — reusable crabjar skills (README, install.sh, skill templates)
- `scripts/module-sizes.py` — module size reporting script
- `archive/` — empty (was for experiment consolidation)
- Per-crate AGENTS.md — complete (all 24 crates + root documented)
- **Cross-project: llm-workspace** — configured via opencode.jsonc instructions + dotfiles symlink graph
- **ADR process**: `specs/` directory with Nygard-style ADR template, README index, and ADR-001 establishing the decision process
- **Removed**: state-docs/, bin/, git/, gitignore/, reference_materials/, browser-tools-mcp/, llmrunner.md (no longer exist)

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
| `prov-zeroclaw-credit` | README + project_map credit: ZeroClaw docs for security model, tool receipts, microkernel, SOP, config schema | 2026-06-06 | Attribution for design influence | crabjar/README.md, crabjar/project_map.md |
| `prov-codex-parity` | ROADMAP.md Priority 9: 5 Codex pattern imports (bounded context, module size governance, snapshot testing, file search, Starlark exec policy) | 2026-06-23 | Feature parity analysis — Codex vs CrabJar | crabjar/README.md, crabjar/project_map.md, codex/AGENTS.md, codex/codex-rs/Cargo.toml |
| `prov-map-drift-2026-06-27` | project_map.md regenerated — 22 members, guard/src fully updated (23 files), crabjar-architecture added to tree, vm_bridge added, host/host-agent added, orchestrator backend/mod.rs documented, shared deps refreshed (async-trait, tauri, rumqttc, tracing-error, etc.), CLI commands updated with guard resolution, section 9 cleaned up (removed stale codeburn/gguf/llm-runner references) | 2026-06-27 | Roadmap 1.3 update | crabjar/project_map.md |
| `prov-map-drift-2026-07-06` | project_map.md regenerated — 24 members (added file_search + crabjar-plugin), tree diagram updated, Core Components table populated, workspace members section refreshed, Section 9 Crabjar Context updated with new crates, Drift Report Last Audit and Known Items updated | 2026-07-06 | Structural alignment refresh | crabjar/project_map.md
| `prov-map-drift-2026-07-08` | project_map.md regenerated — generated date updated, Section 3 Build & Test expanded (module-sizes targets, reproducible-build, test count), Section 2.3 workspace members includes specs/ ADR process, Section 2.4 shared deps consolidated to single-line format, Section 2.5 profiles renamed and simplified, guard/src file listing expanded with policy/context_budget/command_risk/risk_config/db_error files, host/host-agent updated with model_routing/context_compression/decision_gate, Section 9 structure updated (orchestrator prompt envelope, memory context fragments, E2E tests, snapshot baselines), Drift Report Last Audit and Known Items refreshed with current state | 2026-07-08 | Structural alignment refresh + ADR process addition | crabjar/project_map.md
| `prov-map-drift-2026-07-08b` | project_map.md regenerated — Section 9.1 structure list expanded (orchestrator prompt envelope, memory context fragments, E2E tests, snapshot baselines), Section 9.2 test count updated to 691 passing with breakdown by crate, Drift Report Known Items expanded with policy/context_budget/command_risk/risk_config/db_error files and ADR process entry | 2026-07-08 | Structural alignment refresh — second pass on Section 9 + Known Items | crabjar/project_map.md

---

*End of review.*
