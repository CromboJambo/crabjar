//! Rich activity states for crabs — Phase 1 of Habblaud Terrarium Upgrade.
//!
//! Maps effective task states to detailed behavioral activities that drive
//! the renderer's animations and visual indicators. Each activity has an
//! icon, display text, and optional detail context for thought bubbles.

/// What one crab is doing right now — the rich behavioral state.
#[derive(Debug, Clone, PartialEq)]
pub enum CrabActivity {
    /// Executing a task (running code, editing files).
    Working,
    /// Planning or analyzing (model generating, thinking through problem).
    Thinking,
    /// Blocked waiting on an external command to complete.
    WaitingShell,
    /// Needs human decision — blocked task requiring input/approval.
    WaitingInput,
    /// No tasks assigned — idle.
    Idle,
    /// Idle + other crabs nearby — animate social behavior.
    Socializing,
    /// Task completed, waiting for next assignment.
    Done,
    /// Agent inactive (no recent attempts, task abandoned).
    Offline,
}

/// Display metadata for an activity — icon, text, and detail context.
#[derive(Debug, Clone)]
pub struct ActivityInfo {
    pub kind: CrabActivity,
    pub icon: &'static str,
    pub text: String,
    pub detail: Option<String>,
}

impl CrabActivity {
    /// Derive the rich activity from effective state and task context.
    ///
    /// # Parameters
    /// - `effective_state`: Derived state from simulator (queued/running/review/done/failed/blocked)
    /// - `needs_user`: Whether the task requires human input
    /// - `attempt_state`: Current attempt's raw state (running/done/failed/etc.)
    /// - `shell_job_active`: Whether an external command is currently executing
    /// - `recent_activity`: Whether there was recent activity (not stalled)
    pub fn derive(
        effective_state: &str,
        needs_user: bool,
        attempt_state: Option<&str>,
        shell_job_active: bool,
        recent_activity: bool,
    ) -> CrabActivity {
        // Blocked waiting on human input always takes precedence
        if effective_state == "blocked" && needs_user {
            return CrabActivity::WaitingInput;
        }

        match effective_state {
            "running" => {
                if shell_job_active {
                    CrabActivity::WaitingShell
                } else if attempt_state == Some("running") {
                    // Model is actively generating — thinking
                    CrabActivity::Thinking
                } else {
                    CrabActivity::Working
                }
            }
            "done" => CrabActivity::Done,
            "failed" => {
                // Failed but will retry — still working on it
                if recent_activity {
                    CrabActivity::Working
                } else {
                    CrabActivity::Offline
                }
            }
            "review" => CrabActivity::WaitingInput,
            "queued" | "todo" | "scheduled" | "ready" => CrabActivity::Idle,
            _ => CrabActivity::Idle,
        }
    }

    /// Get display metadata for this activity.
    pub fn info(&self, detail: Option<String>) -> ActivityInfo {
        match self {
            CrabActivity::Working => ActivityInfo {
                kind: CrabActivity::Working,
                icon: "🦀",
                text: "working".to_string(),
                detail,
            },
            CrabActivity::Thinking => ActivityInfo {
                kind: CrabActivity::Thinking,
                icon: "💭",
                text: "thinking".to_string(),
                detail,
            },
            CrabActivity::WaitingShell => ActivityInfo {
                kind: CrabActivity::WaitingShell,
                icon: "⏳",
                text: "waiting on command".to_string(),
                detail,
            },
            CrabActivity::WaitingInput => ActivityInfo {
                kind: CrabActivity::WaitingInput,
                icon: "🙋",
                text: "needs decision".to_string(),
                detail,
            },
            CrabActivity::Idle => ActivityInfo {
                kind: CrabActivity::Idle,
                icon: "😴",
                text: "idle".to_string(),
                detail,
            },
            CrabActivity::Socializing => ActivityInfo {
                kind: CrabActivity::Socializing,
                icon: "🗣️",
                text: "socializing".to_string(),
                detail,
            },
            CrabActivity::Done => ActivityInfo {
                kind: CrabActivity::Done,
                icon: "✅",
                text: "done".to_string(),
                detail,
            },
            CrabActivity::Offline => ActivityInfo {
                kind: CrabActivity::Offline,
                icon: "🚪",
                text: "offline".to_string(),
                detail,
            },
        }
    }
}

/// DAGR event types for activity transitions.
/// These map to the terrarium's live update events.
#[derive(Debug, Clone)]
pub struct ActivityEvent {
    pub agent_id: String,
    pub old_activity: CrabActivity,
    pub new_activity: CrabActivity,
}

#[derive(Debug, Clone)]
pub struct ShellJobEvent {
    pub agent_id: String,
    pub command: String,
    pub started_at: u64, // Unix timestamp
}

#[derive(Debug, Clone)]
pub struct PermissionRequestEvent {
    pub agent_id: String,
    pub request_type: String,
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_with_needs_user_is_waiting_input() {
        let activity = CrabActivity::derive("blocked", true, None, false, true);
        assert_eq!(activity, CrabActivity::WaitingInput);
    }

    #[test]
    fn running_with_shell_job_is_waiting_shell() {
        let activity = CrabActivity::derive("running", false, Some("running"), true, true);
        assert_eq!(activity, CrabActivity::WaitingShell);
    }

    #[test]
    fn running_model_generating_is_thinking() {
        let activity = CrabActivity::derive("running", false, Some("running"), false, true);
        assert_eq!(activity, CrabActivity::Thinking);
    }

    #[test]
    fn done_state_is_done_activity() {
        let activity = CrabActivity::derive("done", false, None, false, true);
        assert_eq!(activity, CrabActivity::Done);
    }

    #[test]
    fn failed_with_recent_activity_is_working() {
        let activity = CrabActivity::derive("failed", false, None, false, true);
        assert_eq!(activity, CrabActivity::Working);
    }

    #[test]
    fn failed_without_recent_activity_is_offline() {
        let activity = CrabActivity::derive("failed", false, None, false, false);
        assert_eq!(activity, CrabActivity::Offline);
    }

    #[test]
    fn queued_state_is_idle() {
        let activity = CrabActivity::derive("queued", false, None, false, true);
        assert_eq!(activity, CrabActivity::Idle);
    }

    #[test]
    fn review_state_is_waiting_input() {
        let activity = CrabActivity::derive("review", false, None, false, true);
        assert_eq!(activity, CrabActivity::WaitingInput);
    }

    #[test]
    fn activity_info_has_correct_icon() {
        let info = CrabActivity::Thinking.info(Some("analyzing requirements".to_string()));
        assert_eq!(info.icon, "💭");
        assert_eq!(info.text, "thinking");
        assert_eq!(info.detail.as_deref(), Some("analyzing requirements"));
    }

    #[test]
    fn waiting_input_info() {
        let info = CrabActivity::WaitingInput.info(Some("needs approval for rm -rf".to_string()));
        assert_eq!(info.icon, "🙋");
        assert_eq!(info.text, "needs decision");
    }
}
