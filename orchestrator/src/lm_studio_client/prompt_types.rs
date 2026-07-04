//! Prompt envelope types — structured content with provenance.
//!
//! This module defines the core types for the prompt envelope system:
//! - `SourceLabel`: closed-vocabulary origin labels (no free-text)
//! - `LabeledContent`: content bound to a source label with SHA-256 provenance
//! - `PromptMetadata`: audit metadata attached to every envelope
//! - `PromptEnvelope`: wraps system + user content before sending to the model

use super::types::LmStudioEndpoint;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

// ===========================================================================
// Error types — defined first so methods can reference them
// ===========================================================================

/// Errors from prompt envelope operations.
#[derive(Debug, Error)]
pub enum PromptError {
    #[error("injection detected: '{pattern}' in source '{in_source}'")]
    InjectionDetected {
        pattern: String,
        in_source: SourceLabel,
    },

    #[error("invalid source: expected '{expected}', got '{got}'")]
    InvalidSource { expected: String, got: String },

    #[error("provenance mismatch: expected '{expected}', got '{actual}'")]
    ProvenanceMismatch { expected: String, actual: String },

    #[error("stale envelope: {max_age_seconds}s max, {actual_age_seconds}s old")]
    StaleEnvelope {
        max_age_seconds: u64,
        actual_age_seconds: u64,
    },
}

// ===========================================================================
// Source labels — closed vocabulary, no free-text origin
// ===========================================================================

/// Closed vocabulary for prompt content origin.
///
/// No free-text allowed — prevents spoofing by untrusted input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceLabel {
    /// Built-in system prompt (from config/templates loaded at compile time).
    SystemConfig,
    /// System prompt injected by agent/external source (post-validation).
    SystemInject,
    /// Direct user input.
    UserInput,
    /// Tool execution result.
    ToolOutput,
    /// Untrusted external source (mail, web, etc.).
    ExternalInput,
}

impl std::fmt::Display for SourceLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl SourceLabel {
    /// Returns the display name for logging.
    pub fn name(&self) -> &'static str {
        match self {
            Self::SystemConfig => "system_config",
            Self::SystemInject => "system_inject",
            Self::UserInput => "user_input",
            Self::ToolOutput => "tool_output",
            Self::ExternalInput => "external_input",
        }
    }

    /// Returns true if this label represents a trusted system-level source.
    pub fn is_system(&self) -> bool {
        matches!(self, Self::SystemConfig | Self::SystemInject)
    }

    /// Returns true if this label represents an untrusted/external source.
    pub fn is_external(&self) -> bool {
        matches!(self, Self::ExternalInput)
    }
}

// ===========================================================================
// Labeled content — content with provenance
// ===========================================================================

/// A piece of prompt content with its provenance label.
#[derive(Debug, Clone)]
pub struct LabeledContent {
    /// The source label (closed vocabulary).
    pub label: SourceLabel,
    /// The actual content.
    pub content: String,
    /// SHA-256 of (source_label + content) for integrity verification.
    pub provenance_id: String,
}

impl LabeledContent {
    /// Create a new labeled content with computed provenance.
    pub fn new(label: SourceLabel, content: String) -> Self {
        let provenance_id = compute_provenance(label, &content);
        Self {
            label,
            content,
            provenance_id,
        }
    }

    /// Verify the content hasn't been tampered with.
    pub fn verify_provenance(&self) -> Result<(), PromptError> {
        let expected = compute_provenance(self.label, &self.content);
        if self.provenance_id != expected {
            return Err(PromptError::ProvenanceMismatch {
                expected,
                actual: self.provenance_id.clone(),
            });
        }
        Ok(())
    }

    /// Check if content is non-empty and non-whitespace-only.
    pub fn is_valid(&self) -> bool {
        !self.content.trim().is_empty()
    }
}

// ===========================================================================
// Prompt envelope — wraps all prompt content before sending
// ===========================================================================

/// Metadata attached to every envelope for audit trail.
#[derive(Debug, Clone)]
pub struct PromptMetadata {
    /// Unix timestamp when the envelope was created.
    pub created_at: u64,
    /// Session this envelope belongs to.
    pub session_id: String,
    /// Which endpoint this envelope targets.
    pub endpoint: LmStudioEndpoint,
    /// Schema version of the validator (for future compatibility).
    pub validator_version: u32,
}

impl PromptMetadata {
    pub fn new(session_id: String, endpoint: LmStudioEndpoint) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            created_at: now,
            session_id,
            endpoint,
            validator_version: 1,
        }
    }
}

/// The prompt envelope — wraps all prompt content before sending to the model.
#[derive(Debug, Clone)]
pub struct PromptEnvelope {
    /// Trusted system prompt (from config/templates).
    pub system_prompt: LabeledContent,
    /// User-facing content (input, tool output, or external).
    pub user_content: LabeledContent,
    /// Audit metadata.
    pub metadata: PromptMetadata,
}

impl PromptEnvelope {
    /// Create a new envelope from labeled content.
    pub fn new(
        system_prompt: LabeledContent,
        user_content: LabeledContent,
        session_id: String,
        endpoint: LmStudioEndpoint,
    ) -> Self {
        Self {
            system_prompt,
            user_content,
            metadata: PromptMetadata::new(session_id, endpoint),
        }
    }

    /// Build an envelope from raw strings with the given source labels.
    pub fn from_raw(
        system_content: String,
        system_label: SourceLabel,
        user_content: String,
        user_label: SourceLabel,
        session_id: String,
        endpoint: LmStudioEndpoint,
    ) -> Self {
        Self::new(
            LabeledContent::new(system_label, system_content),
            LabeledContent::new(user_label, user_content),
            session_id,
            endpoint,
        )
    }

    /// Return the combined text that would be sent to the model.
    pub fn to_model_text(&self) -> String {
        format!("{}\n\n{}", self.system_prompt.content, self.user_content.content)
    }

    /// Get the system prompt content (for API serialization).
    pub fn system_content(&self) -> &str {
        &self.system_prompt.content
    }

    /// Get the user content (for API serialization).
    pub fn user_content_str(&self) -> &str {
        &self.user_content.content
    }

    /// Get the source label for the user content.
    pub fn user_source(&self) -> SourceLabel {
        self.user_content.label
    }

    /// Check if the envelope has been tampered with.
    pub fn verify_integrity(&self) -> Result<(), PromptError> {
        self.system_prompt.verify_provenance()?;
        self.user_content.verify_provenance()?;
        Ok(())
    }
}

// ===========================================================================
// Helper functions
// ===========================================================================

/// Compute SHA-256 provenance hash for content.
pub fn compute_provenance(label: SourceLabel, content: &str) -> String {
    let source_name = label.name();
    let input = format!("{}:{}", source_name, content);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}
