# Crabjar Security Model: Capability-Based Access Control

**Status:** Draft  
**Date:** October 16, 2026  
**Author:** Agent (scoped from roadmap)

## Problem Statement

The conductor service (`crabjar-conductor`) deployed on ftw3:8091 has no security model. Any client can:
- Submit goals that get executed by fleet workers
- Register fake workers and steal tasks
- Read/modify any goal or task state
- Impersonate other workers via heartbeats

This is a blocker for production deployment of the conductor service.

## Design Principles

1. **Capability-based**: Permissions are tied to tokens/capabilities, not identity alone
2. **Least privilege**: Workers get only what they need to execute their assigned tasks
3. **Monotonic trust**: Trust can be earned (annealing) but never automatically revoked
4. **Defense in depth**: Multiple layers — network auth, API auth, task-level capabilities

## Security Layers

### Layer 1: Network Authentication (Worker ↔ Conductor)

**Mechanism:** SSH key-based mutual authentication + TLS for HTTP API

- Workers authenticate to conductor using SSH keys (already have `ssh_key` field in Worker struct)
- Conductor validates worker identity via public key verification
- HTTP API uses client certificates or bearer tokens derived from SSH key pairs

```hemlock
// Worker registration flow
worker → POST /workers/register { name, ssh_public_key, capabilities }
conductor → verifies key, stores, issues auth token
worker → uses auth token for all subsequent requests
```

### Layer 2: API Authorization (Token-Based)

**Mechanism:** JWT-like tokens with embedded capabilities and expiration

Token structure:
```json
{
  "sub": "worker-uuid",
  "name": "ftw3",
  "capabilities": ["gpu:nvidia:3070ti", "cpu:8core"],
  "trust_level": 2,
  "scope": ["tasks:claim", "tasks:report", "heartbeat"],
  "exp": 1765900800
}
```

API endpoint authorization matrix:

| Endpoint | Required Scope | Auth Type |
|----------|---------------|-----------|
| POST /goals | `goals:create` | User token |
| GET /goals/{id} | `goals:read` | Any authenticated |
| POST /workers/register | None (first-time) | SSH key verification |
| POST /workers/{id}/heartbeat | `heartbeat` | Worker token |
| GET /tasks/for-worker/{id} | `tasks:claim` | Worker token |
| POST /tasks/{id}/result | `tasks:report` + task ownership | Worker token |
| POST /review | `review:submit` | User token |

### Layer 3: Task-Level Capabilities

**Mechanism:** Tasks carry required capabilities; workers prove they have them

```hemlock
// Task definition includes capability requirements
let task = Task {
    command: "train-model.sh",
    required_capabilities: ["gpu:nvidia:*", "disk:100gb"],
    trust_level_required: 2,
    // ...
};

// Worker claims task — conductor validates capabilities match
if !worker.capabilities.satisfies(task.required_capabilities) {
    return TaskClaimError::InsufficientCapabilities;
}
```

Capability matching rules:
- Exact match: `"cpu:8core"` matches `"cpu:8core"`
- Wildcard: `"gpu:nvidia:*"` matches `"gpu:nvidia:3070ti"`, `"gpu:nvidia:4090"`
- Hierarchical: `"disk:100gb"` satisfies `"disk:*"` (any disk)

### Layer 4: Trust Level Annealing

**Mechanism:** Workers earn higher trust levels through successful task completion

Trust levels (reusing existing `trust_layer` concept):
- **Level 0**: New worker, untrusted — can only run sandboxed tasks
- **Level 1**: Completed <10 tasks successfully — limited filesystem access
- **Level 2**: Completed 10-50 tasks successfully — standard network/file access
- **Level 3**: Completed >50 tasks with no failures for 7 days — elevated privileges

```hemlock
// Trust annealing on task completion
fn on_task_complete(worker_id, success) {
    let worker = get_worker(worker_id);
    if (success) {
        worker.successful_tasks++;
        if (worker.should_upgrade_trust()) {
            upgrade_trust_level(worker);
            issue_new_token(worker);  // Token now has higher trust_level
        }
    } else {
        worker.failed_tasks++;
        if (worker.should_downgrade_trust()) {
            downgrade_trust_level(worker);
        }
    }
}
```

## Implementation Plan

### Phase 1: Worker Authentication (Week 1)
- [ ] Add SSH key verification to `/workers/register`
- [ ] Implement token generation/validation middleware for all endpoints
- [ ] Store worker tokens with expiration in SQLite
- [ ] Test: worker can't claim tasks without valid token

### Phase 2: Task-Level Capabilities (Week 2)
- [ ] Add `required_capabilities` field to Task struct and schema migration
- [ ] Implement capability matching logic (exact + wildcard)
- [ ] Validate capabilities on task claim, not just assignment
- [ ] Test: worker with wrong capabilities can't claim task

### Phase 3: Trust Level Annealing (Week 3)
- [ ] Add `trust_level`, `successful_tasks`, `failed_tasks` to Worker struct
- [ ] Implement annealing logic in conductor's task completion handling
- [ ] Issue new tokens on trust level changes
- [ ] Test: worker earns higher trust after successful tasks

### Phase 4: Integration with Guard Crate (Week 4)
- [ ] Reuse `ExecutionGate` patterns for conductor authorization
- [ ] Align token scopes with guard's `trust_layer` values
- [ ] Share policy engine if applicable
- [ ] Write ADR documenting the security model

## Data Model Changes

### Worker table additions
```sql
ALTER TABLE workers ADD COLUMN trust_level INTEGER NOT NULL DEFAULT 0;
ALTER TABLE workers ADD COLUMN successful_tasks INTEGER NOT NULL DEFAULT 0;
ALTER TABLE workers ADD COLUMN failed_tasks INTEGER NOT NULL DEFAULT 0;
ALTER TABLE workers ADD COLUMN auth_token TEXT UNIQUE;
ALTER TABLE workers ADD COLUMN token_expires_at TEXT;
```

### Task table additions
```sql
ALTER TABLE tasks ADD COLUMN required_capabilities TEXT NOT NULL DEFAULT '[]';
ALTER TABLE tasks ADD COLUMN trust_level_required INTEGER NOT NULL DEFAULT 0;
```

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Token theft | Short expiration (1 hour), auto-refresh, revocation on heartbeat miss |
| Capability spoofing | SSH key verification at registration; conductor validates hardware claims periodically |
| Trust level gaming | Require minimum time between upgrades; track failure rate not just count |
| Single point of failure (conductor) | Conductor state is SQLite-backed; can be replicated later |

## Decisions

1. **Separate auth layers**: Conductor and guard have separate but similar authorization layers, both pulling from the same environment pool (shared credential/token sources)
2. **Self-reported capabilities with earned trust**: Workers self-report capabilities; conductor trusts them initially. Trust is earned over time through successful task completion (like tools/skills). If self-reporting is poor (task failures due to missing capabilities), conductor schedules check-ins and re-evaluates capability claims.
3. **Token refresh**: Implicit on heartbeat — each successful heartbeat extends token validity.

## References

- Existing trust layer system: `guard/src/trust.rs`, `crabjar-tool-binding/src/lib.rs`
- Conductor deployment: `scripts/deploy-conductor-ftw3.sh`
- Fleet coordination ADRs: `specs/ADR-002_herdr_as_execution_substrate.md`
