# ADR-011: Decision Cache + Rule Extraction Layer

**Status:** Accepted  
**Date:** 2026-10-19  
**Author:** crabjar team (CromboJambo)  
**Context:** JEV performance optimization

## Problem

Embedding-based decisions require API calls or local model inference, introducing latency:
- Hash backend: ~1ms (negligible)
- llama-server backend: 50-200ms per decision
- External JEV API: 100-500ms per call

For high-frequency operational decisions (health checks, monitoring alerts), this latency compounds. Additionally, repeated identical observations waste compute.

## Decision

Implement a two-layer optimization: LRU cache for repeated observations + rule extraction to promote frequent decisions to deterministic paths.

## Rationale

1. **Cache hit rate**: Operational environments have repetitive observations (same error messages, same service states). Expected 30-50% cache hit rate in production fleets.
2. **Rule promotion**: Decisions that consistently yield the same outcome for similar observations can be promoted to deterministic rules, eliminating embedding calls entirely for those cases.
3. **Latency reduction target**: 30% overall latency improvement through caching alone; additional improvement from rule promotion.
4. **Cost reduction**: Fewer embedding API calls = lower operational cost at scale.

## Cache Design

```hemlock
// LRU cache with SHA-256 hashing
let cache_key = sha256(backend + criteria_source + observation);
if (cache.has(cache_key)) {
    return cached_result;  // ~0ms latency
}
let result = evaluate_decision(observation, criteria);
cache.set(cache_key, result);
return result;
```

- Cache size: 1000 entries (LRU eviction)
- Key components: backend name + criteria source + observation text
- Hash: SHA-256 for collision resistance
- Persistence: In-memory only (rebuilds on restart)

## Rule Extraction Pattern

Decisions are logged with outcomes. When the same decision context (observation pattern → option) occurs repeatedly with consistent results, a deterministic rule can be extracted:

```hemlock
// After 10+ consistent decisions for similar observations
// Promote to rule: if observation contains "disk full" → disk_full
let rule = {
    pattern: "disk full",
    decision: "disk_full",
    confidence: 0.95,
    occurrences: 42
};
```

Rules bypass the embedding layer entirely for matched observations.

## Implementation Status

- `src/commands/decide_cache.rs`: LRU cache implementation (complete)
- `crabjar-conductor/src/decision/cache.rs`: Conductor-side caching (complete)
- Rule extraction: Specified in conductor integration spec, partially implemented

## Alternatives Considered

1. **No caching**: Simple but wasteful for repetitive observations
2. **Persistent cache (SQLite)**: More complex, benefits unclear given rebuild cost
3. **Pre-computed embeddings for criteria**: Already done (criteria files embed options at load time)

## Consequences

- Cache invalidation strategy needed when criteria files change
- Rule extraction requires outcome logging infrastructure
- Monitoring needed to track cache hit rates and rule promotion effectiveness
- Slight complexity increase in decision pipeline