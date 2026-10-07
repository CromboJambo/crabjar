# Crabjar Roadmap

This document tracks **upcoming** crabjar development priorities. Completed work moves to [CHANGELOG.md](./CHANGELOG.md).

## Current Active Work (July 2026)

### Pre-Release Hardening & Agent Loop Expansion

Final cleanup phase before first public release. Focus: eliminate all remaining technical debt, expand the agent loop with production-grade capabilities, and establish proper documentation processes.

**Completed:**
- ✅ All personal paths, secrets, offensive language, TODO stubs removed
- ✅ Fake return values replaced with proper error types
- ✅ Zero clippy errors across workspace
- ✅ Release build succeeds without errors
- ✅ No secrets in binary (verified via strings analysis)
- ✅ ADR process established (specs/ADR-001)

**In Progress:**
- [ ] **Conversational TUI** — ratatui-based interactive chat interface with session persistence, message history, scrollback, and guard approval display. SQLite-backed session store (`host-binary/tui/session.rs`). Terminal panel wrapping crabjar-terminal for live terminal view within TUI.
- [ ] **Model routing** (Phase 1) — `model_routing.rs` implements ModelRouter with phase-specific backend routing via `LoopPhase` enum (plan/reflect → HTTP, others → heuristic). Per-phase configuration in agent_config.json.
- [ ] **Context compression** (Phase 2) — `context_compression.rs` implements ContextCompressor for token-budgeted context management. Groups older observations by stage/kind to preserve decision-relevant history while shrinking prompt size.
- [ ] **Decision gate** (Phase 3) — `decision_gate.rs` implements DecisionGate for tool call vs direct response decisions. Auto-decide threshold and max tool calls per turn configurable via agent_config.json.

**Upcoming:**
- [ ] Domain allowlist enforcement in ExecutionGate step 10 with per-trust-layer configuration
- [ ] Command risk catalog (`command_risk.rs`) — static classification of HIGH/MEDIUM_RISK_COMMANDS (rm -rf, chmod 777, etc.)
- [ ] Risk config system (`risk_config.rs`) — configurable risk levels and thresholds
- [ ] Context budgeting (`context_budget.rs`) — ContextBudget + MAX_TOKENS_PER_FRAGMENT enforcement in ExecutionGate step 10
- [ ] Static policy engine (`policy.rs`, `policy_types.rs`) — TOML-based declarative policies for pre-execution checks

**Deliverables:** Release-ready binary, ADR process established, proper testing infrastructure, documented agent loop with model routing/compression/decision-gate capabilities.

### Memory Crate Refactoring
- [ ] Split `indexer.rs` (705 LoC) into `extract.rs` (~400 LoC: markdown parsing) and `insert.rs` (~144 LoC: SQLite writes) to satisfy 500 LoC rule
- Fix pre-existing schema/insert column mismatches (`doc_metadata` vs `documents`, `doc_id` vs `doc_path`)

### crabjar-plugin Crate
- [ ] Scaffold WASM plugin runtime crate in `crabjar-plugin/` (deferred until concrete use case)

## Upcoming Phases

### vm-bridge Integration Phase
- [ ] Complete vm-bridge integration with actual VM lifecycle management
- [ ] Implement persistent storage layer for long-term memory retention
- [ ] Build multi-agent coordination framework
- [ ] Develop security model with capability-based access control
- [ ] Create observability stack (metrics, tracing, logging)

### Production Readiness Phase
- [ ] Performance optimization and benchmarking suite
- [ ] Comprehensive integration test matrix
- [ ] Documentation overhaul for end users
- [ ] Deployment automation and CI/CD pipeline hardening

---

## Historical Phases (Completed — see CHANGELOG.md)

All phases through July 10, 2026 are complete. See [CHANGELOG.md](./CHANGELOG.md) for detailed completion records including:

- **Phase 1: Foundation & Core Infrastructure** — Workspace setup, crabjar-core crate, Cargo workspace configuration
- **Phase 2: Execution Environment & Guard System** — VM lifecycle management, execution gate, trust layers
- **Phase 3: Agent Orchestration Layer** — Agent lifecycle, task scheduling, result aggregation
- **Phase 4: Host Integration & Telemetry** — Host binary, F(1) daemon, telemetry pipeline, CLI interface
- **ADR Process Establishment** — specs/ADR-001 through ADR-003
