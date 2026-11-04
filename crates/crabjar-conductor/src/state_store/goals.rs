//! Goal types and operations.

use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::StateStore;

/// High-level objective submitted by a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub description: String,
    pub status: GoalStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    Decomposed,
    Running,
    Completed,
    Failed,
}

impl GoalStatus {
    fn as_str(&self) -> &'static str {
        match self {
            GoalStatus::Decomposed => "decomposed",
            GoalStatus::Running => "running",
            GoalStatus::Completed => "completed",
            GoalStatus::Failed => "failed",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "running" => GoalStatus::Running,
            "completed" => GoalStatus::Completed,
            "failed" => GoalStatus::Failed,
            _ => GoalStatus::Decomposed,
        }
    }
}

fn row_to_goal(row: &Row<'_>) -> rusqlite::Result<Goal> {
    let created_at_str: String = row.get(3)?;
    Ok(Goal {
        id: row.get(0)?,
        description: row.get(1)?,
        status: GoalStatus::from_str(&row.get::<_, String>(2)?),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|_e| rusqlite::Error::ExecuteReturnedResults)?
            .with_timezone(&Utc),
    })
}

// ─── Goal operations ──────────────────────────────────────────────

impl StateStore {
    pub fn create_goal(&self, description: String) -> Result<Goal, rusqlite::Error> {
        let goal = Goal {
            id: Uuid::new_v4().to_string(),
            description,
            status: GoalStatus::Decomposed,
            created_at: Utc::now(),
        };

        self.db.execute(
            "INSERT INTO goals (id, description, status, created_at) VALUES (?, ?, ?, ?)",
            params![
                goal.id,
                goal.description,
                goal.status.as_str(),
                goal.created_at.to_rfc3339()
            ],
        )?;

        Ok(goal)
    }

    pub fn get_goal(&self, id: &str) -> Result<Option<Goal>, rusqlite::Error> {
        self.db
            .query_row(
                "SELECT id, description, status, created_at FROM goals WHERE id = ?",
                params![id],
                row_to_goal,
            )
            .optional()
    }

    pub fn list_goals(&self) -> Result<Vec<Goal>, rusqlite::Error> {
        let mut stmt = self
            .db
            .prepare("SELECT id, description, status, created_at FROM goals")?;
        let rows = stmt.query_map(params![], row_to_goal)?;
        rows.collect()
    }

    pub fn update_goal_status(&self, id: &str, status: GoalStatus) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "UPDATE goals SET status = ? WHERE id = ?",
            params![status.as_str(), id],
        )?;
        Ok(())
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
    fn create_and_get_goal() {
        let (store, _dir) = test_store();
        let goal = store.create_goal("Test goal".to_string()).unwrap();
        let fetched = store.get_goal(&goal.id).unwrap().unwrap();
        assert_eq!(fetched.description, "Test goal");
        assert_eq!(fetched.status, GoalStatus::Decomposed);
    }

    #[test]
    fn update_goal_status() {
        let (store, _dir) = test_store();
        let goal = store.create_goal("Status test".to_string()).unwrap();
        store.update_goal_status(&goal.id, GoalStatus::Running).unwrap();
        let updated = store.get_goal(&goal.id).unwrap().unwrap();
        assert_eq!(updated.status, GoalStatus::Running);
    }

    #[test]
    fn list_goals() {
        let (store, _dir) = test_store();
        store.create_goal("First".to_string()).unwrap();
        store.create_goal("Second".to_string()).unwrap();
        let goals = store.list_goals().unwrap();
        assert_eq!(goals.len(), 2);
    }
}