# Observer Worker State — Third Lifecycle Mode

**Date:** November 19, 2026
**Status:** Draft
**Priority:** P2 (Productivity Improvement)
**North Star:** Workers shouldn't be binary (working/idle). There's value in a state where they're learning the environment without being assigned specific tasks.

## Problem Statement

Crabjar workers currently exist in one of two states: **assigned** (working on a task) or **idle** (waiting for work). This misses a third valuable state — **observer** — where a worker is actively building situational awareness about the environment without modifying authoritative state. Like a new employee who's been given access to read everything but isn't yet trusted to change anything.

## Core Concept

An **observer-state worker** can:
- Read waypoints and traverse the information environment
- Produce findings and observations (recorded in the lineage graph)
- Build context about how systems work
- Propose changes for supervisor review

An observer-state worker **cannot**:
- Modify authoritative state (files, configs, policies)
- Execute destructive operations
- Make decisions that affect other workers
- Bypass guard checks

The key distinction: observers *produce* information; workers *consume and transform* it.

## Architecture

### Worker State Machine

```
                    ┌─────────────┐
         spawn      │             │  task assigned
    ┌──────────────►│  Observer   │──────────────────┐
    │               │             │                  ▼
    │               └─────────────┘           ┌─────────────┐
    │                       ▲                 │             │
    │                       │  task complete  │   Working   │
    │                       │  or timeout     │             │
    │                       └─────────────────│             │
    │                                         └─────────────┘
    │                                                   │
    │  destroyed / recycled                             │  error / blocked
    ▼                                                   │
┌──────────┐                                            │
│ Destroyed│◄───────────────────────────────────────────┘
└──────────┘
```

### Observer State Data Model

```rust
pub struct ObserverState {
    pub worker_id: WorkerId,
    pub environment: EnvironmentId,
    pub started_at: Timestamp,
    
    // What the observer is focusing on
    pub focus_areas: Vec<WaypointId>,       // Which waypoints it's exploring
    pub questions: Vec<String>,             // What it's trying to understand
    
    // What it has produced
    pub observations: Vec<LineageId>,       // Observations recorded in lineage graph
    pub proposals: Vec<LineageId>,          // Proposals submitted for review
    pub findings: Vec<FindingRef>,          // Structured findings (not yet proposals)
    
    // Constraints
    pub max_traversal_depth: u32,           // How deep can it go in waypoints?
    pub max_findings_before_review: usize,  // When does it need to report back?
}

pub struct FindingRef {
    pub id: FindingId,
    pub category: FindingCategory,
    pub description: String,
    pub confidence: f32,                    // How sure is the observer about this?
    pub evidence: Vec<EvidenceRef>,         // Waypoints, artifacts that support this
    pub created_at: Timestamp,
}

pub enum FindingCategory {
    StructuralInsight,     // "These two crates have a circular dependency"
    OperationalConcern,    // "Guard is blocking 5 actions that look legitimate"
    ImprovementIdea,       // "We could cache these lookups to save 20% time"
    AnomalyDetected,       // "This metric is outside normal range"
    KnowledgeGap,          // "There's no documentation for this module"
}
```

### Observer Permissions

Observers get a restricted capability set:

```rust
impl Capabilities for ObserverWorker {
    fn can_read_waypoints(&self) -> bool { true }
    fn can_traverse_waypoints(&self) -> bool { true }  // Within depth limit
    fn can_record_observations(&self) -> bool { true }
    fn can_submit_proposals(&self) -> bool { true }
    
    fn can_modify_state(&self) -> bool { false }
    fn can_execute_commands(&self) -> bool { false }
    fn can_modify_policies(&self) -> bool { false }
    fn can_affect_other_workers(&self) -> bool { false }
}
```

### Transition Triggers

**Observer → Working:**
- Supervisor assigns a task based on the observer's findings
- Observer's proposal is accepted and it's promoted to implement it
- Timeout: if no task assigned within configured period, worker is recycled

**Working → Observer:**
- Task completed but worker has capacity remaining
- Worker requests observation mode (e.g., "I want to understand X before my next task")
- Supervisor reassigns from active work to monitoring role

## Integration Points

### With Waypoint System
Observers are the primary consumers of waypoints. They traverse the hierarchy to build understanding. Their traversal patterns can be analyzed to improve waypoint design (if many observers get stuck at a certain level, that waypoint needs better context).

### With Lineage Provenance
Every observation an observer makes is recorded as a lineage node. If that observation later leads to a policy change, the observer gets recognition through the lineage chain. This creates a feedback loop: observing → proposing → seeing your proposal become reality.

### With Guard System
Observers have minimal guard intervention — they're not doing anything risky. But their proposals go through the normal guard/policy review process before becoming actionable.

## Constraints & Policies

### Resource Limits
- Observers consume compute but don't produce deliverables directly
- Max observer duration per worker (configurable, default 30 minutes)
- Max concurrent observers per environment (to prevent observation storms)

### Finding Quality
- Findings must include evidence references (waypoints or artifacts)
- Findings below a confidence threshold are flagged as "speculative"
- Observers should prioritize findings that affect safety or correctness over optimization ideas

## Success Criteria

1. A new worker can transition from observer to working state within 5 minutes of spawning
2. Observer findings have measurable value — track how many lead to accepted proposals
3. Observers don't modify authoritative state (enforced by capability system)
4. The observer→working transition is smooth — the worker carries its observations into task execution

## Open Questions

- Should observers be able to observe each other? (Meta-observation)
- How do we prevent "observer drift" where a worker stays in observer mode indefinitely because it's comfortable there?
- Can observers be specialized for different finding categories (e.g., security-focused vs. performance-focused)?
