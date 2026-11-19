# Lineage & Provenance Graph — Institutional Memory

**Date:** November 19, 2026
**Status:** Draft
**Priority:** P1 (Core Architecture)
**North Star:** Workers can be destroyed; their contributions shouldn't be. Observations should become policies with traceable origins.

## Problem Statement

Crabjar's guard system and policy layer currently treat rules as static configuration. When a worker observes something, proposes a change, and that proposal eventually becomes policy, there's no link between the original observation and the resulting rule. Workers can't see "this policy came from my observation six months ago." The institution has no memory of how it got to be what it is.

## Core Concept

A **lineage graph** connects observations → proposals → decisions → implementations → recognitions. It's append-only: records aren't overwritten, they're extended. This enables:

1. **Attribution**: Workers can see their contributions in policies created later
2. **Auditability**: Every rule has a traceable origin story
3. **Proposal callbacks**: When a worker's proposal gets implemented, equivalent workers get notified
4. **Institutional continuity**: Knowledge survives individual worker destruction

## Architecture

### Lineage Node Types

```rust
pub enum LineageNode {
    Observation {
        id: LineageId,
        source: SourceRef,           // Worker, sensor, external event
        timestamp: Timestamp,
        description: String,
        context: Option<WaypointId>, // Where in the environment this was observed
    },
    Proposal {
        id: LineageId,
        based_on: Vec<LineageId>,    // Observations that motivated this
        proposer: SourceRef,
        timestamp: Timestamp,
        description: String,
        status: ProposalStatus,
    },
    Decision {
        id: LineageId,
        proposal_id: LineageId,      // Which proposal this decided on
        decision_maker: SourceRef,   // Supervisor, human, policy engine
        timestamp: Timestamp,
        outcome: DecisionOutcome,    // Approved, rejected, modified
        rationale: String,
    },
    Implementation {
        id: LineageId,
        decision_id: LineageId,      // Which decision this implements
        implementer: SourceRef,
        timestamp: Timestamp,
        artifacts: Vec<ArtifactId>,  // Code, config, policy changes
    },
    Recognition {
        id: LineageId,
        implementation_id: LineageId,
        recognized_by: SourceRef,
        timestamp: Timestamp,
        note: String,                // "This policy came from Worker X's observation"
    },
}

pub enum ProposalStatus {
    Pending,
    UnderReview,
    Accepted,
    Rejected { reason: String },
    Superseded { by: LineageId },
}
```

### Storage

New tables in the existing SQLite ledger:

```sql
CREATE TABLE lineage_nodes (
    id TEXT PRIMARY KEY,
    node_type TEXT NOT NULL,         -- observation/proposal/decision/implementation/recognition
    source_ref TEXT NOT NULL,        -- Who/what created this
    timestamp INTEGER NOT NULL,
    description TEXT NOT NULL,
    payload TEXT NOT NULL DEFAULT '{}',  -- JSON: type-specific fields
    status TEXT NOT NULL DEFAULT 'active',
    created_at INTEGER NOT NULL
);

CREATE TABLE lineage_edges (
    from_id TEXT NOT NULL REFERENCES lineage_nodes(id),
    to_id TEXT NOT NULL REFERENCES lineage_nodes(id),
    edge_type TEXT NOT NULL,         -- based_on/proposes/decides/implements/recognizes
    weight REAL DEFAULT 1.0,         // Strength of connection (for relevance scoring)
    created_at INTEGER NOT NULL,
    PRIMARY KEY (from_id, to_id, edge_type)
);

CREATE INDEX idx_lineage_source ON lineage_nodes(source_ref);
CREATE INDEX idx_lineage_type ON lineage_nodes(node_type);
CREATE INDEX idx_lineage_edges_from ON lineage_edges(from_id);
CREATE INDEX idx_lineage_edges_to ON lineage_edges(to_id);
```

### Provenance Query API

```rust
pub trait ProvenanceQuery {
    /// Trace forward: what happened after this observation?
    fn trace_forward(&self, id: &LineageId) -> Result<Vec<LineageNode>>;
    
    /// Trace backward: what led to this implementation?
    fn trace_backward(&self, id: &LineageId) -> Result<Vec<LineageNode>>;
    
    /// Find all proposals based on observations from a specific source
    fn proposals_from_source(&self, source: &SourceRef) -> Result<Vec<LineageNode>>;
    
    /// Find implementations that resulted from a worker's proposals
    fn implementations_from_worker(&self, worker: &SourceRef) -> Result<Vec<LineageNode>>;
    
    /// Check if a specific policy/rule has a traceable origin
    fn rule_origin(&self, rule_id: &RuleId) -> Result<Option<LineageNode>>;
}
```

### Proposal Callback System

When an implementation is created from a decision that accepted a proposal:

1. The system checks the lineage graph for the original proposer
2. If the proposer is still active (same worker type, same environment), it receives a recognition notification
3. If the proposer was destroyed, the recognition is recorded for equivalent workers (same role, same capabilities)
4. The recognition node links back through the full chain: observation → proposal → decision → implementation → recognition

```rust
pub struct RecognitionCallback {
    pub original_proposal_id: LineageId,
    pub implementation_id: LineageId,
    pub original_source: SourceRef,
    pub equivalent_workers: Vec<SourceRef>,  // Workers who can receive this
    pub message: String,                     // "Your observation led to policy X"
}
```

## Integration Points

### With Guard System
- Guard policies can reference lineage IDs as their origin
- When a guard blocks an action citing "lack of provenance," it can point to the specific lineage chain that was checked
- New rules added to the guard system should include a `provenance_id` linking to the observation/proposal that motivated them

### With Waypoint System
- Historical waypoints can reference lineage chains
- Workers traversing historical waypoints see not just "what happened" but "why it happened and who proposed it"

### With Worker Promotion (ADR-016)
- Worker promotion decisions should be recorded as lineage nodes
- A worker's promotion history is part of its provenance chain
- When a promoted worker makes decisions, those decisions carry the weight of its provenance

## Constraints & Policies

### Append-Only Rule
Lineage nodes are never modified or deleted. Corrections are made by adding new nodes that reference the original. This preserves the full institutional history.

### Attribution Integrity
A recognition node must trace back through a complete chain to reach the original observation. If any link in the chain is missing, the recognition is marked as `partial` rather than being fabricated.

### Privacy Boundaries
Lineage records what happened and who was involved, but not necessarily sensitive details about the workers involved. The level of detail in lineage nodes respects the same privacy boundaries as the underlying observations.

## Success Criteria

1. Any guard policy can be traced back to at least one observation or proposal
2. Workers can query "what policies did my observations lead to?" and get accurate results
3. When a proposal gets implemented months later, equivalent workers receive recognition
4. The lineage graph is append-only — no records are silently overwritten

## Open Questions

- How do we handle proposals that were rejected but later re-proposed by different workers?
- What's the retention policy for lineage nodes? (Append-only suggests forever, but storage grows.)
- Can lineage be used for performance evaluation of workers, or does that create perverse incentives?
