---
id: EXPL-001
title: Sparse Neural Lattice for Institutional Memory
status: Exploring
date: 2026-10-01
see_also: [[ADR-007]]
---

# EXPL-001: Sparse Neural Lattice for Institutional Memory

## Status

Exploring. This is a speculative design direction, not yet an architectural decision. Capturing the idea to evaluate whether it can underpin earned autonomy (ADR-007) or if simpler data structures suffice.

## The Core Idea

Instead of a dense neural network where every parameter influences every output, imagine a **sparse graph of learned relationships** where:

- **Nodes** represent meaningful concepts from the environment ("CSV export", "stale file", "accounting period", "this user")
- **Edges** represent learned relationships that accumulate through experience ("stale files older than 90 days → safe to archive when source preserved")
- **Sparsity is a feature**: Most nodes aren't connected; the lattice only grows structure where it's needed

```
dense network:          sparse lattice:
████████████            ●──────●
████████████                │      │
████████████                │      └────●
████████████                │
                            ●───●
                                │
                                └────●──────●
                                       │
                                       ●
```

## Why This Matters for Earned Autonomy

The key property of a sparse lattice is **inspectable locality**. When the agent makes a decision, you can ask: "What part of your learned model was active?" and get a traceable answer:

```
DECISION: Archive file X

ACTIVE STRUCTURE:
X
├─ stale: 143 days              [node: staleness]
├─ source preserved: yes        [node: source_preserved]
├─ downstream references: 0     [node: no_dependents]
├─ same pattern: 31 prior cases [edge weight from experience]
└─ prior approvals: 29          [edge weight from experience]
→ ACT_AUTONOMOUSLY
```

This is radically different from "the neural network generated a 0.94 confidence" — the lattice has **provenance**.

## Topological Uncertainty as Autonomy Control

The sparsity of connections around a proposed action naturally encodes uncertainty:

- **Dense region** (many supporting relationships) → ACT autonomously
- **Weak path** (some support, few approvals) → PROPOSE to human
- **New path** (no prior experience) → ASK before acting
- **Contradictory paths** (conflicting evidence) → STOP and escalate

```
known situation:         novel situation:
●─●─●                    ●────●
│╲│╱│                          │
●─●─●                    sparse connections
                         → be conservative
```

## Connection to Structural Tokenization

This maps well onto the pesti structural tokenizer idea. Instead of feeding raw tokens into an enormous dense model:

1. Parse input into meaningful structural tokens (CSV export, date field, row count)
2. Map those tokens to lattice nodes
3. Traverse edges to find relevant learned relationships
4. Use edge weights and path density to determine autonomy level

The lattice becomes a **learned index** over the agent's experience, queryable by structure rather than memorized patterns.

## Open Questions

1. **Construction**: How do you actually build this? Hand-specified nodes/edges? Learned from approval history? Both?
2. **Node identity**: How do you recognize that "stale CSV export" in one context is the same node as "old data file" in another?
3. **Edge weight updates**: What's the learning rule? Incremental Bayesian updating? Simple counter-based weighting?
4. **Pruning**: Do old/contradicted relationships decay over time? How do you forget wrong lessons?
5. **Scale**: Does this stay sparse as experience grows, or does it become dense and lose its advantages?

## Comparison to Alternatives

| Approach | Legibility | Adaptability | Novelty Handling | Complexity |
|----------|------------|--------------|------------------|------------|
| Dense neural net | Low | High | Poor (overconfident) | High |
| Scalar confidence | Low | Medium | Poor | Low |
| Static rules | High | None | Good (conservative) | Low |
| **Sparse lattice** | **High** | **High** | **Good** | **Medium** |

The sparse lattice aims to combine the legibility of static rules with the adaptability of learned systems, while handling novelty conservatively.

## Next Steps for Exploration

- Prototype: Can we build a minimal lattice in Rust that tracks action classes as nodes and approval history as edges?
- Query test: Given a proposed action, can we traverse the lattice to determine autonomy level?
- Learning test: Does the lattice update correctly when an action is approved/rejected?

If this works, it could underpin ADR-007's earned autonomy model. If not, simpler data structures (approval counters per action class) may suffice.
