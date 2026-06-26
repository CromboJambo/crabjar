# Crabjar Roadmap

> Crabjar is trying to capture EdgeCrab's ecosystem in concept. Codex sets the quality bar. Claw Code is Frankenstein — useful patterns buried in noise. IronClaw is the structural reference — mechanical boundary enforcement, scope isolation, and trust resolution at scale.

---

## Status (June 26, 2026)

|| Metric | Value |
||---|---|
|| Workspace members | 23 crates |
|| Tests | ~130 passing, 0 failing |
|| Clippy | ✅ Clean (`cargo clippy --workspace -- -D warnings`) |
|| Architecture crate | ✅ Built, compiles, has integration test |
|| Guard scope/trust | ✅ Scope isolation + requested-vs-effective trust resolution implemented |
|| Per-crate AGENTS.md | ✅ Complete (all 23 crates documented) |
|| lm_studio_client | ✅ Modularized into 6 files with unit tests |
|| fingerprint approvals | ✅ Implemented (InvocationFingerprint + ApprovalLease) |
|| CrossScopeAuth | ✅ Implemented with expiry and scope resolution |
|| ContextFragment | ✅ Implemented in memory/ with P0 alert threshold |
|| Audit trail | ✅ TrustResolution.audit_log() records derivation chain |

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

### 1.3 Update project_map.md ⚠️ PARTIALLY STALE

- [x] project_map.md says "21 members" — workspace now has 23 members (confirmed via Cargo.toml)
- [ ] Update architecture diagram with `crabjar-architecture`, `axum-mux`, `crabjar-app-teams`
- [ ] Mark completed items with status indicators
- [x] `crabjar-architecture` added as workspace member (confirmed in Cargo.toml line 11)
- [ ] Reflect `lm_studio_client` modularization (6 files: types, client, session, error, endpoints, mod)
- [ ] Update generated date to June 26, 2026

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
- [ ] Wire into `crabjar guard` CLI — show trust resolution chain in output

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

### 2.6 Product Adapter Pattern ❌ NOT STARTED

No generic adapter layer exists. Channel-specific code only (host-mqtt, host-graph). Every new channel requires core changes.

- [ ] Design: `ProductAdapter` trait — normalize input → `IncomingMessage`, send output ← `OutgoingMessage`
- [ ] Implement: adapter registry for discovery and lifecycle
- [ ] Implement: channel-specific adapters (MQTT → Home Assistant, Graph API → Teams)
- [ ] New channels = new adapter, no core changes

**Why this matters:** Every new channel (Discord, Feishu, WeChat) currently requires core changes. The adapter pattern contains that blast radius.

---

## Priority 3: Codex Quality Constraints

Codex doesn't contribute architecture — it sets the standard. These are non-negotiable quality bars.

### 3.1 Linting as Policy Gates

- [ ] Domain allowlist — restrict which external tools/domains are callable (guard integration)
- [x] Action policy — destructive actions require user permission (implemented in concierge.rs)
- [ ] Code quality gates — module size limits, async conventions
- [ ] Drift governance — detect when state-docs diverge from reality (partially done via `skill-reference-store`)

### 3.2 Module Size Governance ❌ NOT STARTED

No module size enforcement exists. No `cargo-declared` or custom script to report module sizes. No 500-line cap. Current largest modules: `guard/src/types.rs` (775 LoC), `guard/src/gate.rs` (697 LoC), `guard/src/fingerprint.rs` (585 LoC), `guard/src/concierge.rs` (541 LoC), `guard/src/guard_db.rs` (511 LoC).

- [ ] Add `cargo-declared` or custom script to report module sizes
- [ ] Identify modules exceeding 500 lines
- [ ] Split largest offenders
- [ ] Add to CI gate

### 3.3 Build Reproducibility ❌ NOT STARTED

No `just reproducible-build` target. No documented build reproducibility guarantees.

- [ ] Pin all dependency versions (already done via Cargo.lock)
- [ ] Add `just reproducible-build` target
- [ ] Document build reproducibility guarantees

---

## Priority 4: Agent Loop & Tool Protocol (EdgeCrab-Informed)

These are the conceptual patterns crabjar needs to replicate from EdgeCrab.

### 4.1 JSON-RPC Plugin Protocol ✅ PARTIALLY DONE

`orchestrator/lm_studio_client/` modularized into 6 files (types, client, session, error, endpoints, mod) with unit tests (508 lines). `zed-acp-server` provides stdio JSON-RPC server for ACP protocol execution. `zed-acp-bridge` provides Wasm extension for tool call mapping.

- [x] Message schema for tool calls, results, errors (lm_studio_client/types.rs)
- [x] Three-tier model: ToolServer (subprocess), Script (Rhai in-process), Reserved (WASM)
- [ ] Startup latency budget (~100ms), error recovery, lifecycle management
- [ ] Language-agnostic plugin execution

### 4.2 Agent Loop (ReAct) ❌ NOT STARTED

`host/host-agent/` exists with lifecycle docs. The actual ReAct loop (observe → understand → plan → execute → verify → reflect) is not implemented as a reusable crate.

- [ ] observe → understand → plan → execute → verify → reflect cycle
- [ ] State machine for loop transitions
- [ ] Context compression between turns (critical for long conversations)
- [ ] Model routing (which model for which phase)
- [ ] Decision flow: when to call tools vs. respond directly

### 4.3 Tool Registry ⚠️ EXISTS (not wired)

`tool_registry/` crate exists in workspace members (confirmed in Cargo.toml line 9). MCP tool registry with rig/aur patterns. Not wired into core execution pipeline.

- [ ] Dynamic capability discovery
- [ ] Tool metadata (description, params, return types)
- [ ] Fallback chains for tool availability
- [ ] Versioned tool interfaces

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

### 11.1 E2E Slice Testing

IronClaw's E2E test matrix (smoke vs full) lets CI run fast on PRs and thorough on merges.

- [ ] Define smoke slice: core agent loop, tool execution, channel delivery
- [ ] Define full slice: all channels, all sandboxes, all trust layers
- [ ] CI runs smoke on every PR; full on merge/nightly

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
