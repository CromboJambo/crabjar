// crabjar/memory/src/context/constants.rs
// Constants for bounded context fragments — inspired by Codex's context-fragments/ crate.

/// Maximum tokens per fragment. Codex hard cap: 10K tokens.
pub const MAX_TOKENS_PER_FRAGMENT: usize = 10_000;

/// P0 alert threshold in tokens. Fragments exceeding this require manual
/// review before being included in model context. Codex convention: >1K tokens.
pub const P0_ALERT_TOKENS: usize = 1_000;

/// Default cumulative context budget (tokens). When the total context
/// budget is exhausted, new fragments are rejected with a hard error.
/// 128K tokens covers most models; callers can override.
pub const DEFAULT_CONTEXT_BUDGET: usize = 128_000;
