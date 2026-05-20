use serde::{Deserialize, Serialize};
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingEntry {
    pub model: String,
    pub input_price: f64,
    pub output_price: f64,
}
