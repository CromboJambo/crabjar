//! crabjar-llm-plug-in: LLM runner plug-in protocol for external model runtime integration.
//!
//! Provides weight manifest output from safetensors DB, inference request/response structs,
//! and runner config for plugging external inference engines into crabjar tool calls and skills.
//!
//! ## Protocol
//!
//! - WeightManifest: JSON output from safetensors DB → external runner consumes
//! - InferenceRequest: prompt + context + skill_refs → runner receives
//! - InferenceResponse: structured output → guard gate consumption
//! - RunnerConfig: external runner endpoint/protocol configuration

pub mod error;
pub mod manifest;
pub mod protocol;

pub use error::{PlugInError, Result};
pub use manifest::WeightManifest;
pub use protocol::{InferenceRequest, InferenceResponse, RunnerConfig};
