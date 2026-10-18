# Milestone 8: Cross-Service Dependency Management via JEV

## Overview

JEV learns optimal cross-service dependency management by evaluating service version compatibility, upgrade impact on dependent services, and coordinating upgrades to minimize disruption across the crabjar fleet.

## Problem

Managing dependencies between distributed microservices is complex:
- Binary compatibility breaks (API changes, schema evolution)
- Version skew between components causes subtle failures
- Upgrade coordination requires manual analysis of dependency graphs
- No learning from past upgrade outcomes (which version combinations work well together)

JEV can learn which dependency management decisions are most effective by analyzing historical upgrade outcomes and making data-driven compatibility assessments.

## Architecture

### Components

1. **Dependency Declaration API** (`POST /dependencies/declare`)
   - Accepts service dependency declarations with version constraints
   - Validates against known compatible version combinations
   - Returns validation result with JEV-based confidence assessment

2. **Upgrade Impact Evaluator** (`POST /upgrade/evaluate`)
   - Evaluates proposed service upgrades against dependent services
   - Uses JEV to predict compatibility impact based on historical data
   - Returns risk assessment and recommended upgrade sequence

3. **Upgrade Outcome Tracker** (`POST /upgrade/outcome`)
   - Records actual outcomes of upgrade decisions
   - Feeds back into JEV's learning loop
   - Tracks: success/failure, downtime, compatibility issues, rollback frequency

### API Design

#### POST /dependencies/declare

Request body:
```json
{
  "service": "crabjar-conductor",
  "version": "0.12.0",
  "dependencies": [
    {"service": "pesti-runner", "min_version": "0.8.0", "max_version": "0.9.9"},
    {"service": "crabjar-agent", "min_version": "0.11.0", "max_version": null}
  ]
}
```

Response:
```json
{
  "success": true,
  "validation": {
    "compatible": true,
    "confidence": 0.92,
    "rationale": "Based on historical upgrade data, conductor v0.12.0 has been successfully deployed with pesti-runner versions in the specified range..."
  }
}
```

#### POST /upgrade/evaluate

Request body:
```json
{
  "service": "pesti-runner",
  "current_version": "0.8.5",
  "proposed_version": "0.9.2",
  "dependent_services": ["crabjar-conductor"]
}
```

Response:
```json
{
  "success": true,
  "upgrade_assessment": {
    "risk_level": "low",
    "confidence": 0.88,
    "rationale": "Historical data shows pesti-runner upgrades from 0.8.x to 0.9.x have 95% compatibility with conductor v0.12.0. Previous similar upgrades resulted in minimal disruption..."
  },
  "recommended_sequence": ["pesti-runner", "crabjar-conductor"]
}
```

## Implementation Plan

### Phase 8.1: Dependency Declaration API
- [ ] Create dependency declaration request/response types
- [ ] Implement POST /dependencies/declare endpoint
- [ ] Integrate with JEV decision evaluation using new criteria file

### Phase 8.2: JEV Upgrade Impact Evaluation
- [ ] Create upgrade-impact-evaluation.json criteria file
- [ ] Define efficiency metrics for upgrade evaluation
- [ ] Test JEV evaluation against historical upgrade data

### Phase 8.3: Upgrade Outcome Tracking
- [ ] Implement POST /upgrade/outcome endpoint
- [ ] Record actual upgrade outcomes for learning
- [ ] Provide feedback loop for continuous improvement

## Success Criteria

- Dependency declarations validated within 5 seconds
- JEV evaluation accuracy ≥70% on historical upgrade data
- Measurable reduction in upgrade-related downtime after JEV-powered coordination
- Continuous learning from upgrade outcomes improves future decisions
