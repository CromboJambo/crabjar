# Crabjar Roadmap

> Crabjar is trying to capture EdgeCrab's ecosystem in concept. Codex sets the quality bar. Claw Code is Frankenstein — useful patterns buried in noise. IronClaw is the structural reference — mechanical boundary enforcement, scope isolation, and trust resolution at scale.

---

## Status (June 22, 2026)

| Metric | Value |
|---|---|
| Workspace members | 21+ crates |
| Tests | 103 passing, 1 failing (`scope::tests::test_scope_cannot_access_different_project`) |
| Clippy | Needs verification (last clean run unknown) |
| Architecture crate | Built, compiles, has integration test |
| Guard scope/trust | Scope isolation + requested-vs-effective trust resolution implemented |
| Per-crate AGENTS.md | Complete (all 21+ crates documented) |

---

## Priority 1: Fix the Foundation

These are blockers. Nothing downstream can be trusted until they're resolved.

### 1.1 Fix Failing Scope Test

The scope isolation test `test_scope_cannot_access_different_project` is failing. This is the core invariant for project-scoped data — if it's broken, every downstream data operation is untrustworthy.

- [ ] Trace `can_access()` logic in `guard/src/scope.rs`
- [ ] The test creates `Scope::user_project("alice", "project-a")` and `Scope::user_project("bob", "project-b")` and asserts mutual inaccessibility
- [ ] Check whether `Scope::user_project()` actually sets the user dimension, or if the comparison logic only checks project
- [ ] Fix the bug, confirm test passes
- [ ] Add test for same-user cross-project (should be blocked) and same-project cross-user (should be allowed)

**Why this matters:** Scope isolation is Priority 2.3 in the IronClaw model. If the invariant is broken, the entire security model collapses.

### 1.2 Verify Clippy Clean

- [ ] Run `cargo clippy --workspace -- -D warnings`
- [ ] Fix any warnings
- [ ] Confirm `just check` passes

**Why this matters:** Codex sets the standard — zero warnings is non-negotiable. Without this, we can't trust anything else.

### 1.3 Update project_map.md

- [ ] project_map.md says "21 members" but the workspace has grown — verify against `cargo metadata`
- [ ] Update architecture diagram with `crabjar-architecture`, `axum-mux`, `crabjar-app-teams`
- [ ] Mark completed items with status indicators
- [ ] Remove `train-extract` if it's no longer a workspace member

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
- [ ] Add `crabjar-architecture` to CI gate (run on every PR)
- [ ] Document layer model in `crabjar-architecture/AGENTS.md` (already exists)
- [ ] Consider adding `crabjar-architecture` as a pre-commit hook
- [ ] Add `cargo-declared` integration to detect drift between declared and actual deps

### 2.2 Scope Isolation Model ✅ DONE (needs bug fix)

**Status:** Implemented in `guard/src/scope.rs` with `Scope` type covering identity, project, tenant, thread dimensions. `can_access()` enforces cross-scope authorization.

**What's next:**
- [ ] Fix the failing test (see 1.1)
- [ ] Add `CrossScopeAuth` approval flow (stub exists, needs implementation)
- [ ] Wire scope into `ExecutionGate::check()` — every action should carry a scope
- [ ] Add scope to `GuardDb` schema (persist scope with pending actions)

### 2.3 Requested-vs-Effective Trust Resolution ✅ DONE

**Status:** Implemented in `guard/src/trust_resolution.rs`. Policy chain: scope → project policy → user policy → default.

**What's next:**
- [ ] Audit: is `effective_trust` actually computed at every gate point, or only in some paths?
- [ ] Add audit trail: log how effective trust was derived (requested → policy → effective)
- [ ] Wire into `crabjar guard` CLI — show trust resolution chain in output

### 2.4 Exact-Invocation Fingerprint Approvals ❌ NOT STARTED

IronClaw's `ironclaw_approvals` uses SHA-256 fingerprints of exact tool invocations (command + args + context) for approval decisions. Crabjar's guard has coarse deny/pending/proceed.

- [ ] Design: fingerprint model — what goes into the hash? (command, args, env, working dir, scope?)
- [ ] Implement: `InvocationFingerprint` type + `SHA-256` computation
- [ ] Update guard: store fingerprint with pending actions, match on approval
- [ ] Prevent approval smuggling: approving `cp src dst` must not approve `cp src malicious`
- [ ] Lease-based approval with TTL

**Why this matters:** Pattern-based approvals are a false sense of security. Exact fingerprints close the gap.

### 2.5 Prompt Envelope (Instruction-Hijack Defense) ❌ NOT STARTED

IronClaw's `ironclaw_prompt_envelope` uses closed-vocabulary source labels and instruction-hijack rejection. The guard gate protects execution but not the prompt itself.

- [ ] Design: closed-vocabulary source labels (no free-text origin)
- [ ] Implement: instruction-hijack detection (reject prompts that inject commands into system instructions)
- [ ] Source attribution chain: every prompt token traces to its origin
- [ ] Prompt templates in files, loaded via `include_str!()`, never constructed inline

**Why this matters:** Prompt injection is the attack vector the guard gate can't see. The envelope protects the LLM's context before it reaches the gate.

### 2.6 Product Adapter Pattern ❌ NOT STARTED

IronClaw's product adapter system provides generic adapter abstraction for multi-product support (Telegram v2, Slack v2, etc.). Crabjar has channel-specific code (host-mqtt, host-graph) but no generic adapter layer.

- [ ] Design: `ProductAdapter` trait — normalize input → `IncomingMessage`, send output ← `OutgoingMessage`
- [ ] Implement: adapter registry for discovery and lifecycle
- [ ] Implement: channel-specific adapters (MQTT → Home Assistant, Graph API → Teams)
- [ ] New channels = new adapter, no core changes

**Why this matters:** Every new channel (Discord, Feishu, WeChat) currently requires core changes. The adapter pattern contains that blast radius.

---

## Priority 3: Codex Quality Constraints

Codex doesn't contribute architecture — it sets the standard. These are non-negotiable quality bars.

### 3.1 Linting as Policy Gates

Codex's `argument-comment-lint` proves that linting can enforce API standards programmatically.

- [ ] Domain allowlist — restrict which external tools/domains are callable (guard integration)
- [ ] Action policy — destructive actions require user permission (already partially done)
- [ ] Code quality gates — module size limits, async conventions
- [ ] Drift governance — detect when state-docs diverge from reality (already partially done via `skill-reference-store`)

### 3.2 Module Size Governance

Hard cap at 500 lines/module (excluding tests). New functionality → new module. Cognitive load management, not arbitrary bureaucracy.

- [ ] Add `cargo-declared` or custom script to report module sizes
- [ ] Identify modules exceeding 500 lines
- [ ] Split largest offenders
- [ ] Add to CI gate

### 3.3 Build Reproducibility

Cargo + `just` wrapper for deterministic builds. No Bazel — just discipline.

- [ ] Pin all dependency versions (already done via Cargo.lock)
- [ ] Add `just reproducible-build` target
- [ ] Document build reproducibility guarantees

---

## Priority 4: Agent Loop & Tool Protocol (EdgeCrab-Informed)

These are the conceptual patterns crabjar needs to replicate from EdgeCrab.

### 4.1 JSON-RPC Plugin Protocol

EdgeCrab's defining feature: process-isolated plugins communicating via JSON-RPC 2.0 over stdin/stdout pipes.

- [ ] Message schema for tool calls, results, errors
- [ ] Three-tier model: ToolServer (subprocess), Script (Rhai in-process), Reserved (WASM)
- [ ] Startup latency budget (~100ms), error recovery, lifecycle management
- [ ] Language-agnostic plugin execution

**Status:** Partially done. `orchestrator/lm_studio_client/` and `axum-mux/` (vm-bridge websocket relay) provide the transport. The full JSON-RPC plugin protocol is not implemented.

### 4.2 Agent Loop (ReAct)

EdgeCrab's `edgecrab-core` agent loop is the control plane crabjar needs:

- [ ] observe → understand → plan → execute → verify → reflect cycle
- [ ] State machine for loop transitions
- [ ] Context compression between turns (critical for long conversations)
- [ ] Model routing (which model for which phase)
- [ ] Decision flow: when to call tools vs. respond directly

**Status:** `host/host-agent/` exists with lifecycle docs. The actual ReAct loop is not implemented as a reusable crate.

### 4.3 Tool Registry

EdgeCrab's `edgecrab-tools` centralized registry pattern.

- [ ] Dynamic capability discovery
- [ ] Tool metadata (description, params, return types)
- [ ] Fallback chains for tool availability
- [ ] Versioned tool interfaces

**Status:** `tool_registry/` crate exists but was removed from workspace members. The pattern is referenced but not wired.

---

## Priority 5: Claw Code Patterns (Useful but Less Distinctive)

Claw Code is OpenAI + Anthropic patterns smashed together without a coherent philosophy. The schema-first discipline is good; the rest is just good engineering.

### 5.1 Declarative Subsystem Schemas

JSON schema format for tool definitions (input/output/execution context). Contract-first approach makes toolsets versionable and independently testable.

**Status:** Partially done via `rmcp` (MCP tool registry). Needs dedicated schema format.

### 5.2 Central Type Contract Layer

Single source of truth for all data structures. Useful but not unique — many projects do this. Implement when the pain of scattered types becomes real.

**Status:** Not started. Monitor for pain signals.

### 5.3 Session Store

Durable session state separate from execution logic.

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

### 8.1 Dual-Backend Persistence Abstraction

IronClaw has PostgreSQL + libSQL abstraction baked into every persistence crate. Crabjar uses rusqlite/bundled sqlite exclusively.

- [ ] `PersistenceBackend` trait: unified read/write interface
- [ ] SQLite backend (current) + PostgreSQL backend (future)
- [ ] Every persistence crate implements both backends
- [ ] Migration path: swap backend without changing business logic

**Why this matters:** Scaling from SQLite to PostgreSQL requires rewriting every persistence crate. An abstraction layer makes the migration a config change.

**Status:** Not started. Low priority until there's a scaling need.

---

## Priority 9: Codex Pattern Imports

Derived from parity analysis against OpenAI Codex (2026-06-23). Codex sets the quality bar; these patterns are worth importing into Crabjar's architecture.

### 9.1 Bounded Context Management

Codex's `context-fragments/` crate enforces hard token caps on everything injected into model context. No unbounded items, no items larger than 10K tokens, highlight items crossing 1k tokens as P0 requiring manual review.

- [ ] Design: `ContextFragment` type with token-bounded size + hard cap
- [ ] Implement: bounded injection API in knowledge store (`memory/`)
- [ ] Add: per-fragment token accounting (not byte counting)
- [ ] Add: P0 alert for fragments exceeding 1k tokens
- [ ] Wire into guard: reject actions that would produce unbounded context fragments

**Why this matters:** Crabjar's knowledge store has no token budget. Without bounded context, long conversations will silently degrade model quality. Codex's approach: everything injected must have a bounded size and a hard cap.

### 9.2 Module Size Governance

Codex enforces a hard cap at 500 lines/module (excluding tests). Modules exceeding ~800 LoC must extract new functionality into new modules. This prevents the codex-core bloat that Crabjar is already seeing.

- [ ] Add `cargo-declared` or custom script to report module sizes
- [ ] Identify modules exceeding 500 lines (current offenders)
- [ ] Split largest offenders — target `guard/` and `orchestrator/` first
- [ ] Add to CI gate as a codex-quality constraint
- [ ] Document module size rule in AGENTS.md (already in Codex's AGENTS.md)

**Why this matters:** Codex-core bloat is the anti-pattern Crabjar must avoid. The 500 LoC rule is cognitive load management, not bureaucracy.

### 9.3 Snapshot Testing for TUI

Codex uses `insta` extensively for TUI regression testing. Any change affecting user-visible UI must include corresponding insta snapshot coverage. This catches regressions in rendering, styling, and output format.

- [ ] Add `insta` to workspace dependencies
- [ ] Add snapshot tests for `codeburn` TUI output
- [ ] Add snapshot tests for `crabjar` CLI JSON output format
- [ ] Document snapshot testing workflow in AGENTS.md
- [ ] CI gate: fail on pending snapshots (require `cargo insta accept`)

**Why this matters:** Crabjar has no regression testing for structured output. Snapshot tests catch format drift before it reaches users. Codex's pattern: UI/text changes must update snapshots as part of the PR.

### 9.4 File Search Engine

Codex's `file-search/` crate provides BM25-based file indexing for agent context. Agents need to find relevant files without scanning the entire workspace.

- [ ] Design: `FileSearch` trait with BM25 indexing backend
- [ ] Implement: incremental file indexing (watch + poll)
- [ ] Implement: query API (keyword, fuzzy, path-based)
- [ ] Add: file relevance scoring with path-aware weighting
- [ ] Wire into knowledge store: `file_search` subcommand

**Why this matters:** Crabjar currently uses `ignore` for file traversal — no indexing, no ranking. For a 21-crate workspace, agents need fast, relevant file discovery. Codex's BM25 approach is battle-tested.

### 9.5 Starlark Execution Policy

Codex's `execpolicy/` uses Starlark (Go-Sanitized) for programmable command execution rules. More flexible than Crabjar's static guard deny/pending/proceed model.

- [ ] Design: `PolicyEngine` trait abstracting static vs. scriptable policies
- [ ] Evaluate: Go-Sanitized Starlark as execution policy language
- [ ] Implement: Starlark policy loader + sandboxed evaluator
- [ ] Add: policy hot-reload without binary restart
- [ ] Backward compat: static guard rules as default policy

**Why this matters:** Static deny/pending/proceed is sufficient for launch but too rigid for production. Starlark enables per-project, per-user, per-command policies without code changes. Codex's `execpolicy/` is the reference implementation.

---

## Priority 10: Developer Experience

### 10.1 ADR Process

EdgeCrab's `specs/` directory formalizes design decisions.

- [ ] `specs/ADR-NNN_<title>.md` template
- [ ] Decision context, options, rationale
- [ ] Cross-references between related ADRs

### 9.2 Config Layering

Multi-level configuration (defaults → user config → project config → CLI flags).

**Status:** Partially done via `.crabjar_config.toml`. Needs formalization.

---

## Open Questions

1. **WASM timeline:** When to invest in WASM plugin support vs. keeping it as a reserved slot?
2. **Model routing:** How to decide which model handles which phase of the ReAct loop?
3. **State-doc staleness:** What threshold triggers staleness warnings? (7 days? content changes?)
4. **Plugin language support:** Which languages for ToolServer plugins? (Rust, Python, Go?)
5. **Context compression strategy:** Summarization vs. selective retention vs. relevance scoring?
6. **Scope isolation granularity:** Which scope dimensions are needed at launch? (identity, project, tenant, thread)
7. **Boundary enforcement trigger:** CI-only gate or also pre-commit hook?
8. **Prompt envelope scope:** Protect all LLM prompts or only user-facing ones?
9. **VM bridge priority:** Is VM-based agent isolation worth the complexity, or should we start with Unix user sandboxing (already in `crabjar-sandbox`)?
10. **Dual-backend persistence:** Do we actually need PostgreSQL, or is SQLite sufficient for the foreseeable future?

---

*Last updated: June 23, 2026*
