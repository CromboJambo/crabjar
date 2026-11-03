//! Slow friend daemon commands (EXPL-005)

use crabjar_lib::SlowFriendCommand;
use crabjar_slow_friend::action::ActionClass;
use crabjar_slow_friend::autonomy::{AutonomyLevel, AutonomyTracker};
use serde_json::json;

pub fn handle(command: SlowFriendCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        SlowFriendCommand::Start {
            tick_interval,
            sleep_after_quiet,
        } => {
            // Daemon mode: report configuration and readiness
            Ok(json!({
                "success": true,
                "slow_friend": {
                    "mode": "daemon",
                    "tick_interval_secs": tick_interval,
                    "sleep_after_quiet_secs": sleep_after_quiet,
                    "status": "configured",
                    "architecture": "tiered-attention-expl-005",
                    "tiers": [
                        {"level": 1, "name": "sensors", "cost": "cheap", "description": "deterministic checks"},
                        {"level": 2, "name": "interpretation", "cost": "medium", "description": "structural analysis"},
                        {"level": 3, "name": "jev_decision", "cost": "expensive", "description": "semantic judgment"}
                    ]
                }
            }))
        }
        SlowFriendCommand::Check => {
            // Single sensor check and report
            Ok(json!({
                "success": true,
                "slow_friend": {
                    "mode": "check",
                    "sensors_run": 0,
                    "alerts": [],
                    "status": "no_sensors_configured"
                }
            }))
        }
        SlowFriendCommand::Autonomy { class } => {
            // Show autonomy tracker status
            let tracker = AutonomyTracker::new();

            if let Some(class_name) = class {
                // Map string back to ActionClass enum (simplified for CLI)
                let permission = match class_name.as_str() {
                    "disk-cleanup" | "disk_cleanup" => {
                        tracker.autonomy_level(&ActionClass::DiskCleanup)
                    }
                    "restart-service" | "restart_service" => {
                        tracker.autonomy_level(&ActionClass::RestartService)
                    }
                    _ => AutonomyLevel::Ask,
                };
                Ok(json!({
                    "success": true,
                    "slow_friend": {
                        "mode": "autonomy",
                        "action_class": class_name,
                        "autonomy_level": format!("{:?}", permission),
                        "status": "reported"
                    }
                }))
            } else {
                // List all action classes
                Ok(json!({
                    "success": true,
                    "slow_friend": {
                        "mode": "autonomy",
                        "action_classes": [],
                        "status": "no_history"
                    }
                }))
            }
        }
    }
}