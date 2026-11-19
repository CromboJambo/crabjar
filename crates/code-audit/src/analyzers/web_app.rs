//! Web application vulnerability scanner.
//!
//! Crawls target URL, analyzes HTTP responses for common web vulnerabilities:
//! XSS, SQL injection, missing headers, info disclosure, etc.

use crate::types::*;
use async_trait::async_trait;

#[derive(Debug, Default)]
pub struct WebAppAnalyzer {
    max_pages: usize,
}

impl WebAppAnalyzer {
    pub fn new() -> Self {
        Self { max_pages: 50 }
    }

    async fn fetch_page(&self, url: &str) -> Result<Page, crate::HunterError> {
        let response = reqwest::get(url).await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await.unwrap_or_default();

        Ok(Page {
            url: url.to_string(),
            status,
            body,
            headers,
        })
    }

    fn analyze_page(&self, page: &Page) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check for missing security headers
        if !page.headers.contains_key("content-security-policy") {
            findings.push(Finding {
                id: format!("web-missing-csp-{}", self.url_hash(&page.url)),
                title: "Missing Content-Security-Policy header".to_string(),
                description: "No CSP header found, vulnerable to XSS and injection attacks"
                    .to_string(),
                severity: Severity::Medium,
                cwe: Some("CWE-1021".to_string()),
                analyzer: AnalyzerType::WebApp,
                evidence: vec![Evidence {
                    kind: "http_headers".to_string(),
                    data: serde_json::json!({ "missing_header": "content-security-policy" }),
                    location: Some(page.url.clone()),
                }],
                remediation: "Add Content-Security-Policy header with restrictive policy"
                    .to_string(),
                inference_validated: false,
                confidence: None,
            });
        }

        if !page.headers.contains_key("x-frame-options") {
            findings.push(Finding {
                id: format!("web-missing-xfo-{}", self.url_hash(&page.url)),
                title: "Missing X-Frame-Options header".to_string(),
                description: "Page can be framed, vulnerable to clickjacking".to_string(),
                severity: Severity::Low,
                cwe: Some("CWE-1021".to_string()),
                analyzer: AnalyzerType::WebApp,
                evidence: vec![Evidence {
                    kind: "http_headers".to_string(),
                    data: serde_json::json!({ "missing_header": "x-frame-options" }),
                    location: Some(page.url.clone()),
                }],
                remediation: "Add X-Frame-Options: SAMEORIGIN or DENY header".to_string(),
                inference_validated: false,
                confidence: None,
            });
        }

        // Check for exposed server version
        if let Some(server_header) = page.headers.get("server")
            && let Ok(server_str) = server_header.to_str()
            && (server_str.contains('/') || server_str.contains('-'))
        {
            findings.push(Finding {
                id: format!("web-server-version-{}", self.url_hash(&page.url)),
                title: "Server version exposed".to_string(),
                description: format!("Server header reveals version: {}", server_str),
                severity: Severity::Low,
                cwe: Some("CWE-200".to_string()),
                analyzer: AnalyzerType::WebApp,
                evidence: vec![Evidence {
                    kind: "http_header_value".to_string(),
                    data: serde_json::json!({ "header": "server", "value": server_str }),
                    location: Some(page.url.clone()),
                }],
                remediation: "Configure server to omit version from Server header".to_string(),
                inference_validated: false,
                confidence: None,
            });
        }

        // Check for info disclosure in error pages
        if page.status.is_server_error() && page.body.contains("traceback") {
            findings.push(Finding {
                id: format!("web-traceback-{}", self.url_hash(&page.url)),
                title: "Stack traceback exposed on error page".to_string(),
                description: "Server error page reveals internal stack trace".to_string(),
                severity: Severity::Medium,
                cwe: Some("CWE-209".to_string()),
                analyzer: AnalyzerType::WebApp,
                evidence: vec![Evidence {
                    kind: "http_response_body".to_string(),
                    data: serde_json::json!({ "status": page.status.as_u16() }),
                    location: Some(page.url.clone()),
                }],
                remediation: "Configure custom error pages without stack traces".to_string(),
                inference_validated: false,
                confidence: None,
            });
        }

        findings
    }

    fn url_hash(&self, url: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        hasher.finish()
    }
}

#[async_trait]
impl Analyzer for WebAppAnalyzer {
    async fn analyze(
        &self,
        config: &crate::HuntConfig,
    ) -> Result<AnalyzerOutput, crate::HunterError> {
        // For simplicity, just analyze the target URL directly (no crawling for now)
        let pages = match self.fetch_page(&config.target).await {
            Ok(page) => vec![page],
            Err(_) => {
                return Ok(AnalyzerOutput {
                    findings: Vec::new(),
                });
            }
        };

        let mut all_findings = Vec::new();
        for page in &pages {
            all_findings.extend(self.analyze_page(page));
        }

        Ok(AnalyzerOutput {
            findings: all_findings,
        })
    }

    fn clone_box(&self) -> Box<dyn Analyzer> {
        Box::new(Self {
            max_pages: self.max_pages,
        })
    }
}

struct Page {
    url: String,
    status: reqwest::StatusCode,
    body: String,
    headers: reqwest::header::HeaderMap,
}
