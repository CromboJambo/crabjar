# JEV Cooperative/Integrated Evolution — Scoped to Production Base

**Status:** Draft  
**Author:** Crombo Jambo / Hermes Agent  
**Date:** 2026-10-17  
**Source:** Adaptive Opponent Model + Computation Theory discussions  

---

## Overview

This document scopes the JEV (Justification-Evaluation-Verification) cooperative/integrated evolution model to the broader production base: crabjar, mirror-lab, pesti-runner, and home-lab infrastructure. The core principle — optimize for less duplicated effort per unit of useful outcome — applies across all systems, not just crabjar's decision layer.

---

## Current State vs. Vision

### Today (Phase 0 - Adversarial Baseline)

**What works:**
- `crabjar decide evaluate` — embedding-based semantic similarity decisions
- SQLite evidence store (`jevs.db`) — persists observations/decisions/outcomes
- Decision criteria library — 4 curated JSON files
- Conductor integration — JevEvaluator at pre-execution, annealing, resource contention gates
- Host-agent decision gate — heuristic tool call vs. respond directly

**What doesn't exist:**
- Cross-system efficiency learning
- Cache/rule promotion across services
- Shared evidence store between crabjar and pesti/mirror-lab
- System 1/2/3 translation layer awareness in production deployments

### Vision (Phases 1 & 2)

A cooperative intelligence substrate where:
- Each component learns when to invoke which other component
- Cached results and rules propagate across the production base
- Efficiency metrics drive architectural evolution
- The system optimizes for total useful outcomes / total resources consumed

---

## Production Base Scoping

### 1. crabjar (Agent Orchestration Framework)

**Current role:** Decision layer, agent loop, execution pipeline  
**JEV integration points:**
- `crabjar-conductor` — JevEvaluator at lifecycle gates (implemented)
- `host/host-agent` — decision gate for tool invocation (heuristic only today)
- `orchestrator/` — model routing decisions (not yet JEV-powered)

**Phase 1 scope for crabjar:**
- Replace heuristic decision gate with JEV-powered cooperative decisions
- Add cache layer to conductor for recurring observation patterns
- Extract rules from frequent embedding decisions (e.g., "always use cached results for status checks")
- Track efficiency metrics per decision type

### 2. pesti-runner (Inference Substrate)

**Current role:** GPU-accelerated LLM inference  
**JEV integration points:**
- Model selection (which model for which task size/complexity)
- Quantization decisions (Q4 vs Q8 vs FP16 based on accuracy needs)
- Batch scheduling (when to batch requests vs. process individually)

**Phase 1 scope for pesti:**
- JEV-powered model routing: "Is this query complex enough to warrant the larger model?"
- Cache inference results for similar prompts
- Learn optimal quantization levels per use case over time

### 3. mirror-lab (Home-Lab Management)

**Current role:** Declarative infrastructure management (lab.toml + labctl)  
**JEV integration points:**
- Deployment decisions: "Which machine should run this service?"
- Resource allocation: "Is there enough capacity on ftw3 for another container?"
- Failure response: "Should I restart, reschedule, or scale up?"

**Phase 1 scope for mirror-lab:**
- JEV-powered deployment optimization: learn which services perform best on which machines
- Cache resource utilization patterns to predict capacity needs
- Rule extraction: "Service X always fails on machine Y due to Z" → deterministic rule

### 4. Home-Lab Infrastructure (Fleet Level)

**Current role:** Multiple machines, containers, services  
**JEV integration points:**
- Workload distribution across nodes
- Container orchestration decisions
- Network routing optimization

**Phase 1 scope for infrastructure:**
- Shared evidence store across all lab machines
- Cooperative load balancing: JEV learns optimal task-to-machine mappings
- Redundancy detection: identify duplicated services or computations across nodes

---

## Cross-System Architecture

### Shared Evidence Store

```hemlock
// Current: per-service SQLite databases
crabjar/jevs.db          # crabjar decisions
pesti/inference_cache.db # pesti inference results
mirror-lab/lab_state.db  # lab state

// Phase 1: shared evidence store with service-specific partitions
shared-evidence-store/
├── jev_observations.db    # All observations across services
├── decision_outcomes.db   # All decision outcomes
└── efficiency_metrics.db  # Cross-service efficiency tracking
```

### JEV Orchestrator (Cross-System)

```hemlock
// Phase 1: Central orchestrator that coordinates JEV decisions across services
jev-orchestrator/
├── src/lib.rs
├── src/routing.rs         // Learn optimal component invocation patterns
├── src/caching.rs         // Cross-service result caching
├── src/rules.rs           // Rule extraction and promotion
└── src/metrics.rs         // Efficiency tracking and reporting
```

### System 1/2/3 Translation Layer (Production)

| System | Component | Role |
|--------|-----------|------|
| **System 1** | pesti-runner, Rust rule engines | Fast, deterministic execution |
| **System 2** | JEV orchestrator, crabjar-conductor | Deliberation, trade-off reasoning |
| **System 3** | LLM inference (ftw3) | Deep thinking, ambiguity resolution |

JEV's job: know when to escalate S1→S2→S3 and when to stay at lower levels.

---

## Implementation Phases

### Phase 1: Cooperative JEV (8-12 weeks)

**Milestone 1: Cross-service cache layer** — COMPLETE (2026-10-17)
- Shared result caching between crabjar and pesti
- TTL + LRU eviction policies
- Cache hit rate tracking

**Implementation:** `crabjar-conductor/src/decision/cache.rs` — LRU decision cache with SHA-256 hashing, TTL support, hit/miss/eviction tracking. Integrated into JevEvaluator evaluation pipeline.

**Milestone 2: Rule extraction pipeline** — COMPLETE (2026-10-17)
- Identify frequent embedding decisions
- Promote to deterministic rules with human review
- Deploy rules across services

**Implementation:** `crabjar-conductor/src/decision/rule_extraction.rs` — pattern analysis on decision logs, rule promotion API endpoints (`/rules/extract`, `/rules/promote`). Decision history logging enabled in evaluate.rs for future rule extraction.

**Milestone 3: JEV-powered model routing (pesti)**
- Learn optimal model selection per task complexity
- Track accuracy vs. cost trade-offs
- Automatic quantization level optimization

**Milestone 4: Deployment optimization (mirror-lab)**
- Learn service-to-machine performance mappings
- Predict capacity needs based on historical patterns
- Rule-based failure response automation

### Phase 2: Integrated JEV (12-24 weeks)

**Milestone 5: Schema co-evolution**
- Components propose schema changes
- JEV evaluates impact on cross-service efficiency
- Automatic migration coordination

**Milestone 6: Interface optimization**
- Learn component-compatible data formats
- Reduce translation overhead between services
- Eliminate redundant serialization/deserialization

**Milestone 7: Redundancy detection**
- Identify duplicated computations across nodes
- Consolidate where possible
- Maintain redundancy only where reliability requires it

---

## Success Metrics

### Phase 1 Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Cache hit rate | ≥50% for recurring patterns | Cross-service cache analytics |
| Decision latency | ≥30% reduction | Average time from observation to decision |
| LLM invocations | ≥40% reduction | Count of System 3 escalations |
| Rule promotions | ≥10 deterministic rules | Human-approved rule deployments |
| Deployment failures | ≥25% reduction | Failed deployment events per week |

### Phase 2 Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Translation overhead | ≥50% reduction | Serialization/deserialization time |
| Redundant computations | ≥75% elimination | Duplicated task detection |
| Schema migration time | ≥60% reduction | Time to propagate schema changes |

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Cache staleness across services | High | TTL + periodic re-evaluation + `--fresh` flag |
| Rule over-extraction | Medium | Human review required for cross-service rule promotion |
| Orchestrator single point of failure | High | Redundant orchestrator instances, failover logic |
| Complexity increase | Medium | Phase 1 adds optimization; Phase 2 is optional evolution |
| Cross-service data consistency | High | Eventual consistency model with conflict resolution |

---

## Next Steps

1. **Immediate:** Implement cross-service cache layer (Milestone 1)
2. **Short-term:** JEV-powered model routing in pesti (Milestone 3)
3. **Medium-term:** Deployment optimization in mirror-lab (Milestone 4)
4. **Long-term:** Integrated JEV with schema co-evolution (Phase 2)

---

## References

- Adaptive Opponent Model discussion (`docs/Adaptive Opponent Model.md`)
- Computation Theory Explained (`docs/Computation Theory Explained.md`)
- JEV Cooperative/Integrated Evolution Spec (`specs/archive/jev-cooperative-integrated-evolution.md`)
- Crabjar Architecture Overview (`AGENTS.md`)
