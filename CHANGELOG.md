# Crabjar Changelog

All notable changes to crabjar, organized chronologically. Completed work moves here from ROADMAP.md as it ships.

## October 26, 2026 — Project Consolidation & Audit Infrastructure (Phases 1-4)

### Phase 1: Absorb agent-oversight into crabjar
- Archived `agent-oversight` repo to `~/projects/archive/` after analysis revealed Rust component was a stub ("Hello, world!") with meaningful work in world-model-optimizer Python suite
- Identified world-model-optimizer as reference material for future crabjar evaluation infrastructure

### Phase 2: Integrate mirror-lab audit module into crabjar-conductor
- Added `mirror-log` path dependency to crabjar-conductor/Cargo.toml (not published to crates.io)
- Created `crabjar-conductor/src/audit.rs` (161 lines) implementing ConductorAuditLog with Connection-based mirror-log API
- Audit event types: TaskCompleted, TaskFailed, WorkerPromoted, WorkerDemoted, GoalDecomposed, RouteDecision
- Fixed 7 pre-existing compilation errors in `universal-structure/src/adapters/lean.rs` (SpanKind enum variant mismatches, missing StructuralToken import)
- Verified conductor compiles successfully with audit module integrated

### Phase 3: Reorganize reference/early-stage repos
- Moved `hermes-source` to `~/projects/reference/` (kshitijk4poor's fork, learning source not active development)
- Moved `crepuscularity` to `~/projects/reference/` (language/runtime learning project)
- Moved `pesti-backup-old-checkout` to `~/projects/archive/` (superseded by active pesti with newer commits)

### Phase 4: Define clear project boundaries
- Updated crabjar/AGENTS.md with explicit scope statement and not-in-scope list
- Updated home-lab/AGENTS.md with explicit scope statement and not-in-scope list
- Created pesti/AGENTS.md with explicit scope statement and not-in-scope list
- Updated mirror-lab/AGENTS.md with explicit scope statement and not-in-scope list
- Created ~/projects/README.md documenting the purpose of each top-level directory

### New Specs Created
- `specs/evidence-based-promotion.md` — 4-phase promotion system using JEV evaluation, guard trust layers, and conductor routing decisions
- `specs/knowledge-compression-triad.md` — Formalized pattern: compressed knowledge + supporting evidence + detailed specialist record
- `specs/project-consolidation-simplification.md` — 3-phase consolidation plan with acceptance criteria

## October 16, 2026 — Remote Skill Discovery Complete (Phases 8-11) + Test Infrastructure

### Phase 11: Trust Management & Cache
- **crabjar-trust-manager crate** — Trust tier management, version-aware skill cache (~/.crabjar/skill-cache/), update detection
- CLI commands: `tool cache list`, `tool updates check`, `tool auth set`
- Automatic trust level assignment based on source (ClawHub official → high, community → medium)

### Phase 10: Checksum Verification
- **crabjar-skill-scanner crate** — Security scanning pipeline with SHA-256 integrity verification
- CLI command: `tool scan <name>` for security analysis of installed skills

### Phase 9: ClawHub Integration
- **crabjar-clawhub-client crate** — ClawHub.io registry API integration with trust tier mapping
- CLI commands: `tool clawhub-search`, `tool clawhub-install`

### Phase 8: GitHub Skill Discovery
- **crabjar-github-client crate** — GitHub API client for skill search/install via crabjar-skill tags
- CLI commands: `tool search`, `tool install`

### Test Infrastructure (New)
- **crabjar-test-registry crate** — Mock HTTP registry server for integration testing without network dependencies
- Supports both ClawHub and GitHub-compatible API shapes with scenario-based test data
- 8 passing integration tests validating discovery→install→scan→cache flow end-to-end

### Architecture
- All 4 remote discovery crates follow the glass abstraction boundary pattern
- Workspace expanded from 25 to 30 members
- Comprehensive test infrastructure spec at specs/TEST-INFRASTRUCTURE.md

## October 16, 2026 — Structural Refactoring (Phase 1: Remove Tape)

### Crate Consolidation
- **Removed `crabjar-mcp-client`** — Incomplete placeholder implementation (90 lines), unused by any other crate
- **Consolidated `crabjar-skill-example` into `crabjar-skills`** — Moved example skill manifest creation to `crabjar-skills/src/example.rs` module with tests
- **Consolidated `crabjar-skill-context` into `crabjar-skills`** — Moved SkillContext struct and methods to `crabjar-skills/src/context.rs` module with tests

### Workspace Changes
- Workspace reduced from 60 to 57 packages (after removing 3 crates)
- All existing tests pass (14 tests in crabjar-skills, including newly consolidated example and context tests)
- No documentation updates needed — removed crates were not referenced in README.md or project_map.md

### Next Steps
- Phase 2: Improve abstractions in `host/host-agent` (15 workspace imports → facade pattern)
- Phase 3: Clean up dependencies (fix xattr compilation error, remove unused deps)
- Phase 4: Overengineer critical paths (comprehensive error handling, improved test coverage)

## October 10, 2026 — Code Review Pipeline (crabjar-review)

### Full Pipeline Implementation (Phases 1-4 Complete)
- **Phase 1 (Deterministic Pre-processor):** File selection with type classification, smart bundling with source+test pairing and monorepo/crate boundary awareness, TOML-based rule matching engine
- **Phase 2 (Agent Review Units):** Scenario-tuned prompt generation, bounded context retrieval tools (file content, surrounding context, project structure), structured JSON output validation for agent findings
- **Phase 3 (Post-processor & GitHub Integration):** Position validation with ±2 line tolerance window, severity validation enforcing bug→critical/major rules, finding deduplication via hash-based matching, severity reflection (downgrading security issues in test files), complete GitHub API client for PR comments and summaries
- **Phase 4 (Benchmarking):** Benchmark runner evaluating post-processor against ground-truth findings to measure precision, recall, and F1 score, with CLI tool and example benchmark data
- **Conductor Integration:** Wired crabjar-review into crabjar-conductor's HTTP API as `POST /review` endpoint accepting repo URL, PR number, commit SHA, and diff content

### Architecture Highlights
- Deterministic pre-processing filters 95%+ of noise before LLM inference (type classification, generated/binary filtering)
- Smart bundling limits context to relevant files while preserving test coverage relationships
- Rule matching engine supports applies_to/not_applies_to file type constraints per rule
- Post-processor validates and deduplicates agent findings before submission to GitHub
- Benchmark suite measures precision/recall/F1 against ground-truth findings

### Testing
- 38 unit tests passing across all modules (file selection, bundling, rules, prompts, context retrieval, post-processing, benchmarking)
- Integration tests covering end-to-end review pipeline with mock agent output
- Benchmark integration tests validating TP/FP/FN tracking and deduplication effects on metrics

## October 10, 2026 — Conductor Deployment & Analyzer Toolkit

### crabjar-conductor Deployment to ftw3
- Deployed conductor as dinit service on ftw3:8091 with pesti-server orchestrator integration
- SQLite-backed persistent state store (goals/tasks/workers tables with FK constraints)
- HTTP API endpoints: POST /goals, GET /goals/{id}, GET /workers
- Orchestrator model called via pesti-server at http://localhost:8081 for goal decomposition and task assignment
- Deployment script `scripts/deploy-conductor-ftw3.sh` handles build, transfer, service config, verification

### Reverse Engineering Analyzer Toolkit (7 analyzers)
- **analyze-har**: HAR file parsing with credential detection, timing analysis, endpoint classification
- **analyze-js**: JavaScript dependency graph, API call extraction, obfuscation detection, minification ratio
- **analyze-native**: ELF/PE/Mach-O binary parsing with section analysis, symbol inspection, entropy scoring
- **analyze-dotnet**: .NET assembly IL bytecode inspection with method body analysis
- **analyze-firmware**: Firmware image format detection and section parsing
- **analyze-apk**: Android APK manifest parsing, permission extraction, DEX inspection
- **analyze-common**: Shared utilities (entropy calculation, base64/hex helpers)

### MCP Server Integration
- All analyzers exposed via `crabjar-analyze` CLI tool and `analyze-mcp-server` for remote access

### Model Provider Update
- Integrated 27B post-trained model as default provider: DavidAU/Qwen3.8-27B-TURBO-Fable-Cold-Fusion-735-882-Heretic-Uncensored-NEO-CODER-MAX-MTP-GGUF
- Model heavily influenced by hermes-agent and unsloth training methodologies

### Developer Tooling Note
- Development workflow relies on hermes-agent, unsloth, and llama.cpp as developer tools (not crabjar dependencies)
- pesti uses llama.cpp for conformance testing but not as a core runner dependency

### crabjar-conductor Crate (Fleet Coordination Layer)
- Scaffolded `crabjar-conductor` crate implementing the conductor pattern for cross-machine agent coordination
- Core types: Goal, Task, Worker with status enums and relationships
- State store abstraction layer (`state_store.rs`) for goals, tasks, workers — in-memory HashMap implementation with SQLite persistence planned
- Worker registry (`worker_registry.rs`) with capability-based lookup (register, get, list, find_by_capability, find_available)
- Task scheduler (`task_scheduler.rs`) with schedule() method for capability-based task-to-worker assignment
- HTTP API layer (`api.rs`) using axum: POST /goals, GET /goals/{id}, GET /workers endpoints
- Conductor core logic (`conductor.rs`) with ConductorConfig and submit_goal() workflow
- Comprehensive spec at `specs/conductor-crate.md` covering 3-layer architecture (local → fleet → cross-environment)

### SQLite Persistent State Store
- Implemented durable SQLite-backed state store replacing in-memory HashMap implementation
- Schema: goals, tasks, workers tables with foreign key constraints and proper indexing
- Row deserialization helpers with DateTime parsing and serde_json for dependencies/capabilities fields
- Persistent Connection architecture (not per-method reopen) to avoid :memory: isolation issues
- CLI entry point with clap (--addr, --port, --pesti-url) and tracing initialization
- 8 passing tests covering goal/task/worker lifecycle, persistence across instances, FK constraints

### vm-bridge Deployment to Home-Lab
- Deployed vm-bridge to ftw3 home-lab node as dinit-managed service on port 8090
- Created deployment script `scripts/deploy-vm-bridge-ftw3.sh` following pesti-server pattern
- Service configuration: manifest.toml with listen_addr, listen_port, libvirt_uri, default_timeout_secs
- Fixed working directory issue in dinit service command for manifest.toml loading

### Orchestrator Fine-Tuning Pipeline Documentation
- Documented QLoRA fine-tuning workflow at `docs/training/orchestrator-fine-tuning.md`
- Moved training scripts to `scripts/training/` (generate, train, evaluate, baseline eval)
- Baseline evaluation shows 100% task decomposition, 0% skill boundary recognition, 0% dependency understanding

## August 23, 2026 — Spatial Habitat & Herdr Integration

- **Spatial Habitat ADR (ADR-003)** — Defined crabjar's persistent spatial habitat model: a representation of the user's lived environment over which computational state (agents, artifacts, pending guard actions) is laid out. Physical truth source remains Home Assistant; crabjar owns the virtual world. Divergence is surfaced to humans, never auto-corrected.
- **Herdr Backend Spike** — Third `TerminalBackend` implementation (`crates/terminal/src/herdr.rs`) using Herdr's workspace/pane model. Maps crabjar session lifecycle to Herdr workspaces, with agent status integration for guard approval flow.
- **ADR-002: Herdr as Execution Substrate** — Formalized the crabjar/Herdr split: Herdr is the execution substrate (flywheel/mechanism), crabjar is the trust layer (clutch). Integration via client/orchestrator pattern; trust boundary stays on crabjar side.
- **Finish Task Feature** — Agents can now mark tasks as complete with structured handoffs including summary, metadata (changed_files, tests_run, decisions), and artifact paths. Supports dependency chains where child tasks auto-promote when parents complete. Review workflow added: agents can request human review before marking done, with reviewer feedback loop for required changes.

## July 10, 2026 — Pre-Release Hardening & Agent Loop Expansion

### Conversational TUI
- Implemented ratatui-based interactive chat interface in `host-binary/tui/` with session persistence, message history, scrollback, and guard approval display
- Added terminal panel (`tui/terminal_panel.rs`) wrapping crabjar-terminal for live terminal view within TUI
- SQLite-backed session store (`tui/session.rs`) for create/load/save/list operations

### Agent Loop (ReAct) Expansion
- **Model routing** (`model_routing.rs`) — Phase-aware routing with `LoopPhase` enum, per-phase backend configuration, default plan/reflect → HTTP, others → heuristic
- **Context compression** (`context_compression.rs`) — Token-budgeted context management with grouping of older observations by stage/kind
- **Decision gate** (`decision_gate.rs`) — ToolCall/RespondDirectly/Defer decision logic with auto-decide threshold and max tool calls per turn
- 43 new tests covering model routing, context compression, decision gate, and loop integration

### Guard Crate Expansion
- **Static policy engine** (`policy.rs`, `policy_types.rs`) — TOML-based declarative policies with configurable checks for dangerous commands, confidence floors, trust layer minimums, scope isolation toggles, domain allowlist modes, context budgets (17 tests)
- **Context budgeting** (`context_budget.rs`) — ContextBudget + MAX_TOKENS_PER_FRAGMENT enforcement in ExecutionGate step 10 (6 tests)
- **Command risk catalog** (`command_risk.rs`, `risk_config.rs`) — HIGH/MEDIUM_RISK_COMMANDS classification (6 tests)

### Pre-Release Hardening
- Removed all personal paths, secrets, offensive language, and TODO stubs
- Replaced fake return values with proper error types
- Verified zero clippy errors across workspace
- Confirmed release build succeeds without errors
- Verified no secrets in binary via `strings` analysis

### Documentation & Process
- Established ADR process in `specs/` with Nygard-style templates, registry index, and ADR-001 establishing the decision process itself
- Updated project_map.md and ROADMAP.md to reflect current state (July 10)

## July 8, 2026 — Memory & Host-Agent Refactoring

### Memory Crate Split
- Split `indexer.rs` (705 LoC) into `extract.rs` (~400 LoC: markdown parsing) and `insert.rs` (~144 LoC: SQLite writes)
- Fixed pre-existing schema/insert column mismatches (`doc_metadata` vs `documents`, `doc_id` vs `doc_path`)

### Host-Agent ReAct Loop Expansion
- Added `model_routing.rs` (435 LoC) — ModelRouter with phase-specific backend routing
- Added `context_compression.rs` (388 LoC) — ContextCompressor with token budget enforcement
- Added `decision_gate.rs` (365 LoC) — DecisionGate for tool call vs direct response decisions

## July 4–5, 2026 — Terminal Integration & Search

### crabjar-terminal: Terminal Multiplexer Integration
- Wezterm mux backend (`wezterm.rs`, ~170 LoC) via `wezterm cli` protocol — spawn, send-text, get-text, split-pane, kill-window
- Zellij action protocol backend (`zellij.rs`, ~165 LoC) — server start, write-chars, dump-screen, detach lifecycle
- Asciinema v2 recording (`recording.rs`, ~180 LoC) with header writing and timestamped event buffering
- TerminalBackend trait with auto-detection (wezterm > zellij)
- TerminalManager for multi-session tracking

### E2E Test Suite Expansion
- **Smoke slice**: 6 tests (~0.3s) — CLI binary, workspace status, guard DB init, tool registry, knowledge store, doctor check
- **Full slice**: 26 tests covering exec pipeline, domain allowlist, scope isolation, telemetry flight recorder, agent loop persistence, tool discovery, guard subcommands, knowledge lifecycle

### File Search Engine
- BM25-based file indexing using Tantivy 0.22 (`file_search/`)
- Incremental indexing with path-aware document generation
- Keyword, fuzzy, and path-based search with relevance scoring
- 6 passing tests covering tokenization, indexing, search relevance, fuzzy matching, path filtering

### crabjar-plugin Crate
- Scaffolded WASM plugin runtime crate in `crabjar-plugin/` (stub, deferred pending concrete use case)

## July 6, 2026 — Test Fixes & Guard Approval Flow

### Pre-existing Test Failures Resolved
Fixed all 8 failing cli.rs tests:
- CLI wraps responses in `{"success": true, "data": {...}}` but tests read from top level (7 tests)
- State-doc indexer stored empty doc_name when frontmatter lacked `name:` field
- Files changed: `tests/cli.rs`, `memory/src/state_docs/indexer.rs`, `src/main.rs`, `crates/terminal/src/lib.rs`

### TUI Guard Approval Flow
- Keyboard shortcuts ('a'/'r') for approve/reject pending guard actions in TUI
- `resolve_pending_queue_entry()` DB method with approve (remove from queue) and reject (move to interrupted log) paths
- Updated status bar with actionable prompt when guard is pending
- 3 new unit tests added

## July 9–10, 2026 — Scope Injection & CI Integration

### Scope Injection Across All Execution Paths
CrossScopeAuth now wired into CLI, orchestrator, host-agent, and TUI:
- Orchestrator's `execute_with_guard()` + all 5 command handlers (run_command, search_logs, recent_events, by_source)
- Host-agent `TaskExecutor` via `.with_scope()` from AgentLoop
- TUI app loop with `"tui"` scope
- Dashboard F(1) handler with `"host"` scope

### CI Integration Status
- E2E test slices defined but not yet wired into GitHub Actions workflows (smoke on PR, full on merge/nightly)

---

## Earlier History (Pre-July 2026)

For earlier completed work, see the git log and historical ROADMAP.md entries. Notable completions include:

- **Prompt Envelope Defense** — Instruction-hijack detection with closed-vocabulary source labels, SHA-256 provenance hash, 40+ tests
- **Product Adapter Pattern** — `ProductAdapter` trait + `AdapterRegistry` in host-core for channel normalization
- **Mechanical Dependency Boundary Enforcement** — `crabjar-architecture` crate with 8-layer model and CI gate
- **Scope Isolation Model** — `Scope` type with identity/project/tenant/thread dimensions, CrossScopeAuth approval flow
- **Requested-vs-Effective Trust Resolution** — Policy chain: scope → project policy → user policy → default, with audit trail
- **Exact-Invocation Fingerprint Approvals** — InvocationFingerprint + SHA-256 computation with ApprovalLease TTL
- **Domain Allowlisting** — Deny-by-default allowlist with trust levels and per-trust-layer enforcement
- **Action Policy Engine** — Destructive actions require user permission via concierge.rs
- **Module Size Governance** — 500 LoC rule enforced across all crates via pre-commit hook and CI gate
- **Build Reproducibility** — `just reproducible-build` with locked deps and deterministic flags
- **Snapshot Testing (insta)** — CLI JSON output format tests and TUI message serialization baselines
- **Dual-Backend Persistence Decision** — Stick with SQLite until real PostgreSQL need appears
