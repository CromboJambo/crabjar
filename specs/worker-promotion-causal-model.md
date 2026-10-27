# Worker Promotion Causal Model

**Status:** Draft  
**Created:** 2026-10-27  
**Related:** ADR-016 (Worker Promotion Provenance), evidence-based-promotion.md, ADR-008 (Podman sandboxing)

## Problem Statement

Crabjar's worker promotion model currently treats trust, capability, and responsibility as independent dimensions that workers gain through promotion. This framing misses the causal relationship between them:

1. **Trust + Capability → Responsibility:** Greater autonomy and ability to act lead to broader scope of ownership
2. **Responsibility → Perception of Opportunity:** Expanded responsibility changes what problems become visible and relevant
3. **Perception → New Evidence:** Pursuing perceived opportunities generates evidence for further promotion

The current model lacks an architectural expression of how promotion changes a worker's *epistemic position* — not just its permissions, but its field of view.

## Causal Model

```
Trust + Capability ──→ Responsibility ──→ Perception of Opportunity ──→ New Work/Evidence
     ↑                        │                            │                   │
     └────────────────────────┴───────────────────────────┘───────────────────┘
                              Feedback Loop
```

### Trust + Capability → Responsibility

Trust (verified provenance, reliability) and capability (specialized tools, demonstrated competence) combine to justify greater responsibility. The system delegates more because it has evidence the worker can handle it.

**Architectural expression:** Guard policies map provenance levels to execution scope. Higher provenance = broader allowed operations.

### Responsibility → Perception of Opportunity

This is the critical causal link. When a worker's responsibility expands from "fix this compiler error" to "maintain this crate," new problems become visible and relevant:

- **Task worker view:** One broken test, one error message
- **Supervisor view:** Patterns across workers, systemic bottlenecks
- **Manager view:** Architectural trade-offs, cross-service dependencies

The opportunities didn't disappear — they were irrelevant until responsibility made them salient. Responsibility changes what exists in the worker's world.

**Architectural expression:** Promotion should expand the worker's context window — what state-docs, provenance records, and dependency graphs are visible to it. A promoted worker gains access to a wider scope of system knowledge.

### Perception → New Evidence

As workers perceive more opportunities, they pursue them and generate new evidence:

- Success/failure data on higher-complexity tasks
- Decision accuracy at broader scopes
- Knowledge contributions (compressed rules, patterns)
- Peer evaluations from coordination

This evidence feeds back into trust and capability assessments for the next promotion cycle.

## Design Principles

### 1. Promotion Changes Context, Not Just Permissions

Promotion is not merely granting more execution rights. It's changing what the worker can see and therefore what it can do. The architectural expression:

- **Lower tiers:** Narrow context (own task, own container)
- **Higher tiers:** Broader context (other workers' provenance, historical outcomes, system-level state-docs)

### 2. Opportunity Perception is Valuable in Its Own Right

A worker can be promoted partly because it reliably identifies important work, even before it's authorized to execute that work. This suggests three distinct events:

- **Observe:** Worker identifies an opportunity
- **Propose:** Worker explains potential benefit, cost, risk
- **Own:** Worker receives responsibility and authority to pursue it

Lower-level workers can contribute discoveries without needing the privileges to implement every discovery they perceive.

### 3. Brakes Must Scale with Speed

As workers gain more autonomy (faster), verification and rollback mechanisms must scale proportionally (better brakes). A promoted worker that perceives systemic opportunities needs stronger safeguards than a task worker fixing one compiler error.

**Architectural expression:** Guard approval flows, provenance requirements, and rollback mechanisms should be tiered — higher-tier workers face stricter verification for high-stakes operations.

### 4. Trust is Evidence of History, Not Proof of Correctness

Provenance establishes what happened, not whether it was right. A promoted worker with comprehensive audit trails can still make consistently bad decisions. Promotion must account for outcome quality, not just record completeness.

## Integration with Existing Architecture

### Guard Layers

The causal model maps to crabjar's existing guard trust layers:

| Trust Layer | Worker Tier | Context Window | Opportunity Perception |
|-------------|-------------|----------------|------------------------|
| Layer 1 (Quarantine) | Task Worker | Own execution only | Single task anomalies |
| Layer 2 (Working) | Gig Worker | Container history | Cross-task patterns |
| Layer 3 (Annealed) | Dinit Service | System service logs | Service-level optimization |
| Layer 4+ | Shared/Crabjar User | Full audit trail | Architectural trade-offs |

### Conductor Task Routing

Higher-rank workers should receive more complex tasks that require broader context. The conductor's routing logic should consider:

- Worker's provenance level (what it can see)
- Historical evidence of opportunity perception (what it notices)
- Decision accuracy at current scope (how well it handles what it sees)

### JEV Decision Layer

Promotion decisions should leverage JEV's evaluation infrastructure:

- Track which opportunities workers perceive vs. miss
- Measure quality of proposals (not just execution success)
- Use historical data to calibrate promotion thresholds

## Implementation Implications

### 1. Context Window Configuration

Each promotion tier should define an explicit context window:

```hemlock
// Example: Gig worker gets access to other workers' provenance
let gig_worker_context = {
    own_history: true,
    container_state: true,
    peer_provenance: true,  // New at this tier
    historical_outcomes: false,  // Not yet
};
```

### 2. Opportunity Reporting Mechanism

Workers need a way to report perceived opportunities without necessarily executing them:

```hemlock
// Worker proposes, supervisor decides
let proposal = worker.propose_opportunity({
    description: "Cross-crate pattern duplication detected",
    evidence: [task_ids],
    estimated_impact: "high",
    risk: "low",
});
supervisor.evaluate(proposal);  // May promote worker to pursue it
```

### 3. Perception Calibration Metric

Measure whether workers accurately distinguish between opportunities worth pursuing versus interesting but irrelevant observations:

```hemlock
let calibration = {
    opportunities_proposed: 10,
    approved_by_supervisor: 6,  // High perception accuracy
    executed_successfully: 5,   // Good execution capability
};
```

## Success Criteria

- [ ] Promotion explicitly expands worker context window (not just permissions)
- [ ] Workers can propose opportunities without executing them
- [ ] Perception calibration metrics tracked per worker
- [ ] Guard verification scales with promotion tier
- [ ] Evidence of opportunity perception incorporated into promotion decisions

## Open Questions

1. How do we measure "perception accuracy" — the ratio of valid opportunities identified vs. noise?
2. Should workers be promoted for perception skill separately from execution capability?
3. What's the appropriate context window expansion at each tier?
4. How do we prevent overconfidence from expanded perception without proportional capability growth?

## References

- ADR-016: Worker Promotion Provenance Model (five-tier identity/persistence model)
- evidence-based-promotion.md (competence metrics and promotion thresholds)
- ADR-008: Podman-based agent sandboxing architecture