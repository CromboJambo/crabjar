// crabjar/src/knowledge_store/confidence.rs
// Confidence defaults and annotation confidence calculation.

use agent_context::state_docs::Annotation;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ConfidenceDefaults {
    pub note_confidence: f64,
    pub question_confidence: f64,
    pub promote_confidence: f64,
    pub provenance_id: String,
    pub set_at: u128,
    pub reason: String,
    pub source: String,
}

impl Default for ConfidenceDefaults {
    fn default() -> Self {
        Self {
            note_confidence: 0.80,
            question_confidence: 0.55,
            promote_confidence: 0.85,
            provenance_id: Uuid::new_v4().to_string(),
            set_at: now_unix_ms(),
            reason: "default confidence baselines".to_string(),
            source: "knowledge_store".to_string(),
        }
    }
}

#[allow(dead_code)]
impl ConfidenceDefaults {
    pub fn with_note_confidence(mut self, value: f64) -> Self {
        self.note_confidence = value;
        self.provenance_id = Uuid::new_v4().to_string();
        self.set_at = now_unix_ms();
        self
    }

    pub fn with_question_confidence(mut self, value: f64) -> Self {
        self.question_confidence = value;
        self.provenance_id = Uuid::new_v4().to_string();
        self.set_at = now_unix_ms();
        self
    }

    pub fn with_promote_confidence(mut self, value: f64) -> Self {
        self.promote_confidence = value;
        self.provenance_id = Uuid::new_v4().to_string();
        self.set_at = now_unix_ms();
        self
    }
}

pub fn annotation_confidence(annotation: &Annotation, defaults: &ConfidenceDefaults) -> f64 {
    let base = match annotation.kind.as_str() {
        "note" => defaults.note_confidence,
        "question" => defaults.question_confidence,
        _ => defaults.note_confidence,
    };

    let message = annotation.message.to_ascii_lowercase();
    let mut confidence: f64 = base;

    for marker in ["maybe", "might", "should", "todo", "follow-up", "follow up", "?"] {
        if message.contains(marker) {
            confidence -= 0.10;
        }
    }

    confidence.clamp(0.20, 0.95)
}

pub fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
