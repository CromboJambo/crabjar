//! Drift governance: detect, report, and optionally reindex stale or drifted state-docs.
//!
//! This ties together the three pieces of the drift detection infrastructure:
//! - `drift_status()` — checksum-based change detection (coasting vs resisting)
//! - `staleness_status()` — time-based trust decay (fresh → moldy)
//! - `reindex()` — recovery action when drift is detected

use crabjar_lib::StateCommand;
use serde_json::{json, Value};

pub fn handle(command: StateCommand) -> Result<Value, Box<dyn std::error::Error>> {
    match command {
        StateCommand::Drift { doc_name, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            
            let querier = agent_context::state_docs::StateDocQuerier::new(
                conn,
                std::path::PathBuf::from(&db_path),
            );
            
            // Step 1: Check drift (checksum mismatch)
            let drift = querier.drift_status(&doc_name);
            
            // Step 2: Check staleness (time-based)
            let staleness = querier.staleness_status(&doc_name);
            
            // Step 3: Determine action needed
            let has_drift = drift["drift"] == json!(true);
            let is_trustworthy = staleness["is_trustworthy"] == json!(true);
            let doc_exists = drift["exists"] == json!(true);
            
            let (status, message) = if !doc_exists {
                ("not_indexed", format!("{}: state-doc not indexed", doc_name))
            } else if has_drift {
                ("drifted", format!("{}: content drifted since indexing", doc_name))
            } else if !is_trustworthy {
                ("stale", format!("{}: stale ({} days old)", doc_name, staleness["days_old"]))
            } else {
                ("coasting", format!("{}: OK", doc_name))
            };
            
            Ok(json!({
                "success": true,
                "message": message,
                "payload": {
                    "doc": doc_name,
                    "status": status,
                    "drift": drift,
                    "staleness": staleness,
                }
            }))
        }
        _ => Err("not a drift command".into()),
    }
}
