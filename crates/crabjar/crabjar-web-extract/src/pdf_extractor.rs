//! PDF content extraction using pdf-extract crate.
//!
//! Downloads PDF files and extracts readable text content,
//! returning results in the same format as HTML extraction.

use crate::error::WebExtractError;
use crate::models::ExtractResult;
use pdf_extract::extract_text_from_mem;
use reqwest::Client;
use tracing::debug;

/// Extract text content from PDF documents.
pub struct PdfExtractor {
    client: Client,
}

impl PdfExtractor {
    /// Create a new PDF extractor with default HTTP client settings.
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("crabjar-web-extract/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Extract text content from a PDF URL.
    pub async fn extract(&self, url: &str, char_limit: usize) -> Result<ExtractResult, WebExtractError> {
        debug!(url, "Downloading PDF for extraction");

        // Download the PDF
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| WebExtractError::HttpRequestFailed(format!("Failed to download PDF: {}", e)))?;

        if !response.status().is_success() {
            return Err(WebExtractError::NetworkError(format!(
                "HTTP {} for PDF download",
                response.status().as_u16()
            )));
        }

        let pdf_bytes = response
            .bytes()
            .await
            .map_err(|e| WebExtractError::HttpRequestFailed(format!("Failed to read PDF bytes: {}", e)))?;

        debug!(url, size = pdf_bytes.len(), "PDF downloaded, extracting text");

        // Extract text using pdf-extract
        let text = extract_text_from_mem(&pdf_bytes)
            .map_err(|e| WebExtractError::PdfError(format!("PDF text extraction failed: {}", e)))?;

        // Clean up the extracted text (remove excessive whitespace)
        let cleaned = clean_pdf_text(&text);

        // Apply character limit with head+tail truncation
        let (content, truncated) = truncate_content(&cleaned, char_limit);

        debug!(url, chars = content.len(), truncated, "PDF extraction complete");

        Ok(ExtractResult {
            url: url.to_string(),
            title: extract_pdf_title(&pdf_bytes).unwrap_or_else(|| "PDF Document".to_string()),
            content,
            error: None,
        })
    }
}

impl Default for PdfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Clean up extracted PDF text.
fn clean_pdf_text(text: &str) -> String {
    // Remove excessive blank lines (more than 2 consecutive newlines)
    let cleaned = regex_replace_blank_lines(text);
    // Trim leading/trailing whitespace from each line
    cleaned
        .lines()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Replace 3+ consecutive newlines with exactly 2.
fn regex_replace_blank_lines(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut blank_count = 0;

    for c in text.chars() {
        if c == '\n' {
            blank_count += 1;
            if blank_count <= 2 {
                result.push(c);
            }
        } else {
            blank_count = 0;
            result.push(c);
        }
    }

    result
}

/// Truncate content to char_limit with head+tail strategy.
fn truncate_content(content: &str, char_limit: usize) -> (String, bool) {
    if content.len() <= char_limit {
        return (content.to_string(), false);
    }

    let half = char_limit / 2;
    let head = &content[..half];
    let tail = &content[content.len().saturating_sub(half)..];

    let truncated = format!(
        "{}\n\n... [truncated {} characters] ...\n\n{}",
        head,
        content.len() - char_limit,
        tail
    );

    (truncated, true)
}

/// Try to extract the PDF title from metadata.
fn extract_pdf_title(pdf_bytes: &[u8]) -> Option<String> {
    // Simple approach: use pdf-extract's document info if available
    // For now, return None and let the caller use a default
    let _ = pdf_bytes; // Suppress unused warning for now
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_pdf_text_removes_excessive_blank_lines() {
        let input = "Line 1\n\n\n\n\nLine 2\n\nLine 3";
        let cleaned = clean_pdf_text(input);
        assert!(!cleaned.contains("\n\n\n"));
        assert!(cleaned.contains("Line 1"));
        assert!(cleaned.contains("Line 2"));
    }

    #[test]
    fn test_truncate_content_short() {
        let (result, truncated) = truncate_content("Hello", 100);
        assert_eq!(result, "Hello");
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_content_long() {
        let long = "a".repeat(1000);
        let (result, truncated) = truncate_content(&long, 100);
        assert!(truncated);
        assert!(result.contains("truncated"));
        assert!(result.len() <= 200); // head + tail + marker
    }

    #[test]
    fn test_regex_replace_blank_lines() {
        let input = "A\n\n\n\n\nB";
        let result = regex_replace_blank_lines(input);
        assert_eq!(result, "A\n\nB");
    }
}
