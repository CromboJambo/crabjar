# Conductor HTTP API Reference

The crabjar conductor provides a comprehensive HTTP API for fleet coordination, task management, and decision-making services. All endpoints return structured JSON responses.

## Base URL

```
http://localhost:8090/api/v1
```

## Authentication

Most endpoints are public. Protected endpoints require JWT Bearer token authentication via the `Authorization` header.

```bash
curl -H "Authorization: Bearer <token>" http://localhost:8090/api/v1/goals
```

## Fleet Management

### Get Fleet Status

Retrieve current status of all registered workers and fleet health metrics.

```http
GET /fleet/status
```

**Response:**
```json
{
  "success": true,
  "workers_online": 5,
  "tasks_in_progress": 12,
  "avg_completion_time_ms": 45000,
  "last_updated": "2026-11-04T12:00:00Z"
}
```

## Worker Management

### Register Worker

Register a new worker node with the conductor.

```http
POST /workers/register
Content-Type: application/json

{
  "hostname": "worker-01",
  "capabilities": ["shell", "browser"],
  "max_concurrent_tasks": 4
}
```

**Response:**
```json
{
  "success": true,
  "worker_id": "w-abc123",
  "status": "registered"
}
```

### Worker Login

Authenticate a worker for task polling.

```http
POST /workers/login
Content-Type: application/json

{
  "hostname": "worker-01",
  "token": "<auth-token>"
}
```

### Poll for Tasks

Worker polls for available tasks assigned to it.

```http
POST /workers/poll
Content-Type: application/json

{
  "worker_id": "w-abc123"
}
```

### Worker Heartbeat

Report worker liveness.

```http
POST /workers/heartbeat
Content-Type: application/json

{
  "worker_id": "w-abc123",
  "status": "idle"
}
```

### List Workers

List all registered workers (requires auth).

```http
GET /workers
Authorization: Bearer <token>
```

## Task Management

### Submit Goal

Submit a new goal for task decomposition and execution.

```http
POST /goals
Authorization: Bearer <token>
Content-Type: application/json

{
  "description": "Deploy service to production",
  "priority": "high",
  "tags": ["deployment", "production"]
}
```

### Get Goal Status

Check status of a submitted goal.

```http
GET /goals/{id}
Authorization: Bearer <token>
```

### Report Task Result

Worker reports completion of assigned task.

```http
POST /tasks/{id}/result
Content-Type: application/json

{
  "status": "completed",
  "output": "Task finished successfully",
  "duration_ms": 120000
}
```

## Decision Evaluation

### Evaluate Decision

Submit a decision for conductor evaluation.

```http
POST /evaluations/decision
Content-Type: application/json

{
  "type": "risk_assessment",
  "context": {
    "action": "deploy_service",
    "environment": "production",
    "confidence": 0.85
  }
}
```

### Get Evaluation Status

Poll status of a background decision evaluation.

```http
GET /decide/evaluate/{id}
```

### List Decision Capabilities

Get registered decision capabilities (public).

```http
GET /decisions/capabilities
```

## Sandbox Management

### Create Sandbox

Create a new isolated execution environment.

```http
POST /sandboxes
Content-Type: application/json

{
  "name": "test-env-01",
  "config": { "timeout_seconds": 3600 }
}
```

### List Sandboxes

List all active sandboxes.

```http
GET /sandboxes
```

### Get Sandbox Details

Retrieve details of a specific sandbox.

```http
GET /sandboxes/{id}
```

### Destroy Sandbox

Remove a sandbox environment.

```http
DELETE /sandboxes/{id}
```

### Assign Worker to Sandbox

Assign a worker to operate within a sandbox.

```http
POST /sandboxes/{id}/worker
Content-Type: application/json

{
  "worker_id": "w-abc123"
}
```

## JEV Semantic Decisions

### Evaluate Structural Decision

Evaluate a decision using Jev/SemIf-style semantic branching.

```http
POST /jev/evaluate
Content-Type: application/json

{
  "expression": "risk_level > medium AND confidence < high",
  "variables": {
    "risk_level": "high",
    "confidence": "medium"
  }
}
```

### Postprocess Decision

Postprocess a decision result.

```http
POST /jev/postprocess
Content-Type: application/json

{
  "decision_id": "d-123",
  "action": "log_outcome"
}
```

### Record Feedback

Record feedback on a decision outcome.

```http
POST /jev/feedback
Content-Type: application/json

{
  "decision_id": "d-123",
  "correct": true,
  "notes": "Decision was accurate"
}
```

## Schema Management

### Propose Schema Change

Propose a schema change for review.

```http
POST /schema/propose
Content-Type: application/json

{
  "table": "tasks",
  "change_type": "add_column",
  "column_name": "priority_score",
  "column_type": "INTEGER"
}
```

## Routing Decisions

### Decide Routing

Get routing decision for a task.

```http
POST /route/decide
Content-Type: application/json

{
  "task_type": "code_review",
  "complexity": "high",
  "available_workers": ["w-1", "w-2", "w-3"]
}
```

### Record Routing Outcome

Record the outcome of a routing decision.

```http
POST /route/outcome
Content-Type: application/json

{
  "routing_id": "r-123",
  "successful": true,
  "duration_ms": 30000
}
```

## Dependencies

### Declare Dependencies

Declare task dependencies.

```http
POST /dependencies/declare
Content-Type: application/json

{
  "task_id": "t-123",
  "depends_on": ["t-100", "t-101"]
}
```

## Upgrade Management

### Evaluate Upgrade

Evaluate whether an upgrade should be performed.

```http
POST /upgrade/evaluate
Content-Type: application/json

{
  "component": "orchestrator",
  "current_version": "0.12.5",
  "target_version": "0.13.0"
}
```

### Record Upgrade Outcome

Record the result of an upgrade.

```http
POST /upgrade/outcome
Content-Type: application/json

{
  "upgrade_id": "u-123",
  "successful": true,
  "rollback_required": false
}
```

## Health Monitoring

### Evaluate Health

Evaluate system health across components.

```http
POST /health/evaluate
Content-Type: application/json

{
  "components": ["database", "cache", "network"]
}
```

### Record Recovery Outcome

Record the outcome of a recovery action.

```http
POST /health/outcome
Content-Type: application/json

{
  "recovery_id": "rc-123",
  "successful": true,
  "time_to_recovery_ms": 5000
}
```

## Cost Optimization

### Evaluate Cost

Evaluate cost optimization opportunities.

```http
POST /cost/evaluate
Content-Type: application/json

{
  "workload_type": "batch_processing",
  "duration_hours": 24,
  "compute_units": 10
}
```

### Record Optimization Outcome

Record the outcome of a cost optimization.

```http
POST /optimization/outcome
Content-Type: application/json

{
  "optimization_id": "opt-123",
  "savings_percentage": 15.5,
  "successful": true
}
```

## Disaster Recovery

### Evaluate Recovery

Evaluate disaster recovery options.

```http
POST /recovery/evaluate
Content-Type: application/json

{
  "failure_type": "database_corruption",
  "affected_components": ["primary_db"],
  "last_backup_age_hours": 2
}
```

### Record Disaster Recovery Outcome

Record the outcome of a disaster recovery operation.

```http
POST /recovery/outcome
Content-Type: application/json

{
  "recovery_id": "dr-123",
  "successful": true,
  "data_loss_records": 0,
  "time_to_recovery_ms": 120000
}
```

## Capacity Management

### Evaluate Capacity

Evaluate capacity planning needs.

```http
POST /capacity/evaluate
Content-Type: application/json

{
  "current_load_percent": 75,
  "projected_growth_percent": 20,
  "time_horizon_days": 30
}
```

### Record Capacity Outcome

Record the outcome of a capacity adjustment.

```http
POST /capacity/outcome
Content-Type: application/json

{
  "capacity_id": "cap-123",
  "successful": true,
  "new_capacity_units": 15
}
```

## Security Management

### Evaluate Security

Evaluate security posture.

```http
POST /security/evaluate
Content-Type: application/json

{
  "vulnerabilities_found": 3,
  "severity_distribution": {
    "critical": 0,
    "high": 1,
    "medium": 2,
    "low": 0
  }
}
```

### Record Security Outcome

Record the outcome of a security action.

```http
POST /security/outcome
Content-Type: application/json

{
  "security_id": "sec-123",
  "successful": true,
  "vulnerabilities_patched": 2
}
```

## Compliance Management

### Evaluate Compliance

Evaluate compliance status.

```http
POST /compliance/evaluate
Content-Type: application/json

{
  "framework": "SOC2",
  "controls_tested": 50,
  "controls_passed": 48
}
```

### Record Compliance Outcome

Record the outcome of a compliance action.

```http
POST /compliance/outcome
Content-Type: application/json

{
  "compliance_id": "comp-123",
  "successful": true,
  "gaps_addressed": 2
}
```

## Documentation Management

### Evaluate Documentation

Evaluate documentation quality and completeness.

```http
POST /docs/evaluate
Content-Type: application/json

{
  "component": "api-reference",
  "metrics": ["completeness", "accuracy", "clarity"]
}
```

### Record Documentation Outcome

Record the outcome of a documentation update.

```http
POST /docs/outcome
Content-Type: application/json

{
  "doc_id": "doc-123",
  "successful": true,
  "sections_updated": 5
}
```

## Monitoring Management

### Evaluate Monitoring

Evaluate monitoring coverage and effectiveness.

```http
POST /monitoring/evaluate
Content-Type: application/json

{
  "systems_monitored": ["api", "database", "cache"],
  "alert_thresholds": { "response_time_ms": 1000 }
}
```

### Record Monitoring Outcome

Record the outcome of a monitoring action.

```http
POST /monitoring/outcome
Content-Type: application/json

{
  "monitoring_id": "mon-123",
  "successful": true,
  "alerts_triggered": 0
}
```

## Drift Management

### Evaluate Drift

Evaluate configuration drift.

```http
POST /drift/evaluate
Content-Type: application/json

{
  "baseline_version": "v1.0",
  "current_version": "v1.2"
}
```

### Record Drift Outcome

Record the outcome of a drift correction.

```http
POST /drift/outcome
Content-Type: application/json

{
  "drift_id": "dr-123",
  "successful": true,
  "configurations_corrected": 3
}
```

## Rules Management

### Extract Rules

Extract rules from decision history.

```http
POST /rules/extract
Content-Type: application/json

{
  "time_range_days": 30,
  "minimum_frequency": 10
}
```

### Promote Rule

Promote a discovered rule to production.

```http
POST /rules/promote
Content-Type: application/json

{
  "rule_id": "r-123",
  "confidence_threshold": 0.9
}
```

## Promotion Management

### Evaluate Promotion

Evaluate whether a worker should be promoted.

```http
GET /promotion/evaluate/{worker_id}
```

### Execute Promotion

Execute a worker promotion.

```http
POST /promotion/execute/{worker_id}
Content-Type: application/json

{
  "new_level": 3,
  "reason": "Consistent high-quality output"
}
```

### Get Promotion History

Retrieve promotion history for a worker.

```http
GET /promotion/history/{worker_id}
```

### Get Worker Metrics

Retrieve performance metrics for a worker.

```http
GET /promotion/metrics/{worker_id}
```

## Audit Log

### Query Audit Log

Query the audit log for training pipeline evaluation.

```http
GET /audit/log
```

**Response:**
```json
{
  "success": true,
  "events": [...],
  "count": 1000
}
```

## Interface Optimization

### Propose Interface Optimization

Propose an optimization to a service interface.

```http
POST /interface/propose
Content-Type: application/json

{
  "service": "api-gateway",
  "optimization_type": "caching",
  "expected_improvement_percent": 25
}
```

## Review Management

### Submit Review

Submit a review for completed work.

```http
POST /review
Authorization: Bearer <token>
Content-Type: application/json

{
  "task_id": "t-123",
  "approved": true,
  "comments": "Looks good"
}
```

## Goals Management (Protected)

### Submit Goal

Submit a new goal for task decomposition.

```http
POST /goals
Authorization: Bearer <token>
Content-Type: application/json

{
  "description": "Implement feature X",
  "priority": "high",
  "tags": ["feature", "q4"]
}
```

### Get Goal Status

Check status of a submitted goal.

```http
GET /goals/{id}
Authorization: Bearer <token>
```

## Metrics

### Get Cache Stats

Retrieve cache performance statistics.

```http
GET /metrics/cache
```

**Response:**
```json
{
  "success": true,
  "cache_hit_rate": 0.85,
  "total_requests": 10000,
  "cached_responses": 8500
}
```