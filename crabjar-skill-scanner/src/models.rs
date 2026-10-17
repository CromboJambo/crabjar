//! Scanner result models.

use serde::{Deserialize, Serialize};

/// Severity level for scan findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

/// Recommendation based on scan results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recommendation {
    Allow,
    Warn,
    Reject,
}

impl std::fmt::Display for Recommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Recommendation::Allow => write!(f, "allow"),
            Recommendation::Warn => write!(f, "warn"),
            Recommendation::Reject => write!(f, "reject"),
        }
    }
}

/// Individual scan finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanFinding {
    pub severity: Severity,
    pub category: String,
    pub message: String,
    pub file: Option<String>,
}

/// Complete scan result for a skill package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub skill_name: String,
    pub version: String,
    pub passed: bool,
    pub recommendation: Recommendation,
    pub findings: Vec<ScanFinding>,
    pub files_scanned: usize,
    pub total_size_bytes: u64,
    pub checksum_verified: bool,
}

impl ScanResult {
    pub fn new(skill_name: &str, version: &str) -> Self {
        Self {
            skill_name: skill_name.to_string(),
            version: version.to_string(),
            passed: true,
            recommendation: Recommendation::Allow,
            findings: Vec::new(),
            files_scanned: 0,
            total_size_bytes: 0,
            checksum_verified: false,
        }
    }

    pub fn add_finding(&mut self, finding: ScanFinding) {
        match finding.severity {
            Severity::Info => {}
            Severity::Warning => {
                if self.recommendation == Recommendation::Allow {
                    self.recommendation = Recommendation::Warn;
                }
            }
            Severity::Error | Severity::Critical => {
                self.passed = false;
                self.recommendation = Recommendation::Reject;
            }
        }
        self.findings.push(finding);
    }

    pub fn summary(&self) -> String {
        let info_count = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Info)
            .count();
        let warn_count = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();
        let error_count = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();
        let critical_count = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count();

        format!(
            "{}: {} files scanned, {} bytes. Findings: {} info, {} warning, {} error, {} critical. Recommendation: {}",
            if self.passed { "PASS" } else { "FAIL" },
            self.files_scanned,
            self.total_size_bytes,
            info_count,
            warn_count,
            error_count,
            critical_count,
            self.recommendation
        )
    }
}
