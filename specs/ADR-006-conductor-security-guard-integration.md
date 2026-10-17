# ADR-006: Conductor Security Model — Guard Crate Integration

**Status:** Accepted  
**Date:** 2026-10-16  
**Author:** CromboJambo (with Hermes Agent)  
**Context:** crabjar-conductor v0.12.0 security model implementation

## Decision

The Crabjar Conductor integrates with the `crabjar-guard` crate for its authorization and trust-layer enforcement, rather than maintaining a separate security subsystem. This provides:

1. **Unified provenance verification** — Task assignment decisions use the same ExecutionGate patterns as crabjar's broader execution pipeline
2. **Consistent trust layer semantics** — Conductor trust levels (0-3) map directly to guard's quarantine/working/annealed layers
3. **Shared policy evaluation** — Future declarative policies can be evaluated through guard's PolicyEngine trait

## Alternatives Considered

### A. Standalone conductor security module
- Rejected: Would create parallel authorization logic, increasing maintenance burden and risk of semantic drift

### B. Full ExecutionGate instantiation for every task claim
- Rejected as initial approach: Requires provenance database setup; would block basic fleet operations during development
- Adopted as Phase 5 target: Once conductor has its own provenance store, full gate checks will be enabled

### C. Token-based auth only (no trust layers)
- Rejected: Trust annealing is a core crabjar principle; removing it loses the "earn access" model that makes fleet orchestration safe

## Implementation Details

### Trust Level Mapping

| Conductor Level | Guard Layer | Meaning | Auto-Assign? |
|-----------------|-------------|---------|--------------|
| 0 (unknown) | 1 (quarantine) | New worker, unproven | No — manual review required |
| 1 (proven) | 2 (working) | Completed tasks successfully | Yes — standard tasks |
| 2+ (trusted) | 3 (annealed) | Long-term reliable performance | Yes — critical tasks |

### Integration Points

1. **`conductor/src/gate.rs`** — New module providing:
   - `conductor_level_to_guard_layer()` — Level mapping function
   - `trust_sufficient_for_auto_assign()` — Pre-assignment trust check
   - `can_claim_task()` — Full gate check (Phase 5 target)

2. **`conductor/src/conductor.rs::tasks_for_worker()`** — Now filters by:
   - Trust level (must be ≥ task requirement AND sufficient for auto-assign)
   - Capability requirements (existing logic)

3. **Token scopes aligned with guard layers** — JWT claims use `trust_level` field matching conductor's 0-3 scale, which maps to guard's layer system at authorization time.

### What This Enables

- **Safe task assignment** — Unknown workers can't auto-claim tasks until proven
- **Annealing integration** — Successful task completion upgrades trust level and JWT scope automatically
- **Audit trail** — Trust level changes are logged with timestamps (existing)
- **Future policy engine** — Declarative policies can reference guard's layer system

### Open Questions / Future Work

- Should conductor have its own provenance store, or rely on a shared one?
- How to handle SSH key verification against known_hosts in multi-worker environments?
- Token revocation on heartbeat miss (currently TTL only)

## References

- `specs/security-model-capability-based-access-control.md` — Full security model spec
- `guard/src/gate.rs` — ExecutionGate implementation
- `guard/src/trust_types.rs` — Trust layer definitions
- crabjar ADR-004: The Glass (abstraction boundary)
