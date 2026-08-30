// crabjar/src/knowledge_store/mod.rs
// Knowledge store — bridge between state-docs and knowledge entries.

mod bridge;
pub mod commands;
mod confidence;

pub use bridge::KnowledgeBridge;

/// Build a structured knowledge response for CLI output.
pub fn knowledge_response(
    message: impl Into<String>,
    data: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "success": true,
        "message": message.into(),
        "data": data,
    })
}
