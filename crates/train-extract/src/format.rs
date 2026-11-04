use crate::extract_types::{Annotation, Chunk, ExtractedData, KnowledgeEntry, LogEvent};
use serde::{Deserialize, Serialize};

use crate::format_impl;

/// A single instruction-tuning sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub instruction: String,
    pub response: String,
    pub source: SampleSource,
    pub weight: f64,
    pub tags: Vec<String>,
    pub provenance_id: String,
    /// Unix timestamp (seconds) of the source data; 0 = unknown.
    #[serde(default)]
    pub created_at: i64,
}

/// Where a sample came from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SampleSource {
    KnowledgeEntry { entry_id: i64, kind: String },
    LogEvent { event_id: String, source: String },
    Chunk { chunk_id: String, event_id: String },
    Annotation { doc_name: String, annotation_id: String, annotation_kind: String },
}

impl SampleSource {
    pub fn to_provenance(&self) -> String {
        match self {
            SampleSource::KnowledgeEntry { entry_id, .. } => format!("knowledge:{entry_id}"),
            SampleSource::LogEvent { event_id, .. } => format!("event:{event_id}"),
            SampleSource::Chunk { chunk_id, .. } => format!("chunk:{chunk_id}"),
            SampleSource::Annotation { doc_name, annotation_id, .. } => {
                format!("annotation:{doc_name}:{annotation_id}")
            }
        }
    }
}

/// Format extracted data into instruction-tuning samples.
pub fn format_samples(data: &ExtractedData) -> Vec<Sample> {
    let mut samples = Vec::new();
    for entry in &data.knowledge_entries {
        samples.extend(format_impl::format_knowledge_entry(entry));
    }
    for event in &data.events {
        samples.extend(format_impl::format_log_event(event));
    }
    for chunk in &data.chunks {
        samples.extend(format_impl::format_chunk(chunk));
    }
    for annotation in &data.annotations {
        samples.extend(format_impl::format_annotation(annotation));
    }
    samples
}

/// Apply weighting to samples based on per-sample confidence decay and tag frequency.
pub fn apply_weighting(
    samples: Vec<Sample>,
    decay_half_life: f64,
    tag_boost: &std::collections::HashMap<String, f64>,
) -> Vec<Sample> {
    let now = chrono::Utc::now().timestamp() as f64;
    samples
        .into_iter()
        .map(|mut sample| {
            let decayed = if sample.created_at > 0 && decay_half_life > 0.0 {
                let age = (now - sample.created_at as f64).max(0.0);
                sample.weight * 2.0f64.powf(-age / decay_half_life)
            } else {
                sample.weight
            };
            let mut tag_multiplier = 1.0;
            for tag in &sample.tags {
                if let Some(boost) = tag_boost.get(tag) {
                    tag_multiplier *= boost;
                }
            }
            sample.weight = (decayed * tag_multiplier).clamp(0.01, 1.0);
            sample
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_types::{Annotation, Chunk, ExtractedData, KnowledgeEntry, LogEvent};
    use agent_context::KnowledgeKind;

    fn make_sample_data() -> ExtractedData {
        let ts = chrono::Utc::now().timestamp();
        ExtractedData {
            knowledge_entries: vec![
                KnowledgeEntry {
                    id: 1,
                    content: "Always use thiserror for library errors.\n\nNever use unwrap in library code.".to_string(),
                    kind: KnowledgeKind::Instruction,
                    tags: vec!["rust".to_string(), "error-handling".to_string()],
                    metadata: serde_json::json!({}),
                    weight: 0.9,
                    source: "user".to_string(),
                    source_type: "user".to_string(),
                    source_id: "test-1".to_string(),
                    provenance_id: "prov-1".to_string(),
                    active: true,
                    created_at: ts,
                },
            ],
            events: vec![LogEvent {
                id: "evt-1".to_string(),
                content: "Updated Cargo.toml".to_string(),
                source: "file".to_string(),
                meta: None,
                timestamp: 1000000,
            }],
            annotations: vec![],
            chunks: vec![],
            sample_count: 2,
        }
    }

    #[test]
    fn format_samples_returns_samples_for_knowledge_entries() {
        let data = make_sample_data();
        let samples = format_samples(&data);
        assert!(!samples.is_empty());
        assert!(samples.iter().any(|s| s.provenance_id.starts_with("knowledge:")));
    }

    #[test]
    fn format_samples_includes_events() {
        let data = make_sample_data();
        let samples = format_samples(&data);
        let event_samples: Vec<_> = samples
            .iter()
            .filter(|s| matches!(s.source, SampleSource::LogEvent { .. }))
            .collect();
        assert_eq!(event_samples.len(), 1);
    }

    #[test]
    fn format_empty_data_returns_empty() {
        let data = ExtractedData::default();
        let samples = format_samples(&data);
        assert!(samples.is_empty());
    }

    #[test]
    fn format_knowledge_entry_creates_instruction_response() {
        let entry = KnowledgeEntry {
            id: 1,
            content: "Test content here.".to_string(),
            kind: KnowledgeKind::Pattern,
            tags: vec!["test".to_string()],
            metadata: serde_json::json!({}),
            weight: 1.0,
            source: "user".to_string(),
            source_type: "user".to_string(),
            source_id: "test".to_string(),
            provenance_id: "prov-1".to_string(),
            active: true,
            created_at: 1000000,
        };
        let samples = format_impl::format_knowledge_entry(&entry);
        assert_eq!(samples.len(), 1);
        assert!(samples[0].instruction.contains("pattern"));
        assert!(samples[0].response.contains("Test content"));
    }

    #[test]
    fn format_log_event_creates_sample() {
        let event = LogEvent {
            id: "evt-1".to_string(),
            content: "Something happened.".to_string(),
            source: "file".to_string(),
            meta: None,
            timestamp: 1000000,
        };
        let samples = format_impl::format_log_event(&event);
        assert_eq!(samples.len(), 1);
        assert!(matches!(samples[0].source, SampleSource::LogEvent { .. }));
    }

    #[test]
    fn format_log_event_skips_empty_content() {
        let event = LogEvent {
            id: "evt-1".to_string(),
            content: "   ".to_string(),
            source: "file".to_string(),
            meta: None,
            timestamp: 1000000,
        };
        let samples = format_impl::format_log_event(&event);
        assert!(samples.is_empty());
    }

    #[test]
    fn format_annotation_includes_resolved_only() {
        let resolved = Annotation {
            id: "ann-1".to_string(),
            doc_name: "test.md".to_string(),
            kind: "note".to_string(),
            message: "Test annotation".to_string(),
            status: "resolved".to_string(),
            resolution_reason: Some("Fixed the issue".to_string()),
            confidence: 0.85,
            created_at: 1000000,
        };
        let samples = format_impl::format_annotation(&resolved);
        assert_eq!(samples.len(), 1);

        let open = Annotation {
            id: "ann-2".to_string(),
            doc_name: "test.md".to_string(),
            kind: "note".to_string(),
            message: "Open annotation".to_string(),
            status: "open".to_string(),
            resolution_reason: None,
            confidence: 0.5,
            created_at: 1000000,
        };
        let samples = format_impl::format_annotation(&open);
        assert!(samples.is_empty());
    }

    #[test]
    fn apply_weighting_reduces_weight_with_decay() {
        let day_ago = chrono::Utc::now().timestamp() - 86400;
        let samples = vec![Sample {
            instruction: "test".to_string(),
            response: "test".to_string(),
            source: SampleSource::KnowledgeEntry { entry_id: 1, kind: "Pattern".to_string() },
            weight: 1.0,
            tags: vec!["test".to_string()],
            provenance_id: "test".to_string(),
            created_at: day_ago,
        }];
        let decayed = apply_weighting(samples, 86400.0, &std::collections::HashMap::new());
        assert!(decayed[0].weight < 1.0);
    }

    #[test]
    fn sample_source_provenance_formats_correctly() {
        let ke = SampleSource::KnowledgeEntry { entry_id: 42, kind: "Pattern".to_string() };
        assert_eq!(ke.to_provenance(), "knowledge:42");
    }
}
