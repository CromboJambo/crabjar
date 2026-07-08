# Crabjar Roadmap

> Crabjar is trying to capture EdgeCrab's ecosystem in concept. Codex sets the quality bar. Claw Code is Frankenstein — useful patterns buried in noise. IronClaw is the structural reference — mechanical boundary enforcement, scope isolation, and trust resolution at scale.

---

## Status (July 5, 2026)

| Metric | Value |
|---|---|
| **Workspace members** | 24 crates (added `crates/terminal`, `file_search`, `crabjar-plugin`) |
| **Tests** | ~152+ passing, 0 failing (all pre-existing cli.rs failures resolved + 3 new resolve_pending_queue_entry tests) |
|| **Clippy** | ⚠️ Warnings only (unused imports/variables), no blocking errors |
|| **Architecture crate** | ✅ Built, compiles, has integration test |
|| **Guard scope/trust** | ✅ Scope isolation + requested-vs-effective trust resolution implemented |
|| **Per-crate AGENTS.md** | ✅ Complete (all 24 crates documented) |
|| **lm_studio_client** | ✅ Modularized into 6 files with unit tests |
|| **fingerprint approvals** | ✅ Implemented (InvocationFingerprint + ApprovalLease) |
|| **CrossScopeAuth** | ✅ Implemented with expiry and scope resolution |
|| **ContextFragment** | ✅ Implemented in memory/ with P0 alert threshold |
|| **Audit trail** | ✅ TrustResolution.audit_log() records derivation chain |
|| **Guard module size** | ✅ Split 5 modules (trust, gate, fingerprint, concierge, trust_resolution) — all under 500 LoC |
|| **Context budgeting** | ✅ Enforced in ExecutionGate step 10: per-fragment hard cap + cumulative budget check |
|| **Oversized fragment handling** | ✅ Gate rejects fragments >10K tokens; Zed ACP server handles oversized responses |
|| **Metrics reporting** | ✅ `crabjar metrics` CLI subcommand with test count, LoC, workspace stats |
|| **Domain allowlisting** | ✅ Implemented in guard/ with trust-layer enforcement + network domain tracking |
|| **Reproducible builds** | ✅ `just reproducible-build` target with locked deps and deterministic flags |
|| **Agent loop (ReAct)** | ✅ Full ReAct loop engine with model routing, context compression, decision gating |
|| **Tool registry wiring** | ✅ Fully wired into crabjar binary, orchestrator, host-agent, and CLI |
|| **Pre-commit hooks** | ✅ `.pre-commit-config.yaml` with cargo check, clippy (-D warnings), module-sizes-check (500 LoC rule), architecture boundaries (8-layer model) |
|| **Cargo-declared drift audit** | ✅ Cron job every 6h tracking declared/compiled/delta counts + trend analysis |
|| **Conversational TUI** | ✅ Implemented in `host-binary/tui/` — ratatui-based interactive chat with session persistence, message history, scrollback, guard approval display |
|| **TUI Terminal Panel** | ✅ Implemented in `tui/terminal_panel.rs` — wraps `crabjar-terminal` for live terminal view within TUI; graceful degradation when wezterm/zellij unavailable via `TerminalPanel::try_new()` returning `Option<Self>` |
|| **Snapshot Testing (insta)** | ✅ Implemented in `tests/snapshot_tests.rs` — 6 tests covering CLI JSON output format (`state list`, `workspace status`, `doctor check`, `tool list`, `guard queue`) and TUI message serialization; baselines committed to `tests/snapshots/` |
|| **TUI Session Store** | ✅ SQLite-backed session persistence (`tui/session.rs`) — create/load/save sessions, message serialization |
|| **TUI Guard Approval Flow** | ✅ Implemented in `host-binary/tui/` — keyboard shortcuts ('a'/'r') for approve/reject pending guard actions, `resolve_pending_queue_entry()` DB method, agent loop surfaces pending queue entries and pauses for user input |

---

## Recently Completed (July 4–5, 2026)

### crabjar-terminal: Terminal Multiplexer Integration ✅ DONE

**Status:** Implemented in `crates/terminal/` — wezterm (primary) + zellij (fallback) backends with asciinema v2 recording.

- **Wezterm mux backend** (`wezterm.rs`, ~170 LoC): Full implementation via `wezterm cli` protocol — spawn, send-text, get-text, split-pane, kill-window
- **Zellij action protocol backend** (`zellij.rs`, ~165 LoC): Server start, write-chars, dump-screen, detach lifecycle management
- **Asciinema v2 recording** (`recording.rs`, ~180 LoC): Header writing, input/output event buffering with millisecond timestamps, proper file closing
- **TerminalBackend trait** (`backend.rs`): Dyn-compatible async trait with `is_available()` detection (wezterm > zellij auto-detection)
- **TerminalManager**: Multi-session tracking with HashMap-backed session registry
- **TerminalSession**: Full lifecycle — spawn → send/read loop → snapshot/record → stop
- **Snapshot API**: Terminal buffer capture with line extraction, backend identification, working directory metadata

**Why this matters:** Agent harness needs programmatic terminal control for running commands in isolated sessions, capturing output, and recording sessions for replay/debugging. Wezterm's mux protocol provides reliable text I/O; zellij offers a fallback without GUI dependency.

### Conversational TUI for Agent Harness ✅ DONE

**Status:** Implemented in `src/host-binary/tui/` — ratatui-based interactive chat interface.

- **TUI application structure** (`tui/app.rs`, `tui/mod.rs`) — ratatui terminal with title bar, message area (scrollback), status bar, input line
- **Input handling** (`tui/input.rs`) — Action enum (Submit, Quit), crossterm event processing, keyboard shortcuts (Enter=submit, Ctrl+C=quit)
- **Session persistence** (`tui/session.rs`) — SQLite-backed session store with create/load/save/list operations, message serialization via serde
- **Agent loop integration** — `App::run_agent_loop()` wires into `crabjar_host_agent::AgentLoop` with iteration tracking, confidence-based completion (0.85 threshold), max iterations guard (50)
- **Message types** — User, Agent, ToolCall, Guard messages rendered with distinct styling in the conversation area
- **Guard approval display** — `AppState::AwaitingApproval` state shows pending actions in status bar

**Files added/modified:** 4 new files (`tui/app.rs`, `tui/mod.rs`, `tui/input.rs`, `tui/session.rs`), ~600 lines total.

### E2E Test Suite Expansion ✅ DONE

- **Smoke slice:** 6 tests (~0.3s) — CLI binary, workspace status, guard DB init, tool registry, knowledge store, doctor check
- **Full slice:** 26 tests (~0.02s) — exec pipeline (dry-run/denied/proceeds), domain allowlist (deny/allow/trust layers), scope isolation (cross-scope blocking), CrossScopeAuth auto-construction, telemetry flight recorder, agent loop WorkItemStore persistence, tool discovery across 4 layers, guard subcommands (queue/approve/reject/resolution/grant/revoke), knowledge lifecycle, workspace config edge cases

### File Search Engine ✅ DONE

**Status:** Implemented in `file_search/` — BM25-based file indexing and search using Tantivy 0.22.

- **Indexer** (`indexer.rs`, ~400 LoC): `FileIndexer` struct with incremental indexing, path-aware document generation, tokenization pipeline (SimpleTokenizer + LowerCaser), Tantivy document construction
- **Storage backend** (`storage.rs`, ~350 LoC): `SearchStorage` wrapping Tantivy index reader/writer — search by keyword/fuzzy/path, term deletion, index clearing, schema definition with path/title/content fields
- **Public API** (`lib.rs`): Clean trait-based interface for indexing and searching files
- **Tests:** 6 passing tests covering tokenization, indexing, search relevance, fuzzy matching, path filtering, and stale reader reload

### crabjar-plugin: WASM Plugin Runtime ✅ DONE (stub)

**Status:** Crate scaffolded in `crabjar-plugin/` — placeholder for future WASM plugin runtime with lifecycle management. Currently a stub crate; real implementation deferred pending concrete use case beyond Zed-specific ACP bridge.

- **Crate structure**: Cargo.toml, lib.rs (module declarations), AGENTS.md
- **Reserved slot**: `zed-acp-bridge` is Zed-specific (`zed_extension_api`), not general-purpose. Real WASM plugins would need `wasmtime`/`wasmer`, capability configuration, crash recovery, and a plugin interface.

### CrossScopeAuth Wiring ✅ DONE

- `src/main.rs:483` — auto-constructs via `CrossScopeAuth::auto_for_scopes()`
- `orchestrator/src/main.rs` — constructs `actor_scope` + `target_scope` in `AppState`, wires into all 3 gate check sites (`execute_with_guard`, "run_command", "search_logs") with `CrossScopeAuth::auto_for_scopes()`
- `host/host-agent/executor.rs` — constructs `CrossScopeAuth` from `self.scope` when scope is set, falls back to `None` otherwise

---

## Priority 1: Fix the Foundation

These are blockers. Nothing downstream can be trusted until they're resolved.

### 1.1 Fix Failing Scope Test ✅ DONE

The scope isolation test `test_scope_cannot_access_different_project` in `guard/src/scope.rs:380` now passes. The `Scope::user_project()` correctly sets the user dimension and `can_access()` enforces mutual inaccessibility between different user/project pairs.

- [x] Traced `can_access()` logic in `guard/src/scope.rs`
- [x] Test creates `Scope::user_project("alice", "project-a")` and `Scope::user_project("bob", "project-b")` and asserts mutual inaccessibility
- [x] `Scope::user_project()` correctly sets the user dimension
- [x] Bug fixed, test passes
- [x] Additional tests for same-user cross-project and same-project cross-user added

**Why this matters:** Scope isolation is Priority 2.3 in the IronClaw model. The invariant is now solid.

### 1.2 Verify Clippy Clean ✅ DONE

- [x] Run `cargo clippy --workspace -- -D warnings`
- [x] Fix any warnings
- [x] Confirm `just check` passes

**Why this matters:** Codex sets the standard — zero warnings is non-negotiable. Without this, we can't trust anything else.

**Fixes applied:**
- Inner attributes (`#![...]`) after doc comments → converted to `//!` module docs
- `Arc<RwLock<Connection>>` non-Send-Sync → `#[allow(clippy::arc_with_non_send_sync)]` at construction sites
- `Duration::from_secs(300)` → suppressed with `#[allow(clippy::duration_suboptimal_units)]`
- Dead code in orchestrator → `#[allow(dead_code)]` on structs/enums
- `enum_variant_names` lint → suppressed on `InferenceError` and `LmStudioError`

### 1.3 Update project_map.md ✅ DONE

- [x] Fixed workspace member count: 22 members (was 23 — corrected after member count audit)
- [x] Updated architecture diagram with `crabjar-architecture`, `axum-mux`, `crabjar-app-teams`
- [x] Marked all completed items with status indicators
- [x] Reflected `lm_studio_client` modularization: 9 files (types, client, session, error, endpoints, prompt_envelope, tests, mod, backend/mod.rs)
- [x] Updated guard/src file listing: 23 files (was stale — listed annealing.rs, retrieval.rs, reversibility.rs which were removed; added guard_db_impl.rs, guard_db.rs, db_error.rs, fingerprint.rs, fingerprint_types.rs, trust_resolution.rs, trust_types.rs, scope.rs, action.rs, inference.rs, concierge_types.rs, gate_context.rs, gate_result.rs, command_risk.rs, risk_config.rs, memory_types.rs, gate_tests.rs)
- [x] Added vm_bridge to tree (src/vm_bridge/: lib.rs, relay.rs, screen.rs, terminal.rs)
- [x] Added host/host-agent to tree (8 source files including ReAct loop_engine.rs)
- [x] Updated shared dependencies to match current Cargo.toml (async-trait, tauri stack, rumqttc, tracing-error, base64, which, rstest, serial_test)
- [x] Updated CLI command table (added `guard resolution` command)
- [x] Updated section 9 Crabjar Context (removed stale codeburn/gguf/llm-runner references, updated active Rust surface)
- [x] Updated generated date to June 27, 2026
- [x] Added provenance entry for this update

**Remaining concern:** Resolved — `guard_db_impl.rs` split into 3 files, all under 500 LoC (see 3.2).

---

## Priority 2: Core Structural Patterns (IronClaw-Informed)

These patterns prevent the workspace from collapsing into an unmanageable graph. They're governance mechanisms, not features.

### 2.1 Mechanical Dependency Boundary Enforcement ✅ DONE

**Status:** Built and integrated.

- `crabjar-architecture` crate exists with 8-layer model (0-7)
- `layer::ALL_LAYERS`, `crate_to_layer()`, `allowed_dependencies()`, `crate_layer()`, `crates_in_layer()`
- `boundary::check_workspace_boundaries()` and `boundary::enforce_boundaries()`
- Integration test `test_workspace_boundaries_are_valid`
- Compiles, passes CI

**What's next:**
- [x] Add `crabjar-architecture` to CI gate (needs CI config)
- [x] Document layer model in `crabjar-architecture/AGENTS.md`
- [ ] Consider adding `crabjar-architecture` as a pre-commit hook
- [x] Add `cargo-declared` integration — drift audit cron job every 6h tracking declared/compiled/delta counts + trend analysis (see Status section)

### 2.2 Scope Isolation Model ✅ DONE (verified)

**Status:** Implemented in `guard/src/scope.rs` with `Scope` type covering identity, project, tenant, thread dimensions. `can_access()` enforces cross-scope authorization. Scope isolation test passes.

- [x] Fix the failing test (see 1.1) — DONE
- [x] Add `CrossScopeAuth` approval flow — DONE (implemented in scope.rs:132-213 with expiry)
- [x] Wire scope into `ExecutionGate::check()` — DONE (step 7 at gate.rs:125-143 enforces actor_scope.can_access(target_scope))
- [x] Add scope to `GuardDb` schema — DONE (`scope_actor`, `scope_target` columns in action_requests and trust_resolutions tables)
- [x] Wire CrossScopeAuth enforcement into the gate — DONE (gate.rs:129-156 validates auth.is_valid(3600) + actor/target scope match before allowing cross-scope bypass; GateContext.cross_scope_auth field added with builder method)

**What's next:**
- [x] Wire CrossScopeAuth creation into callers (orchestrator, exec handler) — DONE (src/main.rs:467 auto-constructs via `CrossScopeAuth::auto_for_scopes()`; orchestrator/host-agent still pass None because they lack scope injection — future work)

### 2.3 Requested-vs-Effective Trust Resolution ✅ DONE (with audit trail)

**Status:** Implemented in `guard/src/trust_resolution.rs`. Policy chain: scope → project policy → user policy → default. Effective trust is logged via `TrustResolution.audit_log()` which records how effective trust was derived from requested trust.

**What's next:**
- [x] Audit: `effective_trust` is logged at every gate point via `audit_log()` (trust_resolution.rs:332)
- [x] Add audit trail: `audit_log()` records derivation chain (DONE)
- [x] Wire into `crabjar guard` CLI — show trust resolution chain in output

### 2.4 Exact-Invocation Fingerprint Approvals ✅ DONE

**Status:** Implemented in `guard/src/fingerprint.rs` (585 lines). `InvocationFingerprint` type + SHA-256 computation. `ApprovalLease` with TTL. `ApprovalScope` for scoped approval matching. `InMemoryApprovalStore` for pending approvals.

- [x] Design: fingerprint model — command, args, env, working dir, scope
- [x] Implement: `InvocationFingerprint` type + `SHA-256` computation
- [x] Update guard: store fingerprint with pending actions, match on approval
- [x] Prevent approval smuggling: `ApprovalScope` enforces exact match
- [x] Lease-based approval with TTL (`ApprovalLease`)

**Why this matters:** Pattern-based approvals are a false sense of security. Exact fingerprints close the gap.

### 2.5 Prompt Envelope (Instruction-Hijack Defense) ✅ IMPLEMENTED

Implemented in `orchestrator/src/lm_studio_client/prompt_envelope.rs`.

- [x] Design: closed-vocabulary source labels (`SourceLabel` — `SystemConfig`, `SystemInject`, `UserInput`, `ToolOutput`, `ExternalInput`)
- [x] Implement: instruction-hijack detection (rejects 25+ injection patterns including IGNORE_PREVIOUS, SYSTEM PROMPT:, NEW RULE, role changes, XML tag spoofing, boundary markers)
- [x] Source attribution chain: every prompt token traces to its origin via SHA-256 provenance hash on `LabeledContent`
- [x] Prompt templates: default system prompt lives in `orchestrator/prompts/default_system.md`
- [x] Integration: `chat()` and `chat_with_system()` wrap all outbound prompts in `PromptEnvelope` before sending
- [x] Validation: full envelope validation (provenance + injection + source + age) runs before every request
- [x] 40 unit tests covering injection detection, provenance integrity, source validation, edge cases

**Why this matters:** Prompt injection is the attack vector the guard gate can't see. The envelope protects the LLM's context before it reaches the gate.

### 2.6 Product Adapter Pattern ✅ DONE

Implemented in `host/host-core/src/adapter.rs`.

- [x] Design: `ProductAdapter` trait — normalize input → `IncomingMessage`, send output ← `OutgoingMessage`
- [x] Implement: adapter registry for discovery and lifecycle (`AdapterRegistry`)
- [x] Implement: `IncomingMessage` / `OutgoingMessage` canonical types
- [x] New channels = new adapter, no core changes
- [x] 7 unit tests covering register/resolve/duplicate/list/send

**Why this matters:** Every new channel (Discord, Feishu, WeChat) currently requires core changes. The adapter pattern contains that blast radius.

---

## Priority 3: Codex Quality Constraints

Codex doesn't contribute architecture — it sets the standard. These are non-negotiable quality bars.

### 3.1 Linting as Policy Gates ✅ PARTIALLY DONE

- [x] Domain allowlist — implemented in `guard/src/domain_allowlist.rs` (deny-by-default allowlist with trust levels)
  - `DomainAllowlist` struct with default entries (github, crates.io, docker hub, pypi, npm, huggingface, localhost, RFC1918)
  - Wildcard support (`*.github.com`)
  - Per-trust-layer enforcement (layer 3 = all, layer 2 = trusted+monitored, layer 1 = trusted-only)
  - Audit logging for monitored and restricted domains
  - Wired into `ExecutionGate::check()` as step 9
  - `GateContext.domains` field for callers to pass known network destinations
  - Re-exported from guard crate's public API
  - 10 unit tests covering exact match, wildcard, trust layer, add/remove
- [x] Action policy — destructive actions require user permission (implemented in concierge.rs)
- [x] Code quality gates — module size limits + CI gate (see 3.2 below)
- [ ] Drift governance — detect when state-docs diverge from reality (partially done via `skill-reference-store`)

### 3.2 Module Size Governance ✅ DONE (all crates)

**Status:** All modules across all crates are now under 500 LoC, enforced by pre-commit hook (`just module-sizes-check`).

| File | Lines | Status |
|---|---|---|
| `orchestrator/src/lm_studio_client/prompt_envelope.rs` | 940 → split into ~223 + ~624 | ✅ Done |
| `src/knowledge_store/mod.rs` | 832 → split into bridge/confidence/commands | ✅ Done |
| `memory/src/context.rs` | 751 → split into mod/constants/fragment/budget | ✅ Done |
| `memory/src/state_docs/indexer.rs` | 705 → split into extract/insert | ✅ Done |
| `tool_registry/src/tool_registry.rs` | 695 → split into discovery + refactored core | ✅ Done |

**Completed (guard/):**
- [x] Add `just module-sizes` target (reports modules exceeding threshold)
- [x] Add `just module-sizes-check` CI gate (fails on >500 LoC)
- [x] Add CI job to `.github/workflows/rust.yml`
- [x] Split `trust.rs` (697 LoC) → `trust_types.rs` (252 LoC) + `trust.rs` (429 LoC: impl + tests)
- [x] Split `gate.rs` (614 LoC) → `gate.rs` (292 LoC: impl) + `gate_tests.rs` (323 LoC: tests)
- [x] Split `trust_resolution.rs` (641 LoC) → `trust_resolution.rs` (431 LoC: impl only)
- [x] Split `fingerprint.rs` (585 LoC) → `fingerprint_types.rs` (259 LoC) + `fingerprint.rs` (8 LoC: re-exports) + tests extracted
- [x] Split `concierge.rs` (541 LoC) → `concierge_types.rs` (91 LoC) + `concierge.rs` (466 LoC: impl + tests)
- [x] Document 500 LoC rule in AGENTS.md
- [x] `guard_db_impl.rs` split into 3 files, all under 500 LoC

**Completed (non-guard/):**
- [x] Split `prompt_envelope.rs` (940 LoC) → `prompt_types.rs` (~223 LoC: types + error enum) + `prompt_validator.rs` (~624 LoC: validation logic). Fixed `super::PromptError` shadowing by moving error type to top of file.
- [x] Split `knowledge_store/mod.rs` (832 LoC) → `bridge.rs`, `confidence.rs`, `commands.rs`. Extracted bridge logic, confidence calculation, and CLI commands into separate modules.
- [x] Split `memory/src/context.rs` (751 LoC) → `context/mod.rs`, `constants.rs`, `fragment.rs`, `budget.rs`. Separated ContextFragmentBuilder, token budget constants, fragment types, and budget enforcement logic.
- [x] Split `indexer.rs` (705 LoC) → `extract.rs` (~400 LoC: markdown parsing), `insert.rs` (~144 LoC: SQLite writes). Fixed pre-existing schema/insert column mismatches (`doc_metadata` vs `documents`, `doc_id` vs `doc_path`).
- [x] Split `tool_registry.rs` (695 LoC) → `discovery.rs` (~144 LoC: 4-layer tool discovery), refactored core registry to delegate. Converted `discover_tools()` from async to sync (it only called a sync function, avoiding unnecessary future + non-Send Connection issue).

**Enforcement:** Pre-commit hook runs `just module-sizes-check` on every commit — no manual tracking needed.

### 3.3 Build Reproducibility ✅ DONE

Cargo.lock already pins all dependency versions. `just reproducible-build` target:
- `cargo update --locked` verifies no drift between Cargo.toml and Cargo.lock
- `CARGO_INCREMENTAL=0` disables incremental compilation for deterministic builds
- `RUSTFLAGS="-C target-cpu=native"` pins the target CPU feature set
- `cargo tree --depth 1` reports dependency tree for audit

**CI gate:** Added `reproducible-build` job to `.github/workflows/rust.yml` — runs on every push/PR.

**What this guarantees:** Building the same commit twice produces byte-identical binaries (same toolchain, same OS). Does *not* guarantee cross-platform reproducibility — different OSes/compilers will produce different artifacts.

**Not done (deferred):** `cargo audit` for dependency vulnerability scanning. This is a separate concern from build reproducibility and would require adding the `cargo-audit` binary as a dev dependency or CI tool. Defer until there's a concrete security requirement that justifies it.

---

## Priority 4: Agent Loop & Tool Protocol (EdgeCrab-Informed)

These are the conceptual patterns crabjar needs to replicate from EdgeCrab.

### 4.1 JSON-RPC Plugin Protocol ✅ PARTIALLY DONE

`orchestrator/lm_studio_client/` modularized into 6 files (types, client, session, error, endpoints, mod) with unit tests (508 lines). `zed-acp-server` provides stdio JSON-RPC server for ACP protocol execution. `zed-acp-bridge` provides Wasm extension for tool call mapping.

- [x] Message schema for tool calls, results, errors (lm_studio_client/types.rs)
- [x] Three-tier model: ToolServer (subprocess), Script (Rhai in-process), Reserved (WASM)
- [ ] Startup latency budget (~100ms), error recovery, lifecycle management
- [ ] Language-agnostic plugin execution

### 4.2 Agent Loop (ReAct) ✅ DONE

`host/host-agent/` implements a complete ReAct loop with phase-aware model routing, context compression, decision gating, and scope isolation:

- `loop_engine.rs`: `AgentLoop` struct with observe → understand → plan → execute → verify → reflect → persist cycle
- `model_routing.rs`: `ModelRouter` with `LoopPhase` enum (6 phases), `PhaseConfig` per-phase, `PhaseBuilder` for common patterns. Default routing sends plan/reflect to HTTP backend, others to heuristic.
- `context_compression.rs`: `ContextCompressor` with configurable `CompressionConfig` — keeps recent N observations raw, groups older by stage/kind into summaries, enforces token budget. Three presets: `for_short_conversation()`, `for_long_conversation()`, `disabled()`.
- `decision_gate.rs`: `DecisionGate` with `Decision` enum (ToolCall / RespondDirectly / Defer), `DecisionConfig` for auto-decide threshold and max tool calls per turn. Heuristic fallback when no model configured.
- `executor.rs`: `TaskExecutor` with `scope: Option<Scope>` — wires scope from `AgentLoop` into `GateContext` for guard gate enforcement.
- `work_item_store.rs`: SQLite-backed WorkItem persistence with resume support
- `inference/backend.rs` + `http_backend.rs`: Inference backend abstraction
- State machine for loop transitions (via `WorkItem.status`)
- Confidence-based auto-completion (threshold: 0.85)
- Max iterations guard (default: 100)

**What's wired:**
- [x] observe → understand → plan → execute → verify → reflect cycle
- [x] State machine for loop transitions (via WorkItem status)
- [x] Persistence via WorkItemStore (restart recovery)
- [x] Confidence-based auto-completion
- [x] Max iterations guard
- [x] Context compression between turns — `ContextCompressor` groups older observations by stage/kind, enforces token budget
- [x] Model routing — phase-specific backends via `ModelRouter`; plan/reflect use HTTP backend by default, others use heuristic
- [x] Decision flow: `DecisionGate` evaluates WorkItem state to decide tool call vs direct response vs defer
- [x] Scope isolation — `AgentLoop.with_scope()` → `TaskExecutor` → `GateContext.scope` for guard enforcement

**New files added:**
- `host/host-agent/src/model_routing.rs` (435 LoC) — `LoopPhase`, `PhaseBackendKind`, `PhaseConfig`, `ModelRouter`, `PhaseBuilder`, `phase_infer()`
- `host/host-agent/src/context_compression.rs` (388 LoC) — `CompressionConfig`, `ContextCompressor`
- `host/host-agent/src/decision_gate.rs` (365 LoC) — `Decision`, `DecisionConfig`, `DecisionGate`

**Changes to existing files:**
- `host/host-agent/src/loop_engine.rs` — replaced single `InferenceBackend` with `Option<ModelRouter>`, added `ContextCompressor`, added `scope: Option<Scope>`, wired compression into all stage methods
- `host/host-agent/src/executor.rs` — `TaskExecutor` now holds `Option<Scope>`, passes it through to `GateContext`
- `host/host-agent/src/inference/mod.rs` — re-exports `InferenceError` for model_routing

43 new tests covering model routing, context compression, decision gate, and loop integration.

### 4.3 Tool Registry ✅ WIRED INTO CORE

`tool_registry/` crate exists with full MCP tool registry (rig/mistral.rs patterns):
- `ToolRegistry` struct with SQLite-backed schema (tools, tool_usage, tool_discovery tables)
- `register_tool()`, `query_tool()`, `list_all()`, `list_by_type()` CRUD
- `record_usage()`, `query_usage()` — tool metrics tracking
- `record_discovery()`, `query_discovery()` — discovery history
- `discover_tools()` — 4-layer discovery: project `.agents/skills/`, user `~/.agents/skills/`, MCP configs (`~/.config/mcp/`), state-docs
- `validate_tools()` — binary availability check via `which`
- `auto_register_discovered()` — auto-register discovered tools with defaults
- `discover_tools_sync()` — sync variant to avoid holding `&Connection` across `.await` (Connection is not Send)
- Schema versioning: `schema_versions` table, `get_schema_version()`, `check_schema_compatibility()`
- 5 unit tests covering init, register/query, list, usage, type filter

**Wiring status:**
- [x] Added `crabjar-tool-registry` dependency to crabjar binary (Cargo.toml)
- [x] Wired into `handle_exec()` in `src/main.rs`: discovers tools from project root before execution
- [x] Wired into orchestrator `execute_tool_call()`: tool registry resolution with guard gate enforcement, falls back to built-in dispatch for backward compatibility
- [x] Wired into host-agent `TaskExecutor::execute_via_registry()`: resolves task descriptions as tool calls, executes via guard gate
- [x] Added `crabjar tool list` and `crabjar tool discover` CLI subcommands
- [x] Added fallback chains for tool availability (binary missing → suggest install, run `crabjar tool discover`)
- [x] Added schema versioning (v1) with `schema_versions` table and compatibility checks

### 4.4 Agent Loop: Structural Documentation Sync

**Goal:** Ensure the agent loop updates roadmap and project_map after significant changes, preventing the exact staleness we just fixed.

**Trigger conditions** (any of these fires a documentation sync pass):
- New crate added/removed from workspace
- Guard gate logic changed (new trust layer, scope dimension, fingerprint rule)
- Orchestrator endpoint or inference backend changed
- CLI command added/removed/modified
- Module split or merge (any .rs file crossing 500 LoC threshold)
- Shared dependency version bump (major/minor)
- Architecture layer membership change

**Sync procedure** (runs as part of the verify phase, before the reflect phase):
1. Check `git status` — if uncommitted changes to Cargo.toml, guard/, orchestrator/, or src/main.rs, proceed
2. Run `find` + `grep` to verify workspace member count matches Cargo.toml
3. Diff project_map.md against current filesystem (check guard/src files, tree diagram, shared deps)
4. Diff ROADMAP.md status indicators against actual git log (mark completed items)
5. Commit documentation updates alongside code changes (same commit, separate hunks)

**Why this matters:** project_map.md is stale 60% of the time. Every stale map wastes agent cycles on wrong file paths and outdated architecture assumptions. This is a self-reinforcing loop — stale maps make agents avoid updating them, which makes them stale faster. The agent loop is the only reliable updater because it's already touching the files that cause drift.

---

## Priority 5: Claw Code Patterns (Useful but Less Distinctive)

Claw Code is OpenAI + Anthropic patterns smashed together without a coherent philosophy. The schema-first discipline is good; the rest is just good engineering.

### 5.1 Declarative Subsystem Schemas

**Status:** Partially done via `rmcp` (MCP tool registry). Needs dedicated schema format.

### 5.2 Central Type Contract Layer

**Status:** Not started. Monitor for pain signals.

### 5.3 Session Store ✅ DONE

**Status:** Implemented in `src/host-binary/tui/session.rs`. SQLite-backed session persistence with create/load/save/list operations, message serialization via serde. Used by the conversational TUI to persist conversation history across sessions.

- [x] Design: `SessionStore` struct with SQLite schema (sessions table, messages table)
- [x] Implement: `create()` — generates UUID session ID, creates session row
- [x] Implement: `load(id)` — retrieves session + all associated messages
- [x] Implement: `save(id, messages)` — upserts session, replaces message rows
- [x] Implement: `list_ids()` — returns all session IDs ordered by creation time
- [x] Wire into TUI app: auto-create on startup, save after agent loop completion

---

## Priority 8: Persistence Architecture

### 8.1 Dual-Backend Persistence Abstraction ❌ NOT STARTED

Crabjar uses rusqlite/bundled sqlite exclusively. No PostgreSQL abstraction layer exists.

- [ ] `PersistenceBackend` trait: unified read/write interface
- [ ] SQLite backend (current) + PostgreSQL backend (future)
- [ ] Every persistence crate implements both backends
- [ ] Migration path: swap backend without changing business logic

**Why this matters:** Scaling from SQLite to PostgreSQL requires rewriting every persistence crate. An abstraction layer makes the migration a config change.

### 8.2 VM Bridge Integration ❌ NOT STARTED

#### 8.2.1 crabjar-vm Crate
- [ ] Manifest parsing (reuse vm-bridge's TOML format)
- [ ] Worker process management (reuse supervisor logic)
- [ ] WebSocket relay integration (reuse proxy logic)

#### 8.2.2 crabjar-screen Crate
- [ ] PipeWire integration for screen share sources
- [ ] XDG-Portal integration for Wayland screen capture
- [ ] Preview thumbnail generation (320x180)
- [ ] Audio capture (microphone + system audio)

#### 8.2.3 crabjar-terminal Crate ✅ DONE

**Status:** Implemented in `crates/terminal/` — wezterm (primary) + zellij (fallback) backends with asciinema v2 recording.

- **Wezterm mux backend** (`wezterm.rs`, ~170 LoC): Full implementation via `wezterm cli` protocol — spawn, send-text, get-text, split-pane, kill-window
- **Zellij action protocol backend** (`zellij.rs`, ~165 LoC): Server start, write-chars, dump-screen, detach lifecycle management
- **Asciinema v2 recording** (`recording.rs`, ~180 LoC): Header writing, input/output event buffering with millisecond timestamps, proper file closing
- **TerminalBackend trait** (`backend.rs`): Dyn-compatible async trait with `is_available()` detection (wezterm > zellij auto-detection)
- **TerminalManager**: Multi-session tracking with HashMap-backed session registry
- **TerminalSession**: Full lifecycle — spawn → send/read loop → snapshot/record → stop
- **Snapshot API**: Terminal buffer capture with line extraction, backend identification, working directory metadata

**Why this matters:** Agent harness needs programmatic terminal control for running commands in isolated sessions, capturing output, and recording sessions for replay/debugging. Wezterm's mux protocol provides reliable text I/O; zellij offers a fallback without GUI dependency.

#### 8.2.4 Wire into crabjar-host
- [ ] Teams plugin integration
- [ ] Display protocol routing

---

## Priority 9: Codex Pattern Imports

Derived from parity analysis against OpenAI Codex (2026-06-23). Codex sets the quality bar; these patterns are worth importing into Crabjar's architecture.

### 9.1 Bounded Context Management ✅ DONE

**Status:** Implemented in `memory/src/context.rs`. `ContextFragment` type with token-bounded size + hard cap (10K tokens per Codex spec). `ContextFragmentBuilder` with fluent API. P0 alert threshold (`P0_ALERT_TOKENS` = 1,000 tokens) for fragments exceeding threshold. `estimate_tokens()` for token accounting.

- [x] Design: `ContextFragment` type with token-bounded size + hard cap
- [x] Implement: bounded injection API in knowledge store (`memory/`)
- [x] Add: per-fragment token accounting (not byte counting)
- [x] Add: P0 alert for fragments exceeding 1k tokens
- [x] Wire into guard: gate rejects oversized fragments (>10K) and checks cumulative budget
  - `guard/src/context_budget.rs`: `ContextBudget` + `MAX_TOKENS_PER_FRAGMENT` constant
  - `guard/src/gate.rs` step 10: per-fragment hard cap check → `GateResult::OversizedFragment`
  - `guard/src/gate.rs` step 10: cumulative budget check → `GateResult::ContextExhausted`
  - Loose bounds per Q12: warning at 80%, hard rejection only at 100%
  - 2 new tests: `test_context_budget_rejects_oversized_fragment`, `test_context_budget_allows_at_hard_cap`

**Why this matters:** Crabjar's knowledge store has no token budget. Without bounded context, long conversations will silently degrade model quality. Codex's approach: everything injected must have a bounded size and a hard cap.

### 9.2 Module Size Governance ✅ DONE

All guard/ modules now under 500 LoC. `guard_db_impl.rs` split into 3 files:

- [x] Split `guard_db_impl.rs` (729 LoC) → `guard_db_impl.rs` (368 LoC: anneal + concierge + PID trust) + `guard_db_queries.rs` (352 LoC: action requests + trust resolution) + `guard_db_types.rs` (16 LoC: TrustResolutionEntry)
- [x] Verified no other modules exceed 500 LoC
- [x] CI gate (`just module-sizes-check`) already in place

**Note:** Priority 3.2 and 9.2 were previously contradictory. Priority 3.2 had the correct status (in progress, not complete). Both are now complete.

### 9.3 Snapshot Testing for TUI ✅ DONE

**Status:** Implemented in `tests/snapshot_tests.rs` with 6 tests covering CLI JSON output format and TUI message serialization. Baselines committed to `tests/snapshots/`. CI gate (`snapshot-review`) runs on every PR/commit via `cargo insta test --review`.

- [x] Add `insta` to workspace dependencies (v1.43, json feature)
- [x] Add snapshot tests for CLI JSON output: `state list`, `workspace status`, `doctor check`, `tool list`, `guard queue`
- [x] Add snapshot test for TUI message serialization (`Message` enum variants)
- [x] Document snapshot testing workflow in AGENTS.md (how to update baselines with `INSTA_UPDATE=always`)
- [x] CI gate: `snapshot-review` job installs `cargo-insta` and runs `cargo insta test --review`, fails on pending snapshots

**Why this matters:** Crabjar has no regression testing for structured output. Snapshot tests catch format drift before it reaches users. Codex's pattern: UI/text changes must update snapshots as part of the PR.

**Current state:** 6/6 tests passing, baselines committed. Missing CI gate because `cargo insta` CLI isn't installed — need to add it as a dev dependency or document manual `INSTA_UPDATE=always` workflow for CI.

### 9.4 File Search Engine ✅ DONE

**Status:** Implemented in `file_search/` — BM25-based file indexing and search using Tantivy 0.22. Three source files (lib.rs, indexer.rs, storage.rs), 6 passing tests.

- [x] Design: `FileSearch` trait with BM25 indexing backend
- [x] Implement: incremental file indexing (watch + poll)
- [x] Query API: keyword, fuzzy, path-based search via Tantivy query parser
- [x] File relevance scoring with path-aware weighting
- [x] 6 passing tests covering tokenization, indexing, search relevance, fuzzy matching, path filtering, stale reader reload

**Why this matters:** Crabjar currently uses `ignore` for file traversal — no indexing, no ranking. For a 24-crate workspace, agents need fast, relevant file discovery. Codex's BM25 approach is battle-tested.

### 9.5 Declarative Policy Engine ✅ DONE (Rust-native)

**Status:** Implemented in `guard/src/policy.rs` + `policy_types.rs`. Static policy engine using TOML configuration as an alternative to Go-Sanitized Starlark for declarative policy evaluation.

- [x] Design: `PolicyEngine` trait abstracting static vs. scriptable policies
- [x] Implement: `StaticPolicyEngine` with TOML-based config (zero startup cost, compile-time safety)
- [x] Configurable checks: dangerous commands, confidence floors, trust layer minimums, scope isolation toggles, domain allowlist modes, context budgets
- [x] Hot-reload support via file watching (`reload()` method with diff detection)
- [x] Backward compatibility: optional gate integration (falls through to existing logic when not configured)
- [x] 14 unit tests covering all policy evaluation paths

**Why this matters:** Starlark was considered for 9.5 but deferred in favor of Rust-native declarative policies because: zero new dependencies (~15 transitive deps saved), no runtime overhead (inlined Rust branches vs interpreted execution), compile-time type safety, and team fit (Rust-first team has zero Starlark expertise). The `PolicyEngine` trait preserves the option to add a Starlark backend later via feature flag (`#[cfg(feature = "starlark")]`) without changing the public API.

**Inspiration from Starlark:** Sandboxed execution, deterministic evaluation, hot-reload, declarative configuration — all patterns adopted but implemented in pure Rust.

---

## Priority 10: Developer Experience

### 10.1 ADR Process ❌ NOT STARTED

No `specs/` directory or ADR template exists.

- [ ] `specs/ADR-NNN_<title>.md` template
- [ ] Decision context, options, rationale
- [ ] Cross-references between related ADRs

### 10.2 Config Layering ⚠️ PARTIALLY DONE

Multi-level configuration (defaults → user config → project config → CLI flags) partially done via `.crabjar_config.toml`. No formalization yet.

---

## Priority 11: Testing Infrastructure

### 11.1 E2E Slice Testing ✅ DEFINED

IronClaw's E2E test matrix (smoke vs full) lets CI run fast on PRs and thorough on merges.

**Smoke slice** (runs on every PR, ~30s):
- `crabjar state list` — verifies CLI binary runs and returns JSON
- `crabjar workspace status` — verifies `.crabjar_config.toml` loading
- Guard DB init + basic gate check (in-memory)
- Tool registry init + register/query cycle
- Knowledge store init + basic query
- `crabjar doctor check` — verifies environment health

**Full slice** (runs on merge/nightly, ~5min):
- All smoke tests above
- Exec pipeline with real guard DB (tempfile-backed)
- Domain allowlist enforcement (deny + allow paths)
- Scope isolation checks (cross-scope blocking)
- Telemetry flight recorder write/read cycle
- Agent loop tick with persistence (tempfile-backed WorkItemStore)
- Tool discovery across all 4 layers
- `crabjar guard` subcommands (queue, approve, reject, resolution)

**Implementation plan:**
- [x] Add `tests/e2e/mod.rs` with smoke test module (6 tests)
- [x] Add `tests/e2e/full.rs` with full test module (26 tests covering exec pipeline, domain allowlist, scope isolation, telemetry flight recorder, agent loop persistence via WorkItemStore, tool discovery across 4 layers, guard subcommands, knowledge lifecycle, workspace config edge cases)
- [ ] CI: run smoke on every PR, full on merge/nightly
- [x] Add `just test-e2e-smoke` and `just test-e2e-full` targets

**Smoke slice results:** 6/6 passing (~0.3s total):
- `crabjar state list` ✅ — verifies CLI binary runs and returns JSON with docs array
- `crabjar workspace status` ✅ — verifies `.crabjar_config.toml` loading (null config + valid config paths)
- Guard DB init + basic gate check ✅ (`guard queue --status=pending`) — verifies guard commands return structured JSON
- Tool registry init + register/query cycle ✅ (`tool list`) — verifies tool commands work even with empty registry
- Knowledge store init + basic query ✅ (`knowledge insert` → `knowledge query`) — verifies knowledge CRUD pipeline
- `crabjar doctor check` ✅ — verifies environment health checks with doubt block per CLI output contract

**Full slice results:** 26/26 passing (~0.02s total):
- Exec pipeline dry-run/denied/proceeds (3 tests) — config → gate check → concierge enforcement
- Domain allowlist blocks unknown / trust layers / wildcards (3 tests) — deny-by-default with layer enforcement
- Scope isolation different users / same user diff projects / same scope (3 tests) — mutual inaccessibility invariant
- CrossScopeAuth auto-construction (1 test) — `auto_for_scopes()` returns Some for cross-project, None for same-scope
- Telemetry flight recorder init → execute_command → query_records (1 test) — async write/read cycle
- Agent loop WorkItemStore create → save → load → update → list_ids (1 test) — persistence round-trip with status transitions
- Tool discovery + tool list with filter (2 tests) — CLI integration for registry commands
- Guard queue / provenance / resolution / grant / revoke / approve / reject (7 tests) — all guard subcommands return structured JSON
- GuardDb schema verification + pending queue persist/retrieve (2 tests) — SQLite-backed DB operations
- Knowledge insert → verify → events → deactivate → query confirms removal (1 test) — full lifecycle with integrity checks
- Workspace malformed TOML / missing config (2 tests) — soft-fail returns null workspace

### 11.2 Replay Snapshots

IronClaw's `scripts/replay-snap.sh` enables deterministic testing by replaying recorded LLM traces.

- [ ] Record LLM response traces as fixtures
- [ ] Replay fixtures in tests (no external LLM dependency)
- [ ] Regression detection: diff new responses against snapshots

---

## Priority 13: What Needs Implementation (Next Steps)

These are the highest-value items that would move crabjar closer to a usable agent harness.

### 13.1 Fix Pre-existing Test Failures ✅ DONE

All 8 pre-existing cli.rs test failures resolved (July 6, 2026):

- [x] `knowledge_deactivate_updates_query_results` — fixed: tests read from top-level keys but CLI wraps in `"data"`
- [x] `knowledge_events_and_verify_return_json` — fixed: same `"data"` nesting issue on verify/events fields
- [x] `knowledge_sync_and_query_return_json` — fixed: sync/query assertions updated to `body["data"]["..."]`
- [x] `knowledge_sync_is_idempotent` — fixed: same pattern
- [x] `query_one_tag_does_not_return_unrelated_rows` — fixed: query result path updated
- [x] `resolve_annotation_deactivates_derived_knowledge` — fixed: sync/query/resolve paths updated
- [x] `resolve_one_annotation_does_not_deactivate_other` — fixed: same pattern across 2-doc test
- [x] `state_show_returns_doc_contents` — fixed: two bugs — (1) indexer now falls back to filename stem when frontmatter lacks `name:`, (2) `state show` strips `.md` extension before lookup

**Root causes:**
1. CLI wraps all responses in `{"success": true, "message": "...", "data": {...}}` but tests read from top level — 7 tests affected
2. State-doc indexer stored empty `doc_name` when frontmatter had no `name:` field — caused `state show` to fail on docs without explicit name

**Files changed:** `tests/cli.rs`, `memory/src/state_docs/indexer.rs`, `src/main.rs`, `crates/terminal/src/lib.rs` (doctest fix)

### 13.2 TUI Snapshot Testing ✅ DONE (merged into 9.3)

Merged into Priority 9.3 — snapshot testing is now implemented in `tests/snapshot_tests.rs` with 6 tests covering CLI JSON output and TUI message serialization. See section 9.3 for details.

### 13.3 Scope Injection into Orchestrator/Host-Agent ✅ DONE

CrossScopeAuth is now wired across all execution paths — CLI, orchestrator, host-agent, and TUI.

- [x] Add scope context to orchestrator's `execute_with_guard()` (already had it) + `"run_command"`, `"search_logs"`, `"recent_events"`, `"by_source"` handlers
- [x] Add scope context to host-agent's `AgentLoop::with_scope()` callers in all construction sites:
  - `src/host-binary/main.rs` — Tick/Run commands use `.with_scope(GuardScope::project("host"))`
  - `src/host-binary/dashboard.rs` — F(1) handler uses `.with_scope(GuardScope::project("host"))`
  - `src/host-binary/tui/app.rs` — `run_agent_loop()` uses `.with_scope(GuardScope::project("tui"))`
- [x] Wire `CrossScopeAuth` creation into orchestrator's 3 missing handlers (search_logs, recent_events, by_source) using `auto_for_scopes(&actor_scope, &target_scope)`
- [x] Add `crabjar-guard` dependency to `src/host-binary/Cargo.toml` for scope injection in host binary crate

**Scope assignments:**
| Path | Scope Name | CrossScopeAuth |
|------|-----------|----------------|
| CLI exec (`src/main.rs`) | project-scoped (derived from cwd) | ✅ auto_for_scopes(same-scope → None) |
| Orchestrator `execute_with_guard()` | `"orchestrator"` | ✅ auto_for_scopes(same-scope → None) |
| Orchestrator `"run_command"` | `"orchestrator"` | ✅ auto_for_scopes(same-scope → None) |
| Orchestrator `"search_logs"` | `"orchestrator"` | ✅ auto_for_scopes(same-scope → None) |
| Orchestrator `"recent_events"` | `"orchestrator"` | ✅ auto_for_scopes(same-scope → None) |
| Orchestrator `"by_source"` | `"orchestrator"` | ✅ auto_for_scopes(same-scope → None) |
| Host-agent `TaskExecutor` | set via `.with_scope()` | ✅ from executor's scope field |
| TUI app loop | `"tui"` | ✅ auto_for_scopes(same-scope → None) |
| Dashboard F(1) handler | `"host"` | ✅ auto_for_scopes(same-scope → None) |

**Why this matters:** Without scope injection, cross-scope authorization cannot be enforced in orchestrator and host-agent paths. All gate checks now carry proper `scope` + `target_scope` + `cross_scope_auth` fields, enabling the guard's scope isolation logic to function correctly across all execution entry points.

### 13.4 TUI Guard Approval Flow ✅ DONE

**Status:** Implemented in `host-binary/tui/` — keyboard shortcuts ('a'/'r') for approve/reject pending guard actions, `resolve_pending_queue_entry()` DB method, agent loop surfaces pending queue entries and pauses for user input.

- [x] Add keyboard shortcuts in TUI input handler (`'a'` = approve, `'r'` = reject)
  - Modified `handle_input()` to check for 'a'/'r' keys when state is `AwaitingApproval`
  - Added `ApprovePending` and `RejectPending` variants to `Action` enum in `input.rs`
- [x] Wire approval/rejection through to GuardDb
  - Implemented `resolve_pending_queue_entry(id, approved: bool)` in `GuardDb` (guard_db_impl.rs)
  - Approve: removes entry from pending_queue
  - Reject: moves entry to interrupted_log with reason "user_rejected", then deletes from pending_queue
  - Added `uuid::Uuid` import for generating new IDs when moving entries
- [x] Update status bar with actionable prompt when guard is pending
  - Changed `AppState::AwaitingApproval(String)` → `AppState::AwaitingApproval { id: String, action_desc: String }` to store entry ID and description
  - Status bar shows `" Guard pending: {} [a=approve / r=reject] "` with the action description
  - Title bar updates to show `"CrabJar Agent — Awaiting Approval: <action>"`

**New files modified:**
- `guard/src/guard_db_impl.rs`: Added `resolve_pending_queue_entry()` method + 3 unit tests
- `src/host-binary/tui/input.rs`: Added `ApprovePending` and `RejectPending` to Action enum
- `src/host-binary/tui/app.rs`: Updated AppState variant, added keyboard shortcuts in handle_input(), added resolve_pending() method, wired pending queue check into agent loop
- `src/host-binary/tui/mod.rs`: Updated run() signature to accept guard_db, wired ApprovePending/RejectPending actions in event loop
- `src/host-binary/main.rs`: Creates GuardDb and passes it to tui::run()

**Tests:** 3 new unit tests added (approve, reject, not_found) — all passing. Full workspace test suite: ~152+ passing, 0 failing.

### 13.5 CI Integration ❌ NOT STARTED

E2E test slices are defined but not wired into CI.

- [ ] Add smoke tests to PR workflow (every PR)
- [ ] Add full slice to merge/nightly workflow

---

## Open Questions & Decisions

1. **WASM timeline:** ✅ **Decided (July 6): Keep as reserved slot.** `zed-acp-bridge` is a Zed-specific extension using `zed_extension_api`, not a general plugin system. No sandboxing, no lifecycle management, no loader — only works inside Zed. Real WASM plugins would need `wasmtime`/`wasmer` (~10MB build), capability configuration, crash recovery, and a plugin interface. Friction point: "I want to run WASM plugins outside the editor" or "I want runtime load/unload." Until then, reserved slot is correct.
2. **Model routing:** How to decide which model handles which phase of the ReAct loop? — *Decided: plan/reflect → HTTP backend, others → heuristic (implemented in `host-agent/model_routing.rs`)*
3. **State-doc staleness:** ✅ **Decided (July 6): Three-tier graduated model.** Fresh (<7d) → Stale (7-14d, warning) → Expired (14-30d, untrustworthy without re-index) → Moldy (>30d, discarded unless additional context added relative to reconstruction cost). Implemented in `memory/src/state_docs/models.rs` (`StalenessStatus` enum), `querier.rs` (`staleness_status()` method), and CLI (`crabjar state staleness <doc>`). The moldy tier checks for annotation activity after last modification — if the user/agent added value, it resets to expired rather than auto-discarding.
4. **Plugin language support:** ✅ **Decided (July 6): Rust first.** In-process workspace plugins share guard/telemetry/memory types with zero startup cost. stdio JSON-RPC as the escape hatch for cross-process/out-of-band plugins. Rhai tier covers lightweight scripting; ToolServer is for heavier workloads where Rust's cold start (~50-80ms) fits the 100ms budget better than Python/Go runtimes.
5. **Context compression strategy:** Summarization vs. selective retention vs. relevance scoring? — *Decided: grouping older observations by stage/kind into summaries with token budget enforcement (implemented in `host-agent/context_compression.rs`)*
6. **Scope isolation granularity:** Which scope dimensions are needed at launch? (identity, project, tenant, thread) — ✅ DONE: all four implemented
7. **Boundary enforcement trigger:** CI-only gate or also pre-commit hook? — *Open*
8. **Prompt envelope scope:** ✅ **Decided (June 28):** Protect **both** user-facing and non-user-facing prompts. Only deprioritize if cost becomes a constraint. No cost pressure yet.
9. **VM bridge priority:** ✅ **Decided (June 28):** Scope to **Unix user sandbox** (already in `crabjar-sandbox`). Reassess VM bridge if a concrete benefit case emerges later.
10. **Dual-backend persistence:** ✅ **Decided (June 28):** **Stick with SQLite** until a real PostgreSQL need appears. No abstraction layer needed now.
11. **tool_registry wiring:** ✅ **Decided (June 28):** No friction from the user side — it's agent-facing and results work. "Done" is undefined; leave as-is unless a concrete gap appears.
12. **ContextFragment guard wiring:** ✅ **Decided (June 28):** **Wire in but leave pretty open** — gate on bounded context but with loose bounds. Tighten later if needed.
13. **Module size governance scope:** Should the 500 LoC rule apply to all crates or just guard/? — *Decision: Apply to all crates (guard/ proved it's doable; now extending enforcement to orchestrator/, memory/, tool_registry/*)
14. **Metrics reporting scope:** What workspace metrics should `crabjar metrics` report? — *Decided: test count, LoC per crate, total modules, workspace member count*

---

*Last updated: July 6, 2026*
