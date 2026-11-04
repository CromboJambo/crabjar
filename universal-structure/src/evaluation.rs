//! JEV evaluation target for code regions.

use crate::semantic::{ErrorHandling, Mutability, OwnershipMode};
use crate::span_kind::SpanKind;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A code region that can be evaluated by JEV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationTarget {
    /// The universal structural span kind.
    pub structure: SpanKind,
    /// Byte offsets in the source file.
    pub range: [usize; 2],
    /// Source text for embedding fallback.
    pub source: String,
    /// Function/method name if applicable.
    #[serde(default)]
    pub name: Option<String>,
    /// Language identifier (e.g., "rust", "javascript", "python").
    #[serde(default)]
    pub language: Option<String>,
    /// Semantic tags from analyzers.
    #[serde(default)]
    pub semantic_tags: HashMap<String, String>,
    /// Typed semantic fields.
    #[serde(default)]
    pub error_handling: Option<ErrorHandling>,
    #[serde(default)]
    pub mutability: Option<Mutability>,
    #[serde(default)]
    pub ownership_mode: Option<OwnershipMode>,
}

impl EvaluationTarget {
    /// Convert to an observation string for JEV embedding evaluation.
    #[must_use]
    pub fn to_observation(&self) -> String {
        let mut obs = format!("[{}] ", self.structure.as_str());

        if let Some(name) = &self.name {
            obs.push_str(&format!("{name} - "));
        }

        // Include typed semantic tags first
        let mut parts = Vec::new();
        if let Some(eh) = &self.error_handling {
            parts.push(format!("error_handling={}", eh.as_str()));
        }
        if let Some(mt) = &self.mutability {
            parts.push(format!("mutability={}", mt.as_str()));
        }
        if let Some(om) = &self.ownership_mode {
            parts.push(format!("ownership_mode={}", om.as_str()));
        }

        // Include raw semantic tags for extensibility
        for (k, v) in &self.semantic_tags {
            parts.push(format!("{k}={v}"));
        }

        if !parts.is_empty() {
            obs.push_str(&format!("({}) ", parts.join(", ")));
        }

        obs.push_str(&self.source);
        obs
    }

    /// Resolve a field name to its string value for condition evaluation.
    #[must_use]
    pub fn parse_value(&self, field: &str) -> Option<String> {
        match field {
            "structure" => Some(self.structure.as_str().to_string()),
            "name" => self.name.clone(),
            "language" => self.language.clone(),
            "error_handling" | "error_handling_mode" => {
                self.error_handling.as_ref().map(|e| e.as_str().to_string())
            }
            "mutability" | "mutability_mode" => {
                self.mutability.as_ref().map(|m| m.as_str().to_string())
            }
            "ownership_mode" => self.ownership_mode.as_ref().map(|o| o.as_str().to_string()),
            _ => self.semantic_tags.get(field).cloned(),
        }
    }
}
