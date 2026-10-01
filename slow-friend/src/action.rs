//! Tier 4: CrabJar Action Execution — actual actions with checkpointing and
//! notification. Every action goes through the guard/concierge pipeline
//! regardless of how it was decided.

use serde::{Deserialize, Serialize};

/// Distinct classes of actions that can be taken.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionClass {
    DiskCleanup,
    RestartService,
    SendNotification,
    ArchiveFile,
    RunBackup,
    Custom(String),
}

impl std::fmt::Display for ActionClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionClass::DiskCleanup => write!(f, "disk_cleanup"),
            ActionClass::RestartService => write!(f, "restart_service"),
            ActionClass::SendNotification => write!(f, "send_notification"),
            ActionClass::ArchiveFile => write!(f, "archive_file"),
            ActionClass::RunBackup => write!(f, "run_backup"),
            ActionClass::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Permission level for an action class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// Ask the user before acting (default for new action classes).
    Ask,
    /// Act but notify the user afterward.
    Notify,
    /// Act autonomously without notification.
    Autonomous,
}

/// A concrete action to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub class: ActionClass,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Result of executing an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionResult {
    Success { details: String },
    Failure { error: String },
    /// User rejected or modified the action proposal.
    Rejected { reason: Option<String> },
}

/// Trait for executing actions through CrabJar's guard/concierge pipeline.
pub trait ActionExecutor: Send + Sync {
    fn execute(&self, action: &Action) -> ActionResult;
}

/// Stub executor for testing — always succeeds.
pub struct TestExecutor;

impl ActionExecutor for TestExecutor {
    fn execute(&self, _action: &Action) -> ActionResult {
        ActionResult::Success {
            details: "Test execution succeeded".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_class_display() {
        assert_eq!(ActionClass::DiskCleanup.to_string(), "disk_cleanup");
    }

    #[test]
    fn test_executor_succeeds() {
        let executor = TestExecutor;
        let action = Action {
            class: ActionClass::DiskCleanup,
            description: "Test".to_string(),
            parameters: serde_json::json!({}),
        };
        match executor.execute(&action) {
            ActionResult::Success { .. } => {}
            _ => panic!("Expected success"),
        }
    }
}
