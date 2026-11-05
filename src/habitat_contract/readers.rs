//! Data readers: pull real state from SQLite databases into contract types.

use super::types::{PendingAction, TheoryStatus};

/// Read the guard pending queue. Read-only: opens the DB directly (no schema
/// init) and returns empty if the file or table is absent.
pub fn read_pending_actions(guard_db: &str) -> Vec<PendingAction> {
    let Ok(conn) = rusqlite::Connection::open(guard_db) else {
        return Vec::new();
    };
    let Ok(mut stmt) = conn.prepare(
        "SELECT id, action_type, command, trust_layer, confidence, queued_at, reason \
         FROM pending_queue ORDER BY queued_at DESC",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok(PendingAction {
            id: row.get(0)?,
            action_type: row.get(1)?,
            command: row.get(2)?,
            trust_layer: row.get(3)?,
            confidence: row.get(4)?,
            queued_at: row.get(5)?,
            reason: row.get(6)?,
        })
    }) else {
        return Vec::new();
    };
    rows.flatten().collect()
}

/// Read the theory state-doc's staleness. Returns the not-indexed default
/// when the DB, the migration, or the doc itself is absent.
pub fn read_theory_status(db_path: &str, theory: &str) -> TheoryStatus {
    let Ok(conn) = rusqlite::Connection::open(db_path) else {
        return TheoryStatus::default();
    };
    let Ok(()) = agent_context::state_docs::migrate(&conn) else {
        return TheoryStatus::default();
    };
    let querier = agent_context::state_docs::StateDocQuerier::new(
        conn,
        std::path::PathBuf::from("state-docs"),
    );
    let status = querier.staleness_status(theory);
    let last_modified = status["last_modified"].as_str().unwrap_or("").to_string();
    if last_modified.is_empty() {
        return TheoryStatus::default();
    }
    TheoryStatus {
        indexed: true,
        status: status["status"].as_str().unwrap_or("fresh").to_string(),
        days_old: status["days_old"].as_f64().unwrap_or(0.0),
        is_trustworthy: status["is_trustworthy"].as_bool().unwrap_or(false),
        warning: status["warning"].as_str().map(String::from),
    }
}
