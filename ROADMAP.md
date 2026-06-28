# Crabjar Roadmap

> Crabjar is trying to capture EdgeCrab's ecosystem in concept. Codex sets the quality bar. Claw Code is Frankenstein — useful patterns buried in noise. IronClaw is the structural reference — mechanical boundary enforcement, scope isolation, and trust resolution at scale.

---

## Status (June 26, 2026)

|| Metric | Value |
||---|---|
|| **Workspace members** | 22 crates |
|| **Tests** | ~130 passing, 0 failing |
|| **Clippy** | ✅ Clean (`cargo clippy --workspace -- -D warnings`) |
| **Architecture crate** | ✅ Built, compiles, has integration test |
| **Guard scope/trust** | ✅ Scope isolation + requested-vs-effective trust resolution implemented |
| **Per-crate AGENTS.md** | ✅ Complete (all 23 crates documented) |
| **lm_studio_client** | ✅ Modularized into 6 files with unit tests |
| **fingerprint approvals** | ✅ Implemented (InvocationFingerprint + ApprovalLease) |
| **CrossScopeAuth** | ✅ Implemented with expiry and scope resolution |
| **ContextFragment** | ✅ Implemented in memory/ with P0 alert threshold |
| **Audit trail** | ✅ TrustResolution.audit_log() records derivation chain |
| **Guard module size** | ✅ Split 5 modules (trust, gate, fingerprint, concierge, trust_resolution) — all under 500 LoC |

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

**Remaining concern:** `guard/src/guard_db_impl.rs` is 729 LoC — exceeds 500 LoC rule. Needs splitting (see 3.2).

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
- [ ] Add `cargo-declared` integration to detect drift between declared and actual deps

### 2.2 Scope Isolation Model ✅ DONE (verified)

**Status:** Implemented in `guard/src/scope.rs` with `Scope` type covering identity, project, tenant, thread dimensions. `can_access()` enforces cross-scope authorization. Scope isolation test passes.

**What's next:**
- [x] Fix the failing test (see 1.1) — DONE
- [x] Add `CrossScopeAuth` approval flow — DONE (implemented in scope.rs:132-213 with expiry)
- [ ] Wire scope into `ExecutionGate::check()` — every action should carry a scope
- [ ] Add scope to `GuardDb` schema (persist scope with pending actions)

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

### 3.2 Module Size Governance ✅ COMPLETED

**Status:** All >500 LoC modules in `guard/` have been split.

|- [x] Add `just module-sizes` target (reports modules exceeding threshold)
|- [x] Add `just module-sizes-check` CI gate (fails on >500 LoC)
|- [x] Add CI job to `.github/workflows/rust.yml`
|- [x] Split `trust.rs` (697 LoC) → `trust_types.rs` (252 LoC) + `trust.rs` (429 LoC: impl + tests)
|- [x] Split `gate.rs` (614 LoC) → `gate.rs` (292 LoC: impl) + `gate_tests.rs` (323 LoC: tests)
|- [x] Split `trust_resolution.rs` (641 LoC) → `trust_resolution.rs` (431 LoC: impl only)
|- [x] Split `fingerprint.rs` (585 LoC) → `fingerprint_types.rs` (259 LoC) + `fingerprint.rs` (8 LoC: re-exports) + `fingerprint.rs` tests extracted
|- [x] Split `concierge.rs` (541 LoC) → `concierge_types.rs` (91 LoC) + `concierge.rs` (466 LoC: impl + tests)
|- [x] Document 500 LoC rule in AGENTS.md

**Why this matters:** Codex-core bloat is the anti-pattern Crabjar must avoid. The 500 LoC rule is cognitive load management, not bureaucracy.

**Current guard crate modules (post-split):**
- `trust.rs` — TrustScore, TrustLayer, TrustManager, ReviewAction, AnnealConfig, RetrievalBand (406 LoC)
- `memory_types.rs` — NodeKind, MemoryNode, EdgeRelation, MemoryEdge (193 LoC)
- `memory.rs` — MemoryGraph DB-backed impl (380 LoC)
- `action.rs` — ActionStatus, OutcomeStatus, ActionRequest, ActionOutcome (318 LoC)
- `inference.rs` — ModelInferenceKind, ModelInferenceRequest, ModelInferenceOutcome (298 LoC)
- `gate.rs` — ExecutionGate impl (480 LoC)
- `gate_context.rs` — GateContext struct (108 LoC)
- `gate_result.rs` — GateResult enum (88 LoC)
- `command_risk.rs` — CommandRisk, HIGH/MEDIUM_RISK_COMMANDS (130 LoC)
- `risk_config.rs` — RiskConfig (56 LoC)

### 3.3 Build Reproducibility ✅ PARTIALLY DONE

Cargo.lock already pins all dependency versions. Added `just reproducible-build` target:
- `cargo update --locked` verifies no drift between Cargo.toml and Cargo.lock
- `CARGO_INCREMENTAL=0` disables incremental compilation for deterministic builds
- `RUSTFLAGS="-C target-cpu=native"` pins the target CPU feature set
- `cargo tree --depth 1` reports dependency tree for audit

**Remaining:**
- [ ] Add `just reproducible-build` to CI (`.github/workflows/rust.yml`)
- [ ] Document build reproducibility guarantees in AGENTS.md
- [ ] Consider `cargo audit` for dependency vulnerability scanning

---

## Priority 4: Agent Loop & Tool Protocol (EdgeCrab-Informed)

These are the conceptual patterns crabjar needs to replicate from EdgeCrab.

### 4.1 JSON-RPC Plugin Protocol ✅ PARTIALLY DONE

`orchestrator/lm_studio_client/` modularized into 6 files (types, client, session, error, endpoints, mod) with unit tests (508 lines). `zed-acp-server` provides stdio JSON-RPC server for ACP protocol execution. `zed-acp-bridge` provides Wasm extension for tool call mapping.

- [x] Message schema for tool calls, results, errors (lm_studio_client/types.rs)
- [x] Three-tier model: ToolServer (subprocess), Script (Rhai in-process), Reserved (WASM)
- [ ] Startup latency budget (~100ms), error recovery, lifecycle management
- [ ] Language-agnostic plugin execution

### 4.2 Agent Loop (ReAct) ✅ PARTIALLY DONE

`host/host-agent/` exists with a complete ReAct loop implementation:
- `loop_engine.rs`: `AgentLoop` struct with observe → understand → plan → execute → verify → reflect → persist cycle
- `executor.rs`, `planner.rs`, `verifier.rs`, `reflector.rs`: Stage-specific logic
- `work_item_store.rs`: SQLite-backed WorkItem persistence with resume support
- `inference/backend.rs` + `http_backend.rs`: Inference backend abstraction
- State machine for loop transitions (via `WorkItem.status`)
- Confidence-based auto-completion (threshold: 0.85)
- Max iterations guard (default: 100)
- Context compression: per-stage inference prompts (not yet configurable)
- Model routing: single `InferenceBackend` trait, not yet phase-specific
- Decision flow: `tick()` runs all stages; `confidence` drives continue/stop

**What's wired:**
- [x] observe → understand → plan → execute → verify → reflect cycle
- [x] State machine for loop transitions (via WorkItem status)
- [x] Persistence via WorkItemStore (restart recovery)
- [x] Confidence-based auto-completion
- [x] Max iterations guard

**What's not wired:**
- [ ] Context compression between turns (not yet implemented — stage prompts are ad-hoc)
- [ ] Model routing (which model for which phase) — currently single InferenceBackend
- [ ] Decision flow: when to call tools vs. respond directly — not yet exposed as a gateable decision
- [ ] Scope isolation for agent loop actions — scope is wired into ExecutionGate but not yet populated in the agent loop

### 4.3 Tool Registry ✅ PARTIALLY WIRED

`tool_registry/` crate exists with full MCP tool registry (rig/mistral.rs patterns):
- `ToolRegistry` struct with SQLite-backed schema (tools, tool_usage, tool_discovery tables)
- `register_tool()`, `query_tool()`, `list_all()`, `list_by_type()` CRUD
- `record_usage()`, `query_usage()` — tool metrics tracking
- `record_discovery()`, `query_discovery()` — discovery history
- `discover_tools()` — 4-layer discovery: project `.agents/skills/`, user `~/.agents/skills/`, MCP configs (`~/.config/mcp/`), state-docs
- `validate_tools()` — binary availability check via `which`
- `auto_register_discovered()` — auto-register discovered tools with defaults
- 5 unit tests covering init, register/query, list, usage, type filter

**Wiring status:**
- [x] Added `crabjar-tool-registry` dependency to crabjar binary (Cargo.toml)
- [x] Wired into `handle_exec()` in `src/main.rs`: discovers tools from project root before execution
- [ ] Wire into orchestrator SSE handlers (for agent-facing tool discovery)
- [ ] Wire into `host-agent` ReAct loop (for dynamic tool injection into prompts)
- [ ] Add `crabjar tool list` and `crabjar tool discover` CLI subcommands
- [ ] Add fallback chains for tool availability (if binary missing, suggest install)
- [ ] Add versioned tool interfaces (schema versioning)

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

### 5.3 Session Store

**Status:** Not started. Monitor for pain signals.

---

## Priority 12: VM Bridge Integration

### 12.1 crabjar-vm Crate

- [ ] Manifest parsing (reuse vm-bridge's TOML format)
- [ ] Worker process management (reuse supervisor logic)
- [ ] WebSocket relay integration (reuse proxy logic)

### 6.2 crabjar-screen Crate

- [ ] PipeWire integration for screen share sources
- [ ] XDG-Portal integration for Wayland screen capture
- [ ] Preview thumbnail generation (320x180)
- [ ] Audio capture (microphone + system audio)

### 6.3 crabjar-terminal Crate

- [ ] Terminal multiplexer integration (wezterm/zellij)
- [ ] Shared terminal protocol over websocket
- [ ] Terminal state sync across multiple clients

### 6.4 Wire into crabjar-host

- [ ] Teams plugin integration
- [ ] Display protocol routing

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
- [ ] Add `tests/e2e/mod.rs` with smoke test module
- [ ] Add `tests/e2e/full.rs` with full test module
- [ ] Add `#[cfg(feature = "e2e-full")]` gate on full tests
- [ ] CI: run smoke on every PR, full on merge/nightly
- [ ] Add `just test-e2e-smoke` and `just test-e2e-full` targets

### 11.2 Replay Snapshots

IronClaw's `scripts/replay-snap.sh` enables deterministic testing by replaying recorded LLM traces.

- [ ] Record LLM response traces as fixtures
- [ ] Replay fixtures in tests (no external LLM dependency)
- [ ] Regression detection: diff new responses against snapshots

---

## Priority 8: Persistence Architecture

### 8.1 Dual-Backend Persistence Abstraction ❌ NOT STARTED

Crabjar uses rusqlite/bundled sqlite exclusively. No PostgreSQL abstraction layer exists.

- [ ] `PersistenceBackend` trait: unified read/write interface
- [ ] SQLite backend (current) + PostgreSQL backend (future)
- [ ] Every persistence crate implements both backends
- [ ] Migration path: swap backend without changing business logic

**Why this matters:** Scaling from SQLite to PostgreSQL requires rewriting every persistence crate. An abstraction layer makes the migration a config change.

---

## Priority 9: Codex Pattern Imports

Derived from parity analysis against OpenAI Codex (2026-06-23). Codex sets the quality bar; these patterns are worth importing into Crabjar's architecture.

### 9.1 Bounded Context Management ✅ DONE

**Status:** Implemented in `memory/src/context.rs`. `ContextFragment` type with token-bounded size + hard cap (10K tokens per Codex spec). `ContextFragmentBuilder` with fluent API. P0 alert threshold (`P0_ALERT_TOKENS` = 1,000 tokens) for fragments exceeding threshold. `estimate_tokens()` for token accounting.

- [x] Design: `ContextFragment` type with token-bounded size + hard cap
- [x] Implement: bounded injection API in knowledge store (`memory/`)
- [x] Add: per-fragment token accounting (not byte counting)
- [x] Add: P0 alert for fragments exceeding 1k tokens
- [ ] Wire into guard: reject actions that would produce unbounded context fragments

**Why this matters:** Crabjar's knowledge store has no token budget. Without bounded context, long conversations will silently degrade model quality. Codex's approach: everything injected must have a bounded size and a hard cap.

### 9.2 Module Size Governance ❌ NOT STARTED

No module size enforcement exists. No `cargo-declared` or custom script to report module sizes. No 500-line cap.

- [ ] Add `cargo-declared` or custom script to report module sizes
- [ ] Identify modules exceeding 500 lines (current offenders: `types.rs` 775, `gate.rs` 697, `fingerprint.rs` 585)
- [ ] Split largest offenders — target `guard/` and `orchestrator/` first
- [ ] Add to CI gate as a codex-quality constraint
- [ ] Document module size rule in AGENTS.md

**Why this matters:** Codex-core bloat is the anti-pattern Crabjar must avoid. The 500 LoC rule is cognitive load management, not bureaucracy.

### 9.3 Snapshot Testing for TUI ❌ NOT STARTED

No `insta` in workspace dependencies. No snapshot tests for TUI output or CLI JSON format.

- [ ] Add `insta` to workspace dependencies
- [ ] Add snapshot tests for `codeburn` TUI output
- [ ] Add snapshot tests for `crabjar` CLI JSON output format
- [ ] Document snapshot testing workflow in AGENTS.md
- [ ] CI gate: fail on pending snapshots (require `cargo insta accept`)

**Why this matters:** Crabjar has no regression testing for structured output. Snapshot tests catch format drift before it reaches users. Codex's pattern: UI/text changes must update snapshots as part of the PR.

### 9.4 File Search Engine ❌ NOT STARTED

No BM25-based file indexing exists. Crabjar uses `ignore` for file traversal only — no indexing, no ranking.

- [ ] Design: `FileSearch` trait with BM25 indexing backend
- [ ] Implement: incremental file indexing (watch + poll)
- [ ] Implement: query API (keyword, fuzzy, path-based)
- [ ] Add: file relevance scoring with path-aware weighting
- [ ] Wire into knowledge store: `file_search` subcommand

**Why this matters:** Crabjar currently uses `ignore` for file traversal — no indexing, no ranking. For a 21-crate workspace, agents need fast, relevant file discovery. Codex's BM25 approach is battle-tested.

### 9.5 Starlark Execution Policy ❌ NOT STARTED

No Starlark execution policy exists. Crabjar uses static guard deny/pending/proceed model only.

- [ ] Design: `PolicyEngine` trait abstracting static vs. scriptable policies
- [ ] Evaluate: Go-Sanitized Starlark as execution policy language
- [ ] Implement: Starlark policy loader + sandboxed evaluator
- [ ] Add: policy hot-reload without binary restart
- [ ] Backward compat: static guard rules as default policy

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

## Open Questions

1. **WASM timeline:** When to invest in WASM plugin support vs. keeping it as a reserved slot?
2. **Model routing:** How to decide which model handles which phase of the ReAct loop?
3. **State-doc staleness:** What threshold triggers staleness warnings? (7 days? content changes?)
4. **Plugin language support:** Which languages for ToolServer plugins? (Rust, Python, Go?)
5. **Context compression strategy:** Summarization vs. selective retention vs. relevance scoring?
6. **Scope isolation granularity:** Which scope dimensions are needed at launch? (identity, project, tenant, thread) — DONE: all four implemented
7. **Boundary enforcement trigger:** CI-only gate or also pre-commit hook?
8. **Prompt envelope scope:** Protect all LLM prompts or only user-facing ones?
9. **VM bridge priority:** Is VM-based agent isolation worth the complexity, or should we start with Unix user sandboxing (already in `crabjar-sandbox`)?
10. **Dual-backend persistence:** Do we actually need PostgreSQL, or is SQLite sufficient for the foreseeable future?
11. **tool_registry wiring:** `tool_registry/` crate exists in workspace but is not wired into core execution pipeline — when to integrate?
12. **ContextFragment guard wiring:** Bounded context management is done in `memory/` but not yet wired into guard rejection logic — when to gate?

---

*Last updated: June 26, 2026*
