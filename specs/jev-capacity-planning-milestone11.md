# Milestone 11: Capacity Planning via JEV

## Overview

JEV learns optimal capacity planning decisions by evaluating resource usage trends, proposing capacity adjustments based on historical prediction accuracy, and continuously improving infrastructure scaling through semantic evaluation.

## Problem

Traditional capacity planning is reactive and rule-based:
- Static thresholds trigger alerts but don't decide optimal response
- No learning from past capacity predictions (which were accurate, which weren't)
- No optimization for resource utilization or latency during scaling
- No consideration of historical success rates for specific capacity adjustments

JEV can learn which capacity planning decisions are most effective by analyzing historical usage trends and capacity decision outcomes, making data-driven infrastructure scaling decisions.

## Architecture

### Components

1. **Capacity Evaluation API** (`POST /capacity/evaluate`)
   - Accepts resource usage trends and current capacity information
   - Triggers JEV evaluation to determine appropriate capacity adjustment
   - Returns capacity planning decision with confidence and rationale

2. **JEV Capacity Planning Impact Evaluator** (new decision criteria)
   - Evaluates proposed capacity adjustments against efficiency metrics
   - Uses historical capacity data to predict outcomes
   - Considers: usage trends, seasonality patterns, growth rates, past prediction accuracy

3. **Capacity Outcome Tracker** (`POST /capacity/outcome`)
   - Records actual outcomes of capacity planning decisions
   - Feeds back into JEV's learning loop
   - Tracks: prediction accuracy, resource utilization after adjustment, time to adjust

### API Design

#### POST /capacity/evaluate

Request body:
```json
{
  "service": "pesti-runner",
  "usage_trends": {
    "cpu_7day_avg_percent": 65,
    "cpu_30day_avg_percent": 52,
    "memory_7day_avg_percent": 78,
    "growth_rate_percent_weekly": 12,
    "peak_hours_daily": ["9am-11am", "2pm-4pm"]
  },
  "current_capacity": {
    "instance_count": 5,
    "cpu_cores_per_instance": 8,
    "memory_gb_per_instance": 32
  },
  "available_adjustments": [
    {"action": "scale_up_instances", "instances": 2},
    {"action": "upgrade_instance_type", "new_type": "larger"},
    {"action": "add_memory_only", "additional_gb": 16}
  ]
}
```

Response:
```json
{
  "success": true,
  "capacity_decision": {
    "recommended_action": "scale_up_instances",
    "confidence": 0.87,
    "rationale": "Based on historical capacity data and current growth trends (12% weekly), scaling up instance count is the most effective approach. Memory utilization is approaching critical levels during peak hours..."
  }
}
```

#### POST /capacity/outcome

Request body:
```json
{
  "service": "pesti-runner",
  "action_taken": "scale_up_instances",
  "outcome": "success",
  "prediction_accuracy_percent": 92,
  "resource_utilization_after_percent": 68,
  "time_to_adjust_hours": 2
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

### Phase 11.1: Capacity Evaluation API
- [ ] Create capacity evaluation request/response types
- [ ] Implement POST /capacity/evaluate endpoint
- [ ] Integrate with JEV decision evaluation using new criteria file

### Phase 11.2: JEV Capacity Planning Impact Evaluation
- [ ] Create capacity-planning-evaluation.json criteria file
- [ ] Define efficiency metrics for capacity evaluation
- [ ] Test JEV evaluation against historical usage data

### Phase 11.3: Capacity Outcome Tracking
- [ ] Implement POST /capacity/outcome endpoint
- [ ] Record actual capacity outcomes for learning
- [ ] Provide feedback loop for continuous improvement

## Success Criteria

- Capacity evaluations completed within 5 seconds
- JEV evaluation accuracy ≥70% on historical usage data
- Measurable improvement in resource utilization after JEV-powered capacity adjustments
- Continuous learning from capacity outcomes improves future predictions
