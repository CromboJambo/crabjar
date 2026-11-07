---
id: ADR-007
title: Three-State Decision Model for Conductor Routing Logic
status: Proposed
date: 2026-10-22
see_also: [[ADR-001]], [[ADR-006]]
---

# ADR-007: Three-State Decision Model for Conductor Routing Logic

## Status

Proposed — under discussion as the formal decision model for conductor routing logic.

## Context

Conductor currently makes binary decisions (approve/reject, true/false) when evaluating whether to route tasks forward, trigger guard approvals, or escalate failures. This binary model is insufficient for several classes of decisions:

1. **Uncertain evaluations**: The agent has partial information and cannot confidently approve or reject — it needs more data before deciding.
2. **Conditional outcomes**: Decisions that depend on factors not currently available (environment state, upstream task completion, resource availability).
3. **Escalation vs. rejection**: When a decision fails, should it be rejected outright or escalated to a human? Binary logic conflates these distinct failure modes.

The ChatGPT session 6ada6edb-9c48-83ea-a505-3b93dfdb6264 articulated this as: "Negative one, zero, and one... pull, push, or equal parts." This maps directly to a ternary decision space where decisions can be negative (reject/withdraw), neutral (hold/pending), or positive (approve/forward).

Binary logic forces premature commitment. A conductor that must choose approve/reject when it genuinely doesn't know will either over-approve (risk) or over-reject (stall). The three-state model explicitly represents uncertainty as a first-class decision outcome rather than hiding it inside one of the binary states.

## Decision

Replace binary decision outcomes in conductor routing logic with a three-state model:

```rust
enum Decision {
    Approve,   // +1 — forward/push/accept
    Pending,   // 0 — hold/wait for more information
    Reject,    // -1 — withdraw/reject/deny
}
```

The conductor evaluates decisions and returns one of these three states. Routing logic must handle all three explicitly; `Pending` is not a fallback but an intentional outcome that triggers waiting or escalation behavior.

### Decision evaluation function signature

```rust
fn evaluate_decision(context: &DecisionContext) -> Decision {
    // ... evaluation logic ...
}
```

### Pending semantics

When the conductor returns `Pending`:
- The task does not advance
- A "waiting for decision" state is recorded in the attempt graph
- Optionally, a timeout or escalation trigger is set (e.g., escalate to human after N minutes)
- The decision may be re-evaluated when new information arrives

### Mapping to existing patterns

The three-state model subsumes existing binary patterns:

| Binary pattern | Three-state equivalent |
|---|---|
| approve/reject | Approve/Reject |
| pass/fail | Approve/Reject (or Pending for inconclusive) |
| true/false guard | Approve/Reject/Pending |
| escalate vs. auto-approve | Escalate → Pending; auto-approve → Approve |

## Options Considered

### Option A: Binary with confidence score

Return `(Decision, confidence: f32)` where low confidence implies uncertainty. Rejected because:
- Confidence is orthogonal to the decision itself — you can be confident in a rejection or uncertain about an approval
- Forces callers to interpret confidence thresholds, which vary by context

### Option B: Error-based pending

Use `Result<Decision, DecisionError>` where errors represent "can't decide." Rejected because:
- Errors are for exceptional conditions, not intentional outcomes
- Conflates "decision failed" with "decision is pending"

### Option C: Four-state model (Approve/Reject/Pending/Escalate)

Separate `Pending` from `Escalate`. Rejected because:
- Escalation is a response to Pending, not a separate decision outcome
- The conductor decides whether to escalate based on timeout/context; the decision itself is still "I don't know yet" (Pending)

### Option D: Three-state model (chosen)

Approve/Pending/Reject with explicit Pending semantics. Chosen because:
- Directly maps to the natural language intuition ("pull, push, or wait")
- Minimal state space that captures all meaningful decision outcomes
- Forces conductor logic to explicitly handle uncertainty rather than hiding it
- Compatible with existing binary patterns (binary is a special case of ternary)

## Consequences

### Positive consequences

- **Explicit uncertainty**: Pending makes uncertainty visible rather than buried in approve/reject choices
- **Better escalation handling**: Pending naturally triggers timeout/escalation logic
- **Cleaner conductor logic**: Decision functions can return "I don't know" without fabricating a binary answer
- **Attempt graph fidelity**: The attempt graph records genuine decision states, not approximations

### Negative consequences (trade-offs)

- **More routing code**: Callers must handle three outcomes instead of two
- **Timeout management**: Pending decisions require timeout/escalation infrastructure
- **Risk of indefinite pending**: Without proper timeouts, tasks may sit in Pending forever

### Ongoing concerns

- Monitor for "Pending abuse" — conductors that default to Pending to avoid making decisions
- Ensure timeout/escalation logic is robust and tested
- Track decision latency (how long decisions take to resolve from Pending)

## Implementation sketch

```rust
// In conductor routing logic:
match evaluate_decision(&ctx) {
    Decision::Approve => forward_task(task),
    Decision::Reject => reject_task(task, reason),
    Decision::Pending => {
        record_pending_attempt(task.id);
        schedule_reevaluation(task.id, timeout);
    }
}

// Timeout handler escalates:
fn on_decision_timeout(task_id) {
    escalate_to_human(task_id, "decision pending too long");
}
```

## References

- Nygard, M. (2011). *Documenting Architecture Decisions*. https://www.infoq.com/articles/Architecture-Decision-Lang
- `specs/README.md` — ADR process and conventions for this project
- ChatGPT session 6ada6edb-9c48-83ea-a505-3b93dfdb6264 — original articulation of three-state decision intuition