//! HTML content extraction using reqwest + scraper.
//!
//! Fetches web pages and converts them to clean, readable text by:
//! - Stripping HTML tags and boilerplate
//! - Preserving meaningful structure (headings, paragraphs)
//! - Enforcing character limits

use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;

use super::error::WebExtractError;
use super::models::ExtractResult;

pub struct HtmlExtractor {
    client: Client,
}

impl Default for HtmlExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlExtractor {
    #[must_use]
    pub fn new() -> Self {
        let timeout = std::env::var("WEB_EXTRACT_TIMEOUT")
            .ok()
            .and_then(|t| t.parse::<u64>().ok())
            .unwrap_or(30);

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .user_agent("crabjar-web-extract/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    pub async fn extract(&self, url: &str, char_limit: usize) -> Result<ExtractResult, WebExtractError> {
        // Fetch the page
        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| WebExtractError::HttpRequestFailed(format!("Failed to fetch {url}: {e}")))?;

        if !response.status().is_success() {
            return Err(WebExtractError::HttpRequestFailed(format!(
                "HTTP {} for {}",
                response.status(),
                url
            )));
        }

        let html = response.text().await.map_err(|e| {
            WebExtractError::HttpRequestFailed(format!("Failed to read body: {e}"))
        })?;

        // Parse HTML and extract text content
        let document = Html::parse_document(&html);

        // Extract title
        let title_selector = Selector::parse("title").unwrap();
        let title = document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        // Try to extract main content area first
        let main_selectors = [
            "article",
            "main",
            ".post-content",
            ".entry-content",
            "#content",
            ".article-body",
            ".prose",
        ];

        let mut text = String::new();
        for selector_str in &main_selectors {
            match Selector::parse(selector_str) {
                Ok(sel) => {
                    if let Some(el) = document.select(&sel).next() {
                        text = self.extract_text_from_element(&el);
                        if !text.is_empty() {
                            break;
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        // If no main content found, extract from body
        if text.is_empty() {
            let body_selector = Selector::parse("body").unwrap();
            if let Some(body) = document.select(&body_selector).next() {
                text = self.extract_text_from_element(&body);
            }
        }

        // Clean up whitespace and normalize newlines
        let cleaned = self.cleanup_text(&text);

        // Enforce character limit
        let content = if cleaned.len() > char_limit {
            format!("{}... (truncated from {} characters)", &cleaned[..char_limit], cleaned.len())
        } else {
            cleaned
        };

        Ok(ExtractResult {
            url: url.to_string(),
            title,
            content,
            error: None,
        })
    }

    fn extract_text_from_element(&self, element: &scraper::ElementRef) -> String {
        let mut text = String::new();
        for child in element.children() {
            if child.value().is_element() {
                let tag_name = child.value().as_element().unwrap().name().to_string();
                
                // Skip navigation and boilerplate
                if ["script", "style", "nav", "footer", "header"].contains(&tag_name.as_str()) {
                    continue;
                }

                // Add extra newlines for block elements
                if ["p", "div", "h1", "h2", "h3", "h4", "h5", "h6", "li", "br"].contains(&tag_name.as_str())
                    && !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }

                if let Some(child_ref) = scraper::ElementRef::wrap(child) {
                    let child_text = self.extract_text_from_element(&child_ref);
                    if tag_name == "a" {
                        // For links, include the URL in parentheses
                        if let Some(href) = child.value().as_element().unwrap().attr("href") {
                            text.push_str(&child_text);
                            text.push_str(&format!(" ({href}))"));
                        } else {
                            text.push_str(&child_text);
                        }
                    } else {
                        text.push_str(&child_text);
                    }
                }
            } else if child.value().is_text() {
                text.push_str(child.value().as_text().unwrap().to_string().trim());
            }
        }

        text
    }

    fn cleanup_text(&self, text: &str) -> String {
        // Remove excessive blank lines (3+ newlines become 2)
        let mut result = String::with_capacity(text.len());
        let mut blank_count = 0;

        for line in text.lines() {
            if line.trim().is_empty() {
                blank_count += 1;
                if blank_count <= 2 {
                    result.push('\n');
                }
            } else {
                blank_count = 0;
                // Trim leading/trailing whitespace from each line
                result.push_str(line.trim());
                result.push('\n');
            }
        }

        result.trim().to_string()
    }
}