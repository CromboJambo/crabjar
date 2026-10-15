# Spec: OpenTelemetry Integration for Crabjar Telemetry

**Date:** 2026-10-14  
**Status:** Draft — implementation specification  
**Trigger:** Comparison with harness revealed industry-standard OTel integration (239 files). crabjar's telemetry crate is a SQLite flight recorder with no external export. This spec wires in OTel as the industry go-to telemetry backend.

## Current State

- `telemetry/` crate: SQLite-based flight recorder (`FlightRecorder`)
- Captures: command execution transcripts, git state, HMAC-SHA256 tool receipts
- Uses `tracing` for internal logs but no external export
- No OTel integration currently

## Industry Standard: OpenTelemetry

OTel provides three signal types:
1. **Traces**: Distributed request tracing with spans (parent-child relationships)
2. **Metrics**: Aggregated measurements (counters, histograms, gauges)
3. **Logs**: Structured event records with context

Industry adoption: Kubernetes, AWS, Google Cloud, Grafana Stack, Datadog all support OTel natively. This is the de facto standard for observability in 2026.

## Architecture: Dual-Backend Design

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
│ OtelExporter    │ (new — optional external export)
│ - spans → OTLP  │
│ - metrics → OTLP│
│ - logs → OTLP   │
└─────────────────┘
         │
         ▼
OTLP Collector (local or remote)
```

**Key principle:** SQLite flight recorder remains the source of truth for local audit. OTel export is an additional channel for fleet-scale observability, not a replacement.

## Dependencies

Add to `telemetry/Cargo.toml`:

```toml
[dependencies]
opentelemetry = { version = "0.27", features = ["trace", "metrics", "logs"] }
opentelemetry_sdk = { version = "0.27", features = ["trace", "metrics", "logs", "rt-tokio"] }
opentelemetry-otlp = { version = "0.27", features = ["trace", "metrics", "logs", "grpc-tonic", "http-proto", "reqwest-client"] }

[features]
default = []
otel = ["dep:opentelemetry", "dep:opentelemetry_sdk", "dep:opentelemetry-otlp"]
```

## Configuration

Environment variable control (matching OTel conventions):

```bash
# Enable OTel export to local collector
CRABJAR_OTEL_ENDPOINT=http://localhost:4317 crabjar exec ...

# Or via config file
telemetry = { otel_endpoint = "http://otel-collector.internal:4318" }
```

Fallback chain:
1. Explicit config file value
2. `CRABJAR_OTEL_ENDPOINT` env var
3. Standard OTel env vars (`OTEL_EXPORTER_OTLP_ENDPOINT`, etc.)
4. Default: disabled (local tracing only)

## Signals to Capture

### 1. Traces (Spans)

**Execution Pipeline Span:**
```rust
let span = tracer.start("crabjar.execution");
span.set_attribute("session.id", session_id);
span.set_attribute("action.type", "exec_command");
span.set_attribute("guard.decision", "proceed");
// ... execution happens ...
span.end();
```

**Guard Evaluation Span:**
```rust
let span = tracer.start("crabjar.guard.evaluate");
span.set_attribute("policy.engine", "static");
span.set_attribute("trust.layer", 3);
span.set_attribute("confidence", 0.85);
// ... evaluation happens ...
span.end();
```

### 2. Metrics

**Counters:**
- `crabjar.actions.executed` — total actions by type (command, edit, read, etc.)
- `crabjar.guard.decisions` — policy decisions (proceed/interrupted/pending)
- `crabjar.trust.updates` — trust layer changes

**Histograms:**
- `crabjar.execution.duration_ms` — command execution time by type
- `crabjar.guard.evaluation_duration_ms` — policy evaluation time

**Gauges:**
- `crabjar.trust.current_score` — current confidence score per session

### 3. Logs

Structured log records for:
- Session start/end
- Guard decisions with context (why interrupted?)
- Execution failures with error details
- Trust layer transitions

## Implementation Plan

### Phase 1: OtelExporter Module

Create `telemetry/src/otel_export.rs`:

```rust
pub struct OtelExporter {
    tracer_provider: TracerProvider,
    logger_provider: LoggerProvider,
    meter_provider: SdkMeterProvider,
}

impl OtelExporter {
    pub fn from_endpoint(endpoint: &str) -> Result<Self> { ... }
    pub fn record_execution(&self, event: &ExecutionEvent) { ... }
    pub fn record_guard_decision(&self, event: &GuardDecision) { ... }
    pub async fn shutdown(&self) { ... }
}
```

### Phase 2: FlightRecorder Integration

Modify `FlightRecorder` to accept optional `OtelExporter`:

```rust
pub struct FlightRecorder<'a> {
    conn: &'a Connection,
    session_id: String,
    hmac_key: Option<Vec<u8>>,
    otel: Option<OtelExporter>,  // NEW
}

impl FlightRecorder {
    pub fn with_otel(mut self, exporter: OtelExporter) -> Self {
        self.otel = Some(exporter);
        self
    }

    async fn execute_command(&mut self, ...) -> Result<String> {
        let cmd_id = Uuid::new_v4().to_string();
        
        // Record to SQLite (existing)
        record_command(...)?;
        
        // NEW: Export to OTel if configured
        if let Some(otel) = &self.otel {
            otel.record_execution(&ExecutionEvent {
                session_id: &self.session_id,
                command_id: &cmd_id,
                command,
                exit_code,
                duration_ms,
                ...
            });
        }
        
        Ok(cmd_id)
    }
}
```

### Phase 3: CLI Integration

Add OTel support to `crabjar exec` and `crabjar conductor`:

```bash
# Single-machine with local collector
CRABJAR_OTEL_ENDPOINT=http://localhost:4317 crabjar exec "cargo test"

# Fleet-scale with remote collector
CRABJAR_OTEL_ENDPOINT=http://otel-collector.internal:4318 crabjar conductor start
```

## Comparison to harness Implementation

| Aspect | harness | crabjar (proposed) |
|--------|---------|-------------------|
| Exporters | OTLP HTTP/gRPC | Same |
| Signals | Traces, metrics, logs | Same |
| Integration point | Separate observe crate | Integrated into telemetry crate |
| Configuration | `otel.exporter` config field | Environment variable + config file |
| Local fallback | None (OTel required) | SQLite flight recorder always active |

**Key difference:** crabjar maintains the SQLite flight recorder as the primary source of truth for local audit, with OTel as an optional export channel. harness appears to use OTel as the primary observability layer.

## Testing Strategy

1. **Unit tests**: OtelExporter construction, shutdown
2. **Integration tests**: Mock OTLP endpoint (in-memory exporter)
3. **E2E test**: Actual OTLP collector (Grafana Tempo/Jaeger) in CI

```rust
#[test]
fn test_otel_exporter_construction() {
    let exporter = OtelExporter::from_endpoint("http://localhost:4317").unwrap();
    // Verify tracer/logger/meter providers initialized
}

#[tokio::test]
async fn test_execution_event_export() {
    let exporter = OtelExporter::from_endpoint("http://localhost:4317").unwrap();
    exporter.record_execution(&ExecutionEvent { ... });
    // Verify span recorded (via in-memory exporter)
}
```

## Doubt Block

```json
{
  "doubt": {
    "assumptions": [
      "OTel 0.27 is stable and compatible with crabjar's Rust toolchain",
      "grpc-tonic feature works on all target platforms (including embedded)",
      "The dual-backend design doesn't introduce unacceptable complexity"
    ],
    "blind_spots": [
      "Did not benchmark OTel export latency impact on execution pipeline",
      "Did not evaluate alternative exporters (Prometheus, StatsD) for specific use cases",
      "Assumed harness's approach is optimal; other fleet systems may differ"
    ],
    "last_validation": "2026-10-14",
    "stale_after": "2026-10-28"
  }
}
```
