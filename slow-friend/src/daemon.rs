//! The Slow Friend Daemon — the main loop that orchestrates all tiers of the
//! attention architecture. Sleeps most of the time, wakes on noteworthy events.

use crate::action::{ActionExecutor, ActionResult};
use crate::autonomy::AutonomyTracker;
use crate::decision::{JevDecision, JevEngine};
use crate::interpretation::InterpretationEngine;
use crate::lattice::SparseLattice;
use crate::sensor::{Sensor, SensorResult};
use std::sync::Arc;

/// Configuration for the slow friend daemon.
pub struct DaemonConfig {
    pub tick_interval_secs: u64,
    pub sleep_after_quiet_secs: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            tick_interval_secs: 60,
            sleep_after_quiet_secs: 300,
        }
    }
}

/// The Slow Friend daemon that orchestrates the tiered attention architecture.
pub struct SlowFriendDaemon<S, I, J, E>
where
    S: Sensor + 'static,
    I: InterpretationEngine + 'static,
    J: JevEngine + 'static,
    E: ActionExecutor + 'static,
{
    sensors: Vec<Arc<S>>,
    interpreter: Arc<I>,
    jev_engine: Arc<J>,
    executor: Arc<E>,
    autonomy: AutonomyTracker,
    lattice: SparseLattice,
    config: DaemonConfig,
}

impl<S, I, J, E> SlowFriendDaemon<S, I, J, E>
where
    S: Sensor + 'static,
    I: InterpretationEngine + 'static,
    J: JevEngine + 'static,
    E: ActionExecutor + 'static,
{
    pub fn new(
        sensors: Vec<Arc<S>>,
        interpreter: Arc<I>,
        jev_engine: Arc<J>,
        executor: Arc<E>,
    ) -> Self {
        Self {
            sensors,
            interpreter,
            jev_engine,
            executor,
            autonomy: AutonomyTracker::new(),
            lattice: SparseLattice::new(),
            config: DaemonConfig::default(),
        }
    }

    /// Run one tick of the daemon loop. Returns true if an action was taken.
    pub fn tick(&mut self) -> bool {
        // Tier 1: Check all sensors
        for sensor in &self.sensors {
            match sensor.check() {
                SensorResult::Quiet => continue,
                SensorResult::Alert(event) => {
                    // Tier 2: Interpret the event
                    let interpretation = self.interpreter.interpret(event);

                    if !interpretation.meaningful {
                        continue;
                    }

                    // Check autonomy level for this action class
                    let action_class = match &interpretation.source_event.event_type[..] {
                        s if s.contains("disk") => crate::action::ActionClass::DiskCleanup,
                        _ => continue,
                    };

                    let autonomy_level = self.autonomy.autonomy_level(&action_class);

                    // Tier 3: Jev decision (only for ambiguous cases)
                    let decision = match autonomy_level {
                        crate::autonomy::AutonomyLevel::Act => {
                            // High autonomy: skip Jev, execute directly
                            self.jev_engine.decide(&interpretation)
                        }
                        _ => {
                            // Lower autonomy: consult Jev for judgment
                            self.jev_engine.decide(&interpretation)
                        }
                    };

                    match decision {
                        JevDecision::Sleep => continue,
                        JevDecision::Execute(action) => {
                            // Tier 4: Execute with CrabJar guard
                            let result = self.executor.execute(&action);

                            // Update earned autonomy based on outcome
                            self.autonomy.record_outcome(&action, &result);

                            // Store in lattice for future reference
                            let key = format!(
                                "event:{}:{}",
                                action.class,
                                chrono::Utc::now().timestamp()
                            );
                            self.lattice.set(
                                &key,
                                serde_json::json!({
                                    "action": action.description,
                                    "result": match result {
                                        ActionResult::Success { .. } => "success",
                                        ActionResult::Failure { .. } => "failure",
                                        ActionResult::Rejected { .. } => "rejected",
                                    },
                                }),
                            );

                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{TestExecutor, ActionClass};
    use crate::decision::RuleBasedJevEngine;
    use crate::interpretation::HeuristicInterpreter;
    use crate::sensor::DiskUsageSensor;

    #[test]
    fn daemon_tick_no_action_when_quiet() {
        let sensors: Vec<Arc<DiskUsageSensor>> = vec![Arc::new(DiskUsageSensor::new(90.0))];
        let mut daemon = SlowFriendDaemon::new(
            sensors,
            Arc::new(HeuristicInterpreter::new()),
            Arc::new(RuleBasedJevEngine::new()),
            Arc::new(TestExecutor),
        );

        // No alert triggered, so no action taken
        assert!(!daemon.tick());
    }
}
