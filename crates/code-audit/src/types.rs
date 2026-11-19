//! Bug bounty hunter types — findings, severity, evidence, and reports.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Severity levels for bug findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Low => write!(f, "LOW"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::High => write!(f, "HIGH"),
            Severity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Target types that can be analyzed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetType {
    WebApp,
    Binary,
    SourceCode,
    Unknown,
}

/// Analyzer types registered with the hunter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnalyzerType {
    StaticCode,
    WebApp,
    Binary,
    PegStructural,
}

/// Evidence attached to a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Type of evidence (e.g., "code_snippet", "http_response", "binary_offset").
    pub kind: String,
    /// Raw evidence data.
    pub data: serde_json::Value,
    /// Location within the target (file path, URL, offset).
    pub location: Option<String>,
}

/// A single bug finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique identifier for this finding.
    pub id: String,
    /// Title summarizing the bug.
    pub title: String,
    /// Detailed description of the vulnerability.
    pub description: String,
    /// Severity rating.
    pub severity: Severity,
    /// CWE identifier if applicable (e.g., "CWE-79" for XSS).
    pub cwe: Option<String>,
    /// Analyzer that discovered this finding.
    pub analyzer: AnalyzerType,
    /// Evidence supporting the finding.
    pub evidence: Vec<Evidence>,
    /// Suggested remediation steps.
    pub remediation: String,
    /// Whether LLM inference validated/enriched this finding.
    pub inference_validated: bool,
    /// Confidence score from 0.0 to 1.0 (inference-only).
    pub confidence: Option<f64>,
}

/// Full hunt report containing all findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuntReport {
    /// Unique identifier for this hunt run.
    pub id: String,
    /// Configuration used for this hunt.
    pub config: super::HuntConfig,
    /// All findings from the hunt.
    pub findings: Vec<Finding>,
    /// When the hunt started.
    pub started_at: DateTime<Utc>,
    /// Duration of the hunt in seconds.
    pub duration_secs: u64,
    /// Provenance metadata.
    pub provenance: analyze_common::AnalysisProvenance,
    /// Doubt block with assumptions and blind spots.
    pub doubt: analyze_common::DoubtBlock,
}

impl HuntReport {
    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Summary statistics for the hunt.
    pub fn summary(&self) -> HuntSummary {
        let mut critical = 0;
        let mut high = 0;
        let mut medium = 0;
        let mut low = 0;

        for finding in &self.findings {
            match finding.severity {
                Severity::Critical => critical += 1,
                Severity::High => high += 1,
                Severity::Medium => medium += 1,
                Severity::Low => low += 1,
            }
        }

        HuntSummary {
            total_findings: self.findings.len(),
            critical,
            high,
            medium,
            low,
            duration_secs: self.duration_secs,
        }
    }
}

/// Summary statistics for a hunt.
#[derive(Debug, Clone)]
pub struct HuntSummary {
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub duration_secs: u64,
}

impl fmt::Display for HuntSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Bug Bounty Hunt Summary\n\
             =======================\n\
             Total findings: {}\n\
             Critical: {}\n\
             High:     {}\n\
             Medium:   {}\n\
             Low:      {}\n\
             Duration: {}s",
            self.total_findings,
            self.critical,
            self.high,
            self.medium,
            self.low,
            self.duration_secs
        )
    }
}

/// Trait that all analyzers must implement.
#[async_trait::async_trait]
pub trait Analyzer: Send + Sync {
    /// Run analysis on the target.
    async fn analyze(
        &self,
        config: &super::HuntConfig,
    ) -> Result<AnalyzerOutput, super::HunterError>;

    /// Clone this analyzer for parallel execution.
    fn clone_box(&self) -> Box<dyn Analyzer>;
}

/// Output from a single analyzer run.
#[derive(Debug)]
pub struct AnalyzerOutput {
    pub findings: Vec<Finding>,
}

impl AnalyzerOutput {
    pub fn into_findings(self) -> Vec<Finding> {
        self.findings
    }
}
