use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    Instruction,
    Pattern,
    Example,
    Context,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    User,
    Agent,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub content: String,
    pub kind: KnowledgeKind,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub weight: f64,
    pub source: Source,
    pub source_type: String,
    pub source_id: String,
    pub provenance_id: String,
}

impl KnowledgeEntry {
    pub fn new(content: impl Into<String>, kind: KnowledgeKind) -> Self {
        Self {
            content: content.into(),
            kind,
            tags: Vec::new(),
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            weight: 1.0,
            source: Source::User,
            source_type: String::new(),
            source_id: String::new(),
            provenance_id: String::new(),
        }
    }

    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    pub fn weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    pub fn meta<V: serde::Serialize>(mut self, key: impl Into<String>, value: V) -> Self {
        if let Ok(val) = serde_json::to_value(value)
            && let Some(obj) = self.metadata.as_object_mut()
        {
            obj.insert(key.into(), val);
        }
        self
    }

    pub fn stale(mut self, after: chrono::DateTime<Utc>) -> Self {
        if let Ok(val) = serde_json::to_value(after)
            && let Some(obj) = self.metadata.as_object_mut()
        {
            obj.insert("stale_after".into(), val);
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRow {
    pub id: i64,
    pub content: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRow {
    pub id: i64,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventKind {
    pub kind: String,
    pub target_id: Option<i64>,
    pub payload: Option<serde_json::Value>,
    pub source: String,
    pub ts: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EventType {
    Insert,
    Deactivate,
    Query,
    Promote,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knowledge_kind_serde() {
        let kind = KnowledgeKind::Instruction;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"instruction\"");
        let de: KnowledgeKind = serde_json::from_str(&json).unwrap();
        assert_eq!(de, KnowledgeKind::Instruction);
    }

    #[test]
    fn knowledge_kind_pattern_serde() {
        let kind = KnowledgeKind::Pattern;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"pattern\"");
    }

    #[test]
    fn knowledge_kind_example_serde() {
        let kind = KnowledgeKind::Example;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"example\"");
    }

    #[test]
    fn knowledge_kind_context_serde() {
        let kind = KnowledgeKind::Context;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"context\"");
    }

    #[test]
    fn source_serde_user() {
        let source = Source::User;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn source_serde_agent() {
        let source = Source::Agent;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"agent\"");
    }

    #[test]
    fn source_serde_system() {
        let source = Source::System;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"system\"");
    }

    #[test]
    fn knowledge_entry_new_defaults() {
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Pattern);
        assert_eq!(entry.content, "test");
        assert_eq!(entry.kind, KnowledgeKind::Pattern);
        assert!(entry.tags.is_empty());
        assert_eq!(entry.weight, 1.0);
        assert_eq!(entry.source, Source::User);
    }

    #[test]
    fn knowledge_entry_tags() {
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Pattern).tags(["rust", "pattern"]);
        assert_eq!(entry.tags, vec!["rust", "pattern"]);
    }

    #[test]
    fn knowledge_entry_weight() {
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Pattern).weight(2.5);
        assert_eq!(entry.weight, 2.5);
    }

    #[test]
    fn knowledge_entry_meta() {
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Pattern).meta("key", "value");
        assert_eq!(entry.metadata["key"], "value");
    }

    #[test]
    fn knowledge_entry_stale() {
        let dt = Utc::now();
        let entry = KnowledgeEntry::new("test", KnowledgeKind::Pattern).stale(dt);
        let stale = entry.metadata["stale_after"].as_str().unwrap();
        assert!(stale.contains(&Utc::now().format("%Y-%m-%d").to_string()));
    }

    #[test]
    fn knowledge_row_clone() {
        let row = KnowledgeRow {
            id: 1,
            content: "test".to_string(),
            tags: vec!["rust".to_string()],
            metadata: serde_json::json!({}),
            active: true,
        };
        let cloned = row.clone();
        assert_eq!(row.id, cloned.id);
        assert_eq!(row.content, cloned.content);
    }

    #[test]
    fn event_kind_clone() {
        let kind = EventKind {
            kind: "test".to_string(),
            target_id: Some(1),
            payload: Some(serde_json::json!({"a": 1})),
            source: "user".to_string(),
            ts: "2026-01-01T00:00:00Z".to_string(),
        };
        let cloned = kind.clone();
        assert_eq!(kind.kind, cloned.kind);
    }

    #[test]
    fn event_type_serde_insert() {
        let json = serde_json::to_string(&EventType::Insert).unwrap();
        assert_eq!(json, "\"Insert\"");
    }

    #[test]
    fn event_type_serde_deactivate() {
        let json = serde_json::to_string(&EventType::Deactivate).unwrap();
        assert_eq!(json, "\"Deactivate\"");
    }

    #[test]
    fn event_type_serde_query() {
        let json = serde_json::to_string(&EventType::Query).unwrap();
        assert_eq!(json, "\"Query\"");
    }

    #[test]
    fn event_type_serde_promote() {
        let json = serde_json::to_string(&EventType::Promote).unwrap();
        assert_eq!(json, "\"Promote\"");
    }
}
