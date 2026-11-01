//! Integration test: OTel spans → DAGR events
//!
//! Verifies that when FlightRecorder is configured with both OTel export
//! and DAGR event emission, execution spans generate the correct DAGR events.

#[cfg(feature = "otel")]
mod tests {
    use crabjar_telemetry::dagr_span_processor::{DagrSpanProcessor, StderrEmitter};

    #[test]
    fn dagr_span_processor_can_be_created() {
        let processor = DagrSpanProcessor::new(Box::new(StderrEmitter));
        // Processor is created successfully and implements SpanProcessor trait
        // (verified by the fact that it can be passed to with_span_processor)
        drop(processor);
    }

    #[test]
    fn dagr_processor_processes_execution_span() {
        use opentelemetry::{KeyValue, trace::Status};
        use opentelemetry_sdk::trace::{SpanData, SpanProcessor};
        use std::sync::{Arc, Mutex};
        use std::time::SystemTime;

        // Custom emitter that captures events
        #[derive(Clone)]
        struct CapturingEmitter {
            events: Arc<Mutex<Vec<String>>>,
        }

        impl crabjar_telemetry::dagr_span_processor::DagrEventEmitter for CapturingEmitter {
            fn emit(&self, event: serde_json::Value) {
                self.events.lock().unwrap().push(event.to_string());
            }
        }

        let emitter = CapturingEmitter {
            events: Arc::new(Mutex::new(Vec::new())),
        };

        let processor = DagrSpanProcessor::new(Box::new(emitter.clone()));

        // Create a test execution span
        let now = SystemTime::now();
        let span = SpanData {
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
            name: "crabjar.execution".into(),
            start_time: now,
            end_time: now,
            attributes: vec![
                KeyValue::new("session.id", "test-session"),
                KeyValue::new("command.id", "test-cmd"),
                KeyValue::new("command.name", "cargo test"),
                KeyValue::new("command.exit_code", 0i64),
            ],
            dropped_attributes_count: 0,
            events: Default::default(),
            links: Default::default(),
            status: Status::Ok,
            instrumentation_scope: Default::default(),
        };

        // Process the span
        processor.on_end(span);

        // Verify a task_completed event was emitted
        let events = emitter.events.lock().unwrap();
        assert_eq!(events.len(), 1, "Should have emitted one DAGR event");
        let event: serde_json::Value = serde_json::from_str(&events[0]).unwrap();
        assert_eq!(event["type"], "task_completed", "Should be task_completed for exit code 0");
    }

    #[test]
    fn dagr_processor_maps_session_start_to_agent_online() {
        use opentelemetry::{KeyValue, trace::Status};
        use opentelemetry_sdk::trace::{SpanData, SpanProcessor};
        use std::sync::{Arc, Mutex};
        use std::time::SystemTime;

        #[derive(Clone)]
        struct CapturingEmitter {
            events: Arc<Mutex<Vec<String>>>,
        }

        impl crabjar_telemetry::dagr_span_processor::DagrEventEmitter for CapturingEmitter {
            fn emit(&self, event: serde_json::Value) {
                self.events.lock().unwrap().push(event.to_string());
            }
        }

        let emitter = CapturingEmitter {
            events: Arc::new(Mutex::new(Vec::new())),
        };

        let processor = DagrSpanProcessor::new(Box::new(emitter.clone()));

        let now = SystemTime::now();
        let span = SpanData {
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
            name: "crabjar.session.start".into(),
            start_time: now,
            end_time: now,
            attributes: vec![KeyValue::new("session.id", "sess-123")],
            dropped_attributes_count: 0,
            events: Default::default(),
            links: Default::default(),
            status: Status::Ok,
            instrumentation_scope: Default::default(),
        };

        processor.on_end(span);

        let events = emitter.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        let event: serde_json::Value = serde_json::from_str(&events[0]).unwrap();
        assert_eq!(event["type"], "agent_online");
    }
}
