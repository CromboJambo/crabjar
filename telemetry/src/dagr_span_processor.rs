//! OTel span → DAGR event processor.
//!
//! Intercepts execution pipeline spans and emits corresponding DAGR habitat
//! events to stderr for terrarium visualization. This makes the terrarium an
//! operational observability dashboard — a spatial representation of agent
//! activity across the fleet in real-time.
//!
//! Architecture:
//!   Execution pipeline → OTel spans → DagrSpanProcessor → DAGR events (stderr)
//!
//! Design principles:
//! 1. OTel spans are the source of truth for execution observability
//! 2. DAGR is the transport layer — spans become structured events any renderer can consume
//! 3. Terrarium is one skin — the same events could drive a TUI dashboard, web UI, etc.
//! 4. "Generic over wire representation" — span-to-event mapping is deterministic; pixels are disposable

use opentelemetry::{KeyValue, trace::Status};
use opentelemetry_sdk::trace::{SpanData, SpanProcessor};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// DAGR event types for execution observability.
mod events {
    /// Task started (agent begins work)
    pub const TASK_STARTED: &str = "task_started";
    /// Task completed successfully
    pub const TASK_COMPLETED: &str = "task_completed";
    /// Task failed
    pub const TASK_FAILED: &str = "task_failed";
    /// Shell job started (long-running command)
    pub const SHELL_JOB_STARTED: &str = "shell_job_started";
    /// Shell job finished
    pub const SHELL_JOB_FINISHED: &str = "shell_job_finished";
    /// Permission requested (guard evaluation below threshold)
    pub const PERMISSION_REQUESTED: &str = "permission_requested";
    /// Blocked on high-risk action
    pub const BLOCKED_TASK: &str = "blocked_task";
    /// Agent online (session started)
    pub const AGENT_ONLINE: &str = "agent_online";
    /// Agent offline (session ended)
    pub const AGENT_OFFLINE: &str = "agent_offline";
    /// Agent idle (no activity for N seconds)
    pub const AGENT_IDLE: &str = "agent_idle";
}

/// DAGR event emitter trait. Allows mocking in tests and different output targets.
pub trait DagrEventEmitter: Send + Sync {
    fn emit(&self, event: Value);
}

/// Standard emitter that writes DAGR events to stderr.
#[derive(Debug)]
pub struct StderrEmitter;

impl DagrEventEmitter for StderrEmitter {
    fn emit(&self, event: Value) {
        eprintln!("DAGR EVENT: {}", event);
    }
}

/// Processes OTel spans and emits corresponding DAGR habitat events.
///
/// Maps execution pipeline spans to terrarium visualization events:
/// - `crabjar.execution` → task_started / task_completed / task_failed
/// - `crabjar.guard.evaluate` → permission_requested / blocked_task
/// - Session lifecycle spans → agent_online / agent_offline
pub struct DagrSpanProcessor {
    emitter: Box<dyn DagrEventEmitter>,
    event_counter: AtomicU64,
}

impl std::fmt::Debug for DagrSpanProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DagrSpanProcessor").finish_non_exhaustive()
    }
}

impl DagrSpanProcessor {
    pub fn new(emitter: Box<dyn DagrEventEmitter>) -> Self {
        Self {
            emitter,
            event_counter: AtomicU64::new(0),
        }
    }

    fn next_event_id(&self) -> String {
        let n = self.event_counter.fetch_add(1, Ordering::Relaxed);
        format!("otel-dagr-{}", n)
    }

    fn now_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Extract span attribute value as string.
    fn attr_str(&self, attrs: &[KeyValue], key: &str) -> Option<String> {
        attrs
            .iter()
            .find(|kv| kv.key.as_str() == key)
            .map(|kv| kv.value.as_str().to_string())
    }

    /// Extract span attribute value as i64.
    fn attr_i64(&self, attrs: &[KeyValue], key: &str) -> Option<i64> {
        attrs
            .iter()
            .find(|kv| kv.key.as_str() == key)
            .and_then(|kv| match &kv.value {
                opentelemetry::Value::I64(v) => Some(*v),
                _ => None,
            })
    }

    /// Emit a DAGR event with standard envelope.
    fn emit_event(&self, event_type: &str, payload: Value) {
        let event = json!({
            "id": self.next_event_id(),
            "type": event_type,
            "timestamp": self.now_timestamp(),
            "source": "otel-span-processor",
            "payload": payload,
        });
        self.emitter.emit(event);
    }

    /// Map an execution span to a DAGR task event.
    fn emit_execution_event(&self, span: &SpanData) {
        let session_id = self
            .attr_str(&span.attributes, "session.id")
            .unwrap_or("unknown".to_string());
        let command_id = self
            .attr_str(&span.attributes, "command.id")
            .unwrap_or("unknown".to_string());
        let command_name = self
            .attr_str(&span.attributes, "command.name")
            .unwrap_or("unknown".to_string());
        let exit_code = self.attr_i64(&span.attributes, "command.exit_code");

        // Determine event type based on span status and exit code
        let (event_type, payload) = match &span.status {
            Status::Error { .. } => (
                events::TASK_FAILED,
                json!({
                    "session_id": session_id,
                    "command_id": command_id,
                    "command": command_name,
                    "exit_code": exit_code,
                    "status_code": 1,
                }),
            ),
            _ if let Some(code) = exit_code => {
                if code != 0 {
                    (
                        events::TASK_FAILED,
                        json!({
                            "session_id": session_id,
                            "command_id": command_id,
                            "command": command_name,
                            "exit_code": code,
                            "status_code": 1,
                        }),
                    )
                } else {
                    (
                        events::TASK_COMPLETED,
                        json!({
                            "session_id": session_id,
                            "command_id": command_id,
                            "command": command_name,
                            "exit_code": code,
                            "status_code": 0,
                        }),
                    )
                }
            },
            _ => (
                events::TASK_STARTED,
                json!({
                    "session_id": session_id,
                    "command_id": command_id,
                    "command": command_name,
                }),
            ),
        };

        self.emit_event(event_type, payload);
    }

    /// Map a guard evaluation span to a DAGR permission event.
    fn emit_guard_event(&self, span: &SpanData) {
        let session_id = self
            .attr_str(&span.attributes, "session.id")
            .unwrap_or("unknown".to_string());
        let action_type = self
            .attr_str(&span.attributes, "action.type")
            .unwrap_or("unknown".to_string());
        let decision = self
            .attr_str(&span.attributes, "guard.decision")
            .unwrap_or("unknown".to_string());
        let trust_layer = self.attr_i64(&span.attributes, "trust.layer").unwrap_or(0);
        let confidence = self
            .attr_str(&span.attributes, "confidence")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.0);

        // Low confidence or interrupted decision → permission requested
        if confidence < 0.7 || decision == "interrupted" {
            let event_type = if action_type == "exec" && trust_layer <= 2 {
                events::BLOCKED_TASK
            } else {
                events::PERMISSION_REQUESTED
            };

            self.emit_event(
                event_type,
                json!({
                    "session_id": session_id,
                    "action_type": action_type,
                    "trust_layer": trust_layer,
                    "confidence": confidence,
                    "decision": decision,
                }),
            );
        }
    }

    /// Map a session lifecycle span to an agent state event.
    fn emit_session_event(&self, span: &SpanData) {
        let session_id = self
            .attr_str(&span.attributes, "session.id")
            .unwrap_or("unknown".to_string());

        if span.name.as_ref() == "crabjar.session.start" {
            self.emit_event(events::AGENT_ONLINE, json!({ "session_id": session_id }));
        } else if span.name.as_ref() == "crabjar.session.end" {
            self.emit_event(events::AGENT_OFFLINE, json!({ "session_id": session_id }));
        }
    }
}

impl SpanProcessor for DagrSpanProcessor {
    fn on_start(&self, _span: &mut opentelemetry_sdk::trace::Span, _cx: &opentelemetry::Context) {
        // Start events are emitted in on_end with the full span data.
    }

    fn on_end(&self, span: SpanData) {
        let name = span.name.as_ref();

        match name {
            "crabjar.execution" => {
                self.emit_execution_event(&span);
            },
            "crabjar.guard.evaluate" => {
                self.emit_guard_event(&span);
            },
            "crabjar.session.start" | "crabjar.session.end" => {
                self.emit_session_event(&span);
            },
            _ => {
                // Other spans: emit as generic shell job events if they have duration
                let duration = span.end_time.duration_since(span.start_time).unwrap_or_default();
                if duration.as_millis() > 1000 {
                    // Long-running command → shell job event
                    self.emit_event(
                        events::SHELL_JOB_FINISHED,
                        json!({
                            "span_name": name,
                            "duration_ms": duration.as_millis(),
                            "status": match &span.status {
                                Status::Error { .. } => "error",
                                _ => "ok",
                            },
                        }),
                    );
                }
            },
        }
    }

    fn force_flush(&self) -> opentelemetry_sdk::error::OTelSdkResult {
        // Nothing to flush — events are emitted synchronously
        Ok(())
    }

    fn shutdown_with_timeout(
        &self,
        _timeout: std::time::Duration,
    ) -> opentelemetry_sdk::error::OTelSdkResult {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Test emitter that collects events for inspection.
    #[derive(Clone)]
    struct CollectingEmitter {
        events: Arc<Mutex<Vec<Value>>>,
    }

    impl CollectingEmitter {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_events(&self) -> Vec<Value> {
            self.events.lock().unwrap().clone()
        }
    }

    impl DagrEventEmitter for CollectingEmitter {
        fn emit(&self, event: Value) {
            self.events.lock().unwrap().push(event);
        }
    }

    fn make_span_data(name: &str, attrs: Vec<KeyValue>, status: Status) -> SpanData {
        let now = SystemTime::now();
        SpanData {
            span_context: opentelemetry::trace::SpanContext::new(
                opentelemetry::trace::TraceId::INVALID,
                opentelemetry::trace::SpanId::INVALID,
                opentelemetry::trace::TraceFlags::new(0),
                false,
                Default::default(),
            ),
            parent_span_id: opentelemetry::trace::SpanId::INVALID,
            parent_span_is_remote: false,
            span_kind: opentelemetry::trace::SpanKind::Server,
            name: std::borrow::Cow::Owned(name.to_string()),
            start_time: now,
            end_time: now,
            attributes: attrs,
            dropped_attributes_count: 0,
            events: Default::default(),
            links: Default::default(),
            status,
            instrumentation_scope: Default::default(),
        }
    }

    #[test]
    fn execution_span_success_emits_task_completed() {
        let emitter = CollectingEmitter::new();
        let processor = DagrSpanProcessor::new(Box::new(emitter.clone()));

        let span = make_span_data(
            "crabjar.execution",
            vec![
                KeyValue::new("session.id", "sess-1"),
                KeyValue::new("command.id", "cmd-1"),
                KeyValue::new("command.name", "cargo test"),
                KeyValue::new("command.exit_code", 0i64),
            ],
            Status::Ok,
        );

        processor.on_end(span);

        let events = emitter.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "task_completed");
        assert_eq!(events[0]["payload"]["command"], "cargo test");
    }

    #[test]
    fn execution_span_failure_emits_task_failed() {
        let emitter = CollectingEmitter::new();
        let processor = DagrSpanProcessor::new(Box::new(emitter.clone()));

        let span = make_span_data(
            "crabjar.execution",
            vec![
                KeyValue::new("session.id", "sess-1"),
                KeyValue::new("command.id", "cmd-2"),
                KeyValue::new("command.name", "rm -rf /"),
                KeyValue::new("command.exit_code", 1i64),
            ],
            Status::Error {
                description: "Command failed".into(),
            },
        );

        processor.on_end(span);

        let events = emitter.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "task_failed");
    }

    #[test]
    fn guard_low_confidence_emits_permission_requested() {
        let emitter = CollectingEmitter::new();
        let processor = DagrSpanProcessor::new(Box::new(emitter.clone()));

        let span = make_span_data(
            "crabjar.guard.evaluate",
            vec![
                KeyValue::new("session.id", "sess-1"),
                KeyValue::new("action.type", "exec"),
                KeyValue::new("guard.decision", "proceed"),
                KeyValue::new("trust.layer", 3i64),
                KeyValue::new("confidence", "0.5"),
            ],
            Status::Ok,
        );

        processor.on_end(span);

        let events = emitter.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "permission_requested");
    }

    #[test]
    fn session_start_emits_agent_online() {
        let emitter = CollectingEmitter::new();
        let processor = DagrSpanProcessor::new(Box::new(emitter.clone()));

        let span = make_span_data(
            "crabjar.session.start",
            vec![KeyValue::new("session.id", "sess-1")],
            Status::Ok,
        );

        processor.on_end(span);

        let events = emitter.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "agent_online");
    }
}
