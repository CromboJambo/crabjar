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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
