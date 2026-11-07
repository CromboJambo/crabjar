use tracing::info;

pub async fn handle(args: &[String], ctx: &super::ToolContext) -> Result<String, String> {
    if args.is_empty() {
        return Err("Error: clarify requires JSON arguments".to_string());
    }

    let input_json = &args[0];
    let input: crate::clarify::ClarifyInput = serde_json::from_str(input_json)
        .map_err(|e| format!("Error parsing clarify arguments: {}", e))?;

    // Generate unique question ID
    let question_id = uuid::Uuid::new_v4().to_string();

    // Store the pending question in the knowledge store with specific tags
    let entry = agent_context::KnowledgeEntry {
        content: serde_json::to_string(&input)
            .map_err(|e| format!("Failed to serialize clarify input: {}", e))?,
        kind: agent_context::KnowledgeKind::Context,
        tags: vec![
            "clarify-pending".to_string(),
            format!("clarify-{}", question_id),
        ],
        metadata: serde_json::json!({
            "question_id": question_id,
            "status": "pending",
            "created_at": chrono::Utc::now().to_rfc3339()
        }),
        weight: 1.0,
        source: agent_context::Source::Agent,
        source_type: "clarify-tool".to_string(),
        source_id: question_id.clone(),
        provenance_id: format!("clarify-{}", question_id),
        confidence: 1.0,
        evidence: agent_context::Evidence {
            supporting_examples: vec![],
            counterexamples: vec![],
            accuracy: 1.0,
        },
        source_observations: vec![],
    };

    let store_lock = ctx.store.lock().map_err(|e| format!("Failed to lock store: {}", e))?;
    match store_lock.insert(entry) {
        Ok(id) => {
            info!("Clarify question stored: id={}, db_row={}", question_id, id);
        }
        Err(e) => {
            return Err(format!("Failed to persist clarify question: {}", e));
        }
    }
    drop(store_lock);

    // Return immediately with question ID — agent should poll for response
    Ok(serde_json::json!({
        "success": true,
        "question_id": question_id,
        "status": "pending",
        "note": "Question sent to user. Poll /acp/clarify/status/:id for answer."
    })
    .to_string())
}
