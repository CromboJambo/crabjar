# Crabjar Changelog

All notable changes to crabjar, organized chronologically. Completed work moves here from ROADMAP.md as it ships.

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
