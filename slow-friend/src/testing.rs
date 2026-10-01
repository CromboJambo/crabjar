//! Testing harness for the slow friend daemon using synthetic events.
//! Generates fake sensor alerts to drive the full tiered attention pipeline.

use crate::action::{ActionClass, ActionExecutor, ActionResult};
use crate::autonomy::AutonomyTracker;
use crate::decision::{JevDecision, JevEngine};
use crate::interpretation::{HeuristicInterpreter, Interpretation, InterpretationEngine};
use crate::lattice::SparseLattice;
use crate::sensor::{Sensor, SensorEvent, SensorResult};
use std::sync::{Arc, Mutex};

/// Synthetic sensor that generates controlled alerts for testing.
pub struct SyntheticSensor {
    pub event_type: String,
    pub source: String,
    pub should_alert: bool,
}

impl Sensor for SyntheticSensor {
    fn name(&self) -> &str {
        "synthetic"
    }

    fn check(&self) -> SensorResult {
        if self.should_alert {
            SensorResult::Alert(SensorEvent {
                source: self.source.clone(),
                event_type: self.event_type.clone(),
                timestamp: chrono::Utc::now().timestamp(),
                payload: serde_json::json!({"synthetic": true}),
            })
        } else {
            SensorResult::Quiet
        }
    }
}

/// Recording executor that tracks which actions were executed.
pub struct RecordingExecutor {
    pub executed_actions: Arc<Mutex<Vec<ActionClass>>>,
}

impl RecordingExecutor {
    pub fn new() -> Self {
        Self {
            executed_actions: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ActionExecutor for RecordingExecutor {
    fn execute(&self, action: &crate::action::Action) -> ActionResult {
        self.executed_actions
            .lock()
            .unwrap()
            .push(action.class.clone());
        ActionResult::Success {
            details: "Test execution".to_string(),
        }
    }
}

/// Run a full synthetic test scenario.
pub fn run_synthetic_test() -> Vec<ActionClass> {
    // Create components
    let sensor = Arc::new(SyntheticSensor {
        event_type: "disk_high_usage".to_string(),
        source: "synthetic_disk".to_string(),
        should_alert: true,
    });

    let interpreter = Arc::new(HeuristicInterpreter::new());

    // Simple Jev engine that acts on disk events
    let jev_engine = Arc::new(crate::decision::RuleBasedJevEngine::new());

    let executor = RecordingExecutor::new();
    let exec_arc = Arc::new(executor);

    // Build daemon
    let mut daemon = crate::daemon::SlowFriendDaemon::new(
        vec![sensor],
        interpreter,
        jev_engine,
        exec_arc.clone(),
    );

    // Run tick — should trigger disk cleanup action
    let took_action = daemon.tick();

    assert!(took_action, "Daemon should have taken action on synthetic alert");

    // Verify the action was recorded
    let actions = exec_arc.executed_actions.lock().unwrap();
    assert!(!actions.is_empty(), "Executor should have recorded an action");
    assert_eq!(actions[0], ActionClass::DiskCleanup);

    actions.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_test_triggers_action() {
        let executed = run_synthetic_test();
        assert_eq!(executed.len(), 1);
        assert_eq!(executed[0], ActionClass::DiskCleanup);
    }

    #[test]
    fn autonomy_tracking_works() {
        let mut tracker = AutonomyTracker::new();

        // Simulate several successful disk cleanup actions
        for _ in 0..12 {
            let action = crate::action::Action {
                class: ActionClass::DiskCleanup,
                description: "Test".to_string(),
                parameters: serde_json::json!({}),
            };
            tracker.record_outcome(
                &action,
                &ActionResult::Success {
                    details: "ok".to_string(),
                },
            );
        }

        // Should have earned autonomous permission after 12 successes
        assert_eq!(
            tracker.autonomy_level(&ActionClass::DiskCleanup),
            crate::autonomy::AutonomyLevel::Act
        );
    }

    #[test]
    fn lattice_stores_event_history() {
        let mut lattice = SparseLattice::new();
        lattice.set(
            "event:disk_cleanup:12345",
            serde_json::json!({"result": "success"}),
        );

        assert!(lattice.contains("event:disk_cleanup:12345"));
        let value = lattice.get("event:disk_cleanup:12345").unwrap();
        assert_eq!(value["result"], "success");
    }
}
