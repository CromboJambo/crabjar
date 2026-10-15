//! OpenTelemetry exporter for Crabjar telemetry events.
//!
//! Provides optional OTLP export of execution traces, metrics, and logs
//! alongside the SQLite flight recorder (which remains the source of truth).

use opentelemetry::{
    global,
    trace::{Span as _, SpanKind, Tracer as _, TracerProvider as _},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{propagation::TraceContextPropagator, Resource};
use std::sync::Arc;

/// Configuration for the OTel exporter.
#[derive(Debug, Clone)]
pub struct OtelConfig {
    /// OTLP endpoint URL (e.g., "http://localhost:4317" or "http://otel-collector.internal:4318").
    pub endpoint: String,
    /// Service name for traces/metrics.
    pub service_name: String,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4317".to_string(),
            service_name: "crabjar".to_string(),
        }
    }
}

/// OpenTelemetry exporter that sends traces, metrics, and logs to an OTLP collector.
pub struct OtelExporter {
    tracer: opentelemetry_sdk::trace::Tracer,
    _provider: Arc<opentelemetry_sdk::trace::TracerProvider>,
}

impl OtelExporter {
    /// Create a new exporter from configuration.
    pub fn from_config(config: &OtelConfig) -> Result<Self, String> {
        // Set up trace context propagation
        global::set_text_map_propagator(TraceContextPropagator::new());

        // Build OTLP span exporter via tonic/gRPC
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&config.endpoint)
            .build()
            .map_err(|e| format!("Failed to build OTLP span exporter: {}", e))?;

        // Build tracer provider with batch exporter and service name resource
        let provider = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                config.service_name.clone(),
            )]))
            .build();

        let tracer = provider.tracer("crabjar.telemetry");

        Ok(Self {
            tracer,
            _provider: Arc::new(provider),
        })
    }

    /// Record an execution event as a trace span.
    pub fn record_execution(
        &self,
        session_id: &str,
        command_id: &str,
        command: &str,
        args: &[String],
        cwd: &str,
        exit_code: i32,
        duration_ms: u64,
        reason: &str,
    ) {
        let mut span = self
            .tracer
            .span_builder(format!("crabjar.execution.{}", command))
            .with_kind(SpanKind::Server)
            .with_attributes(vec![
                KeyValue::new("session.id", session_id.to_string()),
                KeyValue::new("command.id", command_id.to_string()),
                KeyValue::new("command.name", command.to_string()),
                KeyValue::new("command.args", args.join(" ")),
                KeyValue::new("command.cwd", cwd.to_string()),
                KeyValue::new("command.exit_code", exit_code as i64),
                KeyValue::new("command.duration_ms", duration_ms as i64),
                KeyValue::new("command.reason", reason.to_string()),
            ])
            .start(&self.tracer);

        span.end();
    }

    /// Record a guard decision as a trace span.
    pub fn record_guard_decision(
        &self,
        session_id: &str,
        action_type: &str,
        decision: &str,
        trust_layer: i32,
        confidence: f64,
    ) {
        let mut span = self
            .tracer
            .span_builder("crabjar.guard.evaluate")
            .with_kind(SpanKind::Internal)
            .with_attributes(vec![
                KeyValue::new("session.id", session_id.to_string()),
                KeyValue::new("action.type", action_type.to_string()),
                KeyValue::new("guard.decision", decision.to_string()),
                KeyValue::new("trust.layer", trust_layer as i64),
                KeyValue::new("confidence", confidence),
            ])
            .start(&self.tracer);

        span.end();
    }

    /// Record a structured log event.
    pub fn record_log(
        &self,
        level: &str,
        message: &str,
        session_id: &str,
        attributes: Vec<KeyValue>,
    ) {
        let mut attrs = vec![
            KeyValue::new("log.level", level.to_string()),
            KeyValue::new("session.id", session_id.to_string()),
            KeyValue::new("message", message.to_string()),
        ];
        attrs.extend(attributes);

        let mut span = self
            .tracer
            .span_builder(format!("crabjar.log.{}", level))
            .with_kind(SpanKind::Internal)
            .with_attributes(attrs)
            .start(&self.tracer);

        span.end();
    }

    /// Shutdown the exporter (flushes remaining spans).
    pub fn shutdown(&self) {
        global::shutdown_tracer_provider();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = OtelConfig::default();
        assert_eq!(config.endpoint, "http://localhost:4317");
        assert_eq!(config.service_name, "crabjar");
    }
}
