use crate::error::{TrainExtractError, TrainExtractResult};
use agent_context::KnowledgeKind;
use rusqlite::Connection;
use std::collections::HashMap;

/// Extracted knowledge entry from the knowledge store.
#[derive(Debug, Clone)]
pub struct KnowledgeEntry {
    pub id: i64,
    pub content: String,
    pub kind: KnowledgeKind,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub weight: f64,
    pub source: String,
    pub source_type: String,
    pub source_id: String,
    pub provenance_id: String,
    pub active: bool,
    /// Unix timestamp (seconds) of when the entry was created.
    pub created_at: i64,
}

/// Extracted event from mirror-log.
#[derive(Debug, Clone)]
pub struct LogEvent {
    pub id: String,
    pub content: String,
    pub source: String,
    pub meta: Option<String>,
    pub timestamp: i64,
}

/// A chunk of a state-doc section (reserved for the chunking pipeline;
/// not populated by `extract()` yet).
#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: String,
    pub event_id: String,
    pub content: String,
    pub chunk_index: usize,
}

/// A resolved state-doc annotation (from the agent-context state-docs store).
#[derive(Debug, Clone)]
pub struct Annotation {
    pub id: String,
    pub doc_name: String,
    pub kind: String,
    pub message: String,
    pub status: String,
    /// Reason recorded when the annotation was resolved.
    pub resolution_reason: Option<String>,
    pub confidence: f64,
    pub created_at: i64,
}

/// Configuration for data extraction.
#[derive(Debug, Clone)]
pub struct ExtractConfig {
    /// Only include entries with these tags (empty = all tags).
    pub tags: Vec<String>,
    /// Maximum number of entries to extract per source.
    pub max_entries: usize,
    /// Only include events with timestamp >= this unix timestamp (0 = all).
    pub since: i64,
    /// Include mirror-log events.
    pub include_events: bool,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            max_entries: 10000,
            since: 0,
            include_events: true,
        }
    }
}

/// Combined extracted data ready for formatting.
#[derive(Debug, Default)]
pub struct ExtractedData {
    pub knowledge_entries: Vec<KnowledgeEntry>,
    pub events: Vec<LogEvent>,
    /// State-doc chunks (reserved; not populated by `extract()` yet).
    pub chunks: Vec<Chunk>,
    /// Resolved state-doc annotations (reserved; not populated by `extract()` yet).
    pub annotations: Vec<Annotation>,
    /// Number of source samples (entries + events) extracted.
    pub sample_count: usize,
}

impl ExtractedData {
    /// Returns true if no data was extracted.
    pub fn is_empty(&self) -> bool {
        self.knowledge_entries.is_empty() && self.events.is_empty()
    }

    /// Get a summary of the extracted data.
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "knowledge_entries": self.knowledge_entries.len(),
            "events": self.events.len(),
            "chunks": self.chunks.len(),
            "annotations": self.annotations.len(),
            "total_samples": self.sample_count,
        })
    }

    /// Group knowledge entries by tag for weighted sampling.
    pub fn tag_distribution(&self) -> HashMap<String, usize> {
        let mut dist = HashMap::new();
        for entry in &self.knowledge_entries {
            for tag in &entry.tags {
                *dist.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        dist
    }
}
