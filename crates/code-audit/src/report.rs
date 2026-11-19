//! Bug report generation — formats findings into submission-ready reports.

use crate::types::*;
use chrono::Utc;

/// Generate a markdown bug report from hunt results.
pub fn generate_markdown_report(report: &HuntReport) -> String {
    let mut md = String::new();

    // Header
    md.push_str("# Bug Bounty Report\n\n");
    md.push_str(&format!("**Target:** {}\n", report.config.target));
    md.push_str(&format!(
        "**Date:** {}\n",
        Utc::now().format("%Y-%m-%d %H:%M UTC")
    ));
    md.push_str(&format!(
        "**Hunter:** bug-bounty-hunter v{}\n\n",
        env!("CARGO_PKG_VERSION")
    ));

    // Summary
    let summary = report.summary();
    md.push_str("## Summary\n\n");
    md.push_str(&format!("Total findings: **{}**\n", summary.total_findings));
    md.push_str(&format!("- Critical: {}\n", summary.critical));
    md.push_str(&format!("- High: {}\n", summary.high));
    md.push_str(&format!("- Medium: {}\n", summary.medium));
    md.push_str(&format!("- Low: {}\n\n", summary.low));

    // Findings by severity
    for severity in [
        Severity::Critical,
        Severity::High,
        Severity::Medium,
        Severity::Low,
    ] {
        let sev_findings: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.severity == severity)
            .collect();

        if !sev_findings.is_empty() {
            md.push_str(&format!("## {} Findings\n\n", severity));

            for (i, finding) in sev_findings.iter().enumerate() {
                md.push_str(&format!(
                    "### {}. {}\n\n",
                    i + 1,
                    escape_markdown(&finding.title)
                ));

                md.push_str(&format!(
                    "**CWE:** {}\n\n",
                    finding.cwe.as_deref().unwrap_or("N/A")
                ));
                md.push_str(&format!("{}\n\n", escape_markdown(&finding.description)));

                if !finding.evidence.is_empty() {
                    md.push_str("**Evidence:**\n\n");
                    for ev in &finding.evidence {
                        let data = serde_json::to_string_pretty(&ev.data).unwrap_or_default();
                        md.push_str(&format!(
                            "- Type: {}\n  Location: {}\n  ```json\n{}\n```\n\n",
                            ev.kind,
                            ev.location.as_deref().unwrap_or("N/A"),
                            data
                        ));
                    }
                }

                if finding.inference_validated
                    && let Some(conf) = finding.confidence
                {
                    md.push_str(&format!("**LLM Confidence:** {:.0}%\n\n", conf * 100.0));
                }

                md.push_str(&format!(
                    "**Remediation:** {}\n\n---\n\n",
                    escape_markdown(&finding.remediation)
                ));
            }
        }
    }

    // Provenance
    md.push_str("## Analysis Metadata\n\n");
    md.push_str(&format!(
        "- Tool: {} v{}\n",
        report.provenance.tool, report.provenance.version
    ));
    md.push_str(&format!("- Duration: {}s\n", report.duration_secs));

    // Doubt block
    if !report.doubt.assumptions.is_empty() {
        md.push_str("\n**Assumptions:**\n");
        for a in &report.doubt.assumptions {
            md.push_str(&format!("- {}\n", a));
        }
    }

    if !report.doubt.blind_spots.is_empty() {
        md.push_str("\n**Blind Spots:**\n");
        for b in &report.doubt.blind_spots {
            md.push_str(&format!("- {}\n", b));
        }
    }

    md
}

/// Generate a JSON report.
pub fn generate_json_report(report: &HuntReport) -> Result<String, serde_json::Error> {
    report.to_json()
}

fn escape_markdown(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
}
