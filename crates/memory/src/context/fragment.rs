// crabjar/memory/src/context/fragment.rs
// ContextFragment and ContextFragmentBuilder — bounded context items.

use serde::{Deserialize, Serialize};

/// A bounded context fragment — a single item injected into model context.
///
/// Every fragment carries a token count (not byte count) and is validated
/// against the hard cap (`MAX_TOKENS_PER_FRAGMENT`) at construction time.
/// Fragments approaching the P0 threshold (`P0_ALERT_TOKENS`) are flagged
/// so that manual review can be triggered.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextFragment {
    /// Unique identifier for this fragment (e.g., "doc:crabjar-state.md:Section 2.1").
    pub id: String,
    /// Human-readable label for display/logging.
    pub label: String,
    /// Raw content of the fragment.
    pub content: String,
    /// Token count of the content. Computed at construction time.
    pub token_count: usize,
    /// Whether this fragment is flagged for P0 alert (token_count > 1K).
    pub p0_flagged: bool,
    /// Optional metadata (source document, section, confidence, etc.).
    pub metadata: serde_json::Value,
}

impl ContextFragment {
    /// Create a new context fragment, validating token count against the
    /// hard cap.
    ///
    /// # Errors
    /// - `ContextError::ExceedsMaxTokens` if `token_count > MAX_TOKENS_PER_FRAGMENT`
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        content: impl Into<String>,
        token_count: usize,
    ) -> Result<Self, super::ContextError> {
        let content = content.into();

        if token_count > crate::context::constants::MAX_TOKENS_PER_FRAGMENT {
            return Err(super::ContextError::ExceedsMaxTokens {
                actual: token_count,
                max: crate::context::constants::MAX_TOKENS_PER_FRAGMENT,
            });
        }

        let p0_flagged = token_count > crate::context::constants::P0_ALERT_TOKENS;

        Ok(Self {
            id: id.into(),
            label: label.into(),
            content,
            token_count,
            p0_flagged,
            metadata: serde_json::json!({}),
        })
    }

    /// Create a fragment without token count validation.
    ///
    /// # Safety
    /// The caller must ensure `token_count <= MAX_TOKENS_PER_FRAGMENT`.
    pub fn new_unchecked(
        id: impl Into<String>,
        label: impl Into<String>,
        content: impl Into<String>,
        token_count: usize,
    ) -> Self {
        let content = content.into();
        let p0_flagged = token_count > crate::context::constants::P0_ALERT_TOKENS;
        Self {
            id: id.into(),
            label: label.into(),
            content,
            token_count,
            p0_flagged,
            metadata: serde_json::json!({}),
        }
    }

    /// Add metadata key-value pair to this fragment.
    pub fn meta(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(val) = serde_json::to_value(value)
            && let Some(obj) = self.metadata.as_object_mut()
        {
            obj.insert(key.into(), val);
        }
        self
    }

    /// Add metadata from a pre-built JSON value.
    pub fn meta_value(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        if let Some(obj) = self.metadata.as_object_mut() {
            obj.insert(key.into(), value);
        }
        self
    }

    /// Total size in bytes (not tokens). Useful for memory budgeting.
    pub fn byte_size(&self) -> usize {
        self.content.len()
    }

    /// Whether this fragment is flagged for P0 alert.
    pub fn is_p0_flagged(&self) -> bool {
        self.p0_flagged
    }

    /// Whether this fragment is within safe token bounds (< P0 threshold).
    pub fn is_safe(&self) -> bool {
        !self.p0_flagged
    }

    /// Serialize this fragment to JSON for transport/storage.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "label": self.label,
            "content": self.content,
            "token_count": self.token_count,
            "p0_flagged": self.p0_flagged,
            "metadata": self.metadata,
        })
    }
}

// ---------------------------------------------------------------------------
// ContextFragmentBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing `ContextFragment` instances with fluent API.
#[derive(Debug, Clone)]
pub struct ContextFragmentBuilder {
    id: Option<String>,
    label: Option<String>,
    content: Option<String>,
    token_count: Option<usize>,
    metadata: serde_json::Map<String, serde_json::Value>,
}

impl Default for ContextFragmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextFragmentBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            id: None,
            label: None,
            content: None,
            token_count: None,
            metadata: serde_json::Map::new(),
        }
    }

    /// Set the fragment ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the fragment label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the fragment content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set the token count.
    pub fn token_count(mut self, count: usize) -> Self {
        self.token_count = Some(count);
        self
    }

    /// Add a metadata key-value pair.
    pub fn meta(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(val) = serde_json::to_value(value) {
            self.metadata.insert(key.into(), val);
        }
        self
    }

    /// Add metadata from a pre-built JSON value.
    pub fn meta_value(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Build the fragment, validating token count.
    ///
    /// # Errors
    /// - `ContextError::ExceedsMaxTokens` if token count exceeds the hard cap.
    pub fn build(self) -> Result<ContextFragment, super::ContextError> {
        let id = self.id.unwrap_or_default();
        let label = self.label.unwrap_or_default();
        let content = self.content.unwrap_or_default();
        let token_count = self
            .token_count
            .ok_or(super::ContextError::InvalidTokenCount(0))?;

        ContextFragment::new(id, label, content, token_count).map(|mut f| {
            if !self.metadata.is_empty()
                && let Some(obj) = f.metadata.as_object_mut()
            {
                for (k, v) in self.metadata {
                    obj.insert(k, v);
                }
            }
            f
        })
    }

    /// Build without validation (caller guarantees token count is valid).
    pub fn build_unchecked(self) -> ContextFragment {
        let id = self.id.unwrap_or_default();
        let label = self.label.unwrap_or_default();
        let content = self.content.unwrap_or_default();
        let token_count = self.token_count.unwrap_or(0);

        let mut f = ContextFragment::new_unchecked(id, label, content, token_count);
        if !self.metadata.is_empty()
            && let Some(obj) = f.metadata.as_object_mut()
        {
            for (k, v) in self.metadata {
                obj.insert(k, v);
            }
        }
        f
    }
}
