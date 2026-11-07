# JEV Cooperative/Integrated Evolution Spec

**Status:** Draft
**Author:** Crombo Jambo
**Date:** 2026-10-17
**Source:** Adaptive Opponent Model discussion + JEV adversarial baseline implementation

## Overview

This spec defines the evolutionary path for JEV (Justification-Evaluation-Verification) from an **adversarial decision layer** to a **cooperative/integrated intelligence system**. The core insight: optimize the *relationships* between components rather than individual component intelligence.

### Three Evolutionary Models

| Model | Optimization Target | Question Asked |
|-------|-------------------|----------------|
| **Adversarial** (Phase 0 - complete) | Beat the other system | "How do I predict well enough to win?" |
| **Cooperative** (Phase 1 - this spec) | Shared objective | "How do we coordinate toward a common outcome?" |
| **Integrated** (Phase 2 - future) | Relationship structure itself | "How should we divide work and evolve interfaces?" |

### Efficiency Formula

```
Efficiency = Useful shared outcomes / Total resources consumed
```

JEV's primary learning target: improve this ratio by evolving architecture, not just intelligence.

## Phase 1: Cooperative JEV

### Core Principle: Just Enough Variables (JEV)

Provide the minimal decision space and computational resources needed to answer each question efficiently. Learn when to invoke which component, when to cache, when to skip computation.

### Architecture Components

```
┌─────────────────────────────────────────────────────────┐
│                    JEV Orchestrator                     │
│  (learns division of labor, invocation patterns)        │
├──────────┬──────────┬──────────┬──────────┬─────────────┤
│ Embedding│ Rust Rule│ Cached   │ Database │ Shared      │
│ Backend  │ Engine   │ Results  │ Evidence │ Schema      │
│ (LLM)    │ (determ.)│ Manager  │ Store    │ Evolution   │
└──────────┴──────────┴──────────┴──────────┴─────────────┘
```

### Component Responsibilities

#### 1. JEV Orchestrator (`crabjar-orchestrator`)
- **Learns:** When to invoke which component, when computation is redundant
- **Decision logic:** 
  - Is there a cached result for this observation pattern? → Use cache
  - Does a deterministic rule match? → Use Rust rule engine
  - Is the question ambiguous? → Invoke embedding backend (LLM)
- **Evolution:** Updates invocation heuristics based on efficiency metrics

#### 2. Embedding Backend (`crabjar-decide-backend`)
- **Role:** Interpret ambiguous information, compute semantic similarity
- **Backends:** llama-server (primary), hash-based (fallback/offline)
- **Optimization target:** Minimize invocations while maintaining decision quality

#### 3. Rust Rule Engine (`crabjar-decide-rules`)
- **Role:** Execute explicit, repeatable rules with deterministic outcomes
- **Input:** Structured observations + rule definitions
- **Output:** Decision + justification (rule ID matched)
- **Optimization target:** Maximize coverage of decision space without LLM invocation

#### 4. Cached Results Manager (`crabjar-decide-cache`)
- **Role:** Store and retrieve prior decisions for similar observations
- **Eviction policy:** Time-based + usage-based (LRU)
- **Validation:** Periodically re-evaluate cached results against current criteria
- **Optimization target:** Maximize cache hit rate while maintaining freshness

#### 5. Database Evidence Store (`crabjar-memory`)
- **Role:** Preserve prior decisions, outcomes, and learning signals
- **Schema:** Observation patterns → decision quality metrics
- **Query interface:** "What was the outcome of similar past decisions?"

### Decision Flow (Cooperative)

```
Observation arrives
    ↓
[Cache Check] Similar observation cached? 
    ├─ Yes + Fresh → Return cached decision (skip all computation)
    └─ No/Stale → Continue
    ↓
[Rule Match] Deterministic rule matches?
    ├─ Yes → Apply rule, log outcome, return decision
    └─ No → Continue
    ↓
[Embedding Evaluation] Compute semantic similarity to criteria options
    ↓
Decision returned + logged for learning
    ↓
[Learning Signal] Outcome tracked → update cache/rule/orchestrator heuristics
```

### Learning Loop

JEV improves efficiency over time through three mechanisms:

1. **Cache Learning:** Observation patterns that consistently yield the same decision get cached with longer TTL
2. **Rule Extraction:** Frequently matched embedding decisions can be promoted to deterministic rules (with human review)
3. **Orchestrator Heuristics:** Component invocation order and skip conditions evolve based on efficiency metrics

### Metrics Tracked

```json
{
  "decision_id": "uuid",
  "observation_hash": "sha256",
  "components_invoked": ["cache", "rules", "embedding"],
  "computation_time_ms": 45,
  "cache_hit": false,
  "rule_match": false,
  "embedding_confidence": 0.84,
  "decision_quality": null,  // filled when outcome known
  "efficiency_score": 0.92   // computed post-hoc
}
```

### CLI Interface Extensions

```bash
# Evaluate with cooperative optimization (default)
crabjar decide evaluate "observation" /path/to/criteria.json

# Force fresh evaluation (skip cache)
crabjar decide evaluate "observation" /path/to/criteria.json --fresh

# Show efficiency metrics for recent decisions
crabjar decide metrics --last 24h

# Promote cached decision to rule (with human review)
crabjar decide promote-rule <decision_id>

# View learned invocation patterns
crabjar decide learnings --component orchestrator
```

## Phase 2: Integrated JEV (Future Direction)

### Core Principle: Interface Evolution

JEV learns not just *when* to invoke components, but *how* they should interact — evolving the shared schema and division of labor itself.

### Key Capabilities

1. **Schema Co-evolution:** Components propose schema changes; JEV evaluates impact on efficiency
2. **Work Division Learning:** JEV discovers better ways to split work between components
3. **Interface Optimization:** Reduce translation overhead by learning component-compatible data formats
4. **Redundancy Detection:** Identify and eliminate duplicated computation across components

### Evolutionary Pressure

- **Adversarial:** Systems improve by gaining advantage over others
- **Cooperative:** Systems improve by coordinating better toward shared goals
- **Integrated:** Systems improve by changing their relationship structure itself

## Implementation Plan

### Phase 1 Milestones

| Milestone | Deliverable | Status |
|-----------|-------------|--------|
| M1: Cache Layer | Cached results manager with TTL + LRU eviction | ✅ COMPLETE |
| M2: Rule Engine | Deterministic rule matching with decision logging | ✅ COMPLETE |
| M3: Orchestrator | Component invocation logic with skip conditions | ✅ COMPLETE |
| M4: Learning Loop | Outcome tracking + heuristic updates | ✅ COMPLETE |
| M5: Metrics & CLI | Efficiency metrics, `decide metrics`, `decide learnings` | IN PROGRESS |
| M6: Production Validation | Deploy to ftw3, validate efficiency gains vs Phase 0 | PENDING |

### Success Criteria

- **Efficiency gain:** ≥30% reduction in average computation time per decision (vs Phase 0)
- **Cache hit rate:** ≥50% for recurring observation patterns after 1 week of operation
- **Decision quality:** No degradation vs Phase 0 (measured by outcome tracking)
- **Rule promotion:** ≥5 decisions successfully promoted to deterministic rules with human approval

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Cache staleness → outdated decisions | High | TTL + periodic re-evaluation + `--fresh` flag |
| Rule over-extraction → loss of nuance | Medium | Human review required for rule promotion |
| Orchestrator heuristic drift | Low | Log all heuristic changes, allow rollback |
| Complexity increase | Medium | Phase 1 adds optimization; Phase 2 is optional evolution |

## References

- Adaptive Opponent Model discussion (docs/Adaptive Opponent Model.md)
- JEV CLI Integration Spec (specs/jev-cli-integration.md) - Phase 0 baseline
- Crabjar Architecture Overview (AGENTS.md)
