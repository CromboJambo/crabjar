/// SQLite-backed persistence for WorkItem.
///
/// Mirrors the cookie_store pattern: Arc<RwLock<Connection>>, async CRUD.
/// Each WorkItem is serialized to JSON and stored in a single row keyed by id.
use rusqlite::{params, Connection, Result as SqlResult};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crabjar_host_core::WorkItem;

/// Error type for work item store operations.
#[derive(thiserror::Error, Debug)]
pub enum WorkItemStoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("work item not found: {0}")]
    NotFound(Uuid),
    #[error("uuid parse error: {0}")]
    UuidParse(#[from] uuid::Error),
}

/// SQLite-backed persistence layer for WorkItem.
pub struct WorkItemStore {
    db_path: PathBuf,
    conn: Arc<RwLock<Connection>>,
}

impl WorkItemStore {
    /// Open or create the work item store database.
    pub fn open(db_path: PathBuf) -> SqlResult<Self> {
        let conn = Connection::open(&db_path)?;
        Self::init(&conn)?;
        Ok(Self {
            db_path,
            conn: Arc::new(RwLock::new(conn)),
        })
    }

    fn init(conn: &Connection) -> SqlResult<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS work_items (
                id TEXT PRIMARY KEY,
                objective TEXT NOT NULL,
                status TEXT NOT NULL,
                observations TEXT NOT NULL DEFAULT '[]',
                hypothesis TEXT,
                plan TEXT NOT NULL DEFAULT '[]',
                artifacts TEXT NOT NULL DEFAULT '[]',
                confidence REAL NOT NULL DEFAULT 0.0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )?;
        Ok(())
    }

    /// Save a WorkItem to the database (upsert).
    pub async fn save(&self, work_item: &WorkItem) -> SqlResult<()> {
        let tx = self.conn.write().await;
        tx.execute(
            "INSERT OR REPLACE INTO work_items
             (id, objective, status, observations, hypothesis, plan, artifacts, confidence, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                work_item.id.to_string(),
                work_item.objective,
                serde_json::to_string(&work_item.status).unwrap_or_default(),
                serde_json::to_string(&work_item.observations).unwrap_or_default(),
                work_item.hypothesis.as_ref().map(|h| serde_json::to_string(h).unwrap_or_default()),
                serde_json::to_string(&work_item.plan).unwrap_or_default(),
                serde_json::to_string(&work_item.artifacts).unwrap_or_default(),
                work_item.confidence,
                work_item.created_at.timestamp(),
                work_item.updated_at.timestamp(),
            ],
        )?;
        Ok(())
    }

    /// Load a WorkItem by id.
    pub async fn load(&self, id: Uuid) -> Result<WorkItem, WorkItemStoreError> {
        let tx = self.conn.read().await;
        let mut stmt = tx.prepare(
            "SELECT id, objective, status, observations, hypothesis, plan, artifacts, confidence, created_at, updated_at
             FROM work_items WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id.to_string()])?;
        let row = rows
            .next()?
            .ok_or(WorkItemStoreError::NotFound(id))?;

        let id_str: String = row.get(0)?;
        let objective: String = row.get(1)?;
        let status_json: String = row.get(2)?;
        let observations_json: String = row.get(3)?;
        let hypothesis_json: Option<String> = row.get(4)?;
        let plan_json: String = row.get(5)?;
        let artifacts_json: String = row.get(6)?;
        let confidence: f64 = row.get(7)?;
        let created_at: i64 = row.get(8)?;
        let updated_at: i64 = row.get(9)?;

        Ok(WorkItem {
            id: Uuid::parse_str(&id_str)?,
            objective,
            status: serde_json::from_str(&status_json).map_err(WorkItemStoreError::Serialization)?,
            observations: serde_json::from_str(&observations_json).map_err(WorkItemStoreError::Serialization)?,
            hypothesis: hypothesis_json
                .map(|h| serde_json::from_str(&h).map_err(WorkItemStoreError::Serialization))
                .transpose()?,
            plan: serde_json::from_str(&plan_json).map_err(WorkItemStoreError::Serialization)?,
            artifacts: serde_json::from_str(&artifacts_json).map_err(WorkItemStoreError::Serialization)?,
            confidence: confidence as f32,
            created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(created_at, 0)
                .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
            updated_at: chrono::DateTime::<chrono::Utc>::from_timestamp(updated_at, 0)
                .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
        })
    }

    /// List all persisted WorkItem ids (lightweight query, no deserialization).
    pub async fn list_ids(&self) -> SqlResult<Vec<Uuid>> {
        let tx = self.conn.read().await;
        let mut stmt = tx.prepare("SELECT id FROM work_items ORDER BY updated_at DESC")?;
        let ids = stmt.query_map(params![], |row| {
            let s: String = row.get(0)?;
            Uuid::parse_str(&s).map_err(|_| {
                rusqlite::Error::InvalidColumnType(0, "TEXT".to_string(), rusqlite::types::Type::Text)
            })
        })?;
        ids.collect()
    }

    /// Delete a WorkItem by id.
    pub async fn delete(&self, id: Uuid) -> SqlResult<bool> {
        let tx = self.conn.write().await;
        let rows = tx.execute(
            "DELETE FROM work_items WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(rows > 0)
    }

    /// Database path for external access.
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crabjar_host_core::work_item::TaskStatus;

    fn temp_db() -> PathBuf {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("work_items.db");
        let leaked = dir.into_path();
        leaked.join("work_items.db")
    }

    fn sample_work_item() -> WorkItem {
        let mut wi = WorkItem::new("Test persistence");
        wi.add_task("Step one");
        wi.add_task("Step two");
        wi.update_task(0, TaskStatus::Completed, Some("done".into()));
        wi.set_confidence(0.5);
        wi
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let store = WorkItemStore::open(temp_db()).unwrap();
        let wi = sample_work_item();
        let id = wi.id;

        store.save(&wi).await.unwrap();

        let loaded = store.load(id).await.unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.objective, "Test persistence");
        assert!((loaded.confidence - 0.5).abs() < f32::EPSILON);
        assert_eq!(loaded.plan.len(), 2);
    }

    #[tokio::test]
    async fn test_load_not_found() {
        let store = WorkItemStore::open(temp_db()).unwrap();
        let missing = Uuid::new_v4();
        let result = store.load(missing).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_ids() {
        let store = WorkItemStore::open(temp_db()).unwrap();
        let wi1 = sample_work_item();
        let mut wi2 = WorkItem::new("Another task");
        wi2.set_confidence(0.9);

        store.save(&wi1).await.unwrap();
        store.save(&wi2).await.unwrap();

        let ids = store.list_ids().await.unwrap();
        assert_eq!(ids.len(), 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let store = WorkItemStore::open(temp_db()).unwrap();
        let wi = sample_work_item();
        let id = wi.id;

        store.save(&wi).await.unwrap();
        assert!(store.delete(id).await.unwrap());
        assert!(!store.delete(id).await.unwrap()); // already deleted

        let result = store.load(id).await;
        assert!(result.is_err());
    }
}
