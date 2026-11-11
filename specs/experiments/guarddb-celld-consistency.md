# GuardDb ↔ celld Consistency Boundary

**Status:** Design spec — ready for minimal implementation
**Date:** 2026-11-10
**Context:** crabjar needs session state to survive machine lifecycle changes without compromising its authorization model.

## Problem

crabjar has two places where pending authorization state can live:

1. **GuardDb** (local SQLite) — the authority for trust, approvals, execution history
2. **celld cells** (remote, persistent across node lifecycle) — durable mirror that survives machine death

When they disagree, what happens? Who wins? How do we reconcile without loops or implicit authorization?

## Core Principle

> **GuardDb is the local authority. Celld is a durable mirror.**
> 
> Writes go GuardDb-first. Reads prefer local. Reconciliation converges deterministically. Recovery never authorizes.

This follows the glass principle: GuardDb (trust, guard) is inside crabjar's stable substrate. celld is outside — disposable, replaceable behind a contract.

## What Gets Mirrored

**To celld:**
- `action_requests` rows with `status = 'pending'`
- Corresponding `handoffs` envelope (full context for rehydration)
- Status changes (pending → approved/denied/executed/interrupted)

**NOT mirrored (local-only):**
- `memory_nodes`, `memory_edges` — machine-local knowledge graph
- `trust_layers`, `anneal_config` — local trust policy
- `pid_trust`, `revoked_log` — per-process, ephemeral
- `action_transitions` — the full audit trail stays local (too large for cells); only the final status is mirrored

## Reconciliation Protocol

### Write Path (crabjar → both)

```
1. Stage action as pending:
   a. INSERT into GuardDb.action_requests (status='pending')
   b. POST to celld cell (mirror)
   c. If (b) fails: log warning, continue. GuardDb is source of truth.
      Queue celld write for retry on next reconciliation.

2. Decision made (approve/deny):
   a. UPDATE GuardDb.action_requests (status='approved'/'denied')
   b. INSERT into GuardDb.action_transitions (audit trail)
   c. PUT to celld cell (update mirror)
   d. If (c) fails: same as above — retry later.
```

**Key:** GuardDb write always happens first and is never blocked by celld availability.

### Read Path

```
- Pending actions: read from GuardDb (fast, local)
- Fallback: if GuardDb is empty/corrupted, read from celld cell
- Decided actions: GuardDb only (celld mirror may be stale)
```

### Reconciliation (on startup + periodic heartbeat)

```
For each action_id in (GuardDb.pending ∪ celld.pending):
  
  Case A: Both agree on status → nothing to do
  
  Case B: GuardDb says decided, celld says pending
    → Update celld to match GuardDb (GuardDb wins for decisions)
  
  Case C: Celld says decided, GuardDb says pending
    → DANGEROUS CASE. Check GuardDb.action_transitions for this action_id.
      - If transition record exists with matching authority: trust GuardDb's audit trail, update celld
      - If NO transition record: flag for human review. Do NOT auto-accept.
        (This is the "did someone approve it behind our back?" check)
  
  Case D: Action exists only in celld (lost from GuardDb)
    → Rehydrate into GuardDb from the handoff envelope
    → Mark metadata: {"recovered_from": "celld", "recovered_at": timestamp}
    → Status remains whatever celld says (pending stays pending)
  
  Case E: Action exists only in GuardDb (celld write never succeeded)
    → Mirror to celld now (catch-up write)

Reconciliation is idempotent: running N times converges to the same state.
```

## The Guarantees

### 1. Recovery ≠ Authorization
A pending action stays pending across any number of crashes, restarts, and reconciliations. Only an explicit decision with a recorded authority changes status. (Proven by `celld_auth_failure_test.rs` ✅)

### 2. Dropping a Tool Has No Effect
If celld is unreachable:
- crabjar operates normally on GuardDb alone
- No blocking, no loops, no retry storms
- When celld comes back: next reconciliation does a one-time catch-up
- Maximum drift window = time between last successful mirror and celld outage

### 3. Idempotent Convergence
Every write is keyed by `action_id` (UUID). Upsert semantics mean:
- Writing the same action twice → same result
- Rehydrating an already-present action → no-op
- Reconciliation converges regardless of how many times it runs

### 4. Audit Trail Integrity
The `action_transitions` table is append-only and local. It's the ground truth for "who decided what, when, why." Celld mirrors only the final status, not the full audit trail. If celld claims a decision but GuardDb has no transition record → human review required.

## Session-Lifecycle Contract (the glass)

crabjar depends on celld through this interface only:

```rust
trait SessionMirror {
    /// Mirror a pending action to the durable store.
    /// Returns Ok(()) or an error — caller decides whether to retry.
    fn mirror_pending(&self, action: &PendingAction, envelope: &HandoffEnvelope) -> Result<(), MirrorError>;
    
    /// Update the status of a previously mirrored action.
    fn update_status(&self, action_id: &str, status: &ActionStatus) -> Result<(), MirrorError>;
    
    /// List all pending actions in the durable store (for reconciliation/recovery).
    fn list_pending(&self) -> Result<Vec<PendingAction>, MirrorError>;
    
    /// Get a specific action's state (for rehydration).
    fn get_action(&self, action_id: &str) -> Result<Option<MirroredAction>, MirrorError>;
}
```

**Implementation:** `HttpSessionMirror` — talks to the approval-queue worker via HTTP + JSON. No celld Rust SDK needed. If we ever replace celld with something else (MinIO + custom store, PostgreSQL, whatever), only this implementation changes. The trait is the contract.

## Implementation Plan

1. **Define `SessionMirror` trait** in `crates/guard/src/mirror.rs`
2. **Implement `HttpSessionMirror`** — HTTP client to the approval-queue worker
3. **Add mirror hook to GuardDb write path** — after INSERT/UPDATE, call `mirror_pending`/`update_status` (fire-and-forget with retry queue)
4. **Add reconciliation to startup** — compare GuardDb.pending with `list_pending()`, apply the protocol above
5. **Test:** 
   - Write to both, kill celld, restart, verify reconciliation converges
   - Corrupt GuardDb, verify rehydration from celld works
   - Verify Case C (celld says decided, GuardDb doesn't) flags for review

## Files

- Spec: `specs/experiments/guarddb-celld-consistency.md` (this file)
- Trait + impl: `crates/guard/src/mirror.rs` (new)
- Worker: `~/.local/share/celld-test/approval-queue/` (already running on port 8788)
- Test worker: `specs/experiments/celld_auth_failure_test.rs` (authority failure — passed ✅)
