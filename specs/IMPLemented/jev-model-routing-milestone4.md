# JEV-Powered Model Routing in pesti-runner (Milestone 4)

**Status:** In Progress  
**Last Updated:** 2026-10-17  
**Author:** crabjar team (CromboJambo)

## Overview

Integrate JEV semantic decision-making into pesti-runner's model selection pipeline. Instead of static routing or manual model choice, use embedding similarity to match task complexity characteristics with the optimal model/quantization level based on learned patterns and curated criteria.

## Architecture

### Decision Criteria: `model-selection.json`

Curated criteria file defining model routing options:

```json
{
  "version": "1.0.0",
  "name": "model-selection",
  "description": "Route inference requests to optimal model based on task complexity",
  "options": [
    {
      "id": "small-q4",
      "description": "Simple factual query, classification, or short text generation under 100 tokens",
      "model": "Qwen2.5-0.5B-Instruct-Q4_K_M",
      "expected_latency_ms": 500
    },
    {
      "id": "medium-q4",
      "description": "Moderate complexity reasoning, code completion, or medium-length generation",
      "model": "Qwen2.5-7B-Instruct-Q4_K_M",
      "expected_latency_ms": 2000
    },
    {
      "id": "large-q4",
      "description": "Complex multi-step reasoning, long-context analysis, or high-quality creative generation",
      "model": "Qwen2.5-72B-Instruct-Q4_K_M",
      "expected_latency_ms": 8000
    }
  ],
  "threshold": 0.75,
  "metadata": {
    "created": "2026-10-17",
    "owner": "crabjar-team"
  }
}
```

### Conductor API Extension

Add `POST /decide/evaluate` endpoint to crabjar-conductor:

**Request:**
```json
{
  "observation": "User asked for quick fact check about capital cities",
  "criteria_file": "/opt/crabjar/criteria/model-selection.json",
  "context": {
    "task_type": "classification",
    "input_tokens": 45,
    "max_output_tokens": 100
  }
}
```

**Response:**
```json
{
  "success": true,
  "option": "small-q4",
  "confidence": 0.89,
  "model": "Qwen2.5-0.5B-Instruct-Q4_K_M",
  "rationale": "High similarity to simple factual query criterion"
}
```

### pesti-runner Integration

Add `ModelRouter` module that:
1. Observes incoming inference request characteristics
2. Calls conductor's `/decide/evaluate` endpoint
3. Routes to selected model instance
4. Reports outcome back for learning (`POST /decide/feedback`)

## Implementation Plan

### Step 1: Add Evaluate Endpoint to Conductor
- File: `crabjar-conductor/src/api.rs`
- New handler: `evaluate_decision_handler`
- Reuse existing `JevEvaluator::evaluate()` from `decision/evaluate.rs`
- Return structured JSON with option, confidence, and model mapping

### Step 2: Create Model Selection Criteria
- File: `/opt/crabjar/criteria/model-selection.json`
- Define options for small/medium/large models with complexity descriptions

### Step 3: Add ModelRouter to pesti-runner
- New file: `pesti-runner/src/model_router.rs`
- HTTP client to conductor API
- Request observation construction from task characteristics
- Model instance management (load selected model on demand)

### Step 4: Outcome Tracking and Learning
- After inference completes, report accuracy/cost metrics back to conductor
- Enable rule extraction for frequent routing patterns
- Learn optimal quantization levels per use case over time

## Testing Strategy

1. **Unit tests**: ModelRouter observation construction, API request formatting
2. **Integration tests**: End-to-end routing with mock conductor
3. **Load testing**: Verify routing overhead is acceptable (< 50ms decision latency)
4. **Learning validation**: After N decisions, verify rule extraction produces useful patterns

## Success Metrics

- [ ] Model routing decisions made via JEV evaluation (not hardcoded)
- [ ] Decision cache hit rate > 60% for recurring task types
- [ ] Rule extraction identifies at least one deterministic routing pattern after 100 decisions
- [ ] Routing overhead < 50ms per decision

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Conductor API unavailable | Routing fails, no fallback | Local heuristic fallback in ModelRouter |
| Embedding backend slow | High decision latency | Cache-first lookup, short timeout |
| Wrong model selected | Poor output quality or waste | Outcome feedback loop for learning |
| Criteria file stale | Outdated routing patterns | Versioned criteria with validation |

## References

- JEV Production Base Scoping: `specs/jev-production-base-scoping.md`
- JEV Decision Architecture: skill reference `crabjar-development/references/jev-decision-architecture.md`
- pesti-runner Roadmap: `/home/crombo/projects/active/pesti/pesti-runner/ROADMAP.md`
