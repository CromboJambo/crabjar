# project_map.md

> Generated: November 02 2026 — Fresh structural audit (77 workspace members)
> Source: Cargo.toml, filesystem scan, README.md, AGENTS.md, agent_config.md
> Purpose: Structural alignment reference for agent navigation

---

## 1. Overview

CrabJar is a Rust 2024 workspace with 77 crates centered on the `crabjar` CLI. It provides state-docs management, workspace config loading, knowledge-store bridge, orchestrator (Axum SSE server with unified LM client), conductor (fleet orchestration service with capability-based security model), guard (execution gate with trust layers, annealing, scope isolation, fingerprint approvals), telemetry (flight recorder), sandbox (agent isolation), tool registry, skill script runner, skill reference store, file search engine (BM25/Tantivy 0.22), plugin system (WASM runtime), Zed ACP bridge, agent skills, remote skill discovery infrastructure, subagent delegation framework, analysis toolkit (8 analyze-* crates for APK/dotnet/firmware/HAR/JS/native/MCP), VM lifecycle management (vm-core + vm-bridge), durable objects, worker framework, compression pipeline, ML training data extraction and preprocessing, GPU-accelerated dashboard (herdr protocol), git operations, commit message generation, kanban orchestration, SSH bastion host, email/web/browser tools, Home Assistant integration, and deployment tooling. LLM inference (runner, plug-in, safetensors, GGUF parser) is in a separate workspace at `llm-workspace/`.

---

## 2. Architecture

### 2.1 Workspace Layout

```text
crabjar/
├── Cargo.toml               # Workspace root — 77 members, shared deps
├── Cargo.lock               # Locked dependency graph
├── build.rs                 # Root build script
├── AGENTS.md                # Repository guidelines
├── README.md                # Project overview
├── agent_config.md          # Agent configuration
├── Justfile                 # Task runner shortcuts
├── Containerfile, Dockerfile
├── rust-toolchain.toml      # Rust nightly toolchain
├── .crabjar_config.toml     # Workspace config (tool_execution_enabled)
├── index.md                 # Root index
├── REPRO.md                 # Reproduction guide
├── human_reference.md       # Human reference documentation
├── environment_manifest.json# Environment manifest (CPU/GPU/storage)
│
├── project_map.md           # This file
├── ROADMAP.md               # Development roadmap
│
│  Core crabjar crates
├── src/main.rs              # CLI entry point (PR #3: split into command modules)
├── src/lib.rs               # shared library surface
├── src/commands/            # Command modules (post PR #3 api-module-splitting)
├── src/bitwarden/           # bitwarden CLI integration
├── src/knowledge_store/     # knowledge-store commands
├── src/crabjar_config/      # workspace config crate
├── src/host-binary/         # host binary crate (TUI, guard approval flow)
├── src/skill-script-runner/ # skill script discovery and execution
├── src/skill-reference-store/ # skill reference indexing and staleness
├── src/skill-process-management/ # process management skill
├── src/skill-struct-tok-pipeline/ # structural tokenization pipeline
│
│  Supporting crates
├── memory/                  # agent-context crate, SQLite-backed storage (21 files)
│   ├── state_docs/          # state-docs querier (drift_status)
│   └── tests/
├── guard/                   # Trust layers, annealing, execution gate (30 files)
│   └── src/                 # 30 source files including policy engine, domain allowlist
├── telemetry/               # Flight recorder, command executor (7 files)
├── sandbox/                 # Agent isolation (6 files)
├── tool_registry/           # MCP tool registry (5 files)
├── orchestrator/            # Axum SSE server + unified LM client (16 files)
│   ├── src/lm_studio_client/# Unified LM client with SessionStore
│   └── prompts/             # Default system prompt templates
│
│  Analysis toolkit (8 crates)
├── analyze-common/          # Shared analysis types and utilities
├── analyze-apk/             # Android APK analysis
├── analyze-dotnet/          # .NET assembly analysis
├── analyze-firmware/        # Firmware image analysis
├── analyze-har/             # HTTP Archive (HAR) file analysis
├── analyze-js/              # JavaScript bundle analysis
├── analyze-native/          # Native binary analysis
├── analyze-mcp-server/      # MCP server analysis
│
│  VM and worker infrastructure
├── vm/vm-core/              # VM lifecycle management (5 files)
├── crabjar-conductor/       # Fleet orchestration service (51 files)
├── crabjar-worker/          # Worker framework for distributed execution
├── crabjar-durable-objects/ # Durable object storage and retrieval
│
│  Compression pipeline
├── crabjar-compress/        # Content compression engine (8 files)
├── crabjar-compress-mcp/    # MCP server for compression operations
│
│  ML training data pipeline
├── crates/train-extract/    # Training data extraction from HAR/network captures
├── crates/train-secret-stripper/ # PII/secret removal from training data
├── crates/train-dedup/      # Training data deduplication engine
├── crates/train-filewatcher/# File watcher for training data pipeline
│
│  Git operations
├── crates/crabjar-git/      # Git operations (diff, status, add via libgit2)
├── crates/crabjar-commit/   # Commit message generation from diffs
├── crates/crabjar-commit-cli/ # CLI tool for commit staging and messages
│
│  Orchestration and scheduling
├── scheduler/               # Cron jobs, workers, watchers with execution gating (7 files)
├── crabjar-kanban-orchestrator/ # Kanban board orchestration
├── slow-friend/             # Drift detection and adaptation (10 files)
│
│  Web tools
├── crabjar-web-search/      # Web search integration (7 files)
├── crabjar-web-extract/     # Web page content extraction (4 files)
├── crates/crabjar-browser/  # Browser automation
│
│  Communication tools
├── crates/email-smtp/       # SMTP email sending
├── crabjar-skills/          # Reusable skill definitions (6 files)
│
│  Tooling and binding
├── crabjar-tool-binding/    # Tool call binding layer
├── crabjar-analyze/         # Analysis orchestration CLI
├── crabjar-review/          # Code review tool (13 files)
├── crabjar-mcp-tool-registry/ # MCP tool registry service
│
│  Remote skill discovery (Phases 8-11)
├── crabjar-github-client/   # GitHub API client for skill search/install
├── crabjar-clawhub-client/  # ClawHub.io registry API integration
├── crabjar-skill-scanner/   # Security scanning pipeline + checksum verification
├── crabjar-trust-manager/   # Trust tier management, skill cache, update detection
│
│  MCP servers
├── crabjar-unsloth-mcp/     # Unsloth training data and fine-tuning jobs via MCP
├── crabjar-herdr-mcp/       # Herdr fleet management via MCP
│
│  Structure analysis
├── crabjar-structured-aux/  # Structured auxiliary analysis
├── universal-structure/     # Universal structure definitions (11 files)
│
│  Infrastructure
├── crates/ssh-bastion/      # SSH bastion host using russh (7 files)
├── crabjar-deploy/          # Deployment tooling (8 files)
├── crabjar-memory/          # Memory management service (7 files)
├── crabjar-delegation/      # Subagent delegation framework (5 files)
├── crabjar-cron/            # Cron job management (6 files)
│
│  Host runtime (8 crates)
├── host/host-core/          # Event bus, plugin API, WorkItem model (7 files)
├── host/host-system/        # Notifications, clipboard, secrets, tray (5 files)
├── host/host-observe/       # Metrics, tracing, health reporting (4 files)
├── host/host-agent/         # Agent loop — ReAct pattern (15 files)
├── host/host-webview/       # WebView session management, OAuth2 (7 files)
├── host/host-mqtt/          # MQTT client + Home Assistant discovery (6 files)
├── host/host-graph/         # Microsoft Graph API client (4 files)
├── host/host-screen/        # Screen capture + display protocol integration (3 files)
│
│  Host apps
├── apps/teams/              # Teams plugin (reference application)
├── apps/terrarium/          # Terrarium isometric habitat app (10 files)
│
│  GPU dashboard
├── crates/crabjar-gpui/     # GPU-accelerated multi-agent dashboard (herdr protocol)
│
│  Integration bridges
├── axum-mux/                # vm-bridge (per-VM websocket relay, 7 files)
├── zed-acp-bridge/          # Wasm extension (tool call mapping + gate enforcement)
├── zed-acp-server/          # stdio JSON-RPC server (ACP protocol execution)
│
│  Plugin system
├── crabjar-plugin/          # WASM runtime, lifecycle management (stub)
│
│  Governance and architecture
├── crabjar-architecture/    # Mechanical dependency boundary enforcement (8-layer model)
│
│  Non-crate artifacts
├── tests/                   # CLI integration tests
│   ├── cli.rs
│   └── e2e/                 # End-to-end test slices
├── scripts/                 # Build/dev scripts
├── .agents/skills/          # Agent skills (Hermes skill ecosystem)
├── specs/                   # Architecture Decision Records (Nygard-style templates)
├── state-docs/              # State documentation storage
```

### 2.2 Core Components

| Component | Role | Layer | Status |
| :--- | :--- | :--- | :--- |
| `crabjar` | CLI for state-docs management | Observer (Active) | Core |
| `crabjar_config` | TOML config crate | Config | Active |
| `memory` | Agent-context SQLite storage | Storage | Active |
| `orchestrator` | Axum SSE server + unified LM client | Orchestrator | Active |
| `crabjar-conductor` | Fleet orchestration service with capability-based security model | Orchestration | Active |
| `crabjar-worker` | Worker framework for distributed task execution | Orchestration | Active |
| `crabjar-durable-objects` | Durable object storage and retrieval | Storage | Active |
| `guard` | Trust layers, annealing, execution gate, scope isolation | Authorization | Active |
| `telemetry` | Flight recorder, command executor | Telemetry | Active |
| `sandbox` | Agent isolation (Unix user, dinit-container, cgroup) | Isolation | Active |
| `tool_registry` | MCP tool registry | Registry | Active |
| `crabjar-mcp-tool-registry` | MCP tool registry service | Registry | Active |
| `scheduler` | Cron jobs, workers, watchers with execution gating | Scheduling | Active |
| `crabjar-cron` | Cron job management interface | Scheduling | Active |
| `analyze-*` (8 crates) | Analysis toolkit for APK/dotnet/firmware/HAR/JS/native/MCP | Analysis | Active |
| `vm/vm-core` | VM lifecycle management | Infrastructure | Active |
| `axum-mux` | vm-bridge (per-VM websocket relay) | Bridge | Active |
| `crabjar-deploy` | Deployment tooling | DevOps | Active |
| `crates/ssh-bastion` | SSH bastion host using russh | Infrastructure | Active |
| `crabjar-github-client` | GitHub API client for skill search/install (Phase 8) | Remote Discovery | Active |
| `crabjar-clawhub-client` | ClawHub.io registry API integration (Phase 9) | Remote Discovery | Active |
| `crabjar-skill-scanner` | Security scanning pipeline + checksum verification (Phase 10) | Remote Discovery | Active |
| `crabjar-trust-manager` | Trust tier management, skill cache, update detection (Phase 11) | Remote Discovery | Active |
| `crabjar-delegation` | Subagent delegation framework — Hermes-compatible delegate_task tool | Orchestration | Active |
| `crabjar-gpui` | GPU-accelerated multi-agent dashboard using herdr protocol | UI/Dashboard | Active |
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
| `apps/terrarium` | Terrarium isometric habitat app | App | Active |
| `src/host-binary` | Host binary crate with TUI | Binary | Active |
| `src/skill-script-runner` | Skill script discovery and execution | Skill | Active |
| `src/skill-reference-store` | Skill reference indexing and staleness | Reference | Active |
| `src/skill-process-management` | Process management skill | Skill | Active |
| `zed-acp-bridge` | Wasm extension (tool call mapping + gate enforcement) | Bridge | Active |
| `zed-acp-server` | stdio JSON-RPC server (ACP protocol execution) | Bridge | Active |
| `crabjar-compress` | Content compression engine | Pipeline | Active |
| `crates/train-*` (4 crates) | ML training data extraction and preprocessing pipeline | Training | Active |
| `crates/crabjar-git` | Git operations (diff, status, add via libgit2) | Integration | Active |
| `crates/crabjar-commit` | Commit message generation from diffs | Integration | Active |
| `crabjar-kanban-orchestrator` | Kanban board orchestration | Orchestration | Active |
| `slow-friend` | Drift detection and adaptation | Observability | Active |
| `crabjar-web-search` | Web search integration | Tool | Active |
| `crabjar-web-extract` | Web page content extraction | Tool | Active |
| `crates/email-smtp` | SMTP email sending | Communication | Active |
| `crabjar-memory` | Memory management service | Storage | Active |

### 2.3 Workspace Members

Declared in Cargo.toml `[workspace.members]`: 77 crates total.

- **Core**: `memory`, `guard`, `telemetry`, `orchestrator`, `sandbox`, `tool_registry`, `axum-mux`, `crabjar-architecture`
- **Analysis toolkit** (8 crates): `analyze-common`, `analyze-apk`, `analyze-dotnet`, `analyze-firmware`, `analyze-har`, `analyze-js`, `analyze-native`, `analyze-mcp-server`
- **VM and worker infrastructure**: `vm/vm-core`, `crabjar-conductor`, `crabjar-worker`, `crabjar-durable-objects`
- **Compression pipeline**: `crabjar-compress`, `crabjar-compress-mcp`
- **ML training data pipeline** (4 crates): `train-extract`, `train-secret-stripper`, `train-dedup`, `train-filewatcher`
- **Git operations** (3 crates): `crabjar-git`, `crabjar-commit`, `crabjar-commit-cli`
- **Orchestration and scheduling**: `scheduler`, `crabjar-kanban-orchestrator`, `slow-friend`
- **Web tools**: `crabjar-web-search`, `crabjar-web-extract`, `crates/crabjar-browser`
- **Communication tools**: `crates/email-smtp`, `crabjar-skills`
- **Tooling and binding**: `crabjar-tool-binding`, `crabjar-analyze`, `crabjar-review`, `crabjar-mcp-tool-registry`
- **Remote skill discovery** (Phases 8-11): `crabjar-github-client`, `crabjar-clawhub-client`, `crabjar-skill-scanner`, `crabjar-trust-manager`
- **MCP servers**: `crabjar-unsloth-mcp`, `crabjar-herdr-mcp`
- **Structure analysis**: `crabjar-structured-aux`, `universal-structure`
- **Infrastructure**: `crates/ssh-bastion`, `crabjar-deploy`, `crabjar-memory`, `crabjar-delegation`, `crabjar-cron`
- **Host runtime** (8 crates): `host/host-core`, `host/host-system`, `host/host-observe`, `host/host-agent`, `host/host-webview`, `host/host-mqtt`, `host/host-graph`, `host/host-screen`
- **Apps**: `apps/teams`, `apps/terrarium`
- **GPU dashboard**: `crates/crabjar-gpui`
- **Integration bridges**: `axum-mux`, `zed-acp-bridge`, `zed-acp-server`
- **Plugin system**: `crabjar-plugin`
- **Terminal**: `crates/terminal`
- **Skills** (4 crates): `src/skill-script-runner`, `src/skill-reference-store`, `src/skill-process-management`, `src/skill-struct-tok-pipeline`
- **Binary**: `src/host-binary`

### 2.4 Shared Dependencies

Declared in Cargo.toml `[workspace.dependencies]`:
- async-trait (0.1), futures (0.3), tokio (1.51.1, full), tokio-stream (0.1, time)
- serde (1.0, derive), serde_json (1.0), toml (0.8), schemars (0.8)
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

**Version:** 0.12.5 (Rust 2024 edition)
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

### 6.1 `crabjar` CLI (src/main.rs → src/commands/)

> Note: PR #3 merged — main.rs split into command modules under src/commands/.
> CLI surface is the source of truth. If a command is missing from the table, run `crabjar --help` or check `src/commands/` directly.

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

## 7. Change History & Future Work

Completed work is archived in [CHANGELOG.md](./CHANGELOG.md). Upcoming priorities are tracked in [ROADMAP.md](./ROADMAP.md). This section no longer duplicates that content — see those files for the authoritative state.

---

## 8. Architectural Constraints

### Detection ≠ Authorization

Crabjar is a pure observer. It knows what happened but cannot change what happens. This is enforced by design.

### Executor Layer Status

Runtime execution is executor-capable. Execution is opt-in via `.crabjar_config.toml` `tool_execution_enabled`. The single pipeline is: `request → guard → concierge → telemetry → outcome → trust update`. Pending actions persist to GuardDb. All actions require real provenance lookup.

*Security model inspired by [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw) docs: six-layer security (channel pairing → autonomy → workspace → command policy → OS sandbox → tool receipts), three-level autonomy (ReadOnly/Supervised/Full), per-tool overrides, HMAC tool receipts. Crabjar extends with dynamic confidence bands + annealing (decay/reinforcement) — ZeroClaw uses static risk profiles.*

*Authorization-order contract inspired by [CodeWhale](https://github.com/Hmbown/CodeWhale)'s `AUTHORIZATION_ORDER.md`: the gate's layers run in a fixed order and are monotonic — once a layer blocks or defers, no later layer may loosen the decision. An approval from one layer is not a universal bypass; the sole loosening mechanism (`CrossScopeAuth`) is scoped to the scope-isolation layer. Pinned by regression tests in `guard/src/authorization_order.rs` (10 tests).*

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
- Cargo.toml (workspace root + crabjar binary manifest, version 0.12.5)
- Justfile
- Containerfile, Dockerfile
- orchestrator (Axum SSE server + unified LM client with prompt envelope defense)
- crabjar-conductor (fleet orchestration service with four-layer capability-based security model, guard integration)
- crabjar-worker (worker framework for distributed task execution)
- crabjar-durable-objects (durable object storage and retrieval)
- crabjar-gpui (GPU-accelerated multi-agent dashboard using herdr protocol, CrabjarSurfaceAdapter)
- guard (ExecutionGate + GateConcierge + TrustManager + annealing + scope isolation + fingerprint approvals + trust resolution + static policy engine + context budgeting + domain allowlist)
- memory (agent-context crate with state_docs querier, context fragments with token budget)
- telemetry (flight recorder + command executor)
- sandbox (agent isolation)
- tool_registry (MCP tool registry with 4-layer discovery)
- crabjar-mcp-tool-registry (MCP tool registry service)
- scheduler (cron jobs, workers, watchers with execution gating)
- crabjar-cron (cron job management interface)
- analyze-* (8 crates: analysis toolkit for APK/dotnet/firmware/HAR/JS/native/MCP)
- vm/vm-core (VM lifecycle management)
- axum-mux (vm-bridge)
- crabjar-deploy (deployment tooling)
- crates/ssh-bastion (SSH bastion host using russh)
- crabjar-github-client (GitHub API client for skill search/install — Phase 8)
- crabjar-clawhub-client (ClawHub.io registry API integration — Phase 9)
- crabjar-skill-scanner (security scanning pipeline + checksum verification — Phase 10)
- crabjar-trust-manager (trust tier management, skill cache, update detection — Phase 11)
- crabjar-delegation (subagent delegation framework — Hermes-compatible delegate_task tool)
- crabjar-architecture (mechanical dependency boundary enforcement, 8-layer model)
- file_search (BM25-based file indexing with Tantivy 0.22)
- crabjar-plugin (WASM runtime, lifecycle management — stub)
- host/ (8 host crates: core, system, observe, agent, webview, mqtt, graph, screen)
- apps/teams (Teams plugin)
- apps/terrarium (Terrarium isometric habitat app)
- zed-acp-bridge (Wasm extension)
- zed-acp-server (stdio JSON-RPC server)
- crabjar-compress (content compression engine)
- crabjar-compress-mcp (MCP server for compression operations)
- crates/train-* (4 crates: ML training data extraction and preprocessing pipeline)
- crates/crabjar-git (git operations via libgit2)
- crates/crabjar-commit (commit message generation from diffs)
- crates/crabjar-commit-cli (CLI tool for commit staging and messages)
- crabjar-kanban-orchestrator (kanban board orchestration)
- slow-friend (drift detection and adaptation)
- crabjar-web-search (web search integration)
- crabjar-web-extract (web page content extraction)
- crates/crabjar-browser (browser automation)
- crates/email-smtp (SMTP email sending)
- crabjar-skills (reusable skill definitions)
- crabjar-tool-binding (tool call binding layer)
- crabjar-analyze (analysis orchestration CLI)
- crabjar-review (code review tool)
- crabjar-unsloth-mcp (Unsloth training data and fine-tuning jobs via MCP)
- crabjar-herdr-mcp (Herdr fleet management via MCP)
- crabjar-structured-aux (structured auxiliary analysis)
- universal-structure (universal structure definitions)
- crabjar-memory (memory management service)
- crabjar-homeassistant (Home Assistant integration)
- .agents/skills/ (agent skills ecosystem)
- src/bitwarden/ (bitwarden CLI integration)
- src/knowledge_store/ (knowledge store commands with bridge + confidence)
- src/crabjar_config/ (workspace config crate)
- src/skill-script-runner/ (skill script discovery)
- src/skill-reference-store/ (skill reference indexing)
- src/skill-process-management/ (process management skill)
- src/skill-struct-tok-pipeline/ (structural tokenization pipeline)
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

crabjar (binary) + crabjar_config (library) + agent-context (library) + orchestrator + crabjar-conductor + crabjar-worker + crabjar-durable-objects + guard + telemetry + sandbox + tool_registry + crabjar-mcp-tool-registry + scheduler + crabjar-cron + analyze-* (8 crates) + vm/vm-core + axum-mux + crabjar-deploy + ssh-bastion + crabjar-github-client + crabjar-clawhub-client + crabjar-skill-scanner + crabjar-trust-manager + crabjar-delegation + crabjar-architecture + file_search + crabjar-plugin + host/ (8 crates) + apps/teams + apps/terrarium + zed-acp-bridge + zed-acp-server + crabjar-compress + crabjar-compress-mcp + train-* (4 crates) + crabjar-git + crabjar-commit + crabjar-commit-cli + crabjar-kanban-orchestrator + slow-friend + crabjar-web-search + crabjar-web-extract + crabjar-browser + email-smtp + crabjar-skills + crabjar-tool-binding + crabjar-analyze + crabjar-review + crabjar-unsloth-mcp + crabjar-herdr-mcp + crabjar-structured-aux + universal-structure + crabjar-memory + crabjar-homeassistant + crates/terminal

### Test Count

**691 passing across workspace, 0 failing.** Guard: ~240+ tests (scope isolation, trust resolution, annealing, policy engine, domain allowlist, context budgeting). Host-agent: ~35+ tests (model routing, context compression, decision gate, loop integration). Conductor: 17 tests (auth, gate, task assignment, annealing, persistence). File search: 6 passing. E2E: 32 total (6 smoke + 26 full). Snapshot: 6 tests.

---

## 10. Drift Report

### Last Audit

2026-11-02 — Fresh structural audit. Workspace: 77 members (+46 since last documented audit on 2026-10-17 which had 30 members). Major additions since last audit:
- **Analysis toolkit** (8 crates): analyze-common, analyze-apk, analyze-dotnet, analyze-firmware, analyze-har, analyze-js, analyze-native, analyze-mcp-server
- **VM and worker infrastructure**: vm/vm-core, crabjar-conductor, crabjar-worker, crabjar-durable-objects
- **Compression pipeline**: crabjar-compress (8 files), crabjar-compress-mcp
- **ML training data pipeline** (4 crates): train-extract, train-secret-stripper, train-dedup, train-filewatcher
- **Git operations** (3 crates): crabjar-git, crabjar-commit, crabjar-commit-cli
- **Orchestration and scheduling**: scheduler (7 files), crabjar-kanban-orchestrator, slow-friend (10 files)
- **Web tools**: crabjar-web-search (7 files), crabjar-web-extract (4 files), crates/crabjar-browser
- **Communication tools**: crates/email-smtp, crabjar-skills (6 files)
- **Tooling and binding**: crabjar-tool-binding, crabjar-analyze, crabjar-review (13 files), crabjar-mcp-tool-registry
- **MCP servers**: crabjar-unsloth-mcp, crabjar-herdr-mcp
- **Structure analysis**: crabjar-structured-aux, universal-structure (11 files)
- **Infrastructure**: crates/ssh-bastion (7 files), crabjar-deploy (8 files), crabjar-memory (7 files), crabjar-delegation (5 files), crabjar-cron (6 files)
- **Apps**: apps/terrarium (Terrarium isometric habitat app, 10 files)
- **Skills** (2 new): src/skill-process-management, src/skill-struct-tok-pipeline
- **PR #3 merged**: api-module-splitting — main.rs split into command modules under src/commands/
- Version updated: 0.12.0 → 0.12.5

Prior audit entries preserved below for historical reference.

2026-07-10 — Fresh filesystem scan. Workspace: 25 members + `specs/` (ADR process). Guard: 28 source files (added gate_tests.rs, concierge_types.rs, fingerprint_types.rs, trust_types.rs; all under 500 LoC). Memory/: 18 source files total (error.rs, models.rs, schema.rs, store.rs added; context/ split into mod/constants/fragment/budget; state_docs/ has extract.rs + insert.rs). Orchestrator: 11 .rs files (added backend/mod.rs for InferenceBackend trait; lm_studio_client split into prompt_types.rs + prompt_validator.rs). Host/host-agent: 13 source files (inference/ subdir with mod.rs, backend.rs, http_backend.rs). Axum-mux: gained proxy.rs and terminal_relay.rs. File search: 3 source files. crabjar-plugin: stub crate. crabjar-architecture: 3 source files. crates/terminal: 5 source files (added recording.rs). Skills: 32 agent skills. Version: 0.12.0, Rust 2024 edition. Total tests: 691 passing, 0 failing. Clippy clean — zero errors (only pre-existing vm-bridge warnings for missing lib target). ADR process established in specs/. Known phantom items removed: state-docs/, bin/, git/, gitignore/, reference_materials/, browser-tools-mcp/, llmrunner.md.

2026-10-16 — Remote skill discovery infrastructure added (Phases 8-11). Workspace: 29 members (+4 new crates). Added: crabjar-github-client (GitHub API client for skill search/install), crabjar-clawhub-client (ClawHub.io registry API integration with trust tier mapping), crabjar-skill-scanner (security scanning pipeline with SHA-256 checksum verification), crabjar-trust-manager (trust tier management, version-aware skill cache in ~/.crabjar/skill-cache/, update detection). CLI commands added: tool search, tool install, tool clawhub-search, tool clawhub-install, tool scan, tool cache list/remove/clean, tool updates. All crates at version 0.12.0, Rust 2024 edition. Build clean across workspace with no errors or warnings.

2026-10-17 — Conductor security model implemented (Phases 1-4). Workspace: 30 members (+1 new crate). crabjar-conductor now has four-layer capability-based access control: SSH key worker authentication, JWT-like token authorization with embedded capabilities and scopes, task-level capability matching (exact + wildcard), and trust level annealing (workers earn higher trust through successful task completion). Integrated with crabjar-guard's ExecutionGate patterns for unified provenance verification. 17 new tests added (auth, gate, task assignment, annealing, persistence). ADR-006 documents the security model design decisions.

2026-10-17 — Herdr-GPUI integration: CrabjarSurfaceAdapter built to translate conductor API responses to herdr protocol types; multi-agent dashboard UI with split panes and color-coded status indicators; 6 integration tests verify surface adapter behavior. Workspace: 31 members (+1 crabjar-gpui crate).

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
- `guard/src/authorization_order.rs` — monotonic authorization-order invariant tests (10; CodeWhale-informed)
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
- `crabjar-github-client/` — GitHub API client for skill search/install (Phase 8)
- `crabjar-clawhub-client/` — ClawHub.io registry API integration with trust tier mapping (Phase 9)
- `crabjar-skill-scanner/` — Security scanning pipeline + SHA-256 checksum verification (Phase 10)
- `crabjar-trust-manager/` — Trust tier management, version-aware skill cache, update detection (Phase 11)
- `crabjar-conductor/` — Fleet orchestration service with four-layer capability-based security model (Phases 1-4 complete), guard crate integration via gate.rs
- `axum-mux/` — vm-bridge (per-VM websocket relay, screen capture, terminal multiplexer)
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
| `prov-map-drift-2026-07-06` | project_map.md regenerated — 24 members (added file_search + crabjar-plugin), tree diagram updated, Core Components table populated, workspace members section refreshed, Section 9 Crabjar Context updated with new crates, Drift Report Last Audit and Known Items updated | 2026-07-06 | Structural alignment refresh | crabjar/project_map.md |
| `prov-map-drift-2026-07-08` | project_map.md regenerated — generated date updated, Section 3 Build & Test expanded (module-sizes targets, reproducible-build, test count), Section 2.3 workspace members includes specs/ ADR process, Section 2.4 shared deps consolidated to single-line format, Section 2.5 profiles renamed and simplified, guard/src file listing expanded with policy/context_budget/command_risk/risk_config/db_error files, host/host-agent updated with model_routing/context_compression/decision_gate, Section 9 structure updated (orchestrator prompt envelope, memory context fragments, E2E tests, snapshot baselines), Drift Report Last Audit and Known Items refreshed with current state | 2026-07-08 | Structural alignment refresh + ADR process addition | crabjar/project_map.md |
| `prov-map-drift-2026-07-08b` | project_map.md regenerated — Section 9.1 structure list expanded (orchestrator prompt envelope, memory context fragments, E2E tests, snapshot baselines), Section 9.2 test count updated to 691 passing with breakdown by crate, Drift Report Known Items expanded with policy/context_budget/command_risk/risk_config/db_error files and ADR process entry | 2026-07-08 | Structural alignment refresh — second pass on Section 9 + Known Items | crabjar/project_map.md |
| `prov-comparity-codewhale` | README + project_map parity credit: CodeWhale (monotonic authorization order, Fleet identity contract, deterministic scorers, roster-doesn't-execute) + guard/src/authorization_order.rs invariant tests (10) | 2026-08-30 | Feature parity analysis | crabjar/README.md, crabjar/project_map.md, Hmbown/CodeWhale docs (AUTHORIZATION_ORDER.md, FLEET.md) |
| `prov-remote-skill-discovery` | Remote skill discovery infrastructure implemented (Phases 8-11): crabjar-github-client, crabjar-clawhub-client, crabjar-skill-scanner, crabjar-trust-manager crates; CLI commands: tool search/install/clawhub-search/clawhub-install/scan/cache/updates; workspace expanded from 25 to 29 members | 2026-10-16 | Remote skill discovery completion | crabjar/crabjar-github-client, crabjar/crabjar-clawhub-client, crabjar/crabjar-skill-scanner, crabjar/crabjar-trust-manager, crabjar/src/tool_registry_cli.rs, crabjar/specs/SKILLS-REMOTE-DISCOVERY.md |
| `prov-conductor-security-model` | Conductor security model implemented (Phases 1-4): four-layer capability-based access control with SSH key auth, JWT-like tokens, task-level capability matching, and trust level annealing; integrated with crabjar-guard's ExecutionGate patterns; workspace expanded to 30 members; 17 new tests added | 2026-10-17 | Conductor security model completion | crabjar/crabjar-conductor/src/auth.rs, crabjar/crabjar-conductor/src/gate.rs, crabjar/specs/security-model-capability-based-access-control.md, crabjar/specs/ADR-006-conductor-security-guard-integration.md |
| `prov-herdr-gpui-integration` | Herdr-GPUI integration: CrabjarSurfaceAdapter built to translate conductor API responses to herdr protocol types; multi-agent dashboard UI with split panes and color-coded status indicators (Idle=CornflowerBlue, Working=LimeGreen, Blocked=Orange, Done=SteelBlue); 6 integration tests verify surface adapter behavior; workspace expanded to 31 members | 2026-10-17 | Herdr-GPUI integration completion | crabjar/crates/crabjar-gpui/src/surface_adapter.rs, crabjar/crates/crabjar-gpui/tests/herdr_integration.rs |
| `prov-map-drift-2026-11-02` | project_map.md regenerated — fresh structural audit: 77 workspace members (+46 since last documented audit on 2026-10-17). Major additions: analysis toolkit (8 analyze-* crates), VM/worker infrastructure (vm-core, conductor, worker, durable-objects), compression pipeline (crabjar-compress + MCP server), ML training data pipeline (4 train-* crates), git operations (3 crabjar-git/commit crates), orchestration/scheduling (scheduler, kanban-orchestrator, slow-friend), web tools (web-search, web-extract, browser), communication tools (email-smtp, skills), tooling/binding (tool-binding, analyze CLI, review, MCP tool registry), MCP servers (unsloth-mcp, herdr-mcp), structure analysis (structured-aux, universal-structure), infrastructure (ssh-bastion, deploy, memory, delegation, cron), apps/terrarium, 2 new skill crates. PR #3 merged (api-module-splitting: main.rs split into src/commands/). Version updated to 0.12.5. | 2026-11-02 | Structural alignment refresh — complete filesystem scan reveals significant growth beyond last documented state | crabjar/Cargo.toml, filesystem scan, crabjar/project_map.md |

---

*End of review.*