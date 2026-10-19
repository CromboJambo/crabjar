# JEV-Powered Security Hardening API — Architecture Spec (Milestone 21)

## Overview
Extend crabjar-conductor with JEV-powered security hardening decision-making. Components submit security posture evaluation requests, conductor uses JEV semantic decision evaluation to select optimal security actions, and actual outcomes are recorded for continuous learning.

## New Endpoints

### POST /security/evaluate — Evaluate Security Posture & Recommend Hardening Action
Submit a security posture evaluation request; receive JEV-optimized security hardening action recommendation.

**Request Body:**
```json
{
  "system": "web-application",
  "security_posture": {
    "vulnerability_count": 12,
    "critical_vulns": 2,
    "last_scan_days_ago": 3,
    "patch_level": "outdated"
  },
  "available_actions": [
    {"action": "apply_patch", "details": "Apply CVE-2026-12345 patch"},
    {"action": "upgrade_component", "details": "Upgrade to latest version"}
  ]
}
```

**Response Body:**
```json
{
  "success": true,
  "security_decision": {
    "recommended_action": "apply_patch",
    "confidence": 0.89,
    "rationale": "Critical vulnerability with available patch should be addressed immediately..."
  }
}
```

### POST /security/outcome — Record Security Outcome for Learning
Record the actual outcome of a security hardening action to improve future JEV predictions.

**Request Body:**
```json
{
  "system": "web-application",
  "action_taken": "apply_patch",
  "outcome": "successful",
  "vulnerabilities_fixed": 2,
  "downtime_minutes": 15
}
```

**Response Body:**
```json
{
  "success": true,
  "learned": true,
  "updated_model": true,
  "system": "web-application"
}
```

## JEV Security Evaluator Design

### Evaluation Factors
- Vulnerability severity and exploit availability
- Current system security posture (patch level, scan recency)
- Historical success rates for each security action
- Risk tolerance and business impact considerations
- Time sensitivity of vulnerability exploitation

### Action Options
- `apply_patch` — Apply specific vulnerability patch or update
- `upgrade_component` — Upgrade entire component to latest version
- `add_compensating_control` — Add firewall rule, WAF, or other compensating measure
- `isolate_affected_system` — Quarantine vulnerable system from network
- `accept_risk_temporarily` — Document and accept risk with monitoring

## Learning System Design

### Outcome Recording Flow
1. Component executes JEV-recommended security action
2. Component measures actual outcome (vulnerabilities fixed, downtime)
3. Component submits outcome via POST /security/outcome
4. Conductor stores outcome in decision history database
5. Future JEV evaluations consider historical accuracy metrics

### Continuous Improvement Loop
- JEV evaluates current security posture → recommends optimal hardening action
- Component executes action and measures actual results
- Outcome recorded with vulnerabilities fixed and downtime metrics
- JEV learns from outcomes to improve future predictions
- Over time, security hardening decisions become increasingly effective and efficient

## Implementation Plan

1. Add `SecurityEvaluationRequest` and `SecurityOutcomeRequest` types to api.rs
2. Implement `evaluate_security_handler` using spawn_blocking + blocking_lock pattern
3. Implement `record_security_outcome_handler` for learning from actual outcomes
4. Create JEV criteria file: `specs/criteria/security-hardening-evaluation.json`
5. Register routes in conductor API router
6. Add unit tests for security evaluation and outcome recording
