# Positive Boundary Model — Capabilities Over Prohibitions

**Date:** November 19, 2026
**Status:** Draft
**Priority:** P1 (Core Architecture)
**North Star:** Don't tell workers "don't look here." Give them exactly what they need and nothing more. The prohibition itself is information.

## Problem Statement

Crabjar's current guard system is built on **negative capabilities**: a list of things workers *can't* do. This leaks information to the worker — if you're told "you can't access /secrets," now you know /secrets exists. In complex environments with many workers and many resources, this creates an arms race: every new restriction reveals more about the system's structure.

## Core Concept

Replace "here's what you can't do" with "here's what you have." Workers receive a **capability set** — an explicitly granted collection of interfaces, tools, and data sources. Anything not in the set simply doesn't exist from the worker's perspective. No prohibition needed because there's nothing to prohibit against.

### The Information Leak Problem

```
Negative model:                     Positive model:
"Can't read /secrets/keys"          Worker sees: {read_file, list_dir}
→ Worker knows /secrets exists      → Worker only sees mounted volumes
→ Worker might try to guess paths   → No knowledge of unmounted paths
→ Worker might exploit the gap      → Nothing to exploit
```

The negative model tells the worker about the boundary. The positive model *is* the boundary.

## Architecture

### Capability Set Model

```rust
pub struct CapabilitySet {
    pub worker_id: WorkerId,
    pub granted_at: Timestamp,
    pub expires_at: Option<Timestamp>,  // Capabilities can expire
    
    // Tools the worker can use
    pub tools: Vec<ToolGrant>,
    
    // Data sources the worker can access
    pub data_sources: Vec<DataGrant>,
    
    // Actions the worker can perform
    pub actions: Vec<ActionGrant>,
    
    // Network endpoints (if applicable)
    pub network: Option<NetworkGrant>,
}

pub struct ToolGrant {
    pub tool_id: ToolId,
    pub parameters: ParameterConstraints,  // What args are allowed?
    pub rate_limit: Option<RateLimit>,     // How fast can it be called?
}

pub struct DataGrant {
    pub source_id: DataSourceId,
    pub scope: Scope,                      // Which paths/tables/keys?
    pub mode: AccessMode,                  // Read, read-write, execute?
    pub filter: Option<DataFilter>,        // Row/column-level filtering
}

pub struct ActionGrant {
    pub action_id: ActionId,
    pub preconditions: Vec<Precondition>,  // What must be true first?
    pub side_effects: Vec<SideEffect>,     // What will this change?
}

pub enum AccessMode {
    ReadOnly,
    ReadWrite,
    Execute,
}

pub struct Scope {
    pub paths: Vec<PathPattern>,           // File system paths
    pub tables: Vec<TablePattern>,         // Database tables
    pub keys: Vec<KeyPattern>,             // Config/environment keys
    pub endpoints: Vec<EndpointPattern>,   // API endpoints
}
```

### Capability Enforcement

Capabilities are enforced at the **interface layer**, not checked after the fact:

```rust
pub trait CapabilityEnforcer {
    /// Resolve a worker's effective capability set
    fn resolve(&self, worker_id: &WorkerId) -> Result<CapabilitySet>;
    
    /// Check if an action is permitted (returns the grant that permits it)
    fn check(&self, 
             worker_id: &WorkerId, 
             action: &ActionRequest
    ) -> Result<Option<&ActionGrant>>;
    
    /// Get the tool interface as seen by a specific worker
    /// (only shows tools the worker has granted access to)
    fn visible_tools(&self, worker_id: &WorkerId) -> Result<Vec<ToolDescriptor>>;
    
    /// Get the data sources as seen by a specific worker
    fn visible_sources(&self, worker_id: &WorkerId) -> Result<Vec<SourceDescriptor>>;
}
```

The key insight: `visible_tools()` and `visible_sources()` don't return "all tools minus forbidden ones." They return **only** what's granted. The worker never sees the full list.

### Capability Composition

Capabilities compose through intersection, not union:

```rust
impl CapabilitySet {
    /// Two grants on the same resource → most restrictive wins
    pub fn intersect(&self, other: &CapabilitySet) -> CapabilitySet {
        // tools: intersection of granted tools
        // data: intersection of scopes (narrower wins)
        // actions: intersection with precondition conjunction
    }
}
```

If a worker has two capability sets (e.g., from its role and from a specific task), the effective set is the intersection. You can't get more access by combining grants.

### Expiring Capabilities

Capabilities have optional expiration. This supports:
- Temporary elevated access for debugging
- Time-limited data access for analysis tasks
- Automatic de-escalation after task completion

```rust
pub struct ExpiringGrant<T> {
    pub grant: T,
    pub expires_at: Timestamp,
    pub auto_renew: bool,  // Can the worker request renewal?
}
```

## Migration from Negative to Positive Model

### Phase 1: Dual Enforcement
- Keep existing negative capability checks
- Add positive capability sets alongside
- Log discrepancies between what negative model allows vs. what positive model grants

### Phase 2: Positive Primary
- Positive capabilities become the primary enforcement mechanism
- Negative capabilities serve as a safety net (defense in depth)
- Workers only see their granted interfaces

### Phase 3: Negative Retired
- Remove negative capability checks from the hot path
- Keep them in configuration for audit purposes
- All enforcement is through positive grants

## Integration Points

### With Guard System
The guard system becomes a **capability broker** rather than a prohibition checker. Instead of "is this action forbidden?" it answers "does this worker have a grant for this action?"

### With Waypoint System
Waypoints reference capabilities, not prohibitions. A waypoint says "you can access these data sources" rather than "these are the things you can't touch."

### With Observer Workers
Observers get a minimal capability set: read-only access to waypoints and lineage graph, ability to record observations. No write capabilities at all.

## Constraints & Policies

### Least Privilege Default
Workers start with an empty capability set. Grants are added explicitly. No implicit permissions.

### Grant Auditability
Every capability grant is recorded with:
- Who granted it (supervisor, policy engine, human)
- Why it was granted (task reference, lineage ID)
- When it expires
- What the worker can do with it

### No Privilege Escalation Through Composition
Combining two grants never produces more access than either grant individually. If a worker has read access to table A and write access to table B, it doesn't get write access to table A just because it has both grants.

## Success Criteria

1. Workers cannot discover resources they don't have grants for (no "don't look here" leaks)
2. Capability checks happen at the interface layer (O(1) lookup), not after action execution
3. Migration from negative to positive model is backward compatible
4. Every capability grant is auditable with a traceable origin

## Open Questions

- How do we handle dynamic capabilities that change during a task? (Re-granting without worker restart)
- What's the right granularity for data grants? (Table-level vs. row-level vs. column-level)
- Can capabilities be transferred between workers? (Delegation model)
