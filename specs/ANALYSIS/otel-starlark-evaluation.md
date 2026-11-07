# Spec: Otel and Starlark Integration for Crabjar

**Date:** 2026-10-14  
**Status:** Draft — evaluation and proposal  
**Trigger:** Comparison with harness revealed significant investment in OTel (239 files) and Starlark-based policy engine (harness-rules crate). crabjar has a PolicyEngine trait designed for future Starlark backend, but no OTel integration.

## Current State

### Policy Engine
- `guard/src/policy.rs`: `StaticPolicyEngine` using TOML config
- `guard/src/policy_types.rs`: `PolicyEngine` trait with hot-reload support
- Explicitly deferred Starlark: "deferred in favor of Rust-native declarative policies because zero new dependencies, no runtime overhead, compile-time type safety"
- Architecture is forward-compatible: `#[cfg(feature = "starlark")]` can add backend without API changes

### Observability
- Uses `tracing` crate throughout (82 files)
- No OTel integration currently
- Telemetry exists in execution pipeline but not standardized for external export

## Evaluation: Starlark Policy Backend

### Value Proposition
From harness's implementation, Starlark enables:
1. **Complex conditionals**: Time-based policies (`if trust_layer >= 2 and time_between(2am, 5am):`)
2. **Rate limiting**: Per-actor or per-resource rate limits with sliding windows
3. **Multi-factor approval chains**: Require N-of-M approvals based on dynamic conditions
4. **Non-Rust policy authoring**: Ops teams can write policies without Rust expertise

### Cost Analysis
- ~15 transitive dependencies, ~200KB compiled size (per crabjar's own analysis)
- Runtime interpretation overhead vs. inlined Rust branches
- Requires sandbox design for security (limit built-ins, callstack size, source bytes)

### Recommendation: Defer with Feature Flag
Given crabjar's existing `PolicyEngine` trait and the team's deliberate decision to defer Starlark, recommend maintaining current approach but adding a feature flag path:

```toml
# Cargo.toml for guard crate
[features]
default = []
starlark = ["dep:starlark"]
```

Add when demand emerges from:
- Non-Rust policy authors joining the team
- Need for time-based or rate-limiting policies that TOML can't express
- Fleet-scale deployments requiring dynamic policy updates without config reloads

### Minimal Path to Starlark Integration (if needed)
1. Add `starlark` dependency behind feature flag in `guard/Cargo.toml`
2. Implement `StarlarkPolicyEngine` satisfying `PolicyEngine` trait
3. Provide built-in functions: `allow()`, `deny(reason)`, `pending()`, `trust_layer()`, `confidence()`
4. Sandbox: limit callstack to 512, source to 512KB (matching harness's approach)
5. CLI flag: `crabjar exec --policy-engine starlark --policy-file policy.star`

## Evaluation: OpenTelemetry Integration

### Value Proposition
From harness's implementation (harness-observe crate), OTel enables:
1. **Standardized telemetry export**: OTLP protocol to any OTel collector (Grafana, Jaeger, etc.)
2. **Distributed tracing across fleet**: Correlate agent actions across machines
3. **Signal-driven remediation**: Automated alerts and responses based on metrics
4. **Fleet-scale observability**: Essential for multi-agent deployments

### Cost Analysis
- 3 OTel crates: `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`
- Requires OTLP endpoint configuration (local collector or remote)
- Adds runtime overhead for span creation and metric recording
- Integration complexity with existing `tracing` infrastructure

### Recommendation: Adopt as Optional Backend
OTel is valuable for fleet-scale deployments but adds complexity for single-machine use. Recommend optional integration via feature flag:

```toml
# Cargo.toml for host/observe or new observability crate
[features]
default = []
otel = ["dep:opentelemetry", "dep:opentelemetry_sdk", "dep:opentelemetry-otlp"]
```

### Implementation Approach
1. **New crate**: `crabjar-observability` (or extend `host/observe`)
2. **Dual backend design**: 
   - Default: local `tracing-subscriber` with file/terminal output
   - OTel mode: export spans/metrics/logs via OTLP when endpoint configured
3. **Environment variable control**: `CRABJAR_OTEL_ENDPOINT=http://localhost:4317`
4. **Integration points**:
   - Execution pipeline: span per action with trust layer, confidence, outcome
   - Guard evaluation: metric for policy decisions (proceed/interrupted/pending)
   - Fleet dispatch: distributed traces across conductor → agent → execution

### Key OTel Signals to Capture
- `crabjar.action.executed` (counter): by action type, outcome
- `crabjar.guard.decision` (histogram): evaluation time by policy engine
- `crabjar.trust.score` (gauge): confidence scores over time
- `crabjar.fleet.dispatch` (counter): tasks dispatched to agents
- Spans: per-execution with trace context propagation

## Priority Assessment

| Technology | Impact | Complexity | Recommendation |
|------------|--------|------------|----------------|
| OTel integration | High for fleet-scale, low for single-machine | Medium | Adopt as optional backend (feature flag) |
| Starlark policy backend | Low for current needs, high for future flexibility | Medium-High | Defer; add feature flag path when demand emerges |

## Next Steps

1. **OTel**: Implement `crabjar-observability` crate with dual-backend design (tracing + optional OTLP export)
2. **Starlark**: Document the feature flag path in policy_types.rs for future reference
3. **Benchmark**: Measure performance impact of both integrations on execution pipeline latency

## Doubt Block

```json
{
  "doubt": {
    "assumptions": [
      "OTel overhead is acceptable for crabjar's use cases",
      "Starlark can be deferred without blocking near-term roadmap items",
      "The PolicyEngine trait remains sufficient abstraction for Starlark backend"
    ],
    "blind_spots": [
      "Did not benchmark actual OTel export latency in production-like conditions",
      "Did not evaluate alternative policy scripting languages (Lua, Python embedded)",
      "Assumed harness's approach is optimal; other fleet systems may differ"
    ],
    "last_validation": "2026-10-14",
    "stale_after": "2026-10-28"
  }
}
```
