//! Bug Bounty Hunter — AI-powered automated vulnerability discovery.
//!
//! Uses structural code analysis (pesti) combined with LLM inference to find
//! security bugs across web apps, binaries, and source code. Produces structured
//! findings with evidence, severity ratings, and reproduction steps.

pub mod analyzers;
pub mod inference;
pub mod report;
pub mod types;

use analyze_common::{AnalysisProvenance, DoubtBlock};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use thiserror::Error;

pub use self::types::*;

/// Top-level error for bug-bounty-hunter.
#[derive(Error, Debug)]
pub enum HunterError {
    #[error("target not found: {0}")]
    TargetNotFound(String),
    #[error("analysis failed: {0}")]
    AnalysisFailed(String),
    #[error("inference error: {0}")]
    InferenceError(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("url parse error: {0}")]
    UrlParse(#[from] url::ParseError),
}

/// Configuration for a bug bounty hunt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuntConfig {
    /// Target to analyze (URL, path, or repo).
    pub target: String,
    /// Target type determines which analyzers run.
    pub target_type: TargetType,
    /// Maximum severity to report (filter lower).
    pub min_severity: Severity,
    /// Timeout per analyzer in seconds.
    pub timeout_secs: u64,
    /// Whether to use LLM inference for deeper analysis.
    pub use_inference: bool,
    /// Optional model name for inference.
    pub inference_model: Option<String>,
}

impl Default for HuntConfig {
    fn default() -> Self {
        Self {
            target: String::new(),
            target_type: TargetType::Unknown,
            min_severity: Severity::Low,
            timeout_secs: 300,
            use_inference: false,
            inference_model: None,
        }
    }
}

/// The main bug bounty hunter orchestrator.
pub struct BugBountyHunter {
    config: HuntConfig,
    analyzers: HashMap<AnalyzerType, Box<dyn Analyzer>>,
}

impl BugBountyHunter {
    pub fn new(config: HuntConfig) -> Self {
        let mut analyzers = HashMap::new();
        // Register built-in analyzers
        analyzers.insert(
            AnalyzerType::StaticCode,
            Box::new(analyzers::static_code::StaticCodeAnalyzer::new()) as Box<dyn Analyzer>,
        );
        analyzers.insert(
            AnalyzerType::WebApp,
            Box::new(analyzers::web_app::WebAppAnalyzer::default()) as Box<dyn Analyzer>,
        );
        analyzers.insert(
            AnalyzerType::Binary,
            Box::new(analyzers::binary::BinaryAnalyzer::default()) as Box<dyn Analyzer>,
        );
        // PEG structural analyzer (best-effort: skip if grammars unavailable).
        match analyzers::peg_analyzer::PegStructuralAnalyzer::new() {
            Ok(peg) => {
                analyzers.insert(
                    AnalyzerType::PegStructural,
                    Box::new(peg) as Box<dyn Analyzer>,
                );
            }
            Err(e) => {
                eprintln!("[hunter] PEG structural analyzer unavailable: {}", e);
            }
        }

        Self { config, analyzers }
    }

    /// Run the full bug bounty hunt.
    pub async fn hunt(&self) -> Result<HuntReport, HunterError> {
        let start = Instant::now();

        // Determine which analyzers to run based on target type
        let analyzer_types = self.select_analyzers(&self.config.target_type);

        // Run analyzers (sequential for simplicity; can be parallelized later)
        let mut findings = Vec::new();

        for atype in analyzer_types {
            if let Some(analyzer) = self.analyzers.get(&atype) {
                match analyzer.analyze(&self.config).await {
                    Ok(result) => {
                        findings.extend(result.into_findings());
                    }
                    Err(e) => {
                        eprintln!("Analyzer {:?} failed: {}", atype, e);
                    }
                }
            }
        }

        // Filter by minimum severity
        findings.retain(|f| f.severity >= self.config.min_severity);

        // Run inference on high-value findings if enabled
        let enriched_findings = if self.config.use_inference {
            let inferencer = inference::Inferencer::new(self.config.inference_model.clone());
            inferencer.enrich(findings).await
        } else {
            findings
        };

        // Generate report
        let duration = start.elapsed();
        let report = HuntReport {
            id: format!("hunt-{}", Utc::now().timestamp_millis()),
            config: self.config.clone(),
            findings: enriched_findings,
            started_at: Utc::now() - chrono::Duration::seconds(duration.as_secs() as i64),
            duration_secs: duration.as_secs(),
            provenance: AnalysisProvenance::new("bug-bounty-hunter", env!("CARGO_PKG_VERSION")),
            doubt: DoubtBlock::new("24h")
                .assume("target was reachable during analysis")
                .blind_spot("runtime-only bugs not detected without execution"),
        };

        Ok(report)
    }

    fn select_analyzers(&self, target_type: &TargetType) -> Vec<AnalyzerType> {
        match target_type {
            TargetType::WebApp => vec![AnalyzerType::WebApp],
            TargetType::Binary => vec![AnalyzerType::Binary],
            TargetType::SourceCode => vec![AnalyzerType::StaticCode, AnalyzerType::PegStructural],
            TargetType::Unknown => vec![
                AnalyzerType::StaticCode,
                AnalyzerType::WebApp,
                AnalyzerType::Binary,
                AnalyzerType::PegStructural,
            ],
        }
    }
}

/// Convenience function to run a hunt.
pub async fn hunt(config: HuntConfig) -> Result<HuntReport, HunterError> {
    let hunter = BugBountyHunter::new(config);
    hunter.hunt().await
}
