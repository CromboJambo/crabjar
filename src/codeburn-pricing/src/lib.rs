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
        Self { pricing_data: Vec::new() }
    }

    pub fn default() -> Self {
        Self::new()
    }

    pub fn built_in_aliases() -> Vec<String> {
        vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string(), "claude-3".to_string()]
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
        let mut by_activity = serde_json::Map::new();
        let mut by_tool = serde_json::Map::new();
        let mut by_mcp = serde_json::Map::new();
        let mut by_shell = serde_json::Map::new();
        let mut top_sessions = serde_json::Map::new();

        let prices = self.pricing_data.iter().map(|p| (p.model.clone(), p.clone())).collect::<std::collections::HashMap<String, PricingEntry>>();

        for session in classified {
            input_tokens += session.input_tokens;
            output_tokens += session.output_tokens;

            let input_price = prices.get(&session.model).map(|p| p.input_price).unwrap_or(0.0);
            let output_price = prices.get(&session.model).map(|p| p.output_price).unwrap_or(0.0);
            total_cost += (session.input_tokens as f64 * input_price) + (session.output_tokens as f64 * output_price);

            let mut session_cost = serde_json::Map::new();
            session_cost.insert("input_tokens".to_string(), json!(session.input_tokens));
            session_cost.insert("output_tokens".to_string(), json!(session.output_tokens));
            session_cost.insert("cost".to_string(), json!((session.input_tokens as f64 * input_price) + (session.output_tokens as f64 * output_price)));
            by_model.entry(session.model.clone()).or_insert(json!(0.0));
            by_model.insert(session.model.clone(), json!(by_model[&session.model].as_f64().unwrap() + (session.input_tokens as f64 * input_price) + (session.output_tokens as f64 * output_price)));

            by_project.entry(session.project.clone()).or_insert(json!(0.0));
            by_project.insert(session.project.clone(), json!(by_project[&session.project].as_f64().unwrap() + (session.input_tokens as f64 * input_price) + (session.output_tokens as f64 * output_price)));

            daily.entry(session.date.clone()).or_insert(json!(0.0));
            daily.insert(session.date.clone(), json!(daily[&session.date].as_f64().unwrap() + (session.input_tokens as f64 * input_price) + (session.output_tokens as f64 * output_price)));
        }

        let efficiency = if input_tokens > 0 { total_cost / (input_tokens as f64) } else { 0.0 };
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
