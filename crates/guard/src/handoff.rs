//! Handoff contract types for context-preserving state transitions.
//!
//! These types define what information must cross every boundary between
//! processes (or between a process and its persistence layer). The principle:
//!
//! > The sender is responsible for transmitting sufficient context.
//! > The receiver is responsible for validating that context.
//! > Neither is responsible for pretending to know what the other knows.
//!
//! Phase 2 is purely additive — these types exist and concierge starts
//! constructing envelopes, but persistence paths are unchanged.

use crate::action::ActionStatus;
use serde::{Deserialize, Serialize};

/// Information the sender knows and must transmit for the receiver
/// to understand and responsibly continue the work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffContext {
    /// What triggered this action (user request, agent decision, system trigger)
    pub origin: ActionOrigin,

    /// The reasoning that led to this handoff
    pub reasoning: Option<ReasoningTrace>,

    /// Who/what is sending this and what authority they hold
    pub sender: SenderIdentity,

    /// What state the action was in before this handoff
    pub previous_state: ActionStatus,

    /// The intended transition (what the sender wants to happen)
    pub intended_transition: TransitionIntent,

    /// Idempotency key: allows safe retries without duplicate execution
    pub idempotency_key: String,

    /// Schema version of this handoff envelope
    pub schema_version: u32,
}

/// A typed handoff envelope: the unit of interprocess state transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffEnvelope {
    pub context: HandoffContext,
    pub payload: HandoffPayload,
}

/// The concrete data being handed off.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HandoffPayload {
    /// New action request
    ActionRequest {
        /// Serialized action request data (kept as JSON value for flexibility)
        action: serde_json::Value,
    },

    /// State transition on an existing action
    Transition {
        action_id: String,
        to_status: ActionStatus,
        reason: String,
    },

    /// Execution outcome
    Outcome {
        action_id: String,
        success: bool,
        exit_code: Option<i32>,
        evidence: Vec<String>,
    },
}

/// What triggered this action. Replaces the circular `source_event_id` pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionOrigin {
    /// Direct user request via CLI or TUI
    UserRequest {
        session_id: String,
        /// The actual "why" — currently lost in persistence
        reason: String,
    },

    /// Agent-initiated action based on observations
    AgentDecision {
        agent_id: String,
        /// mirror-log event IDs that informed this decision
        observation_ids: Vec<String>,
        rationale: String,
    },

    /// System-triggered (cron, health check, etc.)
    SystemTrigger {
        trigger_type: String,
        schedule_id: Option<String>,
    },

    /// Response to a previous action's outcome
    ReactionTo {
        parent_action_id: String,
        reaction_type: String,
    },
}

/// Who is sending this handoff and what authority they hold.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SenderIdentity {
    /// Human user via CLI/TUI — highest authority
    User { principal: String },

    /// The guard system itself (gate decisions)
    Guard { component: String },

    /// The concierge (outcome routing)
    Concierge,

    /// An agent with a specific trust layer
    Agent { agent_id: String, trust_layer: u32 },

    /// The JEV arbiter (belief-revision decisions)
    Arbiter { confidence: f64 },
}

impl SenderIdentity {
    /// Can this sender perform the given transition?
    ///
    /// This is the authority check. It's intentionally conservative:
    /// if a transition isn't explicitly allowed, it's denied.
    pub fn can_perform(&self, transition: &TransitionIntent) -> bool {
        match (self, transition) {
            // Users can do anything
            (SenderIdentity::User { .. }, _) => true,

            // Guard can create pending actions and interrupt
            (
                SenderIdentity::Guard { .. },
                TransitionIntent::ToPending | TransitionIntent::ToInterrupted { .. },
            ) => true,

            // Concierge routes outcomes: approve or deny
            (
                SenderIdentity::Concierge,
                TransitionIntent::ToApproved | TransitionIntent::ToDenied { .. },
            ) => true,

            // Arbiter can approve/deny with sufficient confidence
            (SenderIdentity::Arbiter { confidence }, TransitionIntent::ToApproved)
                if *confidence >= 0.85 =>
            {
                true
            }
            (SenderIdentity::Arbiter { confidence }, TransitionIntent::ToDenied { .. })
                if *confidence >= 0.85 =>
            {
                true
            }

            // Agents can only create pending actions
            (SenderIdentity::Agent { .. }, TransitionIntent::ToPending) => true,

            _ => false,
        }
    }
}

/// The intended state transition. Replaces raw status strings with typed intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "to", rename_all = "snake_case")]
pub enum TransitionIntent {
    ToPending,
    ToApproved,
    ToDenied { reason: String },
    ToExecuted { evidence: Vec<String> },
    ToInterrupted { reason: String },
}

impl TransitionIntent {
    /// The target status for this transition.
    pub fn to_status(&self) -> ActionStatus {
        match self {
            Self::ToPending => ActionStatus::Pending,
            Self::ToApproved => ActionStatus::TrustApproved,
            Self::ToDenied { .. } => ActionStatus::Denied,
            Self::ToExecuted { .. } => ActionStatus::Executed,
            Self::ToInterrupted { .. } => ActionStatus::Interrupted,
        }
    }

    /// Which statuses can legally transition to this one?
    pub fn valid_from(&self) -> Vec<ActionStatus> {
        match self {
            // ToPending is only valid as an initial state (no previous status)
            Self::ToPending => vec![],
            Self::ToApproved => vec![ActionStatus::Pending],
            Self::ToDenied { .. } => vec![ActionStatus::Pending, ActionStatus::TrustApproved],
            Self::ToExecuted { .. } => vec![ActionStatus::TrustApproved],
            Self::ToInterrupted { .. } => vec![ActionStatus::Pending, ActionStatus::TrustApproved],
        }
    }

    /// Extract the reason if this transition carries one.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::ToDenied { reason } | Self::ToInterrupted { reason } => Some(reason),
            _ => None,
        }
    }
}

/// Preserves the "why" — the reasoning chain that led to a decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTrace {
    pub steps: Vec<ReasoningStep>,
    pub conclusion: String,
    pub confidence: f64,
}

/// A single step in a reasoning chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    /// Source of this reasoning step: "gate", "arbiter:policy_check", "user", etc.
    pub source: String,
    /// What was observed
    pub observation: String,
    /// What was concluded
    pub inference: String,
    /// How this step affected overall confidence (-1.0 to 1.0)
    pub confidence_delta: f64,
}

impl ReasoningTrace {
    /// Create a new empty trace.
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            conclusion: String::new(),
            confidence: 0.0,
        }
    }

    /// Add a step to the trace.
    pub fn step(
        &mut self,
        source: impl Into<String>,
        observation: impl Into<String>,
        inference: impl Into<String>,
        delta: f64,
    ) {
        self.steps.push(ReasoningStep {
            source: source.into(),
            observation: observation.into(),
            inference: inference.into(),
            confidence_delta: delta,
        });
    }

    /// Finalize the trace with a conclusion.
    pub fn conclude(&mut self, conclusion: impl Into<String>) {
        self.conclusion = conclusion.into();
        self.confidence = self
            .steps
            .iter()
            .map(|s| s.confidence_delta)
            .sum::<f64>()
            .clamp(0.0, 1.0);
    }
}

impl Default for ReasoningTrace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_intent_to_status() {
        assert_eq!(
            TransitionIntent::ToPending.to_status(),
            ActionStatus::Pending
        );
        assert_eq!(
            TransitionIntent::ToApproved.to_status(),
            ActionStatus::TrustApproved
        );
        assert_eq!(
            TransitionIntent::ToDenied { reason: "x".into() }.to_status(),
            ActionStatus::Denied
        );
        assert_eq!(
            TransitionIntent::ToExecuted { evidence: vec![] }.to_status(),
            ActionStatus::Executed
        );
        assert_eq!(
            TransitionIntent::ToInterrupted { reason: "x".into() }.to_status(),
            ActionStatus::Interrupted
        );
    }

    #[test]
    fn test_transition_valid_from() {
        // ToApproved only from Pending
        assert_eq!(
            TransitionIntent::ToApproved.valid_from(),
            vec![ActionStatus::Pending]
        );
        // ToDenied from Pending or TrustApproved
        assert_eq!(
            TransitionIntent::ToDenied { reason: "x".into() }.valid_from(),
            vec![ActionStatus::Pending, ActionStatus::TrustApproved]
        );
    }

    #[test]
    fn test_sender_authority_user_can_do_anything() {
        let user = SenderIdentity::User {
            principal: "cli".into(),
        };
        assert!(user.can_perform(&TransitionIntent::ToApproved));
        assert!(user.can_perform(&TransitionIntent::ToDenied { reason: "x".into() }));
        assert!(user.can_perform(&TransitionIntent::ToExecuted { evidence: vec![] }));
    }

    #[test]
    fn test_sender_authority_concierge_limited() {
        let concierge = SenderIdentity::Concierge;
        assert!(concierge.can_perform(&TransitionIntent::ToApproved));
        assert!(concierge.can_perform(&TransitionIntent::ToDenied { reason: "x".into() }));
        // Concierge cannot execute
        assert!(!concierge.can_perform(&TransitionIntent::ToExecuted { evidence: vec![] }));
    }

    #[test]
    fn test_sender_authority_arbiter_confidence_threshold() {
        let confident = SenderIdentity::Arbiter { confidence: 0.9 };
        let uncertain = SenderIdentity::Arbiter { confidence: 0.5 };

        assert!(confident.can_perform(&TransitionIntent::ToApproved));
        assert!(!uncertain.can_perform(&TransitionIntent::ToApproved));
    }

    #[test]
    fn test_reasoning_trace() {
        let mut trace = ReasoningTrace::new();
        trace.step("gate", "command is rm -rf", "high risk detected", 0.3);
        trace.step("trust", "agent has layer 2", "moderate trust", 0.2);
        trace.conclude("approve with monitoring");

        assert_eq!(trace.steps.len(), 2);
        assert_eq!(trace.confidence, 0.5);
        assert_eq!(trace.conclusion, "approve with monitoring");
    }

    #[test]
    fn test_envelope_serialization() {
        let envelope = HandoffEnvelope {
            context: HandoffContext {
                origin: ActionOrigin::UserRequest {
                    session_id: "test-session".into(),
                    reason: "user asked to run ls".into(),
                },
                reasoning: None,
                sender: SenderIdentity::User {
                    principal: "cli".into(),
                },
                previous_state: ActionStatus::Pending,
                intended_transition: TransitionIntent::ToApproved,
                idempotency_key: "test-key-123".into(),
                schema_version: 1,
            },
            payload: HandoffPayload::Transition {
                action_id: "action-456".into(),
                to_status: ActionStatus::TrustApproved,
                reason: "approved via CLI".into(),
            },
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let deserialized: HandoffEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.context.idempotency_key, "test-key-123");
        assert_eq!(deserialized.context.schema_version, 1);
    }
}
