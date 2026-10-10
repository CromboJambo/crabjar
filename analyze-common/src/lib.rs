//! Shared types for crabjar analysis crates.
//!
//! Every analyzer crate (analyze-native, analyze-net, etc.) depends on this
//! crate for its evidence model, error types, and output formatting helpers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use thiserror::Error;

/// Version of the analyze-common crate itself.
pub const ANALYZE_COMMON_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// Evidence model
// ---------------------------------------------------------------------------

/// Provenance metadata for an analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisProvenance {
    /// Name of the analyzer tool that produced this result (e.g. "analyze-native").
    pub tool: String,
    /// Version string of the analyzer tool.
    pub version: String,
    /// ISO-8601 timestamp when analysis was performed.
    pub analyzed_at: DateTime<Utc>,
}

impl AnalysisProvenance {
    pub fn new(tool: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            version: version.into(),
            analyzed_at: Utc::now(),
        }
    }
}

/// The crabjar `doubt` block — captures epistemic limitations of the analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubtBlock {
    /// Assumptions made during analysis that could invalidate results if wrong.
    pub assumptions: Vec<String>,
    /// Known blind spots — things not checked or unable to be determined.
    pub blind_spots: Vec<String>,
    /// ISO-8601 timestamp when this doubt block was last validated.
    pub last_validation: DateTime<Utc>,
    /// Duration after which results should be considered stale (e.g. "7d", "24h").
    pub stale_after: String,
}

impl DoubtBlock {
    pub fn new(stale_after: impl Into<String>) -> Self {
        Self {
            assumptions: Vec::new(),
            blind_spots: Vec::new(),
            last_validation: Utc::now(),
            stale_after: stale_after.into(),
        }
    }

    pub fn assume(mut self, assumption: impl Into<String>) -> Self {
        self.assumptions.push(assumption.into());
        self
    }

    pub fn blind_spot(mut self, spot: impl Into<String>) -> Self {
        self.blind_spots.push(spot.into());
        self
    }
}

/// The full evidence envelope for an analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult<T> {
    /// Unique identifier for this analysis run.
    pub id: String,
    /// Provenance metadata.
    pub provenance: AnalysisProvenance,
    /// The actual analysis data (analyzer-specific).
    pub data: T,
    /// Doubt block capturing limitations and assumptions.
    pub doubt: DoubtBlock,
}

impl<T: Serialize> AnalysisResult<T> {
    pub fn new(
        id: impl Into<String>,
        provenance: AnalysisProvenance,
        data: T,
        doubt: DoubtBlock,
    ) -> Self {
        Self {
            id: id.into(),
            provenance,
            data,
            doubt,
        }
    }

    /// Serialize to pretty JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

// ---------------------------------------------------------------------------
// Error type hierarchy
// ---------------------------------------------------------------------------

/// Top-level error for all analyzer crates.
#[derive(Error, Debug)]
pub enum AnalyzeError {
    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("invalid file format: {0}")]
    InvalidFormat(String),

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Convenience Result type for analyzer crates.
pub type AnalyzeResult<T> = Result<AnalysisResult<T>, AnalyzeError>;

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

/// Print an analysis result as structured JSON to stdout (crabjar CLI contract).
pub fn print_result<T: Serialize>(result: &AnalysisResult<T>) -> std::io::Result<()> {
    let output = serde_json::json!({
        "success": true,
        "data": result,
    });
    writeln!(std::io::stdout(), "{}", serde_json::to_string_pretty(&output)?)?;
    Ok(())
}

/// Print an analysis error as structured JSON to stdout (crabjar CLI contract).
pub fn print_error(err: &AnalyzeError) -> std::io::Result<()> {
    let output = serde_json::json!({
        "success": false,
        "error": err.to_string(),
    });
    writeln!(std::io::stdout(), "{}", serde_json::to_string_pretty(&output)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubt_block_serializes() {
        let doubt = DoubtBlock::new("7d")
            .assume("file is a valid ELF binary")
            .blind_spot("dynamic linking not resolved");

        let json = serde_json::to_string(&doubt).unwrap();
        assert!(json.contains("assumptions"));
        assert!(json.contains("blind_spots"));
        assert!(json.contains("stale_after"));
    }

    #[test]
    fn analysis_result_roundtrip() {
        let provenance = AnalysisProvenance::new("analyze-native", "0.1.0");
        let result = AnalysisResult::new(
            "test-123",
            provenance,
            vec![1, 2, 3],
            DoubtBlock::new("24h"),
        );

        let json = result.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["id"], "test-123");
        assert_eq!(parsed["provenance"]["tool"], "analyze-native");
    }
}
