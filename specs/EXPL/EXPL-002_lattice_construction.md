---
id: EXPL-002
title: Sparse Lattice Construction and Query Protocol
status: Exploring
date: 2026-10-01
see_also: [[EXPL-001]], [[ADR-007]]
---

# EXPL-002: How to Actually Build and Query a Sparse Lattice

## Status

Exploring. Concrete protocol for constructing, updating, and querying a sparse relational graph as institutional memory for earned autonomy.

## Construction Protocol

### Node Types

```pseudocode
enum NodeType {
    ENTITY,      // concrete object (file, email, person)
    PROPERTY,    // attribute (stale, archived, approved)
    ACTION,      // operation (archive, send, delete)
    CONDITION,   // guard clause (older_than_90_days)
    OUTCOME      // result (approved, rejected, reversed)
}

struct Node {
    id: u64,
    type: NodeType,
    label: string,
    created: i64,
    metadata: object  // entity-specific data
}
```

### Edge Types

```pseudocode
enum EdgeType {
    HAS_PROPERTY,      // entity → property
    SATISFIES,         // entity → condition
    TRIGGERED_BY,      // action → condition
    RESULTED_IN,       // action → outcome
    LEARNED_FROM,      // edge weight update source
    SIMILAR_TO         // generalization across instances
}

struct Edge {
    from: u64,
    to: u64,
    type: EdgeType,
    weight: f64,       // approval history encoded as weight
    count: i32,        // number of observations
    last_updated: i64
}
```

### Building the Lattice from Experience

When an action is proposed and reviewed:

1. **Create entity node** for the object (if not exists)
2. **Attach property nodes** describing its state
3. **Link condition nodes** that were evaluated
4. **Record action node** with outcome edge
5. **Update weights** on relevant edges based on approval/rejection

```pseudocode
fn record_experience(entity, properties, conditions, action, outcome) {
    let entity_node = get_or_create_entity(entity);
    
    for (prop in properties) {
        let prop_node = get_or_create_property(prop);
        add_edge(entity_node, prop_node, HAS_PROPERTY);
    }
    
    for (cond in conditions) {
        let cond_node = get_or_create_condition(cond);
        add_edge(entity_node, cond_node, SATISFIES);
    }
    
    let action_node = get_or_create_action(action);
    let outcome_node = get_or_create_outcome(outcome);
    
    // Record the causal chain
    for (cond in conditions) {
        let cond_node = get_condition(cond);
        add_edge(action_node, cond_node, TRIGGERED_BY);
    }
    add_edge(action_node, outcome_node, RESULTED_IN);
    
    // Update weights based on outcome
    update_weights(entity_node, action_node, outcome);
}
```

## Query Protocol: Determining Autonomy Level

Given a proposed action on an entity, query the lattice to determine autonomy level.

### Step 1: Find Similar Past Cases

```pseudocode
fn find_similar_cases(entity_props, action) {
    let candidates = [];
    
    // Find entities that share key properties
    for (prop in entity_props) {
        let prop_node = get_property(prop);
        let related_entities = reverse_lookup(prop_node, HAS_PROPERTY);
        
        for (ent in related_entities) {
            if (has_action_history(ent, action)) {
                candidates.push({
                    entity: ent,
                    similarity: compute_similarity(entity_props, ent.props),
                    history: get_action_history(ent, action)
                });
            }
        }
    }
    
    return candidates;
}
```

### Step 2: Compute Path Density

```pseudocode
fn path_density(candidates, threshold) {
    let total_cases = 0;
    let approved_cases = 0;
    let reversed_cases = 0;
    
    for (c in candidates) {
        if (c.similarity < threshold) continue;
        
        total_cases += c.history.total;
        approved_cases += c.history.approved;
        reversed_cases += c.history.reversed;
    }
    
    return {
        total: total_cases,
        approval_rate: approved_cases / max(total_cases, 1),
        reversal_rate: reversed_cases / max(total_cases, 1)
    };
}
```

### Step 3: Determine Autonomy Level

```pseudocode
fn autonomy_level(density) {
    if (density.total == 0) return ASK;           // unknown
    if (density.approval_rate < 0.7) return PROPOSE;  // weak support
    if (density.reversal_rate > 0.1) return PROPOSE;  // risky
    
    // Dense region with good outcomes → act autonomously
    if (density.total >= 5 && density.approval_rate >= 0.9) {
        return ACT_AUTONOMOUSLY;
    }
    
    return PROPOSE;
}
```

## Weight Update Rules

When an action is approved or rejected, update edge weights:

```pseudocode
fn update_weights(entity_node, action_node, outcome) {
    // Find the path from entity conditions to this action
    let relevant_edges = find_path(entity_node, action_node);
    
    if (outcome == APPROVED) {
        for (edge in relevant_edges) {
            edge.weight *= 1.1;   // strengthen
            edge.count += 1;
        }
    } else if (outcome == REJECTED) {
        for (edge in relevant_edges) {
            edge.weight *= 0.9;   // weaken
            edge.count += 1;
        }
    } else if (outcome == REVERSED) {
        for (edge in relevant_edges) {
            edge.weight *= 0.5;   // significant penalty
            edge.count += 1;
        }
    }
    
    edge.last_updated = now();
}
```

## Pruning and Forgetting

Old or contradicted relationships should decay:

```pseudocode
fn prune_lattice(max_age_days, min_weight) {
    let cutoff = now() - max_age_days * 86400;
    
    // Remove edges that are old and weak
    for (edge in all_edges()) {
        if (edge.last_updated < cutoff && edge.weight < min_weight) {
            remove_edge(edge);
        }
    }
    
    // Remove orphan nodes
    for (node in all_nodes()) {
        if (degree(node) == 0) {
            remove_node(node);
        }
    }
}
```

## Example: Archive Decision Query

```pseudocode
// Proposed: archive file "q3-export.csv"
let entity = { id: "file-123", name: "q3-export.csv", age_days: 143 };
let action = "archive";

// Step 1: Find similar past cases
let similar = find_similar_cases(
    [FILE_TYPE_CSV, AGE_OVER_90_DAYS], 
    action
);

// Found 31 similar cases with 29 approvals, 0 reversals
let density = path_density(similar, 0.8);
// { total: 31, approval_rate: 0.94, reversal_rate: 0.0 }

// Step 2: Determine autonomy level
let level = autonomy_level(density);
// ACT_AUTONOMOUSLY

// Explain the decision
print("Archiving q3-export.csv autonomously because:");
print("- Similar to 31 prior cases");
print("- Approval rate: 94%");
print("- No reversals recorded");
```

## Storage Considerations

- **In-memory**: Adjacency list for fast traversal during decision-making
- **Persistent**: SQLite with nodes and edges tables; rebuild in-memory graph on startup
- **Indexing**: Hash index on node labels for O(1) lookups; B-tree on edge weights for pruning queries

```pseudocode
// Schema sketch
CREATE TABLE nodes (
    id INTEGER PRIMARY KEY,
    type TEXT NOT NULL,
    label TEXT NOT NULL,
    created INTEGER NOT NULL,
    metadata JSON
);

CREATE TABLE edges (
    from_id INTEGER NOT NULL,
    to_id INTEGER NOT NULL,
    type TEXT NOT NULL,
    weight REAL DEFAULT 1.0,
    count INTEGER DEFAULT 1,
    last_updated INTEGER NOT NULL,
    FOREIGN KEY (from_id) REFERENCES nodes(id),
    FOREIGN KEY (to_id) REFERENCES nodes(id)
);

CREATE INDEX idx_node_label ON nodes(label);
CREATE INDEX idx_edge_weight ON edges(weight);
```

## Open Questions for Implementation

1. **Similarity computation**: How to measure structural similarity between entities? Shared properties weighted by importance?
2. **Threshold calibration**: What values for approval_rate and reversal_rate actually work in practice?
3. **Contradiction detection**: When do we have enough conflicting evidence to STOP rather than PROPOSE?
4. **Generalization vs. specificity**: How granular should action classes be? "Archive CSV" vs. "archive any file"?
