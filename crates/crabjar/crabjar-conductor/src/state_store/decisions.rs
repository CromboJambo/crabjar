//! Decision history and promoted rule operations.

use chrono::Utc;
use rusqlite::{OptionalExtension, params};
use uuid::Uuid;

use super::StateStore;

/// A single decision record from history.
#[derive(Debug, Clone)]
pub struct DecisionRecord {
    pub id: String,
    pub observation_hash: String,
    pub chosen_option: String,
    pub confidence: f64,
    pub cache_hit: bool,
    pub created_at: String,
}

/// A promoted decision rule stored in the database.
#[derive(Debug, Clone)]
pub struct PromotedRule {
    pub id: i64,
    pub pattern_id: String,
    pub recommended_option: String,
    pub avg_confidence: f64,
    pub occurrence_count: i64,
    pub promoted_at: String,
}

/// Decision frequency statistics for rule extraction.
#[derive(Debug, Clone)]
pub struct DecisionFrequency {
    pub option: String,
    pub count: i64,
    pub avg_confidence: f64,
}

// ─── Decision history operations ──────────────────────────────────

impl StateStore {
    /// Log a JEV decision to the history store.
    pub fn log_decision(
        &self,
        observation_hash: &str,
        criteria_file: &str,
        chosen_option: &str,
        confidence: f64,
        cache_hit: bool,
    ) -> Result<(), rusqlite::Error> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();

        self.db.execute(
            "INSERT INTO decisions (id, observation_hash, criteria_file, chosen_option, confidence, cache_hit, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                id,
                observation_hash,
                criteria_file,
                chosen_option,
                confidence,
                i32::from(cache_hit),
                created_at
            ],
        )?;

        Ok(())
    }

    /// Get decision history for a specific criteria file.
    pub fn get_decisions_for_criteria(
        &self,
        criteria_file: &str,
        limit: i32,
    ) -> Result<Vec<DecisionRecord>, rusqlite::Error> {
        let mut stmt = self.db.prepare(
            "SELECT id, observation_hash, chosen_option, confidence, cache_hit, created_at FROM decisions WHERE criteria_file = ? ORDER BY created_at DESC LIMIT ?",
        )?;
        let rows = stmt.query_map(params![criteria_file, limit], |row| {
            Ok(DecisionRecord {
                id: row.get(0)?,
                observation_hash: row.get(1)?,
                chosen_option: row.get(2)?,
                confidence: row.get(3)?,
                cache_hit: row.get::<_, i32>(4)? == 1,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    /// Get decision frequency statistics for rule extraction.
    /// Returns option counts grouped by observation hash pattern for a criteria file.
    pub fn get_decision_frequencies(
        &self,
        criteria_file: &str,
    ) -> Result<Vec<DecisionFrequency>, rusqlite::Error> {
        let sql = r"
            SELECT chosen_option, COUNT(*) as count, AVG(confidence) as avg_confidence
            FROM decisions
            WHERE criteria_file = ?
            GROUP BY chosen_option
            ORDER BY count DESC
        ";
        let mut stmt = self.db.prepare(sql)?;
        let rows = stmt.query_map(params![criteria_file], |row| {
            Ok(DecisionFrequency {
                option: row.get(0)?,
                count: row.get(1)?,
                avg_confidence: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    /// Get total decision count for a criteria file.
    pub fn get_decision_count(&self, criteria_file: &str) -> Result<i64, rusqlite::Error> {
        let row = self.db.query_row(
            "SELECT COUNT(*) FROM decisions WHERE criteria_file = ?",
            params![criteria_file],
            |r| r.get::<_, i64>(0),
        )?;
        Ok(row)
    }

    // ─── Promoted rule operations ─────────────────────────────────────

    /// Store a promoted decision rule.
    pub fn promote_rule(
        &self,
        pattern_id: &str,
        criteria_file: &str,
        recommended_option: &str,
        avg_confidence: f64,
        occurrence_count: i64,
    ) -> Result<i64, rusqlite::Error> {
        let promoted_at = Utc::now().to_rfc3339();
        let _result = self.db.execute(
            "INSERT INTO decision_rules (pattern_id, criteria_file, recommended_option, avg_confidence, occurrence_count, promoted_at) VALUES (?, ?, ?, ?, ?, ?)",
            params![pattern_id, criteria_file, recommended_option, avg_confidence, occurrence_count, promoted_at],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    /// Get all promoted rules for a specific criteria file.
    pub fn get_promoted_rules(
        &self,
        criteria_file: &str,
    ) -> Result<Vec<PromotedRule>, rusqlite::Error> {
        let mut stmt = self.db.prepare(
            "SELECT id, pattern_id, recommended_option, avg_confidence, occurrence_count, promoted_at FROM decision_rules WHERE criteria_file = ? ORDER BY promoted_at DESC",
        )?;
        let rows = stmt.query_map(params![criteria_file], |row| {
            Ok(PromotedRule {
                id: row.get(0)?,
                pattern_id: row.get(1)?,
                recommended_option: row.get(2)?,
                avg_confidence: row.get(3)?,
                occurrence_count: row.get(4)?,
                promoted_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    /// Check if a rule exists for the given pattern and criteria.
    pub fn has_rule(&self, pattern_id: &str, criteria_file: &str) -> Result<bool, rusqlite::Error> {
        let row = self.db.query_row(
            "SELECT COUNT(*) FROM decision_rules WHERE pattern_id = ? AND criteria_file = ?",
            params![pattern_id, criteria_file],
            |r| r.get::<_, i64>(0),
        )?;
        Ok(row > 0)
    }

    /// Get the recommended option for a given pattern and criteria.
    pub fn get_rule_option(
        &self,
        pattern_id: &str,
        criteria_file: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.db.query_row(
            "SELECT recommended_option FROM decision_rules WHERE pattern_id = ? AND criteria_file = ?",
            params![pattern_id, criteria_file],
            |r| r.get::<_, String>(0),
        ).optional()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_store() -> (StateStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let store = StateStore::open(&path).unwrap();
        (store, dir)
    }

    #[test]
    fn log_and_query_decisions() {
        let (store, _dir) = test_store();
        store
            .log_decision("hash1", "criteria.jev", "option_a", 0.9, false)
            .unwrap();
        store
            .log_decision("hash2", "criteria.jev", "option_b", 0.7, true)
            .unwrap();

        let decisions = store
            .get_decisions_for_criteria("criteria.jev", 10)
            .unwrap();
        assert_eq!(decisions.len(), 2);

        let count = store.get_decision_count("criteria.jev").unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn promote_and_query_rules() {
        let (store, _dir) = test_store();
        store
            .promote_rule("pattern1", "criteria.jev", "option_a", 0.85, 10)
            .unwrap();

        let rules = store.get_promoted_rules("criteria.jev").unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].recommended_option, "option_a");

        assert!(store.has_rule("pattern1", "criteria.jev").unwrap());
    }
}
