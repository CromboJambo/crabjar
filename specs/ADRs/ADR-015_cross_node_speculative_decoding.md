---
id: ADR-015
title: Cross-Node Speculative Decoding Coordination
status: Proposed
date: 2026-10-21
supersedes: null
see_also: [ADR-014, ADR-013]
---

# ADR-015: Cross-Node Speculative Decoding Coordination

## Status

Proposed — design phase for conductor API extension to coordinate speculative decoding across distributed inference nodes.

## Context

Speculative decoding is an inference acceleration technique where a fast draft model generates candidate tokens, which are then verified by a larger target model. Accepted tokens are returned immediately; rejected tokens trigger regeneration from the last accepted position. This reduces latency compared to sequential single-model decoding.

Current crabjar conductor API supports decision-making endpoints (`/decide/evaluate`) but lacks inference task routing infrastructure. PESTI inference backends run independently on nodes (e.g., jambo at localhost:8080) with no cross-node coordination for speculative workflows.

Forces:
- Different models have different compute characteristics (draft: fast/small; target: slow/large/accurate)
- Distributed deployment enables parallel draft/verify workloads across nodes
- Conductor should route inference tasks based on model capabilities and latency profiles
- Need new API surface for speculative decoding coordination

## Decision

Extend crabjar-conductor API with speculative decoding endpoints that coordinate draft→verify loops between different model instances on potentially different nodes. The conductor acts as an orchestrator for speculative inference workflows, routing draft tokens to fast small models and verification requests to larger target models based on registered model capabilities.

New API surface:
- `POST /inference/speculate` — submit prompt for speculative decoding coordination
- Accepts parameters: prompt, draft_model_id, target_model_id, max_tokens, temperature, top_p
- Returns accepted token stream with verification metadata (acceptance rate, rejection points)
- Conductor internally routes draft inference to appropriate node/model and verification to target model

## Options Considered

**Option A: Client-side speculative coordination**
Client submits separate inference requests to different nodes and coordinates the loop itself. Rejected because conductor should own orchestration logic; clients shouldn't need to know which nodes run which models or handle verification routing.

**Option B: Conductor inference task routing only**
Conductor routes inference tasks to appropriate nodes but doesn't coordinate speculative loops. Clients still manage draft/verify sequencing. Rejected because it misses the core value proposition of conductor-managed speculative decoding.

**Option C: Conductor-managed speculative coordination (chosen)**
Conductor exposes speculative decoding endpoint, internally coordinates draft→verify loop between registered models/nodes, returns accepted token stream. Client submits prompt once and receives optimized output without managing coordination complexity.

## Consequences

### Positive consequences
- Enables inference acceleration across distributed model deployments
- Clients benefit from speculative decoding without implementing coordination logic
- Conductor becomes central orchestrator for inference workflows, not just decision-making
- Model routing infrastructure enables future optimizations (load balancing, failover)

### Negative consequences (trade-offs)
- New API surface increases conductor complexity
- Requires model registration/capability metadata infrastructure
- Network latency between nodes must be low enough that speculative decoding provides benefit
- Verification model must be faster to verify than to generate from scratch

### Ongoing concerns
- Model capability metadata freshness — how does conductor know which models are available on which nodes?
- Latency monitoring — speculative decoding only helps if verification is fast relative to draft generation
- Error handling — what happens when verification node fails during loop?
- Backpressure management — how many speculative requests can conductor coordinate simultaneously?

## Implementation Sketch

```heml
// New endpoint: /inference/speculate
POST /inference/speculate {
    prompt: string,
    draft_model_id: string,
    target_model_id: string,
    max_tokens: i32,
    temperature: f32,
    top_p: f32
}

// Internal coordination loop (conceptual)
fn speculate(prompt, draft_model, target_model) {
    let accepted = [];
    let current_position = 0;
    
    while (accepted.length < max_tokens) {
        // Route draft inference to fast model node
        let draft_tokens = route_inference(draft_model, prompt[current_position:], temperature);
        
        // Route verification to target model node
        let verification = route_verification(target_model, prompt[:current_position], draft_tokens);
        
        for (let i = 0; i < draft_tokens.length; i++) {
            if (verification.accepted[i]) {
                accepted.push(draft_tokens[i]);
            } else {
                break; // Regenerate from this position
            }
        }
    }
    
    return accepted;
}
```

## References

- Nygard, M. (2011). *Documenting Architecture Decisions*. https://www.infoq.com/articles/Architecture-Decision-Lang
- `specs/README.md` — ADR process and conventions for this project