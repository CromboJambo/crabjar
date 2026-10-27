# Evidence-Based Promotion System

**Status:** Draft  
**Created:** 2026-10-27  
**Related:** ADR-009 (Hybrid attention layer), JEV decision layer specs, crabjar-conductor API

## Problem Statement

Crabjar's multi-agent hierarchy currently uses manual or time-based trust level adjustments. Workers are promoted to supervisor roles based on human judgment or elapsed time, not measurable competence evidence. This creates two problems:

1. **Inefficiency:** Competent workers remain at lower ranks longer than necessary, while less competent workers may be promoted prematurely.
2. **Lack of traceability:** When a supervisor makes a poor decision, there's no audit trail showing what evidence justified their promotion to that rank.

The system needs automatic, evidence-based promotion thresholds where worker performance metrics drive rank elevation with attached provenance.

## Architecture Overview

### Core Concepts

**Competence Evidence:** Measurable signals that indicate a worker's ability to handle higher-complexity tasks:
- Task completion rate (success/failure ratio)
- Decision accuracy (alignment with human review or outcome tracking)
- Knowledge compression quality (usefulness of extracted rules/procedures)
- Peer evaluation (other workers' assessment of contributions)

**Promotion Thresholds:** Configurable criteria that must be met before a worker is eligible for rank elevation:
- Minimum evidence volume (e.g., 50 completed tasks)
- Evidence quality thresholds (e.g., >85% success rate)
- Time-based recency requirements (evidence must be recent)

**Rank Hierarchy:**
- Rank 0: Task Worker — executes specific tasks, generates competence evidence
- Rank 1: Supervisor — oversees multiple workers, makes routing decisions
- Rank 2+: Manager — strategic planning, cross-service coordination

### Relationship to Existing Trust Annealing

Crabjar already has an automatic trust level annealing system (`crabjar-conductor/src/state_store.rs`) that upgrades workers based on task success counts alone (5 → level 1, 20 → level 2, 50 → level 3). This spec complements rather than replaces it:

- **Basic trust levels (0-3):** Continue using automatic task-count-based annealing for routine execution permissions. This handles the "can this worker run tasks?" question.
- **Supervisor/manager ranks:** Require explicit competence evaluation beyond task completion — decision accuracy, knowledge contributions, peer evaluations. This handles the "should this worker make decisions about others?" question.

The two systems use different terminology intentionally: "trust level" for automatic execution permissions, "rank" for explicit leadership roles requiring broader competence evidence.

### System Components

```
Worker Performance → Evidence Collector → Competence Evaluator → Promotion Engine
     ↑                    ↓                     ↓                      ↓
Task Execution      SQLite storage       Threshold checks        Rank update + audit log
```

## API Design

### POST /promotion/evaluate
Evaluate whether a worker meets promotion criteria.

**Request:**
```json
{
  "worker_id": "string",
  "target_rank": 1,
  "evidence_window_hours": 720
}
```

**Response:**
```json
{
  "eligible": true,
  "current_rank": 0,
  "target_rank": 1,
  "evidence_summary": {
    "tasks_completed": 127,
    "success_rate": 0.94,
    "decisions_reviewed": 45,
    "accuracy_rate": 0.89
  },
  "thresholds_met": [
    {"criterion": "minimum_tasks", "required": 50, "actual": 127, "met": true},
    {"criterion": "success_rate", "required": 0.85, "actual": 0.94, "met": true}
  ],
  "evidence_id": "ev_abc123"
}
```

### POST /promotion/execute
Execute promotion for an eligible worker.

**Request:**
```json
{
  "worker_id": "string",
  "target_rank": 1,
  "evidence_id": "ev_abc123",
  "rationale": "String explaining why this promotion is justified"
}
```

**Response:**
```json
{
  "success": true,
  "worker_id": "string",
  "previous_rank": 0,
  "new_rank": 1,
  "promotion_id": "pr_def456",
  "timestamp": "2026-10-27T12:00:00Z"
}
```

### GET /promotion/history/{worker_id}
Retrieve promotion history for a worker.

**Response:**
```json
{
  "worker_id": "string",
  "promotions": [
    {
      "promotion_id": "pr_def456",
      "from_rank": 0,
      "to_rank": 1,
      "timestamp": "2026-10-27T12:00:00Z",
      "evidence_id": "ev_abc123",
      "rationale": "String explaining why this promotion is justified"
    }
  ]
}
```

## Data Models

### WorkerCompetenceEvidence
```rust
pub struct WorkerCompetenceEvidence {
    pub evidence_id: String,
    pub worker_id: String,
    pub timestamp: DateTime<Utc>,
    pub task_completions: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub decisions_made: u32,
    pub accurate_decisions: u32,
    pub knowledge_contributions: u32,
    pub peer_evaluations: Vec<PeerEvaluation>,
}

pub struct PeerEvaluation {
    pub evaluator_id: String,
    pub score: f64,  // 0.0 to 1.0
    pub comment: Option<String>,
}
```

### PromotionThresholdConfig
```rust
pub struct PromotionThresholdConfig {
    pub target_rank: u8,
    pub minimum_tasks: u32,
    pub minimum_success_rate: f64,
    pub minimum_decisions: u32,
    pub minimum_accuracy_rate: f64,
    pub evidence_window_hours: u32,
}
```

### PromotionRecord
```rust
pub struct PromotionRecord {
    pub promotion_id: String,
    pub worker_id: String,
    pub from_rank: u8,
    pub to_rank: u8,
    pub timestamp: DateTime<Utc>,
    pub evidence_id: String,
    pub rationale: String,
    pub evaluator_id: Option<String>,  // If human override
}
```

## Phase Breakdown

### Phase 1: Evidence Collection Infrastructure
**Goal:** Collect and store worker competence metrics.

- Add evidence collection hooks to conductor task completion flow
- Create SQLite schema for `worker_competence_evidence` table
- Implement periodic evidence aggregation (hourly)
- Expose raw metrics via `/promotion/metrics/{worker_id}` endpoint

**Acceptance Criteria:**
- Worker task completions are automatically recorded with success/failure status
- Decision accuracy is tracked when human review or outcome data is available
- Evidence is queryable by worker ID and time window

### Phase 2: Threshold Configuration and Evaluation
**Goal:** Define promotion criteria and evaluate eligibility.

- Implement `PromotionThresholdConfig` storage (per-rank, per-fleet)
- Build competence evaluator that compares evidence against thresholds
- Create `/promotion/evaluate` endpoint
- Add CLI command: `crabjar promote evaluate --worker <id> --rank <n>`

**Acceptance Criteria:**
- Thresholds are configurable per target rank
- Evaluation returns detailed breakdown of which criteria are met/unmet
- Evidence window is configurable (default 720 hours = 30 days)

### Phase 3: Promotion Execution and Audit Trail
**Goal:** Execute promotions with full provenance.

- Implement `/promotion/execute` endpoint
- Update worker rank in conductor state
- Create promotion audit log with evidence linkage
- Add CLI command: `crabjar promote execute --worker <id> --rank <n> --evidence <id>`

**Acceptance Criteria:**
- Promotions are idempotent (same worker/rank/evidence = same result)
- Full audit trail from promotion back to raw evidence
- Workers can be demoted with similar audit trail

### Phase 4: Automatic Promotion Triggers
**Goal:** Trigger promotion evaluation automatically when workers cross thresholds.

- Add monitoring loop that checks for newly eligible workers
- Send notifications when promotion is recommended
- Optional auto-promotion mode (configurable per fleet)

**Acceptance Criteria:**
- Workers are evaluated for promotion within 1 hour of crossing threshold
- Notifications include evidence summary and eligibility status
- Auto-promotion can be enabled/disabled per rank transition

## Integration Points

### JEV Decision Layer
Promotion decisions should leverage JEV's existing infrastructure:
- Use JEV's evaluation endpoints for decision accuracy tracking
- Store promotion criteria as JEV decision rules
- Track promotion outcomes for continuous learning

### Guard Layers
Worker rank maps to guard trust layers:
- Rank 0 → Layer 1 (quarantine) — limited execution scope
- Rank 1 → Layer 2 (working) — standard execution scope
- Rank 2+ → Layer 3 (annealed) — broad execution scope

Promotion should automatically update the worker's guard layer assignment.

### Conductor Task Routing
Higher-rank workers should receive more complex tasks:
- Task complexity scoring based on historical difficulty
- Routing considers both worker rank and task complexity
- Evidence includes task complexity handled successfully

## Success Metrics

- [ ] Phase 1: Evidence collection working, raw metrics queryable
- [ ] Phase 2: Threshold evaluation returning accurate eligibility assessments
- [ ] Phase 3: Promotions executable with complete audit trail
- [ ] Phase 4: Automatic promotion triggers firing within SLA
- [ ] End-to-end: Worker promoted from Rank 0 to Rank 1 based on real performance evidence

## Open Questions

1. Should peer evaluations be required for supervisor promotions (Rank 1+)?
2. How should demotion be handled — automatic when performance drops, or manual only?
3. What's the appropriate evidence window for different rank transitions?
4. Should promotion thresholds be fleet-specific or global?

## Reversibility Notes

All changes are additive to existing conductor infrastructure:
- New SQLite tables can be dropped if needed
- API endpoints follow existing patterns and can be removed independently
- Worker rank is stored as a separate field, not modifying existing trust level semantics
- JEV integration uses existing evaluation endpoints, no changes required there
