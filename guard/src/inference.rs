//! Model inference types for the guard system.
//!
//! Types: `ModelInferenceKind`, `ModelInferenceRequest`, `ModelInferenceOutcome`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Model inference provenance type for guard gate tracking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelInferenceKind {
    Prompt,
    ContextAugmented,
    SkillAugmented,
    EmergentSkill,
}

impl fmt::Display for ModelInferenceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelInferenceKind::Prompt => write!(f, "prompt"),
            ModelInferenceKind::ContextAugmented => write!(f, "context-augmented"),
            ModelInferenceKind::SkillAugmented => write!(f, "skill-augmented"),
            ModelInferenceKind::EmergentSkill => write!(f, "emergent-skill"),
        }
    }
}

/// Model inference request gated by trust layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInferenceRequest {
    pub id: String,
    pub provenance_id: String,
    pub model_name: String,
    pub weight_id: String,
    pub inference_kind: ModelInferenceKind,
    pub prompt: String,
    pub context: Vec<String>,
    pub skill_refs: Vec<String>,
    pub trust_layer: u32,
    pub confidence: super::TrustScore,
    pub status: super::ActionStatus,
    pub gate_result: Option<String>,
    pub requested_at: i64,
    pub resolved_at: Option<i64>,
}

impl ModelInferenceRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        provenance_id: impl Into<String>,
        model_name: impl Into<String>,
        weight_id: impl Into<String>,
        inference_kind: ModelInferenceKind,
        prompt: impl Into<String>,
        trust_layer: u32,
        confidence: super::TrustScore,
    ) -> Self {
        Self {
            id: id.into(),
            provenance_id: provenance_id.into(),
            model_name: model_name.into(),
            weight_id: weight_id.into(),
            inference_kind,
            prompt: prompt.into(),
            context: Vec::new(),
            skill_refs: Vec::new(),
            trust_layer,
            confidence,
            status: super::ActionStatus::Pending,
            gate_result: None,
            requested_at: chrono::Utc::now().timestamp(),
            resolved_at: None,
        }
    }

    pub fn with_context(mut self, context: Vec<String>) -> Self {
        self.context = context;
        self
    }

    pub fn with_skill_refs(mut self, refs: Vec<String>) -> Self {
        self.skill_refs = refs;
        self
    }

    pub fn mark_approved(mut self) -> Self {
        self.status = super::ActionStatus::TrustApproved;
        self
    }

    pub fn mark_denied(mut self) -> Self {
        self.status = super::ActionStatus::Denied;
        self
    }

    pub fn mark_resolved(mut self) -> Self {
        self.status = super::ActionStatus::Executed;
        self.resolved_at = Some(chrono::Utc::now().timestamp());
        self
    }
}

/// Model inference outcome for confidence tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInferenceOutcome {
    pub id: String,
    pub inference_id: String,
    pub model_name: String,
    pub weight_id: String,
    pub output_hash: String,
    pub skill_residue: Option<String>,
    pub confidence_delta: f64,
    pub success: bool,
    pub created_at: i64,
}

impl ModelInferenceOutcome {
    pub fn new(
        id: impl Into<String>,
        inference_id: impl Into<String>,
        model_name: impl Into<String>,
        weight_id: impl Into<String>,
        output_hash: impl Into<String>,
        confidence_delta: f64,
        success: bool,
    ) -> Self {
        Self {
            id: id.into(),
            inference_id: inference_id.into(),
            model_name: model_name.into(),
            weight_id: weight_id.into(),
            output_hash: output_hash.into(),
            skill_residue: None,
            confidence_delta,
            success,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn with_skill_residue(mut self, residue: impl Into<String>) -> Self {
        self.skill_residue = Some(residue.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ActionStatus;
    use crate::trust_types::TrustScore;

    #[test]
    fn model_inference_kind_display_prompt() {
        assert_eq!(format!("{}", ModelInferenceKind::Prompt), "prompt");
    }

    #[test]
    fn model_inference_kind_display_context_augmented() {
        assert_eq!(
            format!("{}", ModelInferenceKind::ContextAugmented),
            "context-augmented"
        );
    }

    #[test]
    fn model_inference_kind_display_skill_augmented() {
        assert_eq!(
            format!("{}", ModelInferenceKind::SkillAugmented),
            "skill-augmented"
        );
    }

    #[test]
    fn model_inference_kind_display_emergent_skill() {
        assert_eq!(
            format!("{}", ModelInferenceKind::EmergentSkill),
            "emergent-skill"
        );
    }

    #[test]
    fn model_inference_kind_equality() {
        assert_eq!(ModelInferenceKind::Prompt, ModelInferenceKind::Prompt);
        assert_ne!(
            ModelInferenceKind::Prompt,
            ModelInferenceKind::ContextAugmented
        );
    }

    #[test]
    fn model_inference_request_new() {
        let request = ModelInferenceRequest::new(
            "inference-1",
            "prov-1",
            "gpt-4",
            "weight-1",
            ModelInferenceKind::Prompt,
            "Hello, world!",
            2,
            TrustScore::new(0.7),
        );
        assert_eq!(request.id, "inference-1");
        assert_eq!(request.model_name, "gpt-4");
        assert_eq!(request.trust_layer, 2);
        assert_eq!(request.status, crate::action::ActionStatus::Pending);
        assert!(request.context.is_empty());
        assert!(request.skill_refs.is_empty());
    }

    #[test]
    fn model_inference_request_with_context() {
        let request = ModelInferenceRequest::new(
            "inference-2",
            "prov-2",
            "gpt-4",
            "weight-2",
            ModelInferenceKind::ContextAugmented,
            "test",
            2,
            TrustScore::new(0.5),
        )
        .with_context(vec!["context1".to_string(), "context2".to_string()]);
        assert_eq!(request.context.len(), 2);
    }

    #[test]
    fn model_inference_request_with_skill_refs() {
        let request = ModelInferenceRequest::new(
            "inference-3",
            "prov-3",
            "gpt-4",
            "weight-3",
            ModelInferenceKind::SkillAugmented,
            "test",
            2,
            TrustScore::new(0.5),
        )
        .with_skill_refs(vec!["skill1".to_string()]);
        assert_eq!(request.skill_refs.len(), 1);
    }

    #[test]
    fn model_inference_request_mark_approved() {
        let request = ModelInferenceRequest::new(
            "inference-4",
            "prov-4",
            "gpt-4",
            "weight-4",
            ModelInferenceKind::Prompt,
            "test",
            2,
            TrustScore::new(0.5),
        )
        .mark_approved();
        assert_eq!(request.status, crate::action::ActionStatus::TrustApproved);
    }

    #[test]
    fn model_inference_request_mark_denied() {
        let request = ModelInferenceRequest::new(
            "inference-5",
            "prov-5",
            "gpt-4",
            "weight-5",
            ModelInferenceKind::Prompt,
            "test",
            2,
            TrustScore::new(0.5),
        )
        .mark_denied();
        assert_eq!(request.status, crate::action::ActionStatus::Denied);
    }

    #[test]
    fn model_inference_request_mark_resolved() {
        let request = ModelInferenceRequest::new(
            "inference-6",
            "prov-6",
            "gpt-4",
            "weight-6",
            ModelInferenceKind::Prompt,
            "test",
            2,
            TrustScore::new(0.5),
        )
        .mark_resolved();
        assert_eq!(request.status, crate::action::ActionStatus::Executed);
        assert!(request.resolved_at.is_some());
    }

    #[test]
    fn model_inference_outcome_new() {
        let outcome = ModelInferenceOutcome::new(
            "outcome-1",
            "inference-1",
            "gpt-4",
            "weight-1",
            "hash-abc",
            0.1,
            true,
        );
        assert_eq!(outcome.id, "outcome-1");
        assert_eq!(outcome.inference_id, "inference-1");
        assert_eq!(outcome.model_name, "gpt-4");
        assert_eq!(outcome.output_hash, "hash-abc");
        assert_eq!(outcome.confidence_delta, 0.1);
        assert!(outcome.success);
        assert!(outcome.skill_residue.is_none());
    }

    #[test]
    fn model_inference_outcome_with_skill_residue() {
        let outcome = ModelInferenceOutcome::new(
            "outcome-2",
            "inference-2",
            "gpt-4",
            "weight-2",
            "hash-def",
            -0.05,
            false,
        )
        .with_skill_residue("skill-residue-data");
        assert_eq!(outcome.skill_residue, Some("skill-residue-data".to_string()));
    }
}
