# ADR-013: Layered Topological Maps Implementation Lessons

**Date:** 2026-10-19  
**Status:** Accepted  
**Context:** Implementation of layered topological maps architecture across four phases, validated against multiple real-world Rust workspaces.

## Decision

Document implementation lessons learned from building and validating the layered topological maps system for systematic codebase analysis.

## Rationale

The implementation spanned 12 commits over several sessions, with significant debugging required to resolve integration issues between the JEV decision layer and the structural analysis components. These lessons should inform future architectural work and reduce iteration time on similar projects.

## Lessons Learned

### 1. API Surface Stability is Critical for Cross-Crate Integration

The most time-consuming issue was the mismatch between `EvaluationTarget::from_source()` (expected by workspace.rs) and the actual API (`build_target(SpanKind, ...)`). This occurred because:
- The API evolved during implementation but callers weren't updated
- No integration tests existed to catch the drift

**Mitigation:** Always run integration tests after API changes. Prefer stable, well-documented APIs over evolving ones when multiple components depend on them.

### 2. Borrow Checker Issues in Graph Construction Require Careful Design

Multiple borrow checker errors (E0382) arose during dependency graph construction because:
- Collecting edges while iterating over crates created conflicting borrows
- The natural algorithm (iterate and add edges) doesn't map cleanly to Rust's ownership model

**Mitigation:** When building graphs, collect all data first in independent structures, then apply transformations. Avoid mutating collections during iteration.

### 3. Test Fixture Cleanup Must Be Explicit

The `tempdir` crate auto-cleans up directories when the `TempDir` object is dropped, which caused tests to fail because files were deleted before inspection. The fix was to use `dir.into_path()` to prevent cleanup.

**Mitigation:** For test fixtures that need to persist beyond function scope, explicitly convert to owned paths rather than relying on RAII cleanup.

### 4. Embedding Fallback is Expected Behavior for Mismatched Criteria

When testing against the Bun runtime (107 crates), all evaluations fell back to embedding with 0.0 confidence. This wasn't a bug — it was the system correctly identifying that the error-handling criteria patterns didn't match Bun's code style.

**Mitigation:** Design criteria files to be codebase-specific or provide multiple variants for different coding styles. Document expected fallback behavior in validation reports.

### 5. Budget-Aware Analysis is Essential for Large Workspaces

The crabjar workspace (5 crates) analyzed instantly, but Bun (107 crates) took ~97 seconds. Without budget controls, analysis of very large workspaces could become impractical.

**Mitigation:** Always provide budget parameters (max_crates, max_time) and make them configurable by the caller. Default to reasonable limits for interactive use.

### 6. Circular Dependency Detection Works on Real-World Code

The system correctly identified a circular dependency in Bun's parser/sourcemap/printer modules (`bun_js_printer -> bun_sourcemap -> bun_parsers`). This validates that the cycle detection algorithm works on production code, not just synthetic test cases.

**Mitigation:** Use real-world workspaces for validation, not just toy examples. Real code has edge cases that tests miss.

### 7. Orphaned Crate Detection Identifies Unused Modules

The system found 9 orphaned crates in Bun — modules that nothing depends on. This is valuable architectural insight that could guide cleanup or refactoring efforts.

**Mitigation:** Include orphan detection as a standard part of workspace analysis. It's computationally cheap and provides actionable insights.

## Impact

These lessons inform future work on:
- API design for cross-crate integration
- Graph construction algorithms in Rust
- Test fixture management
- Criteria file design for diverse codebases
- Performance optimization for large-scale analysis

## Alternatives Considered

- Ignoring API drift and fixing it later (led to more debugging time)
- Using unsafe code for graph construction (introduces memory safety risks)
- Skipping real-world validation in favor of synthetic tests (misses edge cases)
