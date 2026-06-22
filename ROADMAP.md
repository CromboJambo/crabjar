# Crabjar Roadmap

> Crabjar is trying to capture EdgeCrab's ecosystem in concept. Codex sets the quality bar. Claw Code is Frankenstein — useful patterns buried in noise.

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

## Priority 2: Codex Quality Constraints

Codex doesn't contribute architecture — it sets the standard. These are non-negotiable quality bars, not features to implement.

### 2.1 Linting as Policy Gates
Codex's `argument-comment-lint` proves that linting can enforce API standards programmatically. Map this to crabjar's guard system:

- Domain allowlist — restrict which external tools/domains are callable
- Action policy — destructive actions require user permission
- Code quality gates — module size limits, async conventions
- Drift governance — detect when state-docs diverge from reality

**Status:** Not started

---

### 2.2 Module Size Governance
Hard cap at 500 lines/module (excluding tests). New functionality → new module. This is cognitive load management, not arbitrary bureaucracy.

**Status:** Not started

---

### 2.3 Build Reproducibility
Cargo + `just` wrapper for deterministic builds. No Bazel — just discipline.

**Status:** Not started

---

## Priority 3: Claw Code Patterns (Useful but Less Distinctive)

Claw Code is OpenAI + Anthropic patterns smashed together without a coherent philosophy. The schema-first discipline is good; the rest is just good engineering.

### 3.1 Declarative Subsystem Schemas
JSON schema format for tool definitions (input/output/execution context). Contract-first approach makes toolsets versionable and independently testable.

**Status:** Not started

---

### 3.2 Central Type Contract Layer
Single source of truth for all data structures. Useful but not unique — many projects do this. Implement when the pain of scattered types becomes real.

**Status:** Not started

---

### 3.3 Session Store
Durable session state separate from execution logic. Useful but not distinctive.

**Status:** Not started

---

## Priority 4: Developer Experience

### 4.1 ADR Process
EdgeCrab's `specs/` directory formalizes design decisions. Crabjar needs the same:
- `specs/ADR-NNN_<title>.md` template
- Decision context, options, rationale
- Cross-references between related ADRs

**Status:** Not started

---

### 4.2 Config Layering
Multi-level configuration (defaults → user config → project config → CLI flags). EdgeCrab's `~/.edgecrab/config.yaml` is a good reference.

**Status:** Not started

---

## Open Questions

1. **WASM timeline:** When to invest in WASM plugin support vs. keeping it as a reserved slot?
2. **Model routing:** How to decide which model handles which phase of the ReAct loop?
3. **State-doc staleness:** What threshold triggers staleness warnings? (7 days? content changes?)
4. **Plugin language support:** Which languages for ToolServer plugins? (Rust, Python, Go?)
5. **Context compression strategy:** Summarization vs. selective retention vs. relevance scoring?

---

*Last updated: June 21, 2026*
