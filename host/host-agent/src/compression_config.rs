/// Configuration for context compression.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Maximum tokens to retain after compression (default: 4096).
    pub max_tokens: usize,
    /// Number of recent observations to keep uncompressed (default: 10).
    pub recent_count: usize,
    /// Whether to enable compression at all (default: true).
    pub enabled: bool,
    /// Maximum number of summary entries (default: 5).
    pub max_summaries: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            recent_count: 10,
            enabled: true,
            max_summaries: 5,
        }
    }
}

impl CompressionConfig {
    /// Create a config optimized for short conversations (fewer summaries).
    pub fn for_short_conversation() -> Self {
        Self {
            max_tokens: 2048,
            recent_count: 5,
            enabled: true,
            max_summaries: 3,
        }
    }

    /// Create a config optimized for long conversations (more retention).
    pub fn for_long_conversation() -> Self {
        Self {
            max_tokens: 8192,
            recent_count: 20,
            enabled: true,
            max_summaries: 10,
        }
    }

    /// Disable compression entirely.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }
}
