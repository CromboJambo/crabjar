# JEV-Powered Capacity Planning API — Architecture Spec (Milestone 19)

## Overview
Extend crabjar-conductor with JEV-powered capacity planning and resource optimization. Components submit capacity evaluation requests, conductor uses JEV semantic decision evaluation to select optimal capacity adjustment actions, and actual outcomes are recorded for continuous learning.

## New Endpoints

### POST /capacity/evaluate — Evaluate Capacity Needs & Recommend Action
Submit a capacity evaluation request; receive JEV-optimized capacity adjustment recommendation.

**Request Body:**
```json
{
  "system": "api-gateway",
  "current_capacity": {
    "instances": 3,
    "cpu_utilization_percent": 85,
    "memory_utilization_percent": 72,
    "queue_depth": 150
  },
  "available_actions": [
    {"action": "scale_up_instances", "details": "Add 2 instances"},
    {"action": "upgrade_instance_type", "details": "Move to larger instance class"}
  ]
}
```

**Response Body:**
```json
{
  "success": true,
  "capacity_decision": {
    "recommended_action": "scale_up_instances",
    "confidence": 0.87,
    "rationale": "High CPU utilization trend indicates scaling up will prevent saturation..."
  }
}
```

### POST /capacity/outcome — Record Capacity Outcome for Learning
Record the actual outcome of a capacity adjustment to improve future JEV predictions.

**Request Body:**
```json
{
  "system": "api-gateway",
  "action_taken": "scale_up_instances",
  "outcome": "successful",
  "prediction_accuracy_percent": 92,
  "actual_cost_increase_dollars": 150.00
}
```

**Response Body:**
```json
{
  "success": true,
  "learned": true,
  "updated_model": true,
  "system": "api-gateway"
}
```

## JEV Capacity Evaluator Design

### Evaluation Factors
- Resource usage trends and growth rates
- Historical capacity prediction accuracy
- Cost vs. performance trade-offs
- Time sensitivity and user impact during scaling
- Risk of over-provisioning vs. under-provisioning

### Action Options
- `scale_up_instances` — Add more instances to handle load
- `upgrade_instance_type` — Move to larger instance class
- `add_memory_only` — Increase memory without CPU changes
- `enable_auto_scaling` — Implement dynamic scaling policies
- `optimize_resource_allocation` — Redistribute resources between services

## Learning System Design

### Outcome Recording Flow
1. Component executes JEV-recommended capacity action
2. Component measures actual outcome (performance, cost, accuracy)
3. Component submits outcome via POST /capacity/outcome
4. Conductor stores outcome in decision history database
5. Future JEV evaluations consider historical accuracy metrics

### Continuous Improvement Loop
- JEV evaluates current capacity needs → recommends optimal action
- Component executes action and measures actual results
- Outcome recorded with prediction accuracy metrics
- JEV learns from outcomes to improve future predictions
- Over time, capacity decisions become increasingly accurate and cost-effective

## Implementation Plan

1. Add `CapacityEvaluationRequest` and `CapacityOutcomeRequest` types to api.rs
2. Implement `evaluate_capacity_handler` using spawn_blocking + blocking_lock pattern
3. Implement `record_capacity_outcome_handler` for learning from actual outcomes
4. Create JEV criteria file: `specs/criteria/capacity-planning-evaluation.json`
5. Register routes in conductor API router
6. Add unit tests for capacity evaluation and outcome recording
