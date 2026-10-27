# Project Consolidation and Simplification Spec

**Status:** Draft  
**Created:** 2026-10-27  
**Scope:** ~/projects/ directory — all active and reference repos

## Problem Statement

The projects directory has accumulated multiple repos with overlapping responsibilities, unclear boundaries, and duplicated concepts. This creates maintenance overhead, confusion about where features belong, and "productivity theater" risk (working on the wrong repo for a given problem). The goal is genuine architectural simplification — eliminating real complexity through proper consolidation — not superficial flattening that hides complexity behind merged codebases.

## Principles

1. **Consolidate by responsibility, not convenience:** Merge repos only when they share a core responsibility, not just because it's easier to maintain one repo.
2. **Preserve clear boundaries:** After consolidation, each remaining project should have a well-defined scope with explicit interfaces to other projects.
3. **Eliminate duplicated concepts:** If two repos implement similar patterns (e.g., trust levels, knowledge compression), consolidate into one canonical implementation.
4. **Reference vs active distinction:** Clearly separate repos that are learning sources from repos that are actively developed and maintained.

## Current State Analysis

### Crombo's Active Projects
- **crabjar** — Agent orchestration framework (main focus)
- **home-lab** — Home lab infrastructure management

### Reference/Inspiration Repos
- **hermes-source** — Hermes Agent architecture reference (not crombo's work)
- **crepuscularity** — Rust language/runtime project (reference/learning)
- **pesti-backup-old-checkout** — Old pesti version (archive candidate)

### Early-Stage/Template Repos
- **mirror-lab** — Transparent log audit path (placeholder commits, early stage)
- **Agendarator** — Scheduling/tracking responsibilities
- **agent-oversight** — Agent monitoring tooling
- **user_coarses** — Education/training content

### Other Projects
- **pesti** — Portable Execution Substrate for Transformer Inference (separate concern)
- **herdr** — Fleet monitoring tool
- **ELYTerminal** — Terminal application
- **experiments** — Various experiments
- **docs** — Documentation
- **state-docs** — State documentation

## Consolidation Plan

### Phase 1: Absorb agent-oversight into crabjar (High Priority) — COMPLETED

**Rationale:** Evidence-based promotion and agent oversight share the same core responsibility — evaluating agent performance. Maintaining them separately creates artificial boundaries and duplicated evaluation logic.

**Finding:** The agent-oversight Rust component (`src/main.rs`) was a stub ("Hello, world!"). The meaningful work is in `world-model-optimizer/`, a Python-based benchmarking/evaluation suite with environment capture packages (appworld, bird-sql, crmarena, gaia2, swe-bench, etc.).

**Actions taken:**
1. Archived standalone `agent-oversight` repo to `~/projects/archive/`
2. Noted that world-model-optimizer Python suite could be referenced later for evaluation infrastructure needs in crabjar's promotion system

**Acceptance Criteria:**
- [x] Standalone agent-oversight repo archived
- [ ] World-model-optimizer evaluation patterns documented as reference for future crabjar evaluation infrastructure

### Phase 2: Integrate mirror-lab as crabjar's audit infrastructure (Medium Priority) — COMPLETED

**Rationale:** Mirror-lab's transparent log audit path maps directly to the knowledge compression triad's "supporting evidence" component. Crabjar needs an audit trail that provides useful evidence without leaking sensitive data.

**Actions taken:**
1. Added `mirror-log` as path dependency to crabjar-conductor (using local ~/projects/active/mirror-lab/mirror-log)
2. Created `crabjar-conductor/src/audit.rs` module wrapping mirror-log for conductor operations
3. Implemented audit event types: TaskCompleted, TaskFailed, WorkerPromoted, WorkerDemoted, GoalDecomposed, RouteDecision
4. Added helper functions: `log_task_completion()`, `log_route_decision()`

**Remaining:**
- [ ] Integrate audit logging hooks into conductor's task completion flow
- [ ] Create API endpoints for querying audit trails: `GET /audit/tasks/{id}`, `GET /audit/workers/{id}`
- [ ] Update knowledge compression spec to reference crabjar's internal audit module

**Finding:** Published mirror-log 0.1.9 uses rusqlite 0.32 which conflicts with crabjar's rusqlite 0.37. Resolved by using local path dependency (mirror-lab workspace already uses rusqlite 0.37).

**Acceptance Criteria:**
- [x] mirror-log integrated as conductor dependency
- [x] Audit module created with event types and logging functions
- [ ] Audit hooks integrated into task completion flow
- [ ] API endpoints for audit trail queries implemented

### Phase 3: Archive or relocate reference/early-stage repos (Low Priority)

**Rationale:** Reference and early-stage repos clutter the active projects directory and create confusion about what's being actively developed.

**Actions:**
1. Move `hermes-source` to `~/projects/reference/` (learning source, not active development)
2. Move `crepuscularity` to `~/projects/reference/` (language/runtime learning)
3. Move `pesti-backup-old-checkout` to `~/projects/archive/` (old version, superseded)
4. Evaluate remaining early-stage repos:
   - **Agendarator:** If not actively developed, move to `~/projects/archive/` or merge into home-lab if relevant
   - **user_coarses:** If education content for crabjar users, consider moving to `crabjar/docs/tutorials/`
   - **experiments:** Keep as-is but add README documenting purpose of each experiment

**Acceptance Criteria:**
- Active projects directory contains only actively developed projects
- Reference repos clearly separated from active development
- Archive contains superseded or completed work

### Phase 4: Define clear project boundaries (Ongoing)

**Rationale:** After consolidation, remaining projects need explicit scope definitions to prevent future drift.

**Actions:**
1. Update `crabjar/AGENTS.md` with explicit scope statement and "not in scope" list
2. Update `home-lab/AGENTS.md` with explicit scope statement
3. Create `~/projects/README.md` documenting the purpose of each top-level directory (active, reference, archive)
4. Document integration points between projects (e.g., crabjar uses home-lab for machine provisioning)

**Acceptance Criteria:**
- Each active project has a clear scope statement
- Integration points between projects are documented
- New contributors can understand which repo to work on for a given problem

## What Should Remain Separate

### pesti
Portable Execution Substrate for Transformer Inference is a separate concern from agent orchestration. It provides LLM inference capabilities that crabjar uses, but has its own development lifecycle and architectural concerns. Keep as independent project.

### home-lab
Home lab infrastructure management is separate from agent orchestration. Crabjar runs on the home-lab fleet, but the two have different responsibilities: home-lab manages machines/services, crabjar manages agents/tasks. Keep as independent project with documented integration points.

### herdr
Fleet monitoring tool serves a different purpose than crabjar's conductor. Herdr monitors infrastructure health; crabjar orchestrates agent work. Keep separate unless clear overlap emerges during consolidation.

## Reversibility Notes

All consolidation actions are reversible:
- Code moved into crabjar can be extracted back into standalone repos if needed
- Directory reorganization (reference/archive) is trivially reversible
- Archive operations preserve git history for future reference

The key is to maintain git history through the consolidation — use `git subtree add` or similar techniques when moving code between repos rather than simple copy/paste.

## Success Metrics

- [ ] Phase 1: agent-oversight fully integrated into crabjar, standalone repo archived
- [ ] Phase 2: mirror-lab audit infrastructure integrated into crabjar knowledge compression system
- [ ] Phase 3: Reference and early-stage repos moved to appropriate directories
- [ ] Phase 4: Clear project boundaries documented for all active projects
- [ ] End-to-end: New contributor can understand which repo to work on for any given problem

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Over-consolidation creates monolith | Preserve module boundaries within crabjar; use separate crates for distinct responsibilities |
| Lost git history during consolidation | Use `git subtree` or similar techniques that preserve commit history |
| Breaking existing integrations | Test all external integrations after each phase; maintain backward-compatible APIs where possible |
| Scope creep during consolidation | Stick to the phased plan; don't add new features during consolidation, just reorganize existing functionality |
