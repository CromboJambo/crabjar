---
id: ADR-016
title: Worker Promotion Provenance Model
status: Proposed
date: 2026-10-27
supersedes: null
see_also: [ADR-008, evidence-based-promotion.md]
---

# ADR-016: Worker Promotion Provenance Model

## Status

Proposed — architecture documented for crabjar's tiered worker promotion system with increasing provenance guarantees at each stage.

## Context

Guard blocks based on "lack of provenance" prevent agents from executing trusted actions when they cannot verify the identity and history of the executor. This creates friction in automated workflows where ephemeral workers need to build trust over time.

Forces:
- Ephemeral task workers have no persistent identity or history
- Guard decisions require provenance verification for non-trivial actions
- Workers must demonstrate reliability before gaining autonomy
- Need measurable progression from untrusted to trusted execution contexts

## Decision

Implement a five-tier worker promotion lifecycle where each tier provides stronger provenance guarantees and identity persistence:

| Tier | Identity | Persistence | Provenance | Guard Impact |
|------|----------|-------------|-----------|--------------|
| 1. Task Worker | Container ID | None (ephemeral) | Execution log only | Blocks non-trivial actions |
| 2. Gig Worker | Persistent container | Container state | Task history + container ID | Allows moderate-risk actions |
| 3. Dinit Service | System service name | dinit config | Service logs + uptime | Allows most actions |
| 4. Shared User | OS user account | ~/.crabjar/ | Shell history + cron logs | Minimal guard intervention |
| 5. Crabjar User | Dedicated crabjar user | Full user environment | Complete audit trail | Automatic approval |

## Tier Details

### Tier 1: Task Worker (Ephemeral)
- Identity: Random container ID (e.g., `task_worker_abc123`)
- Execution context: Podman container with task-specific volume mounts
- Provenance: Execution log tied to container ID and task metadata
- Guard behavior: Blocks actions requiring provenance; requires manual approval for non-trivial operations

### Tier 2: Gig Worker (Persistent Container)
- Identity: Persistent container name (e.g., `gig_worker_alpha`)
- Execution context: Podman container with persistent volumes and state
- Provenance: Task history + container ID + execution patterns over multiple tasks
- Guard behavior: Allows moderate-risk actions based on accumulated task history

### Tier 3: Dinit Service (System-Level)
- Identity: System service name registered with dinit init system
- Execution context: Host shell environment with service-specific config
- Provenance: Service logs, uptime metrics, scheduled execution history
- Guard behavior: Allows most actions; service identity provides strong provenance

### Tier 4: Shared User (OS-Level)
- Identity: OS user account (e.g., `crabjar_worker`)
- Execution context: Full user environment with shell history and cron logs
- Provenance: Complete shell history, cron execution logs, file ownership trails
- Guard behavior: Minimal intervention; OS-level identity provides strong provenance

### Tier 5: Crabjar User (Dedicated)
- Identity: Dedicated crabjar user account with independent config directories
- Execution context: Full user environment with crabjar-specific configuration
- Provenance: Complete audit trail across all crabjar operations
- Guard behavior: Automatic approval for most actions; provenance is comprehensive

## Implementation

The promotion system is implemented in `sandbox/src/promotion.rs` with the following components:

### WorkerManager Struct
```hemlock
pub struct WorkerManager {
    db: Connection,
    sandbox_config: SandboxConfig,
}
```

### Promotion Methods
- `spawn_task_worker(task_id)` — Create ephemeral task worker (Tier 1)
- `promote_to_gig_worker(worker_id)` — Promote to persistent gig worker (Tier 2)
- `promote_to_dinit_service(worker_id, service_name)` — Promote to dinit service (Tier 3)
- `revert_to_task_worker(worker_id)` — Demote back to task worker tier

### Database Schema
Worker metadata stored in `agent_sandboxes` table with columns:
- `worker_id` — Unique identifier for the worker instance
- `isolation_type` — Current promotion tier (task/gig/service)
- `provenance_level` — Numeric provenance level (1-5)
- `promotion_history` — JSON array of promotion/demotion events with timestamps

## Guard Integration

The guard system queries the WorkerManager to determine the current provenance level for each execution context:

```hemlock
let worker = WorkerManager::get_worker(worker_id);
if (worker.provenance_level < required_level) {
    return BlockReason("insufficient_provenance");
}
```

Guard decision logic incorporates provenance level as a factor in risk assessment, allowing higher-tier workers to execute actions that would block lower-tier workers.

## Alternatives Considered

**Option A: Single trust level for all workers**
Simple but provides no incentive for workers to build provenance history. Rejected because it doesn't solve the guard blocking problem.

**Option B: Binary trusted/untrusted model**
Two-state system where workers are either fully trusted or not trusted at all. Rejected because it's too coarse-grained; workers should gain trust incrementally.

**Option C: Five-tier provenance model (chosen)**
Provides granular progression with measurable criteria at each tier. Each promotion requires demonstrated reliability and provides stronger identity guarantees that guards can verify.

## Consequences

### Positive consequences
- Workers build provable history over time, reducing guard interventions
- Clear progression path from ephemeral to persistent execution contexts
- Guard decisions become more accurate as provenance improves
- Automated workflows can operate with less manual approval overhead

### Negative consequences (trade-offs)
- New workers start at lowest trust level (Tier 1), requiring initial manual approvals
- Promotion requires demonstrated reliability, which takes time and successful task completion
- Demotion possible if worker reliability decreases

## Implementation Status

Core promotion logic implemented in `sandbox/src/promotion.rs`. Guard integration pending — guard decision logic needs to query WorkerManager for provenance levels. Database schema defined but not yet deployed to production environments.

## References

- ADR-008: Podman-based agent sandboxing architecture
- evidence-based-promotion.md: Evidence-based promotion specification
- crabjar AGENTS.md: Agent isolation and execution constraints documentation</content>