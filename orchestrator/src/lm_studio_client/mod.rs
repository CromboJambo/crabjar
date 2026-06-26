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
//!
//! # Prompt Envelope
//!
//! Every outbound prompt is wrapped in a `PromptEnvelope` with:
//! - Closed-vocabulary source labels (no free-text origin)
//! - Bounded context — system prompt and user content in labeled slots
//! - Instruction-hijack detection — rejects injected commands
//! - Provenance chain — SHA-256 integrity per content piece
//!
//! See `prompt_envelope` module for details.

#![allow(dead_code)]

// Submodules
mod client;
mod endpoints;
mod error;
mod prompt_envelope;
mod session;
mod types;

// Re-export public API
#[allow(unused_imports)]
pub use client::LmStudioClient;
#[allow(unused_imports)]
pub use error::ToolCallInfo;
#[allow(unused_imports)]
pub use prompt_envelope::{
    PromptEnvelope, PromptError, PromptValidator, SourceLabel,
};
#[allow(unused_imports)]
pub use session::SessionError;
#[allow(unused_imports)]
pub use types::*;
