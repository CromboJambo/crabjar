# OTel → DAGR → Terrarium Integration

**Status:** Draft  
**Date:** 2026-10-14  
**Depends on:** `otel-telemetry-integration.md`, `HABBLAUD-TERRARIUM-UPGRADE.md` (Phase 5)

## Overview

Wire the OTel spans we now generate into DAGR's event system for live terrarium visualization. This makes the terrarium an operational observability dashboard — a spatial representation of agent activity across the fleet in real-time.

**Architecture:**
```
Execution pipeline → OTel spans → DAGR events → Habitat contract → Terrarium rendering
```

## Design Principles

1. **OTel spans are the source of truth for execution observability** — not ad-hoc event emission
2. **DAGR is the transport layer** — spans become structured DAGR events that any renderer can consume
3. **Terrarium is one skin** — the same DAGR events could drive a TUI dashboard, web UI, or other visualizations
4. **"Generic over wire representation"** — OTel → DAGR mapping is deterministic; pixels are disposable

## Span-to-Event Mapping

### Execution Pipeline Spans → Task Events

| OTel Span | DAGR Event Type | Terrarium Behavior |
|-----------|-----------------|-------------------|
| `execute_command` (start) | `task_started` | Crab walks to task location, begins working animation |
| `execute_command` (end, success) | `task_completed` | Crab celebrates, moves to next task |
| `execute_command` (end, failure) | `task_failed` | Crab shows error state (rain cloud icon) |
| Span with `duration` attribute | `shell_job_started` / `shell_job_finished` | Crab watches terminal with duration indicator |

### Guard Evaluation Spans → Permission Events

| OTel Span Attribute | DAGR Event Type | Terrarium Behavior |
|---------------------|-----------------|-------------------|
| `guard.trust_layer` + `guard.confidence` < threshold | `permission_requested` | Crab raises hand, shows approval needed indicator |
| `guard.action_type` = "exec" (high risk) | `blocked_task` | Crab sits at desk with raised arm animation |

### Session Lifecycle Spans → Agent State Events

| OTel Span | DAGR Event Type | Terrarium Behavior |
|-----------|-----------------|-------------------|
| Session started | `agent_online` | Crab appears in habitat, walks to idle location |
| Session ended | `agent_offline` | Crab leaves habitat (fades out) |
| No activity for N seconds | `agent_idle` | Crab begins socializing or napping |

## Implementation Architecture

### 1. OTel Span Processor (New Module)

Create `telemetry/src/dagr_span_processor.rs`:

```rust
/// Processes OTel spans and emits corresponding DAGR events.
pub struct DagrSpanProcessor {
    dagr_feed: Arc<dyn DagrEventEmitter>,
}

impl DagrSpanProcessor {
    pub fn on_span_start(&self, span: &Span) {
        match span.name() {
            "execute_command" => self.emit_task_started(span),
            "guard_evaluate" => self.emit_guard_evaluation(span),
            _ => {}
        }
    }

    pub fn on_span_end(&self, span: &Span) {
        match span.name() {
            "execute_command" => self.emit_task_completed(span),
            "guard_evaluate" => self.emit_permission_result(span),
            _ => {}
        }
    }
}
```

### 2. Integration with FlightRecorder

Modify `telemetry/src/flight_recorder.rs` to attach the DAGR span processor when OTel is enabled:

```rust
pub fn with_otel(&mut self, endpoint: &str) -> Result<&mut Self, FlightRecorderError> {
    let exporter = OtelExporter::from_config(OtelConfig {
        endpoint: Some(endpoint.to_string()),
        service_name: "crabjar".to_string(),
    })?;

    // Attach DAGR span processor to the tracer provider
    let dagr_processor = DagrSpanProcessor::new(self.dagr_emitter.clone());
    exporter.attach_span_processor(dagr_processor);

    self.otel = Some(Arc::new(exporter));
    Ok(self)
}
```

### 3. DAGR Event Emission from CLI

In `src/main.rs`, the habitat contract producer already reads from triage queue, guard pending queue, and kanban tasks. Extend it to also consume DAGR events emitted by the OTel span processor:

```rust
// In handle_exec(), after creating flight_recorder with OTel:
let dagr_events = flight_recorder.consume_dagr_events();
for event in dagr_events {
    emit_to_habitat_contract(event);
}
```

### 4. Habitat Contract Extension

Extend `src/habitat_contract.rs` to include execution observability data from DAGR events:

- Add new project: `"otel-execution"` for execution pipeline spans
- Map DAGR event types to DAGR task states (queued → running → done/failed)
- Include span attributes as task metadata (duration, exit code, trust layer)

## Terrarium Visualization Mapping

### Execution States

- **Working:** Crab at terminal with focused animation (from `task_started` event)
- **Shell Job Running:** Crab watches screen with hourglass (from `shell_job_started`)
- **Task Completed:** Brief celebration animation, then moves to next (from `task_completed`)
- **Task Failed:** Error icon above crab's head (from `task_failed`)

### Guard/Permission States

- **Awaiting Approval:** Crab raises hand at "desk" (from `permission_requested`)
- **Blocked on High-Risk Action:** Crab shows anxious/waiting posture (from `blocked_task`)

### Fleet-Wide Observability

- Multiple crabs working simultaneously = parallel task execution visible
- Crabs waiting for approval = bottlenecks in the pipeline become obvious
- Idle crabs socializing = capacity available for more work

## Configuration

OTel endpoint configuration remains as specified in `otel-telemetry-integration.md`:

- CLI flag: `--otel-endpoint http://localhost:4317`
- Environment variable: `CRABJAR_OTEL_ENDPOINT=http://localhost:4317`
- Default: disabled (no OTel export, no DAGR event emission)

DAGR event emission is automatic when OTel is enabled — no separate configuration needed.

## Phased Rollout

### Phase 1: Span-to-DAGR Mapping (This Spec)
- Implement `DagrSpanProcessor` in telemetry crate
- Wire into FlightRecorder when OTel is enabled
- Emit DAGR events to stderr (like existing terrarium event stream)

### Phase 2: Habitat Contract Integration
- Extend habitat contract producer to consume DAGR events
- Add execution observability data to `run.json`
- Update terrarium simulator to render new event types

### Phase 3: Live Terrarium Dashboard
- Wire terrarium to live DAGR event stream (not just static contract)
- Real-time crab behavior updates based on execution spans
- Fleet-wide observability visualization

## Success Criteria

- OTel spans from execution pipeline generate corresponding DAGR events
- Habitat contract includes execution observability data when OTel is enabled
- Terrarium can render execution states (working, waiting, blocked, idle) from DAGR events
- No impact on local usage when OTel is disabled (default)
- Backward compatible with existing DAGR event consumers

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| OTel span processor adds latency to execution | Use async processing; spans are exported in background thread |
| DAGR event volume overwhelms terrarium UI | Throttle updates; batch state changes per tick (already planned in Phase 5) |
| Span-to-event mapping is lossy | Preserve span attributes as task metadata for drill-down |
| OTel dependency couples telemetry to DAGR | Use trait abstraction (`DagrEventEmitter`) so mock can be used in tests |

## Next Steps

1. Implement `DagrSpanProcessor` module in telemetry crate
2. Wire into FlightRecorder when OTel is enabled
3. Add integration tests that verify span → DAGR event mapping
4. Extend habitat contract producer to consume execution events
5. Update terrarium simulator to render new event types