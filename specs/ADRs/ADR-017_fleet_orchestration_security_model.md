# ADR-017: Fleet Orchestration Security Model

**Date:** October 28, 2026  
**Status:** Accepted  
**Context:** Production deployment of crabjar-conductor requires a security model to prevent unauthorized task submission, worker impersonation, and state tampering.

## Decision

Implement a four-layer capability-based access control model for the conductor service:

1. **Network Authentication**: SSH key-based mutual authentication + TLS for HTTP API
2. **API Authorization**: JWT-like tokens with embedded capabilities, trust level, and expiration
3. **Task-Level Capabilities**: Tasks declare required hardware/capability requirements; workers must prove they have them before claiming
4. **Trust Level Annealing**: Workers earn higher trust levels through successful task completion, enabling access to more sensitive tasks

## Rationale

### Why capability-based over role-based?

The fleet is heterogeneous — workers have different hardware (GPU vs CPU-only), capabilities, and trust histories. Role-based access control (RBAC) assumes homogeneous actors with static roles, which doesn't fit an agent swarm where:
- Workers dynamically join/leave the fleet
- Trust is earned over time, not assigned by admin
- Task requirements vary based on hardware needs

Capability-based access aligns with the crabjar philosophy of "tools are capabilities" — workers prove they can do a task by demonstrating they have the necessary resources.

### Why reuse guard's ExecutionGate patterns?

The guard crate already implements:
- Trust layer enforcement (quarantine → working → annealed)
- Confidence threshold validation (0.65 floor)
- Provenance verification (source event tracking)
- GateResult enum (Proceed/Interrupted/Pending/Escalated)

Reusing these patterns ensures consistency across the crabjar security model and avoids duplicating authorization logic. The conductor's `conductor_gate.rs` module wraps guard's types with fleet-specific semantics:
- Conductor trust levels map to guard layers (0→1, 1→2, 2+→3)
- Task claim eligibility replaces command execution as the gated action
- Worker capabilities replace tool permissions

### Why annealing over static trust?

Static trust levels require manual admin intervention to promote workers. In a self-managing fleet:
- New workers should start with limited privileges (quarantine/unknown)
- Successful task completion should automatically increase trust
- Failed tasks should decrease trust (with hysteresis to prevent oscillation)
- Idle workers should decay in trust (stale = less trusted)

This enables the fleet to self-heal: a compromised worker that fails tasks loses privileges without admin intervention.

## Implementation Details

### Trust Level Mapping

| Conductor Level | Guard Layer | Meaning | Task Access |
|----------------|-------------|---------|-------------|
| 0 (unknown) | 1 (quarantine) | New/compromised worker | Sandboxed tasks only |
| 1 (proven) | 2 (working) | Completed <10 tasks | Standard tasks |
| 2 (trusted) | 3 (annealed) | Completed 10-50 tasks | Elevated tasks |
| 3 (elite) | 3 (annealed) | >50 tasks, no failures | All tasks |

### Token Structure

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

### Capability Matching Rules

- **Exact match**: `"cpu:8core"` matches `"cpu:8core"`
- **Wildcard**: `"gpu:nvidia:*"` matches `"gpu:nvidia:3070ti"`, `"gpu:nvidia:4090"`
- **Hierarchical**: `"disk:100gb"` satisfies `"disk:*"` (any disk)

### Annealing Thresholds

- **Upgrade**: Worker trust level increases after completing 10 successful tasks (proven → trusted) and 50 tasks with no failures for 7 days (trusted → elite)
- **Downgrade**: Failed tasks decrease trust level with hysteresis (requires multiple consecutive failures)
- **Idle Decay**: Workers inactive for >30 days have their trust level reduced by one

## Alternatives Considered

### Alternative 1: OAuth2/OpenID Connect

Use standard OAuth2 flows with an identity provider. Rejected because:
- Requires external infrastructure (identity provider server)
- Overkill for internal fleet orchestration
- Doesn't support hardware capability claims natively

### Alternative 2: mTLS with X.509 Certificates

Use mutual TLS with certificate-based authentication. Rejected because:
- Certificate management complexity (issuance, rotation, revocation)
- Doesn't encode capabilities or trust levels in the auth mechanism
- Better suited for network-layer security (Layer 1), not application authorization

### Alternative 3: Role-Based Access Control (RBAC)

Assign static roles to workers (admin, worker, observer). Rejected because:
- Roles are manually assigned, not earned
- Doesn't scale with dynamic fleet membership
- Can't express hardware capability requirements

## Consequences

### Positive
- Workers can self-register and earn trust without admin intervention
- Task assignment is validated against actual worker capabilities
- Compromised workers automatically lose privileges through annealing
- Consistent security model across conductor and guard crates

### Negative
- Token management adds complexity to conductor API
- Capability spoofing possible if SSH key verification isn't enforced at registration
- Trust level gaming could occur (e.g., completing trivial tasks to earn trust)

### Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Token theft | Short expiration (1 hour), auto-refresh, revocation on heartbeat miss |
| Capability spoofing | SSH key verification at registration; periodic hardware validation |
| Trust level gaming | Require minimum time between upgrades; track failure rate not just count |
| Single point of failure | Conductor state is SQLite-backed; can be replicated later |

## References

- `crabjar-conductor/src/conductor_gate.rs` — Execution gate implementation
- `crabjar-conductor/src/auth.rs` — Token generation and validation
- `guard/src/gate.rs` — Guard execution gate patterns
- `guard/src/trust.rs` — Trust layer definitions
- `specs/security-model-capability-based-access-control.md` — Original spec
