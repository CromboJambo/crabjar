# Crabjar Roadmap

> Crabjar is trying to capture EdgeCrab's ecosystem in concept. Codex sets the quality bar. Claw Code is Frankenstein — useful patterns buried in noise. IronClaw is the structural reference — mechanical boundary enforcement, scope isolation, and trust resolution at scale.

---

## Priority 1: Capture EdgeCrab's Architecture

These are the conceptual patterns crabjar needs to replicate. EdgeCrab is the target ecosystem.

### 1.1 JSON-RPC Plugin Protocol

EdgeCrab's defining feature: process-isolated plugins communicating via JSON-RPC 2.0 over stdin/stdout pipes. This is crabjar's core abstraction.

- Message schema for tool calls, results, errors
- Three-tier model: ToolServer (subprocess), Script (Rhai in-process), Reserved (WASM)
- Startup latency budget (~100ms), error recovery, lifecycle management
- Language-agnostic plugin execution

**Why this matters:** Process isolation is crabjar's stability guarantee. Every tool must be a potential failure point — subprocess boundaries contain blast radius.

**Status:** Not started

---

### 1.2 Agent Loop (ReAct)

EdgeCrab's `edgecrab-core` agent loop is the control plane crabjar needs:

- observe → understand → plan → execute → verify → reflect cycle
- State machine for loop transitions
- Context compression between turns (critical for long conversations)
- Model routing (which model for which phase)
- Decision flow: when to call tools vs. respond directly

**Status:** Not started

---

### 1.3 Tool Registry

EdgeCrab's `edgecrab-tools` centralized registry pattern:

- Dynamic capability discovery
- Tool metadata (description, params, return types)
- Fallback chains for tool availability
- Versioned tool interfaces

**Status:** Not started

---

## Priority 2: IronClaw Structural Patterns

IronClaw's architecture (80 crates, mechanical boundary enforcement, scope isolation) provides the structural discipline crabjar needs at scale. These are not features — they are governance mechanisms that prevent the workspace from collapsing into a tangle.

### 2.1 Mechanical Dependency Boundary Enforcement

IronClaw's `ironclaw_architecture` crate mechanically verifies that low-level crates never depend on high-level ones, enforced as a CI gate. At 21+ workspace members, crabjar is at the threshold where manual awareness breaks down.

- Define a dependency layering model (common → substrate → authority → runtime → agent → product → engine)
- Create `crabjar_architecture` crate with boundary tests
- Each layer declares which lower layers it may import
- CI fails if a crate violates its layer's import rules
- Prevents circular dependencies and high-level leakage

**Why this matters:** Without mechanical enforcement, the workspace grows into an unmanageable graph. Clippy catches code style but not architectural drift.

**Status:** Not started

---

### 2.2 Scope Isolation Model

IronClaw enforces identity, project, tenant, and thread boundaries as first-class type-level constructs. Crabjar has project-scoped config but no formal scope isolation layer.

- `Scope` type with identity, project, tenant, thread dimensions
- Every data operation requires a scope parameter — no blind defaults
- Cross-scope operations require explicit authorization
- Prevents data leakage between projects at the type level

**Why this matters:** The guard gate protects execution but not data. Without scope isolation, a compromised tool can read/write across project boundaries.

**Status:** Not started

---

### 2.3 Requested-vs-Effective Trust Resolution

IronClaw's `ironclaw_trust` distinguishes between what a tool *requests* and what the *effective* trust level is after policy resolution. Crabjar's guard has a simpler deny/pending/proceed model.

- `requested_trust` vs `effective_trust` on every action
- Policy resolution chain: scope → project policy → user policy → default
- Trust decay tracking across the action pipeline
- Audit trail showing how effective trust was derived

**Why this matters:** A tool requesting high trust doesn't mean it should get it. The resolution layer is the actual authorization decision.

**Status:** Not started

---

### 2.4 Exact-Invocation Fingerprint Approvals

IronClaw's `ironclaw_approvals` uses fingerprints of exact tool invocations (not just "allow cp") for approval decisions. Crabjar's guard persists pending actions but the approval model is coarser.

- SHA-256 fingerprint per tool invocation (command + args + context)
- Approval tied to exact fingerprint, not command pattern
- Prevents approval smuggling (approving `cp src dst` doesn't approve `cp src malicious`)
- Lease-based approval with TTL

**Why this matters:** Pattern-based approvals are a false sense of security. Exact fingerprints close the gap.

**Status:** Not started

---

### 2.5 Prompt Envelope (Instruction-Hijack Defense)

IronClaw's `ironclaw_prompt_envelope` uses closed-vocabulary source labels and instruction-hijack rejection. The guard gate protects execution but not the prompt itself.

- Closed-vocabulary labels for prompt sources (no free-text origin)
- Instruction-hijack detection: reject prompts that inject commands into system instructions
- Source attribution chain: every prompt token traces to its origin
- Prompt templates in files, loaded via `include_str!()`, never constructed inline

**Why this matters:** Prompt injection is the attack vector the guard gate can't see. The envelope protects the LLM's context before it reaches the gate.

**Status:** Not started

---

### 2.6 Per-Crate AGENTS.md Routing Maps

IronClaw has `AGENTS.md` and `CLAUDE.md` in every crate, giving AI coding assistants navigation context at the crate level. Crabjar has one AGENTS.md at the root — fine for 21 members but doesn't scale.

- Template for per-crate `AGENTS.md` (purpose, public API, dependencies, pitfalls)
- Auto-generate from Cargo.toml + doc comments
- AI coding assistant navigation at the crate level

**Why this matters:** Root-level AGENTS.md doesn't help an agent working inside `host/host-mqtt/`. Per-crate context reduces cross-referencing and misnavigation.

**Status:** Not started

---

### 2.7 Product Adapter Pattern

IronClaw's product adapter system provides generic adapter abstraction for multi-product support (Telegram v2, Slack v2, etc.). Crabjar has channel-specific code (host-mqtt, host-graph) but no generic adapter layer.

- `ProductAdapter` trait: normalize input → `IncomingMessage`, send output ← `OutgoingMessage`
- Adapter registry for discovery and lifecycle
- Channel-specific adapters implement the trait
- New channels = new adapter, no core changes

**Why this matters:** Every new channel (Discord, Feishu, WeChat) currently requires core changes. The adapter pattern contains that blast radius.

**Status:** Not started

---

## Priority 3: Codex Quality Constraints

Codex doesn't contribute architecture — it sets the standard. These are non-negotiable quality bars, not features to implement.

### 3.1 Linting as Policy Gates

Codex's `argument-comment-lint` proves that linting can enforce API standards programmatically. Map this to crabjar's guard system:

- Domain allowlist — restrict which external tools/domains are callable
- Action policy — destructive actions require user permission
- Code quality gates — module size limits, async conventions
- Drift governance — detect when state-docs diverge from reality

**Status:** Not started

---

### 3.2 Module Size Governance

Hard cap at 500 lines/module (excluding tests). New functionality → new module. This is cognitive load management, not arbitrary bureaucracy.

**Status:** Not started

---

### 3.3 Build Reproducibility

Cargo + `just` wrapper for deterministic builds. No Bazel — just discipline.

**Status:** Not started

---

## Priority 4: Claw Code Patterns (Useful but Less Distinctive)

Claw Code is OpenAI + Anthropic patterns smashed together without a coherent philosophy. The schema-first discipline is good; the rest is just good engineering.

### 4.1 Declarative Subsystem Schemas

JSON schema format for tool definitions (input/output/execution context). Contract-first approach makes toolsets versionable and independently testable.

**Status:** Not started

---

### 4.2 Central Type Contract Layer

Single source of truth for all data structures. Useful but not unique — many projects do this. Implement when the pain of scattered types becomes real.

**Status:** Not started

---

### 4.3 Session Store

Durable session state separate from execution logic. Useful but not distinctive.

**Status:** Not started

---

## Priority 5: Developer Experience

### 5.1 ADR Process

EdgeCrab's `specs/` directory formalizes design decisions. Crabjar needs the same:
- `specs/ADR-NNN_<title>.md` template
- Decision context, options, rationale
- Cross-references between related ADRs

**Status:** Not started

---

### 5.2 Config Layering

Multi-level configuration (defaults → user config → project config → CLI flags). EdgeCrab's `~/.edgecrab/config.yaml` is a good reference.

**Status:** Not started

---

## Priority 6: Testing Infrastructure (IronClaw Patterns)

IronClaw's testing discipline is a pattern worth adopting directly.

### 6.1 E2E Slice Testing

IronClaw's E2E test matrix (smoke vs full) lets CI run fast on PRs and thorough on merges.

- Smoke slice: core agent loop, tool execution, channel delivery
- Full slice: all channels, all sandboxes, all trust layers
- CI runs smoke on every PR; full on merge/nightly

**Status:** Not started

---

### 6.2 Replay Snapshots

IronClaw's `scripts/replay-snap.sh` enables deterministic testing by replaying recorded LLM traces.

- Record LLM response traces as fixtures
- Replay fixtures in tests (no external LLM dependency)
- Regression detection: diff new responses against snapshots

**Status:** Not started

---

## Priority 7: Persistence Architecture (IronClaw Patterns)

### 7.1 Dual-Backend Persistence Abstraction

IronClaw has PostgreSQL + libSQL abstraction baked into every persistence crate. Crabjar uses rusqlite/bundled sqlite exclusively.

- `PersistenceBackend` trait: unified read/write interface
- SQLite backend (current) + PostgreSQL backend (future)
- Every persistence crate implements both backends
- Migration path: swap backend without changing business logic

**Why this matters:** Scaling from SQLite to PostgreSQL requires rewriting every persistence crate. An abstraction layer makes the migration a config change.

**Status:** Not started

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

---

*Last updated: June 21, 2026*
