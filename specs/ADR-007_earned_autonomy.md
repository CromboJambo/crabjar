---
id: ADR-007
title: Earned Autonomy via Action-Class Tracking
status: Proposed
date: 2026-10-01
supersedes: null
see_also: [[ADR-004]]
---

# ADR-007: Earned Autonomy via Action-Class Tracking

## Status

Proposed. The team's current best understanding of how CrabJar should gain operational autonomy over time — not through opaque confidence scores, but through structured institutional memory of what actions have been approved under which conditions.

## Context

CrabJar needs to move from "always ask" toward "handle known-safe cases automatically" without accumulating unrestricted power. The naive approach is a scalar confidence score that rises as the agent succeeds more often. But this has three fatal flaws:

1. **Opaque**: A 0.94 confidence number tells you nothing about *why* the agent thinks it's safe
2. **Uninspectable**: You can't audit which past experiences produced that number
3. **Brittle to distribution shift**: The agent may be overconfident in novel situations that superficially resemble familiar ones

The alternative is a **junior-employee model**: CrabJar observes, proposes changes for human review, learns from approvals and rejections, and gradually earns the right to act autonomously on specific classes of actions — while always knowing when to escalate.

This aligns with the execution pipeline already in place: `request → guard → concierge → telemetry → outcome → trust update`. The missing piece is a principled way for the trust layer to evolve based on accumulated experience.

## Decision

Implement earned autonomy as **action-class tracking with topological uncertainty**:

### 1. Action Class Registry

Every distinct type of action is registered as an `ActionClass` with:
- A name and description (e.g., "archive stale CSV export")
- Structural conditions under which it's considered safe (age thresholds, downstream references, source preservation)
- Permission level: `PROPOSE`, `ACT_WITH_NOTIFICATION`, or `ACT_AUTONOMOUSLY`

### 2. Approval History Tracking

For each action class, maintain counters and records:
- Total attempts
- Approved vs. rejected by human
- Reversed after-the-fact
- Exceptions encountered

This creates **institutional memory**: "I've seen this person handle this class of problem correctly 37 times."

### 3. Topological Uncertainty Model

Instead of a scalar confidence, autonomy is determined by the **density of supporting relationships** around a proposed action:

```
known path (dense support)    → ACT
weak path (some support)      → PROPOSE
new path (no prior experience)→ ASK
contradictory paths           → STOP
```

A well-known region has multiple converging approval records. A novel situation is sparse — the agent recognizes it doesn't know and becomes conservative.

### 4. Inspectable Decision Provenance

Every decision must be explainable by tracing back to the specific action-class history:

```
DECISION: Archive file X
ACTIVE STRUCTURE:
- X is stale (143 days)
- Source preserved: yes
- Downstream references: 0
- Same pattern: 31 prior cases
- Prior approvals: 29
→ ACT_AUTONOMOUSLY
```

## Options Considered

### Option A: Scalar Confidence Score
A single number that rises with success. Simple but opaque; no way to audit why a decision was made or whether the agent is overconfident in novel situations.

**Rejected** because it doesn't scale to institutional memory and provides no inspectability.

### Option B: Reinforcement Learning with Reward Function
Train a policy network on reward signals (approved = +1, rejected = -1). Powerful but requires careful reward engineering; the learned policy becomes a black box.

**Rejected** because it trades legibility for optimization — exactly what we want to avoid in an earned-autonomy model.

### Option C: Static Rule-Based Permissions
Hard-code which action classes are safe at each permission level. Simple and auditable but doesn't learn or adapt.

**Partially adopted** as the initial state; all action classes start at `PROPOSE` and must earn higher permissions through tracked experience.

### Option D: Action-Class Tracking with Topological Uncertainty (Chosen)
Structured records of approval history per action class, with autonomy determined by the density and consistency of past approvals. Provides legibility, adaptability, and principled conservatism in novel situations.

## Consequences

### Positive consequences
- **Legible trust**: Users can inspect exactly why CrabJar decided to act autonomously or ask for permission
- **Gradual, earned autonomy**: No sudden jumps in power; each promotion is backed by tracked experience
- **Principled conservatism**: Novel situations automatically trigger more caution (sparse → propose/ask)
- **Institutional memory**: The system remembers not just what happened but under which conditions decisions were acceptable

### Negative consequences (trade-offs)
- **Cold-start problem**: All action classes start at `PROPOSE`; it takes time to earn autonomy
- **Record-keeping overhead**: Must maintain per-action-class counters and history
- **Condition specification burden**: Defining "structural conditions" for each action class requires care

### Ongoing concerns
- How to handle distribution shift when the environment changes (e.g., new file formats, different retention policies)
- Whether to allow demotion of permission levels when rejections occur
- How to aggregate across related action classes (e.g., "archive X" and "archive Y" are both "archive stale files")

## References

- Nygard, M. (2011). *Documenting Architecture Decisions*. https://www.infoq.com/articles/Architecture-Decision-Lang
- `specs/ADR-004.md` — The glass abstraction boundary (agent operates through public interface)
- ChatGPT conversation: "Game World Interface" (2026-10-01) — earned autonomy as junior-employee model
