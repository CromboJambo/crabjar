# Crabjar Roadmap

This document tracks **upcoming** crabjar development priorities. Completed work moves to [CHANGELOG.md](./CHANGELOG.md).

## Current State (November 2026)

All P1 drop-in replacement blockers are complete. crabjar provides feature parity with Hermes Agent's core tooling suite. Recent focus has been on security hardening (ADR-045 red team analysis), clippy cleanup, and pre-release preparation.

See [FEATURE-PARITY-HERMES.md](./specs/FEATURE-PARITY-HERMES.md) for the complete gap analysis.

## Production Readiness Phase

### Performance Optimization & Benchmarking
- [ ] Performance benchmarking suite for execution pipeline
- [ ] Latency profiling across guard → concierge → telemetry path
- [ ] Memory usage optimization for long-running orchestrator instances

### Comprehensive Integration Test Matrix
- [ ] Cross-crate integration test coverage expansion
- [ ] E2E test slices wired into GitHub Actions (smoke on PR, full on merge/nightly)
- [ ] Regression test suite for security model changes

### Documentation Overhaul
- [ ] End-user documentation for TUI and CLI workflows
- [ ] API reference for conductor HTTP endpoints
- [ ] Deployment guides for fleet coordination setup

### CI/CD Pipeline Hardening
- [ ] Automated snapshot review in CI (insta baseline updates)
- [ ] Reproducible build verification on every PR
- [ ] Dependency security audit automation (cargo-audit integration)

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
