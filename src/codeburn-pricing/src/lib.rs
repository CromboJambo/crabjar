use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PricingError {
    #[error("pricing data not available: {0}")]
    DataUnavailable(String),
    #[error("pricing parse error: {0}")]
    ParseError(String),
}

pub type PricingResult<T> = Result<T, PricingError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingEngine {
    pub pricing_data: Vec<PricingEntry>,
}

impl PricingEngine {
    pub fn new() -> Self {
        Self {
            pricing_data: Vec::new(),
        }
    }

    pub fn built_in_aliases() -> Vec<String> {
        vec![
            "gpt-4".to_string(),
            "gpt-3.5-turbo".to_string(),
            "claude-3".to_string(),
        ]
    }

    pub async fn calculate(
        &self,
        classified: &[codeburn_provider::SessionData],
        currency: Option<&str>,
    ) -> Result<PricingMetrics, PricingError> {
        let mut total_cost = 0.0;
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut by_project = serde_json::Map::new();
        let mut by_model = serde_json::Map::new();
        let mut daily = serde_json::Map::new();
        let by_activity = serde_json::Map::new();
        let by_tool = serde_json::Map::new();
        let by_mcp = serde_json::Map::new();
        let by_shell = serde_json::Map::new();
        let top_sessions = serde_json::Map::new();

        let prices = self
            .pricing_data
            .iter()
            .map(|p| (p.model.clone(), p.clone()))
            .collect::<std::collections::HashMap<String, PricingEntry>>();

        for session in classified {
            input_tokens += session.input_tokens;
            output_tokens += session.output_tokens;

            let input_price = prices
                .get(&session.model)
                .map(|p| p.input_price)
                .unwrap_or(0.0);
            let output_price = prices
                .get(&session.model)
                .map(|p| p.output_price)
                .unwrap_or(0.0);
            total_cost += (session.input_tokens as f64 * input_price)
                + (session.output_tokens as f64 * output_price);

            let mut session_cost = serde_json::Map::new();
            session_cost.insert("input_tokens".to_string(), json!(session.input_tokens));
            session_cost.insert("output_tokens".to_string(), json!(session.output_tokens));
            session_cost.insert(
                "cost".to_string(),
                json!(
                    (session.input_tokens as f64 * input_price)
                        + (session.output_tokens as f64 * output_price)
                ),
            );
            by_model.entry(session.model.clone()).or_insert(json!(0.0));
            by_model.insert(
                session.model.clone(),
                json!(
                    by_model[&session.model].as_f64().unwrap()
                        + (session.input_tokens as f64 * input_price)
                        + (session.output_tokens as f64 * output_price)
                ),
            );

            by_project
                .entry(session.project.clone())
                .or_insert(json!(0.0));
            by_project.insert(
                session.project.clone(),
                json!(
                    by_project[&session.project].as_f64().unwrap()
                        + (session.input_tokens as f64 * input_price)
                        + (session.output_tokens as f64 * output_price)
                ),
            );

            daily.entry(session.date.clone()).or_insert(json!(0.0));
            daily.insert(
                session.date.clone(),
                json!(
                    daily[&session.date].as_f64().unwrap()
                        + (session.input_tokens as f64 * input_price)
                        + (session.output_tokens as f64 * output_price)
                ),
            );
        }

        let efficiency = if input_tokens > 0 {
            total_cost / (input_tokens as f64)
        } else {
            0.0
        };
        let currency_str = currency.unwrap_or("USD");

        Ok(PricingMetrics {
            total_cost,
            input_tokens,
            output_tokens,
            efficiency,
            style: currency_str.to_string(),
            daily,
            by_project,
            by_model,
            by_activity,
            by_tool,
            by_mcp,
            by_shell,
            top_sessions,
        })
    }
}

impl Default for PricingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingEntry {
    pub model: String,
    pub input_price: f64,
    pub output_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingMetrics {
    pub total_cost: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub efficiency: f64,
    pub style: String,
    pub daily: serde_json::Map<String, serde_json::Value>,
    pub by_project: serde_json::Map<String, serde_json::Value>,
    pub by_model: serde_json::Map<String, serde_json::Value>,
    pub by_activity: serde_json::Map<String, serde_json::Value>,
    pub by_tool: serde_json::Map<String, serde_json::Value>,
    pub by_mcp: serde_json::Map<String, serde_json::Value>,
    pub by_shell: serde_json::Map<String, serde_json::Value>,
    pub top_sessions: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> PricingEngine {
        PricingEngine {
            pricing_data: vec![
                PricingEntry {
                    model: "gpt-4".to_string(),
                    input_price: 0.03,
                    output_price: 0.06,
                },
                PricingEntry {
                    model: "gpt-3.5-turbo".to_string(),
                    input_price: 0.0015,
                    output_price: 0.002,
                },
                PricingEntry {
                    model: "claude-3".to_string(),
                    input_price: 0.008,
                    output_price: 0.024,
                },
            ],
        }
    }

    fn make_session(
        model: &str,
        input: u64,
        output: u64,
        project: &str,
        date: &str,
    ) -> codeburn_provider::SessionData {
        codeburn_provider::SessionData {
            provider_name: "test".into(),
            provider: "test".into(),
            format: codeburn_provider::DataFormat::Jsonl,
            model: model.to_string(),
            date: date.to_string(),
            input_tokens: input,
            output_tokens: output,
            task_category: "test".into(),
            project: project.to_string(),
            message_id: "msg-1".into(),
            provenance: codeburn_provider::ProvenanceEntry {
                source: "test".into(),
                provenance_id: "p1".into(),
                provider_id: "id1".into(),
                data_path: "p".into(),
                format: "f".into(),
                ingestion_timestamp: 0,
            },
        }
    }

    #[test]
    fn pricing_engine_new_creates_empty() {
        let engine = PricingEngine::new();
        assert!(engine.pricing_data.is_empty());
    }

    #[test]
    fn pricing_engine_default_creates_empty() {
        let engine: PricingEngine = Default::default();
        assert!(engine.pricing_data.is_empty());
    }

    #[test]
    fn built_in_aliases_returns_expected() {
        let aliases = PricingEngine::built_in_aliases();
        assert!(aliases.contains(&"gpt-4".to_string()));
        assert!(aliases.contains(&"gpt-3.5-turbo".to_string()));
        assert!(aliases.contains(&"claude-3".to_string()));
    }

    #[test]
    fn calculate_empty_sessions_returns_zero_cost() {
        let engine = make_engine();
        let result =
            futures::executor::block_on(async { engine.calculate(&[], Some("USD")).await })
                .unwrap();
        assert_eq!(result.total_cost, 0.0);
        assert_eq!(result.input_tokens, 0);
        assert_eq!(result.output_tokens, 0);
        assert_eq!(result.efficiency, 0.0);
    }

    #[test]
    fn calculate_single_session_gpt4() {
        let engine = make_engine();
        let session = make_session("gpt-4", 1000, 500, "proj-a", "2026-05-24");
        let result =
            futures::executor::block_on(async { engine.calculate(&[session], Some("USD")).await })
                .unwrap();
        assert_eq!(result.input_tokens, 1000);
        assert_eq!(result.output_tokens, 500);
        // 1000 * 0.03 + 500 * 0.06 = 30 + 30 = 60
        assert!((result.total_cost - 60.0).abs() < f64::EPSILON);
        assert_eq!(result.style, "USD");
    }

    #[test]
    fn calculate_single_session_gpt35() {
        let engine = make_engine();
        let session = make_session("gpt-3.5-turbo", 5000, 1000, "proj-b", "2026-05-24");
        let result =
            futures::executor::block_on(async { engine.calculate(&[session], Some("EUR")).await })
                .unwrap();
        assert_eq!(result.input_tokens, 5000);
        assert_eq!(result.output_tokens, 1000);
        // 5000 * 0.0015 + 1000 * 0.002 = 7.5 + 2 = 9.5
        assert!((result.total_cost - 9.5).abs() < f64::EPSILON);
        assert_eq!(result.style, "EUR");
    }

    #[test]
    fn calculate_unknown_model_uses_zero_price() {
        let engine = make_engine();
        let session = make_session("unknown-model", 1000, 500, "proj-a", "2026-05-24");
        let result =
            futures::executor::block_on(async { engine.calculate(&[session], Some("USD")).await })
                .unwrap();
        assert_eq!(result.total_cost, 0.0);
        assert_eq!(result.input_tokens, 1000);
        assert_eq!(result.output_tokens, 500);
    }

    #[test]
    fn calculate_multiple_sessions_aggregates() {
        let engine = make_engine();
        let sessions = vec![
            make_session("gpt-4", 1000, 500, "proj-a", "2026-05-24"),
            make_session("claude-3", 2000, 1000, "proj-b", "2026-05-24"),
        ];
        let result =
            futures::executor::block_on(async { engine.calculate(&sessions, Some("USD")).await })
                .unwrap();
        assert_eq!(result.input_tokens, 3000);
        assert_eq!(result.output_tokens, 1500);
        // gpt-4: 1000*0.03 + 500*0.06 = 60
        // claude-3: 2000*0.008 + 1000*0.024 = 16 + 24 = 40
        assert!((result.total_cost - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn calculate_daily_breakdown() {
        let engine = make_engine();
        let sessions = vec![
            make_session("gpt-4", 1000, 500, "proj-a", "2026-05-24"),
            make_session("gpt-4", 2000, 1000, "proj-a", "2026-05-25"),
        ];
        let result =
            futures::executor::block_on(async { engine.calculate(&sessions, Some("USD")).await })
                .unwrap();
        assert_eq!(result.daily.len(), 2);
        assert!(result.daily.contains_key("2026-05-24"));
        assert!(result.daily.contains_key("2026-05-25"));
    }

    #[test]
    fn calculate_by_project_breakdown() {
        let engine = make_engine();
        let sessions = vec![
            make_session("gpt-4", 1000, 500, "proj-a", "2026-05-24"),
            make_session("claude-3", 2000, 1000, "proj-b", "2026-05-24"),
        ];
        let result =
            futures::executor::block_on(async { engine.calculate(&sessions, Some("USD")).await })
                .unwrap();
        assert_eq!(result.by_project.len(), 2);
        assert!(result.by_project.contains_key("proj-a"));
        assert!(result.by_project.contains_key("proj-b"));
    }

    #[test]
    fn calculate_by_model_breakdown() {
        let engine = make_engine();
        let sessions = vec![
            make_session("gpt-4", 1000, 500, "proj-a", "2026-05-24"),
            make_session("claude-3", 2000, 1000, "proj-a", "2026-05-24"),
        ];
        let result =
            futures::executor::block_on(async { engine.calculate(&sessions, Some("USD")).await })
                .unwrap();
        assert_eq!(result.by_model.len(), 2);
        assert!(result.by_model.contains_key("gpt-4"));
        assert!(result.by_model.contains_key("claude-3"));
    }

    #[test]
    fn calculate_efficiency_with_zero_input() {
        let engine = make_engine();
        let session = make_session("gpt-4", 0, 500, "proj-a", "2026-05-24");
        let result =
            futures::executor::block_on(async { engine.calculate(&[session], Some("USD")).await })
                .unwrap();
        assert_eq!(result.efficiency, 0.0);
    }

    #[test]
    fn calculate_efficiency_with_nonzero_input() {
        let engine = make_engine();
        let session = make_session("gpt-4", 1000, 500, "proj-a", "2026-05-24");
        let result =
            futures::executor::block_on(async { engine.calculate(&[session], Some("USD")).await })
                .unwrap();
        // efficiency = total_cost / input_tokens = 60.0 / 1000 = 0.06
        assert!((result.efficiency - 0.06).abs() < f64::EPSILON);
    }

    #[test]
    fn pricing_entry_serde_roundtrip() {
        let entry = PricingEntry {
            model: "gpt-4".to_string(),
            input_price: 0.03,
            output_price: 0.06,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let restored: PricingEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.model, entry.model);
        assert!((restored.input_price - entry.input_price).abs() < f64::EPSILON);
        assert!((restored.output_price - entry.output_price).abs() < f64::EPSILON);
    }

    #[test]
    fn pricing_metrics_serde_roundtrip() {
        let mut daily = serde_json::Map::new();
        daily.insert("2026-05-24".to_string(), serde_json::json!(60.0));
        let metrics = PricingMetrics {
            total_cost: 60.0,
            input_tokens: 1000,
            output_tokens: 500,
            efficiency: 0.06,
            style: "USD".to_string(),
            daily,
            by_project: serde_json::Map::new(),
            by_model: serde_json::Map::new(),
            by_activity: serde_json::Map::new(),
            by_tool: serde_json::Map::new(),
            by_mcp: serde_json::Map::new(),
            by_shell: serde_json::Map::new(),
            top_sessions: serde_json::Map::new(),
        };
        let json = serde_json::to_string(&metrics).unwrap();
        let restored: PricingMetrics = serde_json::from_str(&json).unwrap();
        assert!((restored.total_cost - metrics.total_cost).abs() < f64::EPSILON);
        assert_eq!(restored.input_tokens, metrics.input_tokens);
        assert_eq!(restored.output_tokens, metrics.output_tokens);
    }

    #[test]
    fn pricing_error_data_unavailable() {
        let err = PricingError::DataUnavailable("no data".to_string());
        assert!(err.to_string().contains("no data"));
    }

    #[test]
    fn pricing_error_parse_error() {
        let err = PricingError::ParseError("bad format".to_string());
        assert!(err.to_string().contains("bad format"));
    }

    #[test]
    fn pricing_error_debug_format() {
        let err = PricingError::DataUnavailable("missing".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("DataUnavailable"));
    }

    #[test]
    fn calculate_default_currency_when_none() {
        let engine = make_engine();
        let session = make_session("gpt-4", 1000, 500, "proj-a", "2026-05-24");
        let result =
            futures::executor::block_on(async { engine.calculate(&[session], None).await })
                .unwrap();
        assert_eq!(result.style, "USD");
    }
}
