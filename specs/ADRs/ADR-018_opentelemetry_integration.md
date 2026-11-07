# ADR-018: OpenTelemetry Integration for Fleet-Scale Observability

**Date:** November 2, 2026  
**Status:** Accepted  
**Trigger:** Need for industry-standard telemetry export for fleet-scale observability while maintaining SQLite flight recorder as local source of truth.

## Problem Statement

Crabjar's existing telemetry system uses a SQLite-based flight recorder that captures command execution transcripts, git state, and tool receipts locally. While excellent for local audit trails, it lacks external export capabilities needed for:

- Fleet-scale observability across multiple machines
- Integration with Grafana/Jaeger dashboards
- Distributed tracing across agent sessions
- Standardized metrics collection (counters, histograms, gauges)

The `harness` project uses OpenTelemetry as its primary observability layer (239 OTel-related files), establishing it as the industry standard for agent infrastructure.

## Decision

Implement dual-backend telemetry architecture:

1. **SQLite Flight Recorder** — remains the local source of truth for audit trails, command transcripts, and tool receipts
2. **OpenTelemetry Exporter** — optional external export channel for fleet-scale observability via OTLP protocol

The OTel exporter is opt-in via environment variable (`CRABJAR_OTEL_ENDPOINT`) or config file, defaulting to disabled (local tracing only). This preserves backward compatibility while enabling production-grade observability when needed.

## Architecture

```
Execution Pipeline Events
         │
         ▼
┌─────────────────┐
│ FlightRecorder  │ (existing SQLite backend)
│ - command transcripts
│ - git state
│ - tool receipts
└─────────────────┘
         │
         ▼
┌─────────────────┐
│ OtelExporter    │ (optional external export)
│ - spans → OTLP  │
│ - metrics → OTLP│
│ - logs → OTLP   │
└─────────────────┘
         │
         ▼
OTLP Collector (local or remote)
```

## Implementation Details

### Dependencies
Added to `telemetry/Cargo.toml` as optional dependencies behind the `otel` feature flag:
- `opentelemetry` 0.33 (trace, metrics, logs features)
- `opentelemetry_sdk` 0.33 (trace, metrics, logs, rt-tokio features)
- `opentelemetry-otlp` 0.33 (grpc-tonic feature for OTLP export)

### Modules
- `telemetry/src/otel_export.rs` — OtelExporter with `record_execution()`, `record_guard_decision()`, `record_log()` methods
- `telemetry/src/dagr_span_processor.rs` — Custom span processor that maps OTel spans to DAGR habitat events for terrarium visualization

### Configuration
Environment variable control matching OTel conventions:
```bash
CRABJAR_OTEL_ENDPOINT=http://localhost:4317 crabjar exec "cargo test"
```

Fallback chain:
1. Explicit config file value
2. `CRABJAR_OTEL_ENDPOINT` env var
3. Standard OTel env vars (`OTEL_EXPORTER_OTLP_ENDPOINT`)
4. Default: disabled (local tracing only)

## Signals Captured

### Traces (Spans)
- `crabjar.execution` — command execution with session/command IDs, exit code, duration
- `crabjar.guard.evaluate` — policy evaluation with trust layer and confidence scores
- Session lifecycle spans (`crabjar.session.start`/`end`)

### Metrics (Planned)
- Counters: actions executed by type, guard decisions, trust updates
- Histograms: execution duration, guard evaluation duration
- Gauges: current trust score per session

## Comparison to Harness

| Aspect | harness | crabjar |
|--------|---------|---------|
| Exporters | OTLP HTTP/gRPC | Same |
| Signals | Traces, metrics, logs | Traces (metrics/logs planned) |
| Integration point | Separate observe crate | Integrated into telemetry crate |
| Configuration | `otel.exporter` config field | Environment variable + config file |
| Local fallback | None (OTel required) | SQLite flight recorder always active |

**Key difference:** Crabjar maintains the SQLite flight recorder as the primary source of truth for local audit, with OTel as an optional export channel. Harness uses OTel as the primary observability layer.

## Testing

30 unit tests + 3 integration tests in `telemetry/` crate:
- OtelExporter construction and configuration
- Execution event recording with attributes
- Guard decision recording
- DAGR span processor event mapping (task_started/completed/failed, permission_requested, agent_online/offline)

All tests pass with the `otel` feature enabled.

## Alternatives Considered

1. **Replace SQLite with OTel as primary backend** — Rejected because local audit trails should work without external dependencies; SQLite provides durability and queryability that pure OTel export lacks.

2. **Use Prometheus/StatsD instead of OTel** — Rejected because OTel is the emerging industry standard with broader adoption (Kubernetes, AWS, Google Cloud, Grafana Stack all support it natively).

3. **Make OTel mandatory rather than optional** — Rejected to preserve backward compatibility and allow single-machine usage without external collector dependencies.

## Consequences

### Positive
- Fleet-scale observability now possible with standard tooling (Grafana, Jaeger)
- Distributed tracing across agent sessions enabled
- Industry-standard integration reduces friction for users familiar with OTel ecosystem
- DAGR span processor enables terrarium visualization of execution pipeline activity

### Negative
- Added dependency surface (opentelemetry crates)
- Slight complexity increase in telemetry crate
- Requires OTLP collector deployment for fleet-scale use

## References
- Spec: `specs/otel-telemetry-integration.md`
- Related: `specs/otel-dagr-terrarium-integration.md` (depends on this ADR)
- OpenTelemetry spec: https://opentelemetry.io/docs/specs/