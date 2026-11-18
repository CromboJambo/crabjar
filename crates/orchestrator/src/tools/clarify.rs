//! Clarify tool — ask users structured questions (Hermes API parity).
//!
//! Supports 1-5 questions per call with single-select, multi-select, and open-ended responses.

use tracing::info;
use crate::clarify::{ClarifyInput, ClarifyQuestion};

pub async fn handle(args: &[String], ctx: &super::ToolContext) -> Result<String, String> {
    if args.is_empty() || args[0].is_empty() {
        return Err("Error: clarify requires JSON arguments".to_string());
    }

    let input_json = &args[0];
    let input: ClarifyInput = serde_json::from_str(input_json)
        .map_err(|e| format!("Error parsing clarify arguments: {}", e))?;

    // Validate question count (Hermes allows 1-5)
    if input.questions.is_empty() {
        return Err("Error: at least one question required".to_string());
    }
    if input.questions.len() > 5 {
        return Err("Error: maximum 5 questions per clarify call".to_string());
    }

    // Validate each question
    for (i, q) in input.questions.iter().enumerate() {
        if q.question.trim().is_empty() {
            return Err(format!("Error: question {} has empty text", i + 1));
        }
        if !q.choices.is_empty() && q.choices.len() > 4 {
            return Err(format!("Error: question {} has more than 4 choices (max 4)", i + 1));
        }
    }

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
            "created_at": chrono::Utc::now().to_rfc3339(),
            "question_count": input.questions.len()
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

    let store_lock = ctx
        .store
        .lock()
        .map_err(|e| format!("Failed to lock store: {}", e))?;
    match store_lock.insert(entry) {
        Ok(id) => {
            info!(
                "Clarify form stored: id={}, db_row={}, questions={}",
                question_id,
                id,
                input.questions.len()
            );
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
        "questions": input.questions.len(),
        "note": "Form sent to user. Poll /acp/clarify/status/:id for answers."
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clarify_single_question() {
        let result = handle(
            &[r#"{"questions": [{"question": "What's your name?"}]}"#.to_string()],
            &make_ctx(),
        )
        .await;
        assert!(result.is_ok(), "Error: {:?}", result.err());
        let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(value["success"], true);
        assert_eq!(value["questions"], 1);
    }

    #[tokio::test]
    async fn test_clarify_multi_question() {
        let result = handle(
            &[r#"{"questions": [
                {"question": "What's your name?"},
                {"question": "How are you?", "choices": ["Good", "Bad"]},
                {"question": "Any other comments?"}
            ]}"#.to_string()],
            &make_ctx(),
        )
        .await;
        assert!(result.is_ok(), "Error: {:?}", result.err());
        let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(value["questions"], 3);
    }

    #[tokio::test]
    async fn test_clarify_too_many_questions() {
        let result = handle(
            &[r#"{"questions": [
                {"question": "Q1"},
                {"question": "Q2"},
                {"question": "Q3"},
                {"question": "Q4"},
                {"question": "Q5"},
                {"question": "Q6"}
            ]}"#.to_string()],
            &make_ctx(),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_clarify_too_many_choices() {
        let result = handle(
            &[r#"{"questions": [{"question": "Pick one", "choices": ["A", "B", "C", "D", "E"]}]}"#
                .to_string()],
            &make_ctx(),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_clarify_empty_questions() {
        let result = handle(&[r#"{"questions": []}"#.to_string()], &make_ctx()).await;
        assert!(result.is_err());
    }

    fn make_ctx() -> crate::tools::ToolContext {
        use agent_context::{Source};
        // Use a unique path per test to avoid concurrent test conflicts
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = format!("/tmp/crabjar-test-clarify-{}-{}.db", std::process::id(), counter);
        // Remove any existing test DB
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
        // Create the DB file with the tables Store needs
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS knowledge_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                kind TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                metadata TEXT NOT NULL DEFAULT '{}',
                weight REAL NOT NULL DEFAULT 1.0,
                source TEXT NOT NULL DEFAULT 'user',
                active INTEGER NOT NULL DEFAULT 1,
                source_type TEXT NOT NULL DEFAULT '',
                source_id TEXT NOT NULL DEFAULT '',
                provenance_id TEXT NOT NULL DEFAULT ''
            )",
            [],
        ).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS event_rows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
            [],
        ).unwrap();
        drop(conn);
        let store = agent_context::Store::open(String::from(path)).unwrap();
        crate::tools::ToolContext {
            store: std::sync::Arc::new(std::sync::Mutex::new(store)),
            process_registry: std::sync::Arc::new(tokio::sync::Mutex::new(
                crate::process_management::ProcessRegistry::new(),
            )),
        }
    }
}
