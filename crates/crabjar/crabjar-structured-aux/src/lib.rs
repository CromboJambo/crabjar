//! # crabjar-structured-aux
//!
//! Structured auxiliary decision calls via OpenRouter's JEV endpoint.
//! Routes selected Hermes auxiliary tasks through bounded Jev decision calls
//! instead of free-form chat prompts for improved reliability and cost efficiency.
//!
//! This plugin replaces the `hermes-structured-aux-models` Python plugin with a native
//! Rust implementation that integrates directly into crabjar's plugin system.

pub mod handlers;
pub mod plugin;
pub mod protocol;

pub use crate::plugin::StructuredAuxPlugin;

/// Factory function to create the structured aux plugin.
pub fn create_structured_aux_plugin(api_key: Option<String>) -> StructuredAuxPlugin {
    StructuredAuxPlugin::new(api_key)
}
