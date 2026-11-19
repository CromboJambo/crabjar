# Waypoint System — Durable Navigational Objects

**Date:** November 19, 2026
**Status:** Draft
**Priority:** P1 (Core Architecture)
**North Star:** Workers should be able to orient themselves in an environment without being given the entire universe upfront. Waypoints are navigation hints, not security boundaries.

## Problem Statement

Crabjar workers currently receive either a full context dump or a bare task assignment. There's no intermediate layer for progressive orientation — how does a worker understand where it is, what's relevant, and where to look next without loading everything? The guard system tells workers what they *can't* do (negative capabilities), but doesn't help them find what they *should* be doing.

## Core Concept

A **waypoint** is a durable, typed object that serves as a navigational landmark through the information environment. It's not an instruction, not a permission, not documentation — it's a pointer with context. Workers traverse waypoints progressively rather than receiving the entire environment at once.

### Waypoint Hierarchy

```
Root waypoint (environment orientation)
  └── Project waypoint (project context)
       └── System waypoint (available interfaces)
            └── Task waypoint (current objective + relevant artifacts)
```

Workers enter at the root and drill down as needed. Each level provides enough context to decide whether to continue deeper or look elsewhere.

## Architecture

### Waypoint Types

| Type | Purpose | Example |
|------|---------|---------|
| **Orientation** | Understand where you are | "This is the crabjar repo, here's what each crate does" |
| **Structural** | Understand how components relate | "crabjar-guard depends on crabjar-state for pending actions" |
| **Historical** | Understand how you got here | "ADR-016 established worker promotion tiers" |
| **Operational** | Understand current conditions | "Guard is currently blocking 3 pending actions" |
| **Task** | Understand the current objective | "Fix the race condition in guard approval flow" |
| **Recovery** | Find last known-good state | "Last clean build was commit abc123, before the refactor" |

### Waypoint Data Model

```rust
pub struct Waypoint {
    pub id: WaypointId,
    pub parent_id: Option<WaypointId>,      // Hierarchy traversal
    pub category: WaypointCategory,          // Orientation/Structural/etc.
    pub description: String,                 // What this waypoint tells you
    pub destinations: Vec<WaypointId>,       // Where to go next
    pub references: Vec<ArtifactId>,         // Related artifacts/docs
    pub relevance: RelevanceScore,           // How important is this?
    pub freshness: Timestamp,                // When was this last verified?
    
    // Transformation lineage (for compressed waypoints)
    pub predecessor: Option<WaypointId>,     // Which waypoint produced this?
    pub transformation: Option<FoldRef>,     // What transformation was applied?
    pub recoverability: Recoverability,      // Can we get back to source?
}

pub enum WaypointCategory {
    Orientation,
    Structural,
    Historical,
    Operational,
    Task,
    Recovery,
}

pub enum Recoverability {
    Lossless,                    // Full source accessible
    Bounded { max_depth: u32 },  // Can traverse N levels back
    ReferenceOnly,               // Has pointers but not full content
    Unknown,                     // Don't know yet (valid state!)
}
```

### Storage

Waypoints are stored in the existing SQLite ledger (`crabjar-state`). New tables:

```sql
CREATE TABLE waypoints (
    id TEXT PRIMARY KEY,
    parent_id TEXT REFERENCES waypoints(id),
    category TEXT NOT NULL,
    description TEXT NOT NULL,
    destinations TEXT NOT NULL DEFAULT '[]',  -- JSON array of WaypointIds
    references TEXT NOT NULL DEFAULT '[]',    -- JSON array of ArtifactIds
    relevance REAL DEFAULT 0.5,
    freshness INTEGER NOT NULL,               -- unix timestamp
    predecessor_id TEXT REFERENCES waypoints(id),
    transformation_id TEXT,                   -- FK to transformations table
    recoverability TEXT NOT NULL DEFAULT 'unknown',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_waypoints_parent ON waypoints(parent_id);
CREATE INDEX idx_waypoints_category ON waypoints(category);
```

### Traversal API

```rust
pub trait WaypointNavigator {
    /// Get the root waypoint for an environment
    fn root(&self, env: EnvironmentId) -> Result<Waypoint>;
    
    /// Get child waypoints of a given waypoint
    fn children(&self, id: &WaypointId) -> Result<Vec<Waypoint>>;
    
    /// Traverse from current to destination (with depth limit)
    fn traverse(
        &self, 
        start: &WaypointId, 
        max_depth: u32
    ) -> Result<Vec<Waypoint>>;
    
    /// Find waypoints by category within a scope
    fn find_by_category(
        &self, 
        scope: &WaypointId, 
        category: WaypointCategory
    ) -> Result<Vec<Waypoint>>;
    
    /// Get the lineage (ancestry chain) of a waypoint
    fn lineage(&self, id: &WaypointId) -> Result<Vec<Waypoint>>;
}
```

## Integration Points

### With Guard System
- Waypoints are **not** security boundaries. Access control remains in crabjar-guard.
- A worker can *see* a waypoint without having permission to access its referenced artifacts.
- The guard enforces what the worker can *do*; waypoints tell the worker where to *look*.

### With Worker Lifecycle
- New workers start at the root waypoint for their environment.
- Observer-state workers (see OBSERVER-WORKER-STATE spec) traverse waypoints to build situational awareness.
- Task assignment includes a task waypoint pointing to relevant context.

### With Loss Accounting
- Compressed waypoints carry transformation lineage (see LOSS-ACCOUNTING spec).
- Workers can check `recoverability` before relying on a compressed representation.
- If a worker needs more detail than the current waypoint provides, it traverses backward through `predecessor` links.

## Constraints & Policies

### Compression Depth Limits

| Policy | Max Ancestry Depth | Use Case |
|--------|-------------------|----------|
| Quick context | 1 fold | Status checks, simple queries |
| Working context | 3 folds | Normal task execution |
| Deep investigation | 6 folds | Debugging, root cause analysis |
| Full provenance | Unbounded | Audits, compliance |

Workers operating on compressed waypoints must respect their assigned depth limit. If they need to go deeper than allowed, they request an upgrade from the supervisor.

### Freshness Verification

Waypoints have a `freshness` timestamp. Workers should verify freshness before relying on operational or task waypoints. Stale waypoints (older than configured threshold) trigger a re-verification step rather than being used as-is.

## Success Criteria

1. A new worker can orient itself in an environment by traversing ≤ 3 waypoints without loading full context
2. Waypoint traversal is measurable — we know how many hops a worker made and what it accessed
3. Compressed waypoints carry lineage that allows recovery to source within the configured depth limit
4. Waypoints don't leak information about resources the worker can't access (no "don't look here" prohibitions)

## Open Questions

- Can observers discover new waypoints autonomously, or do they only propose them for supervisor approval?
- How do we handle conflicting waypoints (two workers disagree on what's relevant)?
- What's the right freshness threshold for different waypoint categories?
