//! Tier 2: Structural Interpretation — when sensors detect something, parse it
//! for meaning. Determine severity, novelty, and context without invoking the
//! expensive Jev decision layer.

use crate::sensor::SensorEvent;
use serde::{Deserialize, Serialize};

/// Result of interpreting a sensor event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interpretation {
    /// Is this event meaningful enough to warrant further attention?
    pub meaningful: bool,
    /// Severity on [0.0, 1.0] scale.
    pub severity: f64,
    /// Is this a novel pattern or something we've seen before?
    pub novelty: f64,
    /// Human-readable summary of what's going on.
    pub summary: String,
    /// The original event that was interpreted.
    pub source_event: SensorEvent,
}

/// Interpretation engine that analyzes sensor events for meaning.
///
/// This is the "structural parsing" layer — it looks at the shape and content
/// of events to determine if they're worth escalating to Jev decisions. Uses
/// pattern matching, heuristics, and lattice queries (for novelty detection)
/// rather than expensive LLM inference.
pub trait InterpretationEngine: Send + Sync {
    fn interpret(&self, event: SensorEvent) -> Interpretation;
}

/// Simple heuristic interpretation engine for demonstration.
pub struct HeuristicInterpreter;

impl HeuristicInterpreter {
    pub fn new() -> Self {
        Self
    }
}

impl InterpretationEngine for HeuristicInterpreter {
    fn interpret(&self, event: SensorEvent) -> Interpretation {
        // Simple heuristic: disk events are somewhat severe, others less so
        let is_disk = event.source.contains("disk") || event.event_type.contains("disk");
        let severity = if is_disk {
            0.7
        } else if event.event_type.contains("crash") {
            0.9
        } else {
            0.3
        };

        Interpretation {
            meaningful: true,
            severity,
            novelty: 0.5,
            summary: format!("Event from {}: {}", event.source, event.event_type),
            source_event: event,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor::SensorEvent;

    #[test]
    fn interprets_events() {
        let interp = HeuristicInterpreter::new();
        let event = SensorEvent {
            source: "disk".to_string(),
            event_type: "high_usage".to_string(),
            timestamp: 0,
            payload: serde_json::json!({"usage": 95.0}),
        };

        let result = interp.interpret(event);
        assert!(result.meaningful);
        assert_eq!(result.severity, 0.7);
    }
}
