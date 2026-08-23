/// Decision flow gate — determines whether the agent should call a tool
/// or respond directly to the user.
///
/// This is a core agent behavior gate that controls tool invocation.
/// Without it, the agent either always calls tools (over-reliance) or
/// never does (under-utilization). The gate uses a combination of:
/// - Model-assisted decision (when a model is available)
/// - Heuristic fallback (when no model is configured)
/// - Trust layer enforcement (deny/pending/proceed)
///
/// ## Design
///
/// The gate evaluates the current WorkItem state and decides:
/// 1. `ToolCall { tool: ..., confidence: f32 }` — call a specific tool
/// 2. `RespondDirectly { response: String }` — respond without tools
/// 3. `Defer { reason: String }` — defer decision (insufficient info)
///
/// This mirrors the IronClaw pattern of making agent decisions
/// gateable and auditable rather than implicit.
use crabjar_host_core::WorkItem;
use std::fmt;

/// The type of decision made by the gate.
#[derive(Debug, Clone)]
pub enum Decision {
    /// Call a specific tool.
    ToolCall {
        /// Tool name to invoke.
        tool: String,
        /// Confidence in the decision (0.0–1.0).
        confidence: f32,
    },
    /// Respond directly without tool calls.
    RespondDirectly {
        /// The direct response text.
        response: String,
        /// Reason for not calling tools.
        reason: String,
    },
    /// Defer the decision — insufficient information.
    Defer {
        /// Why the decision was deferred.
        reason: String,
    },
}

impl Decision {
    /// Check if this decision results in a tool call.
    pub fn is_tool_call(&self) -> bool {
        matches!(self, Self::ToolCall { .. })
    }

    /// Check if this decision results in a direct response.
    pub fn is_direct_response(&self) -> bool {
        matches!(self, Self::RespondDirectly { .. })
    }

    /// Check if this decision was deferred.
    pub fn is_deferred(&self) -> bool {
        matches!(self, Self::Defer { .. })
    }
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ToolCall { tool, confidence } => {
                write!(f, "tool_call({} @ {:.0}%)", tool, confidence * 100.0)
            }
            Self::RespondDirectly { response, reason } => {
                write!(
                    f,
                    "respond_directly ({}): {}",
                    reason,
                    if response.len() > 80 {
                        format!("{}...", &response[..80])
                    } else {
                        response.clone()
                    }
                )
            }
            Self::Defer { reason } => write!(f, "defer ({}: {})", reason, reason),
        }
    }
}

/// Configuration for the decision gate.
#[derive(Debug, Clone)]
pub struct DecisionConfig {
    /// Minimum confidence to auto-decide (below this, defer).
    pub auto_decide_threshold: f32,
    /// Maximum tool calls before forcing a direct response.
    pub max_tool_calls_per_turn: usize,
    /// Whether to use model-assisted decisions.
    pub use_model_decision: bool,
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            auto_decide_threshold: 0.7,
            max_tool_calls_per_turn: 5,
            use_model_decision: false,
        }
    }
}

impl DecisionConfig {
    /// Enable model-assisted decisions.
    pub fn with_model_decision(mut self) -> Self {
        self.use_model_decision = true;
        self
    }

    /// Disable model-assisted decisions (heuristic only).
    pub fn heuristic_only(mut self) -> Self {
        self.use_model_decision = false;
        self
    }

    /// Set the auto-decide confidence threshold.
    pub fn with_auto_decide_threshold(mut self, threshold: f32) -> Self {
        self.auto_decide_threshold = threshold;
        self
    }
}

/// The decision gate — evaluates whether to call tools or respond directly.
pub struct DecisionGate {
    config: DecisionConfig,
}

impl DecisionGate {
    /// Create a new decision gate with the given config.
    pub fn new(config: DecisionConfig) -> Self {
        Self { config }
    }

    /// Create with default config (heuristic only).
    pub fn default_gate() -> Self {
        Self::new(DecisionConfig::default())
    }

    /// Make a decision based on the current WorkItem state.
    ///
    /// Strategy:
    /// 1. If no plan exists, defer (nothing to execute)
    /// 2. If all tasks are complete, respond directly
    /// 3. If no tasks remain, respond directly
    /// 4. If model decision is enabled and confidence >= threshold, use model
    /// 5. Otherwise, use heuristic decision
    pub fn decide(&self, work_item: &WorkItem) -> Decision {
        // No plan yet — defer until we have something to work with
        if work_item.plan.is_empty() {
            return Decision::Defer {
                reason: "no plan yet".into(),
            };
        }

        // All tasks complete — respond directly
        let pending: Vec<_> = work_item
            .plan
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    crabjar_host_core::work_item::TaskStatus::Pending
                        | crabjar_host_core::work_item::TaskStatus::InProgress
                )
            })
            .collect();

        if pending.is_empty() {
            return Decision::RespondDirectly {
                response: "All planned tasks completed.".into(),
                reason: "all tasks done".into(),
            };
        }

        // Check if we've exceeded max tool calls this turn
        let completed_count = work_item
            .plan
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    crabjar_host_core::work_item::TaskStatus::Completed
                )
            })
            .count();

        if completed_count >= self.config.max_tool_calls_per_turn {
            return Decision::RespondDirectly {
                response: format!(
                    "Reached max tool calls ({}) for this turn. Current progress: {}/{} tasks.",
                    self.config.max_tool_calls_per_turn,
                    completed_count,
                    work_item.plan.len()
                ),
                reason: "max tool calls reached".into(),
            };
        }

        // Use model-assisted decision if enabled
        if self.config.use_model_decision {
            // In practice, this would call the model router here.
            // For now, fall through to heuristic with a note.
            tracing::info!(
                "Model-assisted decision requested but not yet implemented; using heuristic"
            );
        }

        // Heuristic decision
        self.heuristic_decision(work_item)
    }

    /// Heuristic decision logic.
    ///
    /// Rules:
    /// - If confidence >= auto_decide_threshold and there are pending tasks → tool call
    /// - If confidence is low (< 0.3) and many failures → respond directly (stop trying)
    /// - If no plan yet → defer
    /// - Otherwise → tool call for the first pending task
    fn heuristic_decision(&self, work_item: &WorkItem) -> Decision {
        // Low confidence with failures → stop trying, respond directly
        if work_item.confidence < 0.3 {
            let failed: usize = work_item
                .plan
                .iter()
                .filter(|t| {
                    matches!(
                        t.status,
                        crabjar_host_core::work_item::TaskStatus::Failed { .. }
                    )
                })
                .count();
            if failed > 0 {
                return Decision::RespondDirectly {
                    response: format!(
                        "Low confidence ({:.0}%) with {} failures. Stopping tool calls.",
                        work_item.confidence * 100.0,
                        failed
                    ),
                    reason: "low confidence with failures".into(),
                };
            }
        }

        // Find the first pending/in-progress task and recommend it
        if let Some(task) = work_item.plan.iter().find(|t| {
            matches!(
                t.status,
                crabjar_host_core::work_item::TaskStatus::Pending
                    | crabjar_host_core::work_item::TaskStatus::InProgress
            )
        }) {
            // Extract tool name from task description (first word)
            let tool_name = task
                .description
                .split_whitespace()
                .next()
                .unwrap_or("unknown")
                .to_string();

            Decision::ToolCall {
                tool: tool_name,
                confidence: work_item.confidence,
            }
        } else {
            // No pending tasks but not all complete — some may be failed
            Decision::Defer {
                reason: "no pending tasks but incomplete plan".into(),
            }
        }
    }

    /// Check if a decision should be auto-approved based on trust layer.
    pub fn should_auto_approve(&self, decision: &Decision, trust_layer: u8) -> bool {
        // Low trust layers require manual approval for tool calls
        if trust_layer < 2 {
            return false;
        }

        match decision {
            Decision::RespondDirectly { .. } => true, // Direct responses are always auto-approved
            Decision::Defer { .. } => false,
            Decision::ToolCall { confidence, .. } => {
                // Auto-approve tool calls with high confidence at trusted layers
                *confidence >= self.config.auto_decide_threshold
            }
        }
    }

    /// Get the recommended action for logging/metrics.
    pub fn action_label(&self, decision: &Decision) -> &'static str {
        match decision {
            Decision::ToolCall { .. } => "tool_call",
            Decision::RespondDirectly { .. } => "direct_response",
            Decision::Defer { .. } => "defer",
        }
    }
}

impl Default for DecisionGate {
    fn default() -> Self {
        Self::default_gate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crabjar_host_core::work_item::TaskStatus;

    fn make_work_item(objective: &str) -> WorkItem {
        WorkItem::new(objective)
    }

    #[test]
    fn test_decide_no_plan_defers() {
        let gate = DecisionGate::default_gate();
        let wi = make_work_item("test objective");
        let decision = gate.decide(&wi);
        assert!(decision.is_deferred());
    }

    #[test]
    fn test_decide_all_tasks_completed_responds_directly() {
        let gate = DecisionGate::default_gate();
        let mut wi = make_work_item("test objective");
        wi.add_task("task 1");
        wi.update_task(0, TaskStatus::Completed, Some("done".into()));

        let decision = gate.decide(&wi);
        assert!(decision.is_direct_response());
    }

    #[test]
    fn test_decide_pending_tasks_recommends_tool() {
        let gate = DecisionGate::default_gate();
        let mut wi = make_work_item("test objective");
        wi.add_task("cargo_check --workspace");
        wi.set_confidence(0.5);

        let decision = gate.decide(&wi);
        assert!(decision.is_tool_call());

        if let Decision::ToolCall { tool, .. } = decision {
            assert_eq!(tool, "cargo_check");
        }
    }

    #[test]
    fn test_decide_low_confidence_with_failures_responds_directly() {
        let gate = DecisionGate::default_gate();
        let mut wi = make_work_item("test objective");
        wi.add_task("task 1");
        wi.update_task(
            0,
            TaskStatus::Failed {
                reason: "error".into(),
            },
            None,
        );
        wi.set_confidence(0.2);

        let decision = gate.decide(&wi);
        assert!(decision.is_direct_response());
    }

    #[test]
    fn test_decide_max_tool_calls_reached() {
        let config = DecisionConfig {
            max_tool_calls_per_turn: 3,
            ..DecisionConfig::default()
        };
        let gate = DecisionGate::new(config);
        let mut wi = make_work_item("test objective");

        // Add 3 completed tasks
        for i in 0..3 {
            wi.add_task(format!("task {}", i));
            wi.update_task(i, TaskStatus::Completed, Some("done".into()));
        }
        // Add 1 pending task
        wi.add_task("pending task");

        let decision = gate.decide(&wi);
        assert!(decision.is_direct_response());
    }

    #[test]
    fn test_auto_approve_direct_response() {
        let gate = DecisionGate::default_gate();
        let decision = Decision::RespondDirectly {
            response: "test".into(),
            reason: "test".into(),
        };
        assert!(gate.should_auto_approve(&decision, 2));
    }

    #[test]
    fn test_auto_approve_tool_call_requires_trust() {
        let gate = DecisionGate::default_gate();
        let decision = Decision::ToolCall {
            tool: "test".into(),
            confidence: 0.8,
        };
        assert!(!gate.should_auto_approve(&decision, 1)); // low trust
        assert!(gate.should_auto_approve(&decision, 2)); // trusted
    }

    #[test]
    fn test_auto_approve_low_confidence_tool_call() {
        let gate = DecisionGate::default_gate();
        let decision = Decision::ToolCall {
            tool: "test".into(),
            confidence: 0.5,
        };
        // Below threshold (0.7), so not auto-approved even at trusted layer
        assert!(!gate.should_auto_approve(&decision, 3));
    }

    #[test]
    fn test_auto_approve_high_confidence_tool_call() {
        let gate = DecisionGate::default_gate();
        let decision = Decision::ToolCall {
            tool: "test".into(),
            confidence: 0.9,
        };
        assert!(gate.should_auto_approve(&decision, 3));
    }

    #[test]
    fn test_decision_display() {
        let tool = Decision::ToolCall {
            tool: "cargo_check".into(),
            confidence: 0.85,
        };
        assert!(format!("{}", tool).contains("cargo_check"));
        assert!(format!("{}", tool).contains("85%"));

        let direct = Decision::RespondDirectly {
            response: "All done".into(),
            reason: "complete".into(),
        };
        assert!(format!("{}", direct).contains("respond_directly"));

        let defer = Decision::Defer {
            reason: "no plan".into(),
        };
        assert!(format!("{}", defer).contains("defer"));
    }

    #[test]
    fn test_config_with_model_decision() {
        let config = DecisionConfig::default()
            .with_model_decision()
            .with_auto_decide_threshold(0.8);
        assert!(config.use_model_decision);
        assert!((config.auto_decide_threshold - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_config_heuristic_only() {
        let config = DecisionConfig::default().heuristic_only();
        assert!(!config.use_model_decision);
    }
}
