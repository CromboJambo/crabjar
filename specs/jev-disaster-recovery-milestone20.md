# JEV-Powered Disaster Recovery API — Architecture Spec (Milestone 20)

## Overview
Extend crabjar-conductor with JEV-powered disaster recovery decision-making. Components submit disaster recovery evaluation requests, conductor uses JEV semantic decision evaluation to select optimal recovery actions, and actual outcomes are recorded for continuous learning.

## New Endpoints

### POST /recovery/evaluate — Evaluate Disaster State & Recommend Recovery Action
Submit a disaster state evaluation request; receive JEV-optimized recovery action recommendation.

**Request Body:**
```json
{
  "system": "primary-database",
  "failure_scenario": "hardware-failure",
  "system_state": {
    "uptime_hours": 168,
    "last_backup_age_hours": 24,
    "replica_count": 3,
    "data_loss_risk_percent": 15
  },
  "available_actions": [
    {"action": "restore_from_backup", "details": "Restore from last full backup"},
    {"action": "failover_to_replica", "details": "Promote primary replica"}
  ]
}
```

**Response Body:**
```json
{
  "success": true,
  "recovery_decision": {
    "recommended_action": "failover_to_replica",
    "confidence": 0.92,
    "rationale": "Recent replica with low data loss risk provides faster recovery than backup restore..."
  }
}
```

### POST /recovery/outcome — Record Recovery Outcome for Learning
Record the actual outcome of a disaster recovery action to improve future JEV predictions.

**Request Body:**
```json
{
  "system": "primary-database",
  "action_taken": "failover_to_replica",
  "outcome": "successful",
  "recovery_time_minutes": 12,
  "actual_data_loss_percent": 0.5
}
```

**Response Body:**
```json
{
  "success": true,
  "learned": true,
  "updated_model": true,
  "system": "primary-database"
}
```

## JEV Recovery Evaluator Design

### Evaluation Factors
- Failure scenario type and severity
- Current system state (uptime, backup age, replica count)
- Historical success rates for each recovery action
- Time sensitivity and SLA requirements
- Data loss potential vs. recovery speed trade-offs

### Action Options
- `restore_from_backup` — Restore from last full/incremental backup
- `failover_to_replica` — Promote primary or secondary replica
- `manual_repair` — Trigger manual intervention for complex failures
- `rollback_to_previous_version` — Roll back to known-good state
- `redeploy_from_scratch` — Complete redeployment of affected system

## Learning System Design

### Outcome Recording Flow
1. Component executes JEV-recommended recovery action
2. Component measures actual outcome (recovery time, data loss, success)
3. Component submits outcome via POST /recovery/outcome
4. Conductor stores outcome in decision history database
5. Future JEV evaluations consider historical accuracy metrics

### Continuous Improvement Loop
- JEV evaluates current disaster state → recommends optimal recovery action
- Component executes action and measures actual results
- Outcome recorded with recovery time and data loss metrics
- JEV learns from outcomes to improve future predictions
- Over time, disaster recovery decisions become increasingly effective and efficient

## Implementation Plan

1. Add `DisasterRecoveryEvaluationRequest` and `DisasterRecoveryOutcomeRequest` types to api.rs
2. Implement `evaluate_recovery_handler` using spawn_blocking + blocking_lock pattern
3. Implement `record_disaster_recovery_outcome_handler` for learning from actual outcomes
4. Create JEV criteria file: `specs/criteria/disaster-recovery-evaluation.json`
5. Register routes in conductor API router
6. Add unit tests for recovery evaluation and outcome recording
