//! Core types for universal structural token representation.

use crate::span_kind::SpanKind;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Universal span tree with language metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanTree {
    pub spans: Vec<Span>,
    pub source: String,
    pub language: String,
    #[serde(default)]
    pub semantic_tags: HashMap<String, String>,
}

/// Universal structural token with position info and language-specific tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralToken {
    /// The universal span kind.
    pub kind: SpanKind,
    /// Source text of this token/span.
    pub text: String,
    /// Byte offsets in the source file [start, end).
    pub range: [usize; 2],
    /// Line number (1-indexed) for display purposes.
    #[serde(default)]
    pub line: Option<u32>,
    /// Language-specific tags preserved alongside universal representation.
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

impl StructuralToken {
    pub fn new(kind: SpanKind, text: impl Into<String>, range: [usize; 2]) -> Self {
        Self {
            kind,
            text: text.into(),
            range,
            line: None,
            tags: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

/// Hierarchical span representing a nested code region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// The universal span kind.
    pub kind: SpanKind,
    /// Opening token (if any).
    #[serde(default)]
    pub open_token: Option<StructuralToken>,
    /// Child spans within this region.
    #[serde(default)]
    pub body: Vec<Span>,
    /// Closing token (if any).
    #[serde(default)]
    pub close_token: Option<StructuralToken>,
    /// Source text for embedding fallback.
    #[serde(default)]
    pub source: String,
}

impl Span {
    #[must_use]
    pub fn new(kind: SpanKind) -> Self {
        Self {
            kind,
            open_token: None,
            body: Vec::new(),
            close_token: None,
            source: String::new(),
        }
    }

    /// Get all descendant spans of a specific kind.
    #[must_use]
    pub fn find_all(&self, target_kind: SpanKind) -> Vec<&Span> {
        let mut results = Vec::new();
        if self.kind == target_kind {
            results.push(self);
        }
        for child in &self.body {
            results.extend(child.find_all(target_kind));
        }
        results
    }

    /// Get the depth of this span tree.
    #[must_use]
    pub fn depth(&self) -> usize {
        let mut max_child = 0;
        for child in &self.body {
            let child_depth = child.depth();
            if child_depth > max_child {
                max_child = child_depth;
            }
        }
        1 + max_child
    }

    /// Count total spans in this tree.
    #[must_use]
    pub fn span_count(&self) -> usize {
        let mut count = 1;
        for child in &self.body {
            count += child.span_count();
        }
        count
    }
}

/// Budget configuration for resolution control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    /// Maximum number of tokens to process.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Minimum node size in bytes before recursing.
    #[serde(default = "default_min_node_bytes")]
    pub min_node_bytes: usize,
}

fn default_max_tokens() -> usize {
    10000
}

fn default_min_node_bytes() -> usize {
    50
}

impl Budget {
    #[must_use]
    pub fn new(max_tokens: usize, min_node_bytes: usize) -> Self {
        Self {
            max_tokens,
            min_node_bytes,
        }
    }

    #[must_use]
    pub fn unlimited() -> Self {
        Self::new(usize::MAX, 0)
    }
}

impl Default for Budget {
    fn default() -> Self {
        Budget {
            max_tokens: default_max_tokens(),
            min_node_bytes: default_min_node_bytes(),
        }
    }
}
