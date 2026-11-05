//! Web content extraction for crabjar agents.
//!
//! Provides web page content extraction via multiple backends:
//! - `HtmlExtractor`: Simple HTML pages using reqwest + html2text
//! - `PdfExtractor`: PDF documents (planned)
//! - `PlaywrightExtractor`: JavaScript-heavy SPAs (planned)
//!
//! Integrates with orchestrator tool calling infrastructure so agents can
//! extract detailed content from URLs found via web search.

mod error;
mod html_extractor;
mod models;

pub use error::WebExtractError;
pub use html_extractor::HtmlExtractor;
pub use models::{ExtractResponse, ExtractResult};

/// Extract readable content from web URLs.
///
/// # Arguments
/// * `urls` - List of URLs to extract content from (max 5)
/// * `char_limit` - Maximum characters per page (default: 15000)
///
/// # Returns
/// `ExtractResponse` with results for each URL.
pub async fn extract(
    urls: &[String],
    char_limit: Option<usize>,
) -> Result<ExtractResponse, WebExtractError> {
    let char_limit = char_limit.unwrap_or(15000);
    let mut results = Vec::new();

    let html_extractor = HtmlExtractor::new();

    for url in urls.iter().take(5) {
        // PDF detection by extension (future: content-type sniffing)
        if url.to_lowercase().ends_with(".pdf") {
            tracing::info!(url, "Using PDF extractor (placeholder)");
            results.push(ExtractResult {
                url: url.clone(),
                title: String::new(),
                content: String::new(),
                error: Some("PDF extraction not yet implemented".to_string()),
            });
        } else {
            tracing::debug!(url, "Using HTML extractor");
            match html_extractor.extract(url, char_limit).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::warn!(url, error = %e, "Extraction failed");
                    results.push(ExtractResult {
                        url: url.clone(),
                        title: String::new(),
                        content: String::new(),
                        error: Some(format!("{e}")),
                    });
                }
            }
        }
    }

    Ok(ExtractResponse { results })
}
