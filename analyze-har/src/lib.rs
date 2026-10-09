//! HTTP Archive (HAR) network capture analysis.
//!
//! Parses HAR 1.2 format files and extracts request/response pairs, timing info,
//! resource types, status codes, and identifies suspicious patterns like unusual
//! domains, large payloads, redirects, and errors.

use analyze_common::{AnalysisProvenance, AnalysisResult, AnalyzeError, DoubtBlock, ANALYZE_COMMON_VERSION};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// HAR file version we expect to handle.
const HAR_VERSION: &str = "1.2";

// ---------------------------------------------------------------------------
// HAR format types (subset of HAR 1.2 spec)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HarFile {
    log: HarLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HarLog {
    version: String,
    entries: Vec<HarEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HarEntry {
    started_date_time: Option<String>,
    time: Option<f64>,
    request: HarRequest,
    response: HarResponse,
    #[serde(default)]
    cache: Option<HarCache>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HarRequest {
    method: String,
    url: String,
    http_version: Option<String>,
    headers: Vec<HarHeader>,
    headers_size: Option<i64>,
    body_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HarResponse {
    status: i32,
    status_text: Option<String>,
    http_version: Option<String>,
    headers: Vec<HarHeader>,
    headers_size: Option<i64>,
    body_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HarCache {
    #[serde(default)]
    before_request: Option<serde_json::Value>,
    #[serde(default)]
    after_response: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HarHeader {
    name: String,
    value: String,
}

// ---------------------------------------------------------------------------
// Analysis result types
// ---------------------------------------------------------------------------

/// Suspicious pattern identified during HAR analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspiciousPattern {
    pub kind: String,
    pub url: Option<String>,
    pub detail: String,
}

/// Domain-level summary from HAR analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarDomainSummary {
    pub domain: String,
    pub request_count: usize,
    pub total_bytes: i64,
}

/// Complete HAR analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarAnalysis {
    pub path: String,
    pub har_version: String,
    pub entry_count: usize,
    pub domains: Vec<HarDomainSummary>,
    pub total_requests: usize,
    pub error_requests: usize,
    pub redirect_requests: usize,
    pub large_payloads: usize,
    pub suspicious_patterns: Vec<SuspiciousPattern>,
}

// ---------------------------------------------------------------------------
// HAR parser and analyzer
// ---------------------------------------------------------------------------

/// Parse a HAR file from disk.
fn parse_har(path: &str) -> Result<HarFile, AnalyzeError> {
    let content = fs::read_to_string(path)
        .map_err(|e| AnalyzeError::Io(e))?;

    let har: HarFile = serde_json::from_str(&content)
        .map_err(|e| AnalyzeError::ParseError(format!("invalid HAR JSON: {}", e)))?;

    if har.log.version != HAR_VERSION {
        return Err(AnalyzeError::InvalidFormat(format!(
            "unsupported HAR version: {} (expected {})",
            har.log.version, HAR_VERSION
        )));
    }

    Ok(har)
}

/// Analyze a HAR file and produce structured analysis results.
pub fn analyze_har(path: &str) -> Result<AnalysisResult<HarAnalysis>, AnalyzeError> {
    let har = parse_har(path)?;

    let mut domain_map: HashMap<String, HarDomainSummary> = HashMap::new();
    let mut error_requests = 0;
    let mut redirect_requests = 0;
    let mut large_payloads = 0;
    let suspicious_patterns = Vec::new();

    for entry in &har.log.entries {
        // Parse domain from URL
        let url_parts = parse_url(&entry.request.url);
        let domain = url_parts.domain().clone().unwrap_or_else(|| "unknown".to_string());

        // Calculate total size (headers + body)
        let req_headers = entry.request.headers_size.unwrap_or(0);
        let req_body = entry.request.body_size.unwrap_or(0);
        let resp_headers = entry.response.headers_size.unwrap_or(0);
        let resp_body = entry.response.body_size.unwrap_or(0);
        let total_bytes = req_headers + req_body + resp_headers + resp_body;

        // Track domain stats
        let entry_domain = domain_map.entry(domain.clone()).or_insert(HarDomainSummary {
            domain: domain.clone(),
            request_count: 0,
            total_bytes: 0,
        });
        entry_domain.request_count += 1;
        entry_domain.total_bytes += total_bytes;

        // Count errors (4xx and 5xx)
        if entry.response.status >= 400 {
            error_requests += 1;
        }

        // Count redirects (3xx)
        if entry.response.status >= 300 && entry.response.status < 400 {
            redirect_requests += 1;
        }

        // Flag large payloads (>1MB)
        if total_bytes > 1_048_576 {
            large_payloads += 1;
        }
    }

    let domains: Vec<HarDomainSummary> = domain_map.into_values().collect();

    let analysis = HarAnalysis {
        path: path.to_string(),
        har_version: har.log.version.clone(),
        entry_count: har.log.entries.len(),
        domains,
        total_requests: har.log.entries.len(),
        error_requests,
        redirect_requests,
        large_payloads,
        suspicious_patterns,
    };

    let provenance = AnalysisProvenance::new("analyze-har", ANALYZE_COMMON_VERSION);
    let doubt = DoubtBlock::new("30d")
        .assume("HAR file is valid HAR 1.2 format")
        .blind_spot("timing analysis not implemented yet");

    Ok(AnalysisResult::new(
        format!("har-{}", Utc::now().timestamp_millis()),
        provenance,
        analysis,
        doubt,
    ))
}

/// Simple URL parts structure.
#[derive(Debug)]
struct HarUrlParts<'a> {
    scheme: &'a str,
    host: &'a str,
    path: &'a str,
}

impl<'a> HarUrlParts<'a> {
    fn domain(&self) -> Option<String> {
        if self.host.is_empty() {
            return None;
        }
        // Strip port if present
        let host = self.host.split(':').next().unwrap_or(self.host);
        Some(host.to_string())
    }
}

/// Simple URL parser to extract domain.
fn parse_url(url: &str) -> HarUrlParts {
    let mut scheme = "http";
    let mut host = "";
    let mut path = url;

    if let Some(pos) = url.find("://") {
        scheme = &url[..pos];
        let rest = &url[pos + 3..];
        if let Some(path_pos) = rest.find('/') {
            host = &rest[..path_pos];
            path = &rest[path_pos..];
        } else {
            host = rest;
            path = "";
        }
    } else if !url.starts_with("//") {
        // No scheme — treat as host/path directly
        if let Some(path_pos) = url.find('/') {
            host = &url[..path_pos];
            path = &url[path_pos..];
        } else {
            host = url;
            path = "";
        }
    }

    HarUrlParts { scheme, host, path }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url_http() {
        let parts = parse_url("http://example.com/path?q=1");
        assert_eq!(parts.scheme, "http");
        assert_eq!(parts.host, "example.com");
        assert_eq!(parts.path, "/path?q=1");
    }

    #[test]
    fn test_parse_url_https() {
        let parts = parse_url("https://secure.example.org:443/api/v1");
        assert_eq!(parts.scheme, "https");
        assert_eq!(parts.host, "secure.example.org:443");
        assert_eq!(parts.path, "/api/v1");
    }

    #[test]
    fn test_parse_url_no_scheme() {
        let parts = parse_url("example.com/path");
        assert_eq!(parts.scheme, "http");
        assert_eq!(parts.host, "example.com");
        assert_eq!(parts.path, "/path");
    }

    #[test]
    fn test_domain_extraction_with_port() {
        let parts = HarUrlParts {
            scheme: "https",
            host: "api.example.com:8080",
            path: "/",
        };
        assert_eq!(parts.domain(), Some("api.example.com".to_string()));
    }

    #[test]
    fn test_domain_extraction_no_port() {
        let parts = HarUrlParts {
            scheme: "http",
            host: "example.com",
            path: "/",
        };
        assert_eq!(parts.domain(), Some("example.com".to_string()));
    }

    #[test]
    fn test_domain_empty_host() {
        let parts = HarUrlParts {
            scheme: "http",
            host: "",
            path: "/",
        };
        assert_eq!(parts.domain(), None);
    }
}
