# crabjar-web-extract

Web content extraction for crabjar agents. Fetches URLs and converts HTML/PDF to clean markdown.

## Usage

```rust
use crabjar_web_extract::extract;

let results = extract(&["https://example.com"], 10_000).await?;
```

This crate is published to crates.io for use by external projects (e.g. [tuirowser](https://github.com/CromboJambo/tuirowser)).
