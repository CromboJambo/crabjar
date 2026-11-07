# Milestone 5: Schema Co-evolution via JEV

## Overview

JEV evaluates proposed schema changes for cross-service efficiency impact before deployment, enabling automatic migration coordination across crabjar components.

## Problem

Schema changes in distributed systems are risky:
- Breaking compatibility between services
- Unknown performance impact (serialization overhead, query latency)
- Manual migration coordination is error-prone and slow

JEV's semantic understanding can predict schema change impact by analyzing historical decision patterns and system behavior.

## Architecture

### Components

1. **Schema Change Proposal API** (`POST /schema/propose`)
   - Accepts proposed schema changes with justification
   - Triggers JEV impact evaluation
   - Returns evaluation result and migration plan

2. **JEV Schema Impact Evaluator** (new decision criteria)
   - Evaluates proposed changes against efficiency metrics
   - Uses historical decision data to predict impact
   - Considers: serialization overhead, query performance, compatibility

3. **Migration Coordinator** (`POST /schema/migrate`)
   - Executes approved schema migrations
   - Coordinates across affected components
   - Handles rollback on failure

### API Design

#### POST /schema/propose

Request body:
```json
{
  "component": "crabjar-conductor",
  "change_type": "add_column",
  "target_table": "tasks",
  "column_name": "priority_score",
  "column_type": "REAL",
  "justification": "Enable JEV-based task prioritization for improved scheduling efficiency"
}
```

Response:
```json
{
  "success": true,
  "evaluation": {
    "option": "approve",
    "confidence": 0.85,
    "rationale": "Historical data shows priority-based scheduling reduces average task completion time by 12-18%..."
  },
  "migration_plan": {
    "steps": ["add_column_tasks_priority_score"],
    "rollback_steps": ["drop_column_tasks_priority_score"],
    "affected_components": ["crabjar-conductor", "pesti-runner"]
  }
}
```

#### POST /schema/migrate

Request body:
```json
{
  "proposal_id": "prop-abc123",
  "force": false
}
```

Response:
```json
{
  "success": true,
  "migrated_components": ["crabjar-conductor"],
  "pending_components": ["pesti-runner"],
  "status": "in_progress"
}
```

## Implementation Plan

### Phase 5.1: Schema Change Proposal API
- [ ] Create schema proposal request/response types
- [ ] Implement POST /schema/propose endpoint
- [ ] Integrate with JEV decision evaluation using new criteria file

### Phase 5.2: JEV Schema Impact Evaluation
- [ ] Create schema-change-evaluation.json criteria file
- [ ] Define efficiency metrics for schema change evaluation
- [ ] Test JEV evaluation against historical schema change data

### Phase 5.3: Migration Coordination
- [ ] Implement migration plan generation
- [ ] Create POST /schema/migrate endpoint
- [ ] Build rollback mechanism
- [ ] Add migration status tracking

## Success Criteria

- Schema change proposals evaluated within 5 seconds
- JEV evaluation accuracy ≥70% on historical schema changes
- Automatic migration with rollback capability
- Cross-component coordination for multi-service migrations
