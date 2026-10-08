# Crabjar Roadmap

This document tracks **upcoming** crabjar development priorities. Completed work moves to [CHANGELOG.md](./CHANGELOG.md).

### Pre-Release Hardening & Agent Loop Expansion

Final cleanup phase before first public release. Focus: eliminate all remaining technical debt, expand the agent loop with production-grade capabilities, and establish proper documentation processes.

**Completed:**
- ✅ All personal paths, secrets, offensive language, TODO stubs removed
- ✅ Fake return values replaced with proper error types
- ✅ Zero clippy errors across workspace
- ✅ Release build succeeds without errors
- ✅ No secrets in binary (verified via strings analysis)
- ✅ ADR process established (specs/ADR-001)

**Completed (September 2026):**
- ✅ Conversational TUI — ratatui-based interactive chat with session persistence, scrollback, guard approval display, habitat panel
- ✅ Model routing (`model_routing.rs`, 438 LoC) — phase-specific backend routing via `LoopPhase` enum
- ✅ Context compression (`context_compression.rs`, 385 LoC) — token-budgeted context management with observation grouping
- ✅ Decision gate (`decision_gate.rs`, 469 LoC) — tool call vs direct response decisions with auto-decide threshold

**Completed (July 2026):**
- ✅ Domain allowlist enforcement in ExecutionGate step 9 with per-trust-layer configuration
- ✅ Command risk catalog (`command_risk.rs`) — static classification of HIGH/MEDIUM_RISK_COMMANDS
- ✅ Risk config system (`risk_config.rs`) — configurable risk levels and thresholds
- ✅ Context budgeting (`context_budget.rs`) — ContextBudget + MAX_TOKENS_PER_FRAGMENT enforcement in ExecutionGate step 10
- ✅ Static policy engine (`policy.rs`, `policy_types.rs`) — TOML-based declarative policies for pre-execution checks

**Completed (October 2026):**
- ✅ crabjar-conductor crate scaffolded with core orchestration types (Goal, Task, Worker), state store abstraction, HTTP API layer
- ✅ vm-bridge deployed to ftw3 home-lab node as dinit service on port 8090
- ✅ Orchestrator fine-tuning pipeline documented (docs/training/orchestrator-fine-tuning.md)

**Upcoming:**
- [ ] Implement persistent storage layer for conductor (SQLite-backed state store)
- [ ] Build multi-agent coordination framework (conductor pattern on ftw3)
- [ ] Develop security model with capability-based access control
- [ ] Create observability stack (metrics, tracing, logging)

**Deliverables:** Release-ready binary, ADR process established, proper testing infrastructure, documented agent loop with model routing/compression/decision-gate capabilities.

### Memory Crate Refactoring
- ✅ Split `indexer.rs` into `extract.rs` (400 LoC) and `insert.rs` (152 LoC) to satisfy 500 LoC rule
- ✅ Fixed pre-existing schema/insert column mismatches (`doc_metadata` vs `documents`, `doc_id` vs `doc_path`)

### crabjar-plugin Crate
- ✅ Scaffolded WASM plugin runtime crate in `crabjar-plugin/` (stub, deferred pending concrete use case)

## Upcoming Phases

### vm-bridge Integration Phase
- ✅ Scaffolded `vm/vm-core` crate with domain types (VmId, VmState, VmSpec), lifecycle events, typed error hierarchy
- ✅ Implemented LibvirtBackend stub with libvirt FFI bindings for VM lifecycle operations
- ✅ Built VmManager high-level API with policy integration hooks and state caching
- ✅ Unit tests for domain types pass (5/5)
- ✅ Integrate vm-core with axum-mux bridge for HTTP/WebSocket control plane (lifecycle.rs: /vms endpoints, LifecycleState, mounted in supervisor)
- ✅ Deployed to ftw3 home-lab node as dinit service on port 8090

### Fleet Coordination Phase (Conductor Pattern)
- ✅ Scaffolded `crabjar-conductor` crate with core orchestration types and HTTP API layer
- [ ] Implement persistent storage layer for conductor (SQLite-backed state store)
- [ ] Build multi-agent coordination framework (deploy conductor to ftw3 as central node)
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
