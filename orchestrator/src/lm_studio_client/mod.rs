//! lm_studio_client: Unified client for LM Studio's multiple API endpoints.
//!
//! Supports three endpoints with a toggle:
//! - Native `/api/v1/chat` — stateful chat via `previous_response_id`
//! - OpenAI-compatible `/v1/chat/completions` — full message history
//! - Anthropic-compatible `/v1/messages` — full message history
//!
//! The client abstracts endpoint differences so the orchestrator doesn't
//! need to know which endpoint it's talking to.
//!
//! Session state is managed via `SessionStore` — for the native endpoint
//! this tracks `response_id` for continuation; for OpenAI/Anthropic it
//! tracks the full message history.

#![allow(dead_code)]

// Submodules
mod types;
mod session;
mod endpoints;
mod client;
mod error;

// Re-export public API
pub use types::*;
pub use session::{SessionState, SessionStore, SessionError};
pub use client::LmStudioClient;
pub use error::{LmStudioError, ToolCallInfo, detect_available_endpoints};
