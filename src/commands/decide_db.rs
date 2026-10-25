//! JEV Database Evidence Store
//!
//! Persistent storage for decision history using SQLite. Enables:
//! - Learning loop (track rule vs embedding accuracy over time)
//! - Audit trail for decisions
//! - Confidence drift detection
//! - Cache persistence across process restarts

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A stored decision record for the learning loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: i64,
    pub observation_preview: String,
    pub observation_hash: String,
    pub decision_option: String,
    pub confidence: f64,
    pub method: String, // "rule_engine", "embedding", "cache"
    pub components_invoked: Vec<String>,
    pub computation_time_ms: u64,
    pub outcome: Option<String>, // "correct", "incorrect", "timeout" (filled later)
    pub created_at: i64,
}

/// Database-backed evidence store for JEV decisions.
pub struct EvidenceStore {
    conn: Connection,
}

impl EvidenceStore {
    /// Open or create the evidence database at the given path.
    pub fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)?;

        // Create tables if they don't exist
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS decisions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                observation_preview TEXT NOT NULL,
                observation_hash TEXT NOT NULL UNIQUE,
                decision_option TEXT NOT NULL,
                confidence REAL NOT NULL,
                method TEXT NOT NULL,
                components_invoked TEXT NOT NULL,
                computation_time_ms INTEGER NOT NULL,
                outcome TEXT,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS rule_performance (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                rule_id TEXT NOT NULL,
                decision_option TEXT NOT NULL,
                total_matches INTEGER NOT NULL DEFAULT 0,
                correct_matches INTEGER NOT NULL DEFAULT 0,
                last_updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_decisions_hash ON decisions(observation_hash);
            CREATE INDEX IF NOT EXISTS idx_decisions_method ON decisions(method);
            CREATE INDEX IF NOT EXISTS idx_rule_performance_id ON rule_performance(rule_id);",
        )?;

        Ok(Self { conn })
    }

    /// Store a decision record. Returns the record ID.
    pub fn store_decision(
        &self,
        record: &DecisionRecord,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let components_json = serde_json::to_string(&record.components_invoked)?;

        // Insert or ignore (observation_hash is unique — cache hit means same decision)
        let result = self.conn.execute(
            "INSERT OR IGNORE INTO decisions (
                observation_preview, observation_hash, decision_option, confidence,
                method, components_invoked, computation_time_ms, outcome, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                record.observation_preview,
                record.observation_hash,
                record.decision_option,
                record.confidence,
                record.method,
                components_json,
                record.computation_time_ms,
                record.outcome,
                record.created_at
            ],
        );

        match result {
            Ok(_) => Ok(self.conn.last_insert_rowid()),
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Update the outcome of a decision (for learning loop feedback).
    pub fn update_outcome(
        &self,
        observation_hash: &str,
        outcome: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let result = self.conn.execute(
            "UPDATE decisions SET outcome = ? WHERE observation_hash = ?",
            params![outcome, observation_hash],
        );
        Ok(result?)
    }

    /// Update rule performance metrics.
    pub fn update_rule_performance(
        &self,
        rule_id: &str,
        decision_option: &str,
        correct: bool,
        now: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Insert or update rule performance
        self.conn.execute(
            "INSERT INTO rule_performance (rule_id, decision_option, total_matches, correct_matches, last_updated_at)
             VALUES (?, ?, 1, ?, ?)
             ON CONFLICT(rule_id) DO UPDATE SET
                total_matches = total_matches + 1,
                correct_matches = correct_matches + ?,
                last_updated_at = ?",
            params![rule_id, decision_option, if correct { 1 } else { 0 }, now, if correct { 1 } else { 0 }, now],
        )?;
        Ok(())
    }

    /// Get rule performance for a specific rule.
    pub fn get_rule_performance(
        &self,
        rule_id: &str,
    ) -> Result<Option<(i64, i64)>, Box<dyn std::error::Error>> {
        let row = self.conn.query_row(
            "SELECT total_matches, correct_matches FROM rule_performance WHERE rule_id = ?",
            params![rule_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        );

        match row {
            Ok(perf) => Ok(Some(perf)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Get recent decisions for audit trail.
    pub fn recent_decisions(
        &self,
        limit: usize,
    ) -> Result<Vec<DecisionRecord>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, observation_preview, observation_hash, decision_option, confidence,
                    method, components_invoked, computation_time_ms, outcome, created_at
             FROM decisions ORDER BY created_at DESC LIMIT ?",
        )?;

        let mut rows = stmt.query(params![limit as i64])?;
        let mut records = Vec::new();

        loop {
            let row = match rows.next() {
                Ok(Some(row)) => row,
                Ok(None) => break,
                Err(e) => return Err(Box::new(e)),
            };
            let components_json: String = row.get(6)?;
            let components: Vec<String> = serde_json::from_str(&components_json)?;

            records.push(DecisionRecord {
                id: row.get(0)?,
                observation_preview: row.get(1)?,
                observation_hash: row.get(2)?,
                decision_option: row.get(3)?,
                confidence: row.get(4)?,
                method: row.get(5)?,
                components_invoked: components,
                computation_time_ms: row.get(7)?,
                outcome: row.get(8)?,
                created_at: row.get(9)?,
            });
        }

        Ok(records)
    }

    /// Get decision statistics by method.
    pub fn method_stats(&self) -> Result<Vec<(String, i64, f64)>, Box<dyn std::error::Error>> {
        let mut stmt = self
            .conn
            .prepare("SELECT method, COUNT(*), AVG(confidence) FROM decisions GROUP BY method")?;

        let mut rows = stmt.query(params![])?;
        let mut stats = Vec::new();

        loop {
            let row = match rows.next() {
                Ok(Some(row)) => row,
                Ok(None) => break,
                Err(e) => return Err(Box::new(e)),
            };
            stats.push((row.get(0)?, row.get(1)?, row.get(2)?));
        }

        Ok(stats)
    }

    /// Get decision statistics by method within a time range.
    pub fn method_stats_since(
        &self,
        since_epoch: i64,
    ) -> Result<Vec<(String, i64, f64)>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT method, COUNT(*), AVG(confidence) FROM decisions WHERE created_at >= ? GROUP BY method",
        )?;

        let mut rows = stmt.query(params![since_epoch])?;
        let mut stats = Vec::new();

        loop {
            let row = match rows.next() {
                Ok(Some(row)) => row,
                Ok(None) => break,
                Err(e) => return Err(Box::new(e)),
            };
            stats.push((row.get(0)?, row.get(1)?, row.get(2)?));
        }

        Ok(stats)
    }

    /// Get decisions that could be promoted to rules (high confidence, repeated patterns).
    pub fn promotion_candidates(
        &self,
        limit: usize,
    ) -> Result<Vec<DecisionRecord>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, observation_preview, observation_hash, decision_option, confidence,
                    method, components_invoked, computation_time_ms, outcome, created_at
             FROM decisions
             WHERE method = 'embedding' AND confidence >= 0.85
             ORDER BY confidence DESC, created_at ASC
             LIMIT ?",
        )?;

        let mut rows = stmt.query(params![limit as i64])?;
        let mut records = Vec::new();

        loop {
            let row = match rows.next() {
                Ok(Some(row)) => row,
                Ok(None) => break,
                Err(e) => return Err(Box::new(e)),
            };
            let components_json: String = row.get(6)?;
            let components: Vec<String> = serde_json::from_str(&components_json)?;

            records.push(DecisionRecord {
                id: row.get(0)?,
                observation_preview: row.get(1)?,
                observation_hash: row.get(2)?,
                decision_option: row.get(3)?,
                confidence: row.get(4)?,
                method: row.get(5)?,
                components_invoked: components,
                computation_time_ms: row.get(7)?,
                outcome: row.get(8)?,
                created_at: row.get(9)?,
            });
        }

        Ok(records)
    }

    /// Get learning insights by component.
    pub fn learnings_by_component(
        &self,
        component: &str,
        limit: usize,
    ) -> Result<Vec<DecisionRecord>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, observation_preview, observation_hash, decision_option, confidence,
                    method, components_invoked, computation_time_ms, outcome, created_at
             FROM decisions
             WHERE components_invoked LIKE ?
             ORDER BY created_at DESC
             LIMIT ?",
        )?;

        let pattern = format!("%{}%", component);
        let mut rows = stmt.query(params![pattern, limit as i64])?;
        let mut records = Vec::new();

        loop {
            let row = match rows.next() {
                Ok(Some(row)) => row,
                Ok(None) => break,
                Err(e) => return Err(Box::new(e)),
            };
            let components_json: String = row.get(6)?;
            let components: Vec<String> = serde_json::from_str(&components_json)?;

            records.push(DecisionRecord {
                id: row.get(0)?,
                observation_preview: row.get(1)?,
                observation_hash: row.get(2)?,
                decision_option: row.get(3)?,
                confidence: row.get(4)?,
                method: row.get(5)?,
                components_invoked: components,
                computation_time_ms: row.get(7)?,
                outcome: row.get(8)?,
                created_at: row.get(9)?,
            });
        }

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn make_store() -> (EvidenceStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jev_test.db");
        let store = EvidenceStore::open(path.to_string_lossy().as_ref()).unwrap();
        (store, dir)
    }

    #[test]
    fn store_and_retrieve_decision() {
        let (store, _dir) = make_store();

        let record = DecisionRecord {
            id: 0,
            observation_preview: "Test observation".to_string(),
            observation_hash: "abc123".to_string(),
            decision_option: "restart_service".to_string(),
            confidence: 0.9,
            method: "rule_engine".to_string(),
            components_invoked: vec!["rule_engine".to_string()],
            computation_time_ms: 0,
            outcome: None,
            created_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };

        let id = store.store_decision(&record).unwrap();
        assert!(id > 0);

        // Retrieve via recent decisions
        let recent = store.recent_decisions(10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].decision_option, "restart_service");
    }

    #[test]
    fn unique_observation_hash() {
        let (store, _dir) = make_store();

        let record1 = DecisionRecord {
            id: 0,
            observation_preview: "Test".to_string(),
            observation_hash: "same_hash".to_string(),
            decision_option: "option_a".to_string(),
            confidence: 0.9,
            method: "rule_engine".to_string(),
            components_invoked: vec![],
            computation_time_ms: 0,
            outcome: None,
            created_at: 1000,
        };

        let record2 = DecisionRecord {
            id: 0,
            observation_preview: "Test".to_string(),
            observation_hash: "same_hash".to_string(),
            decision_option: "option_b".to_string(),
            confidence: 0.8,
            method: "embedding".to_string(),
            components_invoked: vec![],
            computation_time_ms: 100,
            outcome: None,
            created_at: 2000,
        };

        store.store_decision(&record1).unwrap();
        store.store_decision(&record2).unwrap();

        // Should only have one record (INSERT OR IGNORE)
        let recent = store.recent_decisions(10).unwrap();
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn method_stats() {
        let (store, _dir) = make_store();

        let record1 = DecisionRecord {
            id: 0,
            observation_preview: "Test 1".to_string(),
            observation_hash: "hash1".to_string(),
            decision_option: "option_a".to_string(),
            confidence: 0.9,
            method: "rule_engine".to_string(),
            components_invoked: vec![],
            computation_time_ms: 0,
            outcome: None,
            created_at: 1000,
        };

        let record2 = DecisionRecord {
            id: 0,
            observation_preview: "Test 2".to_string(),
            observation_hash: "hash2".to_string(),
            decision_option: "option_b".to_string(),
            confidence: 0.7,
            method: "embedding".to_string(),
            components_invoked: vec![],
            computation_time_ms: 100,
            outcome: None,
            created_at: 2000,
        };

        store.store_decision(&record1).unwrap();
        store.store_decision(&record2).unwrap();

        let stats = store.method_stats().unwrap();
        assert_eq!(stats.len(), 2);

        // Find rule_engine stat
        let rule_stat = stats.iter().find(|s| s.0 == "rule_engine").unwrap();
        assert_eq!(rule_stat.1, 1);
        assert!((rule_stat.2 - 0.9).abs() < 0.001);
    }
}
