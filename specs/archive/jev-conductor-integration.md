# JEV Conductor API Integration Specification

**Status:** Proposed  
**Last Updated:** 2026-10-17  
**Author:** crabjar team (CromboJambo)

## Overview

Design for automatic JEV semantic decision-making triggered by goals submitted to the crabjar conductor. The integration adds decision gates at key points in the goal lifecycle: before task execution, during annealing/trust-update loops, and as part of fleet-wide policy enforcement.

This spec builds on the existing `crabjar-conductor` crate architecture and integrates with pesti-server for goal decomposition, crabjar-guard for ExecutionGate patterns, and the JEV CLI infrastructure defined in `jev-cli-integration.md`.

## Architecture

### Decision Points in Goal Lifecycle

JEV decisions integrate at three key points in the conductor's goal processing pipeline:

```
Goal submitted → [Decision Gate 1: Pre-execution] → Task decomposition
       ↓
Tasks executed → [Decision Gate 2: Annealing/Trust Update] → Result aggregation
       ↓
Fleet-wide policy → [Decision Gate 3: Cross-goal coordination] → Final outcome
```

### Decision Gate 1: Pre-Execution Readiness

Before the conductor decomposes a goal into tasks, evaluate whether current fleet conditions are favorable for execution.

**Trigger:** `POST /goals` endpoint receives new goal

**Criteria used:** `deployment-gate.json` (or custom criteria specified in goal metadata)

**Flow:**
1. Conductor receives goal with optional `decision_criteria` field
2. If criteria specified, call JEV evaluate against current fleet state description
3. Based on decision:
   - `proceed`: Continue to task decomposition
   - `hold`: Queue goal for later execution, notify user
   - `abort`: Reject goal with rationale, log decision

### Decision Gate 2: Annealing/Trust Update Loop

During the conductor's iterative annealing process (where tasks are refined and trust scores updated), use JEV to evaluate whether current task outcomes justify continuing or adjusting strategy.

**Trigger:** After each task completion batch in annealing loop

**Criteria used:** Custom criteria defined per goal type

**Flow:**
1. Task batch completes with mixed success/failure results
2. Conductor summarizes outcomes into observation text
3. JEV evaluates whether to continue, adjust strategy, or escalate
4. Decision influences next annealing iteration parameters

### Decision Gate 3: Cross-Goal Coordination

When multiple goals are competing for fleet resources, use JEV to make prioritization decisions based on semantic understanding of goal urgency and impact.

**Trigger:** Resource contention detected (worker queue exceeds threshold)

**Criteria used:** `alert-classification.json` or custom priority criteria

**Flow:**
1. Conductor detects resource contention across active goals
2. Summarize each competing goal's context into observation text
3. JEV evaluates relative urgency/impact
4. Adjust task scheduling priorities based on decisions

## API Design

### Goal Submission with Decision Criteria

Extend the `POST /goals` endpoint to accept optional decision criteria configuration:

```hemlock
POST /goals
{
    "description": "Deploy new version of service X",
    "priority": "high",
    
    // NEW: JEV decision integration
    "decision_criteria": {
        "pre_execution": {
            "criteria_file": "/opt/crabjar/criteria/deployment-gate.json",
            "threshold": 0.85,
            "on_hold": "queue_for_later",
            "on_abort": "notify_and_log"
        },
        "annealing": {
            "criteria_file": "/opt/crabjar/criteria/service-triage.json",
            "observation_template": "Task batch completed: {success_count} succeeded, {failure_count} failed. Failure patterns: {failure_summary}"
        }
    }
}
```

### Decision Results in Goal Status

Goal status responses include decision metadata:

```hemlock
GET /goals/{id}
{
    "id": "goal-123",
    "description": "Deploy new version of service X",
    "status": "running",
    
    // NEW: Decision history
    "decisions": [
        {
            "gate": "pre_execution",
            "timestamp": "2026-10-17T15:30:00Z",
            "criteria_file": "/opt/crabjar/criteria/deployment-gate.json",
            "observation": "Fleet load: 65% CPU, 4 of 8 workers available. Recent deployments: 2 successful in last hour.",
            "result": {
                "option": "proceed",
                "confidence": 0.89,
                "rationale": "High similarity to ready-for-deployment criterion"
            }
        },
        {
            "gate": "annealing",
            "timestamp": "2026-10-17T15:45:00Z",
            "criteria_file": "/opt/crabjar/criteria/service-triage.json",
            "observation": "Task batch completed: 3 succeeded, 1 failed. Failure patterns: timeout on worker node ftw3.",
            "result": {
                "option": "hold",
                "confidence": 0.78,
                "rationale": "Moderate similarity to network issue criterion"
            }
        }
    ],
    
    "tasks": [ ... ]
}
```

### Decision Configuration Endpoint

Manage fleet-wide decision criteria defaults:

```hemlock
GET /config/decisions
{
    "default_criteria": {
        "pre_execution": "/opt/crabjar/criteria/deployment-gate.json",
        "annealing": null,  // No default; must be specified per goal
        "resource_contention": "/opt/crabjar/criteria/alert-classification.json"
    },
    "thresholds": {
        "pre_execution": 0.85,
        "annealing": 0.75,
        "resource_contention": 0.80
    }
}

PUT /config/decisions
{
    "default_criteria": { ... },
    "thresholds": { ... }
}
```

## Implementation Details

### Conductor Crate Changes (`crabjar-conductor/src/`)

New module structure:

```
crabjar-conductor/src/
├── decision/
│   ├── mod.rs           # Decision gate orchestration (~150 LoC)
│   ├── evaluate.rs      # JEV evaluation client (~250 LoC)
│   └── policy.rs        # Decision policy configuration (~200 LoC)
├── conductor.rs         # Updated to call decision gates
├── task_scheduler.rs    # Updated for resource contention decisions
└── ... (existing modules)
```

### JEV Evaluation Client (`decision/evaluate.rs`)

The evaluation client wraps the JEV CLI tool or calls llama-server directly:

```hemlock
pub struct Jevevaluator {
    backend: EmbeddingBackend,  // llama-server HTTP client
    criteria_cache: HashMap<String, DecisionCriteria>,
}

impl Jevevaluator {
    pub async fn evaluate(
        &self, 
        observation: &str, 
        criteria_file: &str
    ) -> Result<DecisionResult, EvaluationError> {
        // Load or retrieve cached criteria
        let criteria = self.load_criteria(criteria_file).await?;
        
        // Embed observation and criteria options
        let obs_embedding = self.backend.embed(observation).await?;
        let option_embeddings = self.embed_options(&criteria.options).await?;
        
        // Compute similarities and find best match
        let result = semantic::evaluate(&obs_embedding, &option_embeddings);
        
        // Apply threshold and format response
        Ok(self.apply_threshold(result, criteria.threshold))
    }
}
```

### Integration with crabjar-guard ExecutionGate

Decision gates integrate with the existing guard layer's ExecutionGate pattern:

```hemlock
// In conductor.rs task execution loop
async fn execute_task(task: &Task) -> TaskResult {
    // Existing guard check
    let gate_result = crabjar_guard::evaluate(&task.command, context).await;
    
    if gate_result.requires_approval() {
        // NEW: Use JEV to assess risk of waiting vs proceeding
        let risk_decision = jev_evaluate(
            &format!("Command '{}' blocked by guard. Risk level: {}", 
                     task.command, gate_result.risk_level),
            "/opt/crabjar/criteria/deployment-gate.json"
        ).await;
        
        if risk_decision.option == "proceed" && risk_decision.confidence > 0.9 {
            // High confidence that proceeding is safe — auto-approve
            return proceed_with_task(task);
        }
    }
    
    // Existing flow continues...
}
```

### Annealing Loop Integration

In the conductor's annealing/trust-update loop, JEV decisions inform strategy adjustments:

```hemlock
async fn anneal_iteration(goal: &Goal, iteration: usize) -> AnnealResult {
    let batch_result = execute_task_batch(&goal.tasks).await;
    
    // NEW: Evaluate batch outcome with JEV
    let observation = format!(
        "Annealing iteration {}. Task batch: {} succeeded, {} failed. \
         Failure patterns: {}. Fleet load: {:.0}%.",
        iteration,
        batch_result.success_count,
        batch_result.failure_count,
        summarize_failures(&batch_result.failures),
        get_fleet_cpu_load()
    );
    
    let decision = jev_evaluate(&observation, goal.decision_criteria.annealing).await;
    
    match decision.option {
        "proceed" => {
            // Continue annealing with same strategy
            continue_annealing(goal, iteration + 1).await
        }
        "hold" => {
            // Pause and reassess — possibly adjust task parameters
            pause_and_reassess(goal, decision.rationale).await
        }
        "abort" => {
            // Give up on this goal decomposition strategy
            abort_annealing(goal, decision.rationale).await
        }
        _ => {
            // Unknown option — default to continue
            continue_annealing(goal, iteration + 1).await
        }
    }
}
```

## Testing Strategy

### Unit Tests (`crabjar-conductor/tests/decision/`)

- `evaluate_client_handles_llama_server`: Mock HTTP responses from llama-server backend
- `criteria_caching_works`: Criteria loaded once and reused across evaluations
- `threshold_application_correct`: Decisions below threshold marked as uncertain
- `policy_configuration_loading`: Default decision policies load correctly

### Integration Tests (`crabjar-conductor/tests/api/`)

- `goal_submission_with_criteria`: POST /goals with decision_criteria field accepted
- `decision_results_in_goal_status`: GET /goals/{id} includes decisions array
- `annealing_decision_triggers_strategy_change`: Mock JEV evaluation affecting annealing loop
- `resource_contention_priority_adjustment`: Multiple competing goals trigger priority decisions

### End-to-End Tests (`tests/e2e/conductor_jev.rs`)

- Full pipeline: goal submission → pre-execution decision → task execution → annealing decision → result
- Requires running conductor service and llama-server backend
- Validates decision history is correctly recorded and returned

### Test Fixtures

```
crabjar-conductor/tests/fixtures/
├── goals/
│   ├── with-decision-criteria.json    # Goal with JEV config
│   └── without-decision-criteria.json # Goal without JEV config (default behavior)
└── decisions/
    ├── pre-execution-proceed.json     # Mock decision result: proceed
    ├── pre-execution-hold.json        # Mock decision result: hold
    └── annealing-adjust.json          # Mock decision result for annealing loop
```

## Deployment Notes

1. Build and deploy updated conductor binary to ftw3
2. Ensure llama-server is running with Qwen3.5-4B-Q4_K_S at http://localhost:8080
3. Deploy criteria libraries to `/opt/crabjar/criteria/` on ftw3
4. Update conductor dinit service if environment variables needed for JEV backend configuration
5. Test with sample goal submission including decision_criteria field

## Future Work

- **Learned thresholds**: Auto-adjust decision thresholds based on historical outcome data
- **Multi-criteria evaluation**: Evaluate against multiple criteria simultaneously for complex decisions
- **Decision explainability**: Generate natural language explanations for conductor decisions using pesti-server
- **Fleet-wide decision coordination**: Cross-node JEV evaluation for distributed decision-making
