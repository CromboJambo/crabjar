# ADR-012: Evaluate/Outcome Endpoint Pattern

**Status:** Accepted  
**Date:** 2026-10-19  
**Author:** crabjar team (CromboJambo)  
**Context:** JEV conductor API design

## Problem

JEV decision-making spans multiple operational domains (service triage, deployment gates, cost optimization, security hardening, capacity planning, etc.). Each domain has different evaluation criteria but shares the same fundamental pattern: evaluate an observation against criteria, make a decision, record the outcome for learning.

We needed an API design that scales across domains without code duplication while maintaining domain-specific semantics.

## Decision

Each JEV domain in crabjar-conductor gets two endpoints:
- `POST /{domain}/evaluate` — Evaluate an observation and return a decision
- `POST /{domain}/outcome` — Record the actual outcome for continuous learning

This pattern is repeated across all 18 JEV milestone domains.

## Rationale

1. **Uniform interface**: Clients (CLI, habitat contracts, other services) interact with all JEV domains through the same HTTP POST pattern
2. **Domain isolation**: Each domain has its own criteria, cache, and learning history
3. **Continuous learning**: The outcome endpoint enables tracking decision accuracy over time, feeding back into threshold calibration and rule extraction
4. **Scalability**: Adding a new decision domain requires implementing two handlers following the established pattern — no API design changes needed

## Endpoint Contract

### Evaluate Endpoint

```hemlock
POST /{domain}/evaluate
Body: { "observation": "<text>", "metadata": {...} }
Response: {
    "success": true,
    "decision": {
        "option": "<chosen_option_id>",
        "confidence": 0.85,
        "rationale": "<explanation>"
    },
    "doubt": {
        "assumptions": [...],
        "blind_spots": [...],
        "last_validation": "<timestamp>",
        "stale_after": "<timestamp>"
    }
}
```

### Outcome Endpoint

```hemlock
POST /{domain}/outcome
Body: {
    "decision_id": "<from evaluate response>",
    "actual_outcome": "<what actually happened>",
    "correct": true/false,
    "notes": "<optional context>"
}
Response: { "success": true, "recorded": true }
```

## Implementation Pattern

All domain handlers follow the same structure:

```hemlock
async fn evaluate_{domain}_handler(req) {
    let observation = req.body.observation;
    // 1. Load domain-specific criteria
    // 2. Check cache
    // 3. Call conductor.evaluate_decision() with domain context
    // 4. Record decision in DB
    // 5. Return structured JSON with doubt block
}

async fn record_{domain}_outcome_handler(req) {
    let outcome = req.body;
    // 1. Look up original decision
    // 2. Update decision record with outcome
    // 3. Feed back into learning system (threshold calibration, rule extraction)
}
```

## Domains Using This Pattern

| Domain | Evaluate Endpoint | Outcome Endpoint |
|--------|-------------------|------------------|
| Base evaluation | `/evaluate` | — |
| Autonomous routing | `/routing/evaluate` | `/routing/outcome` |
| Dependency management | `/upgrade/evaluate` | `/upgrade/outcome` |
| Self-healing | `/health/evaluate` | `/recovery/outcome` |
| Cost optimization | `/cost/evaluate` | `/optimization/outcome` |
| Capacity planning | `/capacity/evaluate` | `/capacity/outcome` |
| Disaster recovery | `/recovery/evaluate` | `/disaster-recovery/outcome` |
| Security hardening | `/security/evaluate` | `/security/outcome` |
| Compliance automation | `/compliance/evaluate` | `/compliance/outcome` |
| Documentation automation | `/documentation/evaluate` | `/documentation/outcome` |
| Monitoring automation | `/monitoring/evaluate` | `/monitoring/outcome` |
| Drift detection | `/drift/evaluate` | `/drift/outcome` |

## Alternatives Considered

1. **Single generic endpoint with domain parameter**: Less clear semantics, harder to version per-domain
2. **WebSocket streaming for outcomes**: Overkill for fire-and-forget outcome recording
3. **GraphQL mutations**: More flexible but heavier client dependency; REST POST is simpler and sufficient

## Consequences

- 18+ endpoint pairs to maintain (but uniform pattern reduces complexity)
- Outcome recording is opt-in — decision quality depends on clients actually reporting outcomes
- Learning feedback loop requires sufficient outcome data volume
- API versioning strategy needed if endpoint contracts evolve