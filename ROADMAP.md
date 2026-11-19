# Crabjar Roadmap

This document tracks **upcoming** crabjar development priorities. Completed work moves to [CHANGELOG.md](./CHANGELOG.md).

## Current State (November 2026)

**Full P1 + P2 tool parity achieved.** All 25 core Hermes Agent tools at ✅ — crabjar is a drop-in replacement for Hermes Agent's tool surface. Version 0.13.0 shipped with complete first-class implementations of execute_code, terminal, file tools (read/write/patch/search), delegate_task (full spawn/list/steer/stop/continue lifecycle), session_search (FTS5), and all P2 productivity tools.

**v0.14.0 adds the Waypoint Navigation System** — durable navigational objects for agent context management (`crates/memory/src/waypoints/`). Implements WAYPOINT-SYSTEM spec (P1): SQLite-backed waypoints with categories, recoverability states, freshness checks, and nearest-neighbor orientation. 16 unit tests, all under 500 LoC CI gate.

**Test suite fully stabilized.** Fixed pre-existing compilation and runtime failures across conductor, vision, guard, and terrarium crates (edition 2024 module resolution, missing migrations, broken test data). Full workspace test suite now passes with zero failures.

New crates added: `crabjar-vision` (image analysis), `crabjar-exec` (Rust code execution engine), `crabjar-browser-vault` (Bitwarden credential bridge). Module size governance enforced via CI gate (`just module-sizes-check`, 500 LoC limit) — all 11 oversized modules split.

See [FEATURE-PARITY-HERMES.md](./specs/FEATURE-PARITY-HERMES.md) for the complete parity assessment. Active architectural specs in `specs/ACTIVE/`: LINEAGE-PROVENANCE (P1), POSITIVE-BOUNDARY-MODEL (P1), LOSS-ACCOUNTING (P2), OBSERVER-WORKER-STATE (P2), CRATE-PUBLISHING-STRATEGY (operational).

## Production Readiness Phase

### Performance Optimization & Benchmarking
- [x] Performance benchmarking suite for execution pipeline — **complete** (commit 61d9ec2)
  - Baseline: gate_check_happy_path avg 1192ns, domain allowlist +209ns overhead
  - Trust layers have no significant latency impact
- [x] Memory profiling benchmark — **complete** (commit f735e46)
  - Initial spike of ~2MB (SQLite DB + first-time allocations), then stable
  - No memory leak detected across 10,000 operations
  - Domain allowlist and context budget add only +16KB each after 10k ops
- [x] High-risk command latency analysis — **complete** (commit a4eeef3)
  - Command risk assessment adds virtually no latency (~30ns overhead)
  - Original "100μs max" variance was warmup noise, not real performance characteristic
  - Many args (100 files) adds ~660ns due to string join in risk assessment
- [ ] Latency profiling across guard → concierge → telemetry path
- [ ] Memory usage optimization for long-running orchestrator instances

### Comprehensive Integration Test Matrix
- [x] Cross-crate integration test coverage — **complete** (commit c2562ff)
- [x] E2E test slices in GitHub Actions — **complete**: smoke on PR, full nightly schedule (commit e27bf1b)
- [x] Regression test suite for security model changes — **complete** (ADR-045 red team tests, commit 4a2ae08)

### Documentation Overhaul
- [ ] End-user documentation for TUI and CLI workflows
- [ ] API reference for conductor HTTP endpoints
- [ ] Deployment guides for fleet coordination setup

### CI/CD Pipeline Hardening
- [x] Dependency security audit automation — **complete** (cargo-audit job on every PR, commit e415b24)
- [ ] Automated snapshot review in CI (insta baseline updates)
- [ ] Reproducible build verification on every PR

## Nice-to-Have Features (P3 Differentiation)

These capabilities would differentiate crabjar from Hermes but are not required for drop-in replacement. See [FEATURE-PARITY-HERMES.md](./specs/FEATURE-PARITY-HERMES.md) for details.

### P3-1: Image Generation Tools
**Status:** Planned  
**Hermes equivalent:** `image_generate` tools  
**CrabJar gap:** No image generation; agents cannot create visual content

### P3-2: Desktop Project Management
**Status:** Planned  
**Hermes equivalent:** `desktop_project` tools  
**CrabJar gap:** No desktop project management; agents cannot organize or manipulate project files on desktop

### P3-3: Mirror-log Attention Tracking System
**Status:** Planned  
**Hermes equivalent:** `mirror-log-mcp` tools (15 tools for attention logging/observation tracking)  
**CrabJar gap:** No observation/attention tracking; agents cannot track their own attention patterns or user engagement

---

## Historical Phases (Completed — see CHANGELOG.md)

All phases through November 2026 are complete. See [CHANGELOG.md](./CHANGELOG.md) for detailed completion records including:

- **Pre-Release Hardening & Agent Loop Expansion** (July–November 2026)
- **Memory Crate Refactoring** (July 2026)
- **vm-bridge Integration Phase** (October 2026)
- **Fleet Coordination Phase** (October 2026)
- **Code Review Pipeline** (October 2026)
- **P1 Drop-in Replacement Blockers** (October–November 2026) — all 8 complete
- **P2 Productivity Improvements** (November 2026) — memory compat, Home Assistant tools, skill ecosystem expansion
