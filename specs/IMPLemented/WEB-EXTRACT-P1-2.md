# Web Content Extraction

**Status:** Implemented  
**Implementation Date:** October 31, 2026  
**Priority:** P1-2 (Drop-in replacement blocker)  
**Target:** crabjar-web-extract crate  
**Estimated effort:** 3-5 days  

## Overview

Provide web content extraction functionality that fetches URLs and returns clean, readable text/markdown. Enables search→extract workflows where agents first find relevant pages via web_search, then extract detailed content for analysis.

## Architecture Decision: Multiple Extraction Backends

**Primary backend:** Self-hosted Playwright instance (for JavaScript-heavy pages)  
**Fallback 1:** reqwest + html2text (simple HTML pages, fast)  
**Fallback 2:** Readability.js via wasm-bindgen (complex page layouts)  

### Why Multi-Backend?
- Different pages require different extraction strategies
- Simple HTML pages don't need full browser automation
- JavaScript-heavy SPAs require Playwright or similar
- PDF support requires separate handling

## API Design

```hemlock
// Tool definition for orchestrator
define tool web_extract {
    description: "Extract readable content from web URLs",
    parameters: {
        urls: { type: "array[string]", required: true, description: "List of URLs to extract" },
        char_limit: { type: "integer", default: 15000, description: "Max characters per page" }
    }
}

// Response format
{
    "success": true,
    "results": [
        {
            "url": "https://example.com",
            "title": "Page Title",
            "content": "Clean extracted text content...",
            "error": null
        },
        {
            "url": "https://pdf.example.com/doc.pdf",
            "title": "PDF Document",
            "content": "Extracted PDF text...",
            "error": null
        }
    ]
}
```

## Implementation Plan

### Phase 1: Crate Setup (Day 1)
- [ ] Create `crabjar-web-extract` crate in workspace
- [ ] Add dependencies: reqwest, html2text, serde, tokio, thiserror
- [ ] Define ExtractResponse and ExtractResult types
- [ ] Implement error handling with WebExtractError enum

### Phase 2: Simple HTML Extraction (Day 1-2)
- [ ] Implement HtmlExtractor using reqwest + html2text
- [ ] Handle common HTML structures (articles, docs, blogs)
- [ ] Strip navigation, ads, and boilerplate content
- [ ] Character limit enforcement with truncation

### Phase 3: PDF Support (Day 2-3)
- [ ] Implement PdfExtractor using pdf-extract crate or similar
- [ ] Handle text layer extraction from PDFs
- [ ] Support arxiv papers and technical documents
- [ ] Error handling for scanned/image-only PDFs

### Phase 4: JavaScript-Heavy Pages (Day 3-4)
- [ ] Implement Playwright-backed extractor for SPAs
- [ ] Wait for content to load before extraction
- [ ] Handle dynamic content and infinite scroll
- [ ] Timeout management for slow-loading pages

### Phase 5: Integration & Testing (Day 4-5)
- [ ] Register web_extract tool in orchestrator
- [ ] Add to conductor's tool manifest
- [ ] Write integration tests with real URLs
- [ ] Test error handling and edge cases
- [ ] Update CHANGELOG.md

## Configuration

### Environment Variables
- `WEB_EXTRACT_TIMEOUT` - Request timeout in seconds (default: 30)
- `PLAYWRIGHT_URL` - URL of Playwright instance for JS-heavy pages
- `PDF_ENGINE` - PDF extraction engine ("pdf-extract" or "pdftotext")

### Conductor Configuration
```toml
[tools.web_extract]
enabled = true
char_limit = 15000
timeout_seconds = 30
use_playwright = false  # Enable for JS-heavy pages
```

## Error Handling

- **Network errors:** Retry up to 2 times with increasing delay
- **Timeout:** Configurable per-request timeout
- **Malformed HTML:** Graceful degradation, return whatever text is available
- **PDF errors:** Return error message in result, don't fail entire batch
- **Character limit:** Truncate with "... (truncated)" indicator

## Testing Strategy

1. Unit tests for each extraction backend
2. Integration tests with real URLs (Wikipedia, arxiv, etc.)
3. PDF extraction tests with sample documents
4. Error handling tests (timeout, network failure, malformed content)
5. Character limit enforcement tests

## Open Questions

- Should we cache extracted content to reduce duplicate fetches? (TBD)
- How to handle authentication-required pages? (Skip for now)
- Should agents be able to specify extraction strategy? (No, keep it automatic)

## Success Criteria

- Agents can call `web_extract` tool and receive clean text content
- Handles HTML pages, PDFs, and JavaScript-heavy sites
- Character limit enforcement works correctly
- Error handling covers all failure modes
- Integration tests pass consistently
- Search→extract workflow functions end-to-end

## References

- [html2text-rs](https://github.com/BlinkingRobot/html2text-rs)
- [pdf-extract](https://docs.rs/pdf-extract/)
- [Playwright for Rust](https://playwright.dev/docs/intro)