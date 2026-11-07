//! crabjar-code-audit-mcp: MCP server wrapping code-audit for automated vulnerability discovery.
//!
//! Exposes security auditing tools that use structural code analysis (pesti) combined
//! with LLM inference to find vulnerabilities across web apps, binaries, and source code.

use code_audit::{HuntConfig, TargetType, hunt};
use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AuditCodeRequest {
    /// Path to source file or directory to analyze.
    pub path: String,
    /// Optional: enable LLM inference for deeper analysis (slower). Default: false.
    #[serde(default)]
    pub use_inference: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AuditWebAppRequest {
    /// URL of the web application to audit.
    pub url: String,
    /// Optional: enable LLM inference for deeper analysis (slower). Default: false.
    #[serde(default)]
    pub use_inference: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AuditBinaryRequest {
    /// Path to the binary file to analyze.
    pub path: String,
    /// Optional: enable LLM inference for deeper analysis (slower). Default: false.
    #[serde(default)]
    pub use_inference: bool,
}

#[derive(Debug, Clone)]
pub struct CodeAuditServer;

fn build_summary(report: &code_audit::HuntReport) -> serde_json::Value {
    let mut critical = 0;
    let mut high = 0;
    let mut medium = 0;
    let mut low = 0;
    for f in &report.findings {
        match f.severity {
            code_audit::Severity::Critical => critical += 1,
            code_audit::Severity::High => high += 1,
            code_audit::Severity::Medium => medium += 1,
            code_audit::Severity::Low => low += 1,
        }
    }

    serde_json::json!({
        "hunt_id": report.id,
        "findings_count": report.findings.len(),
        "summary": {
            "total_findings": report.findings.len(),
            "critical": critical,
            "high": high,
            "medium": medium,
            "low": low,
            "duration_secs": report.duration_secs
        },
        "findings": report.findings
    })
}

#[tool_router(server_handler)]
impl CodeAuditServer {
    #[tool(
        description = "Run static security analysis on source code. Uses pesti structural tokens and AST parsing to detect vulnerabilities like injection flaws, hardcoded credentials, unsafe patterns, and memory safety issues in C/C++/Rust/Python/JavaScript."
    )]
    async fn audit_code(&self, params: Parameters<AuditCodeRequest>) -> String {
        let req = params.0;

        let config = HuntConfig {
            target: req.path,
            target_type: TargetType::SourceCode,
            use_inference: req.use_inference,
            ..Default::default()
        };

        match hunt(config).await {
            Ok(report) => build_summary(&report).to_string(),
            Err(e) => serde_json::json!({ "error": format!("{}", e) }).to_string(),
        }
    }

    #[tool(
        description = "Run security audit on a web application. Performs reconnaissance, endpoint discovery, and vulnerability testing including XSS, CSRF, injection flaws, authentication bypass, and insecure configurations."
    )]
    async fn audit_webapp(&self, params: Parameters<AuditWebAppRequest>) -> String {
        let req = params.0;

        let config = HuntConfig {
            target: req.url,
            target_type: TargetType::WebApp,
            use_inference: req.use_inference,
            ..Default::default()
        };

        match hunt(config).await {
            Ok(report) => build_summary(&report).to_string(),
            Err(e) => serde_json::json!({ "error": format!("{}", e) }).to_string(),
        }
    }

    #[tool(
        description = "Run security analysis on a binary executable. Detects embedded credentials, hardcoded strings, vulnerability indicators, and suspicious patterns in compiled binaries."
    )]
    async fn audit_binary(&self, params: Parameters<AuditBinaryRequest>) -> String {
        let req = params.0;

        let config = HuntConfig {
            target: req.path,
            target_type: TargetType::Binary,
            use_inference: req.use_inference,
            ..Default::default()
        };

        match hunt(config).await {
            Ok(report) => build_summary(&report).to_string(),
            Err(e) => serde_json::json!({ "error": format!("{}", e) }).to_string(),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = CodeAuditServer;
    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    running.waiting().await?;
    Ok(())
}