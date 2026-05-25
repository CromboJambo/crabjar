use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("provider not found: {0}")]
    NotFound(String),
    #[error("discovery error: {0}")]
    DiscoveryError(String),
    #[error("database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataFormat {
    Jsonl,
    Sqlite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRegistry {
    pub providers: Vec<SessionData>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn discover(path: &std::path::Path) -> Result<Self, ProviderError> {
        let conn = rusqlite::Connection::open(path)?;
        let mut providers = Vec::new();
        let mut stmt = conn.prepare("SELECT provider_name, provider, format, model, date, input_tokens, output_tokens, task_category, project, message_id, provenance FROM sessions")?;
        let cursor = stmt.query_map([], |row| {
            let provenance_str: String = row.get(10)?;
            Ok(SessionData {
                provider_name: row.get(0)?,
                provider: row.get(1)?,
                format: serde_json::from_str(&row.get::<usize, String>(2)?).unwrap(),
                model: row.get(3)?,
                date: row.get(4)?,
                input_tokens: row.get(5)?,
                output_tokens: row.get(6)?,
                task_category: row.get(7)?,
                project: row.get(8)?,
                message_id: row.get(9)?,
                provenance: serde_json::from_str(&provenance_str).unwrap(),
            })
        })?;
        for session in cursor {
            providers.push(session?);
        }
        Ok(Self { providers })
    }

    pub fn read_sessions(&self) -> Result<Vec<SessionData>, ProviderError> {
        Ok(self.providers.clone())
    }

    pub fn provider_sessions(&self, name: &str) -> Result<Vec<SessionData>, ProviderError> {
        Ok(self
            .providers
            .iter()
            .filter(|s| s.provider_name == name)
            .cloned()
            .collect())
    }

    pub fn today_usage(&self) -> Result<String, ProviderError> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let total: u64 = self
            .providers
            .iter()
            .filter(|s| s.date == today)
            .map(|s| s.input_tokens + s.output_tokens)
            .sum();
        Ok(total.to_string())
    }

    pub fn today_usage_json(
        &self,
    ) -> Result<serde_json::Map<String, serde_json::Value>, ProviderError> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let mut map = serde_json::Map::new();
        map.insert("date".to_string(), json!(today));
        map.insert(
            "total_tokens".to_string(),
            json!(
                self.providers
                    .iter()
                    .filter(|s| s.date == today)
                    .map(|s| s.input_tokens + s.output_tokens)
                    .sum::<u64>()
            ),
        );
        Ok(map)
    }

    pub fn month_usage_json(
        &self,
    ) -> Result<serde_json::Map<String, serde_json::Value>, ProviderError> {
        let month = chrono::Utc::now().format("%Y-%m").to_string();
        let mut map = serde_json::Map::new();
        map.insert("month".to_string(), json!(month));
        map.insert(
            "total_tokens".to_string(),
            json!(
                self.providers
                    .iter()
                    .filter(|s| s.date.starts_with(&month))
                    .map(|s| s.input_tokens + s.output_tokens)
                    .sum::<u64>()
            ),
        );
        Ok(map)
    }

    pub fn multi_period_export_json(
        &self,
    ) -> Result<serde_json::Map<String, serde_json::Value>, ProviderError> {
        let mut map = serde_json::Map::new();
        let mut by_date = serde_json::Map::new();
        for date in self
            .providers
            .iter()
            .map(|s| s.date.clone())
            .collect::<std::collections::HashSet<String>>()
        {
            by_date.insert(
                date.clone(),
                json!(
                    self.providers
                        .iter()
                        .filter(|s| s.date == date)
                        .map(|s| s.input_tokens + s.output_tokens)
                        .sum::<u64>()
                ),
            );
        }
        map.insert("by_date".to_string(), json!(by_date));
        map.insert(
            "total_tokens".to_string(),
            json!(
                self.providers
                    .iter()
                    .map(|s| s.input_tokens + s.output_tokens)
                    .sum::<u64>()
            ),
        );
        Ok(map)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_registry_new_is_empty() {
        let registry = ProviderRegistry::new();
        assert!(registry.providers.is_empty());
    }

    #[test]
    fn provider_registry_read_sessions_empty() {
        let registry = ProviderRegistry::new();
        let sessions = registry.read_sessions().unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn provider_registry_provider_sessions_filters() {
        let mut registry = ProviderRegistry::new();
        registry.providers = vec![
            SessionData {
                provider_name: "claude".into(),
                provider: "anthropic".into(),
                format: DataFormat::Jsonl,
                model: "claude-3".into(),
                date: "2026-05-24".into(),
                input_tokens: 100,
                output_tokens: 50,
                task_category: "test".into(),
                project: "proj-a".into(),
                message_id: "msg-1".into(),
                provenance: ProvenanceEntry {
                    source: "test".into(),
                    provenance_id: "prov-1".into(),
                    provider_id: "pid-1".into(),
                    data_path: "/tmp/test.jsonl".into(),
                    format: "jsonl".into(),
                    ingestion_timestamp: 12345,
                },
            },
            SessionData {
                provider_name: "gpt-4".into(),
                provider: "openai".into(),
                format: DataFormat::Sqlite,
                model: "gpt-4".into(),
                date: "2026-05-24".into(),
                input_tokens: 200,
                output_tokens: 100,
                task_category: "fix".into(),
                project: "proj-b".into(),
                message_id: "msg-2".into(),
                provenance: ProvenanceEntry {
                    source: "test".into(),
                    provenance_id: "prov-2".into(),
                    provider_id: "pid-2".into(),
                    data_path: "/tmp/test2.jsonl".into(),
                    format: "jsonl".into(),
                    ingestion_timestamp: 12346,
                },
            },
        ];

        let claude_sessions = registry.provider_sessions("claude").unwrap();
        assert_eq!(claude_sessions.len(), 1);
        assert_eq!(claude_sessions[0].provider_name, "claude");

        let gpt_sessions = registry.provider_sessions("gpt-4").unwrap();
        assert_eq!(gpt_sessions.len(), 1);
        assert_eq!(gpt_sessions[0].provider_name, "gpt-4");

        let empty = registry.provider_sessions("nonexistent").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn provider_registry_today_usage_json() {
        let mut registry = ProviderRegistry::new();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        registry.providers = vec![SessionData {
            provider_name: "claude".into(),
            provider: "anthropic".into(),
            format: DataFormat::Jsonl,
            model: "claude-3".into(),
            date: today.clone(),
            input_tokens: 100,
            output_tokens: 50,
            task_category: "test".into(),
            project: "proj".into(),
            message_id: "msg-1".into(),
            provenance: ProvenanceEntry {
                source: "t".into(),
                provenance_id: "p1".into(),
                provider_id: "id1".into(),
                data_path: "p".into(),
                format: "f".into(),
                ingestion_timestamp: 0,
            },
        }];

        let usage = registry.today_usage_json().unwrap();
        assert_eq!(usage["date"], today);
        assert!(usage["total_tokens"].as_u64().unwrap() > 0);
    }

    #[test]
    fn provider_registry_month_usage_json() {
        let mut registry = ProviderRegistry::new();
        let month = chrono::Utc::now().format("%Y-%m").to_string();
        registry.providers = vec![SessionData {
            provider_name: "claude".into(),
            provider: "anthropic".into(),
            format: DataFormat::Jsonl,
            model: "claude-3".into(),
            date: format!("{}-15", month),
            input_tokens: 100,
            output_tokens: 50,
            task_category: "test".into(),
            project: "proj".into(),
            message_id: "msg-1".into(),
            provenance: ProvenanceEntry {
                source: "t".into(),
                provenance_id: "p1".into(),
                provider_id: "id1".into(),
                data_path: "p".into(),
                format: "f".into(),
                ingestion_timestamp: 0,
            },
        }];

        let usage = registry.month_usage_json().unwrap();
        assert_eq!(usage["month"], month);
        assert!(usage["total_tokens"].as_u64().unwrap() > 0);
    }

    #[test]
    fn provider_registry_multi_period_export() {
        let mut registry = ProviderRegistry::new();
        registry.providers = vec![
            SessionData {
                provider_name: "claude".into(),
                provider: "anthropic".into(),
                format: DataFormat::Jsonl,
                model: "claude-3".into(),
                date: "2026-05-20".into(),
                input_tokens: 100,
                output_tokens: 50,
                task_category: "test".into(),
                project: "proj".into(),
                message_id: "msg-1".into(),
                provenance: ProvenanceEntry {
                    source: "t".into(),
                    provenance_id: "p1".into(),
                    provider_id: "id1".into(),
                    data_path: "p".into(),
                    format: "f".into(),
                    ingestion_timestamp: 0,
                },
            },
            SessionData {
                provider_name: "gpt-4".into(),
                provider: "openai".into(),
                format: DataFormat::Sqlite,
                model: "gpt-4".into(),
                date: "2026-05-21".into(),
                input_tokens: 200,
                output_tokens: 100,
                task_category: "fix".into(),
                project: "proj".into(),
                message_id: "msg-2".into(),
                provenance: ProvenanceEntry {
                    source: "t".into(),
                    provenance_id: "p2".into(),
                    provider_id: "id2".into(),
                    data_path: "p2".into(),
                    format: "f".into(),
                    ingestion_timestamp: 0,
                },
            },
        ];

        let export = registry.multi_period_export_json().unwrap();
        let by_date: serde_json::Map<String, serde_json::Value> =
            export["by_date"].as_object().unwrap().clone();
        assert_eq!(by_date.len(), 2);
        assert!(by_date.contains_key("2026-05-20"));
        assert!(by_date.contains_key("2026-05-21"));
        assert!(export["total_tokens"].as_u64().unwrap() > 0);
    }

    #[test]
    fn provider_registry_today_usage_numeric() {
        let mut registry = ProviderRegistry::new();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        registry.providers = vec![SessionData {
            provider_name: "claude".into(),
            provider: "anthropic".into(),
            format: DataFormat::Jsonl,
            model: "claude-3".into(),
            date: today.clone(),
            input_tokens: 100,
            output_tokens: 50,
            task_category: "test".into(),
            project: "proj".into(),
            message_id: "msg-1".into(),
            provenance: ProvenanceEntry {
                source: "t".into(),
                provenance_id: "p1".into(),
                provider_id: "id1".into(),
                data_path: "p".into(),
                format: "f".into(),
                ingestion_timestamp: 0,
            },
        }];

        let usage = registry.today_usage().unwrap();
        assert_eq!(usage, "150");
    }

    #[test]
    fn data_format_serde_jsonl() {
        let fmt = DataFormat::Jsonl;
        let json = serde_json::to_string(&fmt).unwrap();
        assert_eq!(json, "\"Jsonl\"");
    }

    #[test]
    fn data_format_serde_sqlite() {
        let fmt = DataFormat::Sqlite;
        let json = serde_json::to_string(&fmt).unwrap();
        assert_eq!(json, "\"Sqlite\"");
    }

    #[test]
    fn data_format_serde_roundtrip_jsonl() {
        let fmt = DataFormat::Jsonl;
        let json = serde_json::to_string(&fmt).unwrap();
        let restored: DataFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, fmt);
    }

    #[test]
    fn data_format_serde_roundtrip_sqlite() {
        let fmt = DataFormat::Sqlite;
        let json = serde_json::to_string(&fmt).unwrap();
        let restored: DataFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, fmt);
    }

    #[test]
    fn session_data_clone_works() {
        let session = SessionData {
            provider_name: "claude".into(),
            provider: "anthropic".into(),
            format: DataFormat::Jsonl,
            model: "claude-3".into(),
            date: "2026-05-24".into(),
            input_tokens: 100,
            output_tokens: 50,
            task_category: "test".into(),
            project: "proj".into(),
            message_id: "msg-1".into(),
            provenance: ProvenanceEntry {
                source: "t".into(),
                provenance_id: "p1".into(),
                provider_id: "id1".into(),
                data_path: "p".into(),
                format: "f".into(),
                ingestion_timestamp: 0,
            },
        };
        let cloned = session.clone();
        assert_eq!(cloned.provider_name, session.provider_name);
    }

    #[test]
    fn provenance_entry_clone_works() {
        let entry = ProvenanceEntry {
            source: "t".into(),
            provenance_id: "p1".into(),
            provider_id: "id1".into(),
            data_path: "p".into(),
            format: "f".into(),
            ingestion_timestamp: 12345,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.provenance_id, entry.provenance_id);
    }

    #[test]
    fn provider_error_not_found_message() {
        let err = ProviderError::NotFound("model x".into());
        assert!(err.to_string().contains("model x"));
    }

    #[test]
    fn provider_error_discovery_error_message() {
        let err = ProviderError::DiscoveryError("disk full".into());
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn provider_registry_default_is_empty() {
        let registry = ProviderRegistry::default();
        assert!(registry.providers.is_empty());
    }

    #[test]
    fn provider_registry_read_sessions_returns_cloned() {
        let mut registry = ProviderRegistry::new();
        registry.providers = vec![SessionData {
            provider_name: "claude".into(),
            provider: "anthropic".into(),
            format: DataFormat::Jsonl,
            model: "claude-3".into(),
            date: "2026-05-24".into(),
            input_tokens: 100,
            output_tokens: 50,
            task_category: "test".into(),
            project: "proj".into(),
            message_id: "msg-1".into(),
            provenance: ProvenanceEntry {
                source: "t".into(),
                provenance_id: "p1".into(),
                provider_id: "id1".into(),
                data_path: "p".into(),
                format: "f".into(),
                ingestion_timestamp: 0,
            },
        }];

        let sessions = registry.read_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        // Verify it's a clone (modifying original doesn't affect returned)
        registry.providers.clear();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn provider_registry_multi_period_with_same_date() {
        let mut registry = ProviderRegistry::new();
        registry.providers = vec![
            SessionData {
                provider_name: "claude".into(),
                provider: "anthropic".into(),
                format: DataFormat::Jsonl,
                model: "claude-3".into(),
                date: "2026-05-20".into(),
                input_tokens: 100,
                output_tokens: 50,
                task_category: "test".into(),
                project: "proj".into(),
                message_id: "msg-1".into(),
                provenance: ProvenanceEntry {
                    source: "t".into(),
                    provenance_id: "p1".into(),
                    provider_id: "id1".into(),
                    data_path: "p".into(),
                    format: "f".into(),
                    ingestion_timestamp: 0,
                },
            },
            SessionData {
                provider_name: "gpt-4".into(),
                provider: "openai".into(),
                format: DataFormat::Sqlite,
                model: "gpt-4".into(),
                date: "2026-05-20".into(),
                input_tokens: 200,
                output_tokens: 100,
                task_category: "fix".into(),
                project: "proj".into(),
                message_id: "msg-2".into(),
                provenance: ProvenanceEntry {
                    source: "t".into(),
                    provenance_id: "p2".into(),
                    provider_id: "id2".into(),
                    data_path: "p2".into(),
                    format: "f".into(),
                    ingestion_timestamp: 0,
                },
            },
        ];

        let export = registry.multi_period_export_json().unwrap();
        let by_date: serde_json::Map<String, serde_json::Value> =
            export["by_date"].as_object().unwrap().clone();
        assert_eq!(by_date.len(), 1); // only one unique date
        let total_on_20: u64 = by_date["2026-05-20"].as_u64().unwrap();
        assert_eq!(total_on_20, 450); // 100+50+200+100
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub provider_name: String,
    pub provider: String,
    pub format: DataFormat,
    pub model: String,
    pub date: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub task_category: String,
    pub project: String,
    pub message_id: String,
    pub provenance: ProvenanceEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    pub source: String,
    pub provenance_id: String,
    pub provider_id: String,
    pub data_path: String,
    pub format: String,
    pub ingestion_timestamp: i64,
}
