//! Hermes-compatible memory system for CrabJar.
//!
//! Provides MEMORY.md and USER.md file-based persistent memory with
//! identical format and tool schema to Hermes Agent's memory system.
//!
//! # Features
//! - Two stores: agent notes (MEMORY.md) and user profile (USER.md)
//! - Character budget enforcement (2,200 / 1,375 chars)
//! - Batch operations with atomic success/failure
//! - Substring matching for patch/remove
//! - Hermes-compatible tool schema and response format

pub mod capacity;
pub mod error;
pub mod parser;
#[cfg(feature = "mirror-log")]
pub mod mirror_log;
pub mod store;
pub mod tool;

pub use capacity::MemoryCapacity;
pub use error::{MemoryError, MemoryResult};
pub use store::MemoryStore;
pub use tool::{MemoryAction, MemoryTool, MemoryToolInput};

/// Character limit for MEMORY.md (agent notes)
pub const MEMORY_CHAR_LIMIT: usize = 2200;

/// Character limit for USER.md (user profile)
pub const USER_CHAR_LIMIT: usize = 1375;
