# Hybrid Attention + Semantic Decision Architecture

Combines mirror-log's fast rule-based attention layer with JEV's embedding-based semantic matching for optimal throughput and depth.

## Problem Statement

Single-approach decision systems face a trade-off:
- **Rule-based (mirror-log)**: Fast (~135µs/query) but no semantic understanding — relies on temporal decay and access frequency alone
- **Semantic (JEV)**: Deep understanding via embedding similarity (~679µs/eval) but slower, requires embedding backend

Neither approach alone is optimal for fleet-scale decision-making where thousands of observations need filtering before deep analysis.

## Architecture

```
Observation Stream → [Attention Layer] → High-Attention Items → [JEV Decision Gate] → Action
                        (fast filter)        (semantic eval)          (execute)
```

### Stage 1: Attention Triage (Mirror-Log)

**Purpose**: Rapidly filter thousands of observations to identify items requiring deeper analysis.

**Mechanism**:
- Temporal decay scoring based on event age
- Access frequency weighting (recency bias)
- Source type prioritization (alerts > metrics > journal)
- SQLite-based, purely local computation

**Performance Target**: <200µs per observation
**Throughput**: ~5,000 observations/second single-threaded

### Stage 2: Semantic Evaluation (JEV Decision Gate)

**Purpose**: Deep semantic matching of high-attention items against decision criteria.

**Mechanism**:
- Embedding-based similarity via llama-server API
- Criteria files define decision thresholds and actions
- Background evaluation support for non-blocking operation
- Pre-execution readiness checks

**Performance Target**: <1ms per evaluation (synchronous)
**Throughput**: ~1,000 evaluations/second with parallel backend

### Escalation Logic

```hemlock
fn process_observation(obs: Observation): Decision {
    // Stage 1: Fast attention scoring
    let attention_score = mirror_log.calculate_attention(conn, obs);
    
    if (attention_score < THRESHOLD_LOW) {
        return Decision::IGNORE;  // ~90% of observations
    }
    
    if (attention_score < THRESHOLD_HIGH) {
        return Decision::LOG;     // Monitor but don't act
    }
    
    // Stage 2: Semantic evaluation for high-attention items
    let criteria = load_criteria_file(obs.source_type);
    let jev_result = conductor.evaluate_decision(obs, criteria);
    
    if (jev_result.ready) {
        return Decision::EXECUTE(jev_result.action);
    } else {
        return Decision::HOLD(jev_result.rationale);
    }
}
```

## Threshold Configuration

Thresholds are configurable per fleet profile via `decision_policy.json`:

```hemlock
{
    "attention_thresholds": {
        "low": 0.3,      // Below: ignore
        "high": 0.7      // Above: escalate to JEV
    },
    "jev_criteria_files": {
        "alert": "critical_alerts.json",
        "metrics": "performance_degradation.json",
        "journal": "operational_anomalies.json"
    }
}
```

## Implementation Plan

### Phase 1: Integration Layer (Week 1)
- Add `crabjar-mirror-log` dependency to crabjar-conductor
- Implement `AttentionLayer::calculate_attention()` wrapper in conductor
- Wire attention scoring into observation ingestion pipeline

### Phase 2: Escalation Logic (Week 2)
- Implement threshold-based escalation from attention → JEV
- Add background evaluation support for escalated items
- Implement decision policy configuration loading

### Phase 3: Optimization (Week 3)
- Batch JEV evaluations for throughput improvement
- Implement adaptive thresholds based on fleet load
- Add metrics and observability for both stages

## Expected Performance Gains

| Metric | JEV Only | Hybrid Model | Improvement |
|--------|----------|--------------|-------------|
| Avg latency per observation | 679µs | ~150µs (90% filtered early) | 4.5x faster |
| Throughput | 1,000 obs/sec | 5,000+ obs/sec | 5x higher |
| Embedding API calls | 100% of observations | ~10% (high-attention only) | 90% reduction |

## Trade-offs

**Advantages**:
- Dramatically reduced embedding API load
- Lower average latency for fleet-wide observation processing
- Semantic depth preserved where it matters most

**Disadvantages**:
- More complex architecture with two decision layers
- Threshold tuning required per fleet profile
- Potential for missed semantic signals if attention thresholds are too aggressive

## Rollout Strategy

1. Deploy hybrid model to staging environment with synthetic observation streams
2. A/B test against JEV-only baseline using historical observation data
3. Tune thresholds based on precision/recall metrics
4. Gradually roll out to production fleets with monitoring and rollback capability
