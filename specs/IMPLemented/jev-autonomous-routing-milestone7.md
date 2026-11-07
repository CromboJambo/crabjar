# Milestone 7: Autonomous Task Routing via JEV

## Overview

JEV learns optimal task routing decisions by evaluating task characteristics against service capabilities and historical routing outcomes, continuously improving dispatch efficiency across the crabjar fleet.

## Problem

Traditional capability-based task assignment is static:
- Binary matching (worker has capability X → assign task requiring X)
- No learning from past assignments (which workers handle similar tasks faster/better)
- No optimization for resource utilization or latency
- No consideration of worker load, skill proficiency, or historical success rates

JEV can learn which routing decisions are most efficient by analyzing historical assignment outcomes and making data-driven dispatch recommendations.

## Architecture

### Components

1. **Autonomous Routing Decision API** (`POST /route/decide`)
   - Accepts task description and available worker information
   - Triggers JEV evaluation to determine optimal worker assignment
   - Returns routing decision with confidence and rationale

2. **JEV Routing Impact Evaluator** (new decision criteria)
   - Evaluates proposed routing against efficiency metrics
   - Uses historical assignment data to predict outcomes
   - Considers: task complexity, worker proficiency, current load, historical success rates

3. **Routing Outcome Tracker** (`POST /route/outcome`)
   - Records actual outcomes of routing decisions
   - Feeds back into JEV's learning loop
   - Tracks: completion time, success/failure, resource usage, quality metrics

### API Design

#### POST /route/decide

Request body:
```json
{
  "task_id": "t-123",
  "task_type": "code_review",
  "complexity": "medium",
  "required_capabilities": ["rust", "security"],
  "available_workers": [
    {"worker_id": "w-1", "current_load": 0.3, "historical_success_rate": 0.85},
    {"worker_id": "w-2", "current_load": 0.7, "historical_success_rate": 0.92}
  ]
}
```

Response:
```json
{
  "success": true,
  "routing_decision": {
    "assigned_worker": "w-1",
    "confidence": 0.88,
    "rationale": "Worker w-1 has lower current load (0.3 vs 0.7) and sufficient historical success rate (0.85). While w-2 has higher success rate, the load difference suggests better overall throughput with w-1..."
  }
}
```

#### POST /route/outcome

Request body:
```json
{
  "task_id": "t-123",
  "assigned_worker": "w-1",
  "outcome": "success",
  "completion_time_seconds": 450,
  "quality_score": 0.9
}
```

Response:
```json
{
  "success": true,
  "learned": true,
  "updated_model": true
}
```

## Implementation Plan

### Phase 7.1: Autonomous Routing Decision API
- [ ] Create routing decision request/response types
- [ ] Implement POST /route/decide endpoint
- [ ] Integrate with JEV decision evaluation using new criteria file

### Phase 7.2: JEV Routing Impact Evaluation
- [ ] Create autonomous-routing-evaluation.json criteria file
- [ ] Define efficiency metrics for routing evaluation
- [ ] Test JEV evaluation against historical routing data

### Phase 7.3: Routing Outcome Tracking
- [ ] Implement POST /route/outcome endpoint
- [ ] Record actual routing outcomes for learning
- [ ] Provide feedback loop for continuous improvement

## Success Criteria

- Routing decisions made within 5 seconds
- JEV evaluation accuracy ≥70% on historical routing data
- Measurable improvement in task completion times after JEV-powered routing
- Continuous learning from routing outcomes improves future decisions
