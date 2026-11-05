mod compressor;
/// Context compression for the agent loop.
///
/// Between turns, the agent's observation array can grow unboundedly,
/// consuming context window and degrading model quality. This module
/// provides a `ContextCompressor` that condenses observations while
/// preserving critical information.
///
/// ## Compression Strategy
///
/// 1. **Keep recent**: Always retain the last N observations (configurable)
/// 2. **Summarize older**: Group older observations by stage/kind and summarize
/// 3. **Token budget**: Enforce a maximum token count (default: 4096)
/// 4. **Phase-aware**: Different phases may need different compression ratios

mod config;
#[cfg(test)]
mod tests;

pub use compressor::ContextCompressor;
pub use config::CompressionConfig;
