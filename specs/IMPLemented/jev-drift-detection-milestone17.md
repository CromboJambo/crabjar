# Milestone 17: Configuration Drift Detection via JEV

## Overview

JEV learns optimal configuration drift detection decisions by evaluating system configuration against expected state, proposing remediation actions based on historical outcomes, and continuously improving infrastructure management through semantic evaluation.

## Problem

Traditional configuration drift detection is reactive and rule-based:
- Static drift scans report issues but don't decide optimal remediation strategy
- No learning from past drift decisions (which were accurate, which weren't)
- No optimization for speed vs accuracy trade-offs based on historical data
- No consideration of historical success rates for specific remediation approaches

JEV can learn which drift detection decisions are most effective by analyzing historical configuration and improvement data, making data-driven infrastructure management decisions.

## Architecture

### Components

1. **Drift Evaluation API** (`POST /drift/evaluate`)
   - Accepts configuration state information and expected baseline
   - Triggers JEV evaluation to determine optimal drift remediation action
   - Returns drift decision with confidence and rationale

2. **JEV Drift Quality Evaluator** (new decision criteria)
   - Evaluates proposed drift actions against quality metrics
   - Uses historical drift/remediation data to predict outcomes
   - Considers: drift severity, impact on services, user feedback, past remediation success rates

3. **Drift Outcome Tracker** (`POST /drift/outcome`)
   - Records actual outcomes of drift detection decisions
   - Feeds back into JEV's learning loop
   - Tracks: prediction accuracy, time to remediate, service impact, recurrence rate

### API Design

#### POST /drift/evaluate

Request body:
```json
{
  "system": "production-api",
  "configuration_state": {
    "expected_version": "1.2.3",
    "actual_version": "1.2.2",
    "changed_files": ["config.yaml", "env.sh"],
    "days_since_change": 7,
    "user_feedback_score": 4.1
  },
  "available_actions": [
    {"action": "revert_to_baseline"},
    {"action": "update_expected_baseline"},
    {"action": "manual_review"}
  ]
}
```

Response:
```json
{
  "success": true,
  "drift_decision": {
    "recommended_action": "revert_to_baseline",
    "confidence": 0.89,
    "rationale": "Based on historical drift remediation data and current configuration state (7 days since change, version mismatch), reverting to baseline will have the highest impact on service stability. Past reverts to similar configurations have shown that this approach minimizes user impact..."
  }
}
```

#### POST /drift/outcome

Request body:
```json
{
  "system": "production-api",
  "action_taken": "revert_to_baseline",
  "outcome": "success",
  "prediction_accuracy_percent": 95,
  "remediation_time_minutes": 12,
  "service_impact_score": 0.85
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

### Phase 17.1: Drift Evaluation API
- [ ] Create drift evaluation request/response types
- [ ] Implement POST /drift/evaluate endpoint
- [ ] Integrate with JEV decision evaluation using new criteria file

### Phase 17.2: JEV Drift Quality Evaluation
- [ ] Create drift-quality-evaluation.json criteria file
- [ ] Define quality metrics for drift evaluation
- [ ] Test JEV evaluation against historical drift data

### Phase 17.3: Drift Outcome Tracking
- [ ] Implement POST /drift/outcome endpoint
- [ ] Record actual drift detection outcomes for learning
- [ ] Provide feedback loop for continuous improvement

## Success Criteria

- Drift evaluations completed within 5 seconds
- JEV evaluation accuracy ≥70% on historical drift data
- Measurable improvement in service stability after JEV-powered drift decisions
- Continuous learning from drift outcomes improves future predictions
