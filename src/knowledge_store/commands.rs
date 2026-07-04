use super::{KnowledgeBridge, knowledge_response};
use agent_context::KnowledgeKind;
use serde_json::json;

use crate::KnowledgeCommand;

#[allow(async_fn_in_trait)]
pub trait KnowledgeCommandExt {
    async fn execute(
        &self,
        bridge: &KnowledgeBridge,
    ) -> Result<serde_json::Value, agent_context::Error>;
}

impl KnowledgeCommandExt for KnowledgeCommand {
    #[allow(async_fn_in_trait)]
    async fn execute(
        &self,
        bridge: &KnowledgeBridge,
    ) -> Result<serde_json::Value, agent_context::Error> {
        match self {
            Self::Index { doc } => {
                let ids = bridge.sync_state_doc_annotations(doc)?;
                Ok(knowledge_response(
                    format!("synced annotations for {}", doc),
                    json!({ "doc": doc, "ids": ids }),
                ))
            }
            Self::Sync { doc } => {
                let ids = bridge.sync_state_doc_annotations(doc)?;
                Ok(knowledge_response(
                    format!("synced annotations for {}", doc),
                    json!({ "doc": doc, "ids": ids }),
                ))
            }
            Self::Query { tags } => {
                let flattened: Vec<&str> = tags
                    .iter()
                    .flat_map(|t| t.split(','))
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                let rows = bridge.query_state_docs(&flattened, 100, "")?;
                Ok(knowledge_response(
                    "query complete",
                    json!({ "rows": rows }),
                ))
            }
            Self::Insert {
                content,
                kind,
                tags,
            } => {
                let kind = match kind.to_lowercase().as_str() {
                    "instruction" => KnowledgeKind::Instruction,
                    "pattern" => KnowledgeKind::Pattern,
                    "example" => KnowledgeKind::Example,
                    "context" => KnowledgeKind::Context,
                    _ => {
                        return Err(agent_context::Error::Internal(format!(
                            "unknown knowledge kind: {}",
                            kind
                        )));
                    }
                };
                let id = bridge.insert_entry(content, kind, tags.clone())?;
                Ok(knowledge_response(
                    "knowledge entry inserted",
                    json!({ "id": id }),
                ))
            }
            Self::Verify => {
                let bad_ids = bridge.verify()?;
                Ok(knowledge_response(
                    "verification complete",
                    json!({ "bad_ids": bad_ids }),
                ))
            }
            Self::Events { limit } => {
                let events = bridge.get_events(*limit)?;
                Ok(knowledge_response(
                    "events retrieved",
                    json!({ "events": events }),
                ))
            }
            Self::Deactivate { id, reason } => {
                bridge.deactivate(*id, agent_context::Source::User, Some(reason))?;
                Ok(knowledge_response(
                    "knowledge entry deactivated",
                    json!({ "id": id, "reason": reason }),
                ))
            }
            Self::ResolveAnnotation {
                doc,
                annotation_id,
                reason,
            } => {
                let (deactivated, resolved) =
                    bridge.resolve_annotation(doc, annotation_id, reason)?;
                Ok(knowledge_response(
                    "annotation resolved",
                    json!({
                        "deactivated": deactivated,
                        "resolved": {
                            "id": resolved.id,
                            "status": resolved.status,
                        }
                    }),
                ))
            }
            Self::Promote { id, reason } => {
                let promoted = bridge.promote_quarantined(*id, reason)?;
                Ok(knowledge_response(
                    "knowledge entry promoted",
                    json!({ "id": id, "promoted": promoted }),
                ))
            }
        }
    }
}
