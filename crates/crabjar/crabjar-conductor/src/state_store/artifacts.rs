//! Deployment artifact management.
//!
//! Captures, stores, and retrieves output artifacts from completed deployments
//! (logs, results, generated files). Supports both SQLite BLOB storage for small
//! artifacts (< 1MB) and filesystem paths for large files.

use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use super::StateStore;

/// Maximum size for SQLite BLOB storage (1MB). Larger artifacts are stored on disk.
pub const MAX_BLOB_SIZE: usize = 1_048_576;

/// Artifact type classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArtifactType {
    Log,
    Result,
    File,
    Error,
}

impl ArtifactType {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactType::Log => "log",
            ArtifactType::Result => "result",
            ArtifactType::File => "file",
            ArtifactType::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "result" => ArtifactType::Result,
            "file" => ArtifactType::File,
            "error" => ArtifactType::Error,
            _ => ArtifactType::Log,
        }
    }
}

/// Storage location for an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactStorage {
    /// Stored as BLOB in SQLite (small artifacts).
    Blob(Vec<u8>),
    /// Stored on filesystem (large artifacts).
    Path(PathBuf),
}

impl ArtifactStorage {
    #[must_use]
    pub fn size(&self) -> usize {
        match self {
            ArtifactStorage::Blob(data) => data.len(),
            ArtifactStorage::Path(_) => 0, // Unknown until read from disk
        }
    }
}

/// A deployment artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentArtifact {
    pub id: String,
    pub deployment_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,
    /// MIME type hint (optional).
    pub content_type: Option<String>,
    /// Size in bytes.
    pub size: usize,
    /// Storage location (serialized as JSON).
    pub storage: ArtifactStorage,
    /// When the artifact was created.
    pub created_at: String,
}

fn row_to_artifact(row: &Row<'_>) -> rusqlite::Result<DeploymentArtifact> {
    let blob_data: Option<Vec<u8>> = row.get(6)?;
    let path_str: Option<String> = row.get(7)?;

    let storage = if let Some(data) = blob_data {
        ArtifactStorage::Blob(data)
    } else if let Some(path) = path_str {
        ArtifactStorage::Path(PathBuf::from(path))
    } else {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            6,
            rusqlite::types::Type::Null,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "artifact has neither blob nor path",
            )),
        ));
    };

    Ok(DeploymentArtifact {
        id: row.get(0)?,
        deployment_id: row.get(1)?,
        name: row.get(2)?,
        artifact_type: ArtifactType::from_str(&row.get::<_, String>(3)?),
        content_type: row.get(4)?,
        size: row.get(5)?,
        storage,
        created_at: row.get(8)?,
    })
}

// ─── Artifact operations ────────────────────────────────────────

impl StateStore {
    /// Store an artifact for a deployment.
    pub fn store_artifact(
        &self,
        deployment_id: &str,
        name: String,
        artifact_type: ArtifactType,
        content_type: Option<String>,
        data: Vec<u8>,
    ) -> Result<DeploymentArtifact, rusqlite::Error> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let size = data.len();

        // Decide storage location based on size
        if size <= MAX_BLOB_SIZE {
            // Store as BLOB in SQLite
            self.db.execute(
                "INSERT INTO deployment_artifacts (id, deployment_id, name, type, content_type, size, blob_data, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![id, deployment_id, name, artifact_type.as_str(), content_type, size, data, created_at],
            )?;
        } else {
            // Store on filesystem, record path in DB
            let artifacts_dir = self.artifacts_dir();
            std::fs::create_dir_all(&artifacts_dir).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)),
                )
            })?;
            let file_path = artifacts_dir.join(format!("{id}.artifact"));

            std::fs::write(&file_path, &data).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)),
                )
            })?;

            self.db.execute(
                "INSERT INTO deployment_artifacts (id, deployment_id, name, type, content_type, size, path, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![id, deployment_id, name, artifact_type.as_str(), content_type, size, file_path.to_string_lossy().to_string(), created_at],
            )?;

            let storage = ArtifactStorage::Path(file_path);
            return Ok(DeploymentArtifact {
                id,
                deployment_id: deployment_id.to_string(),
                name,
                artifact_type,
                content_type,
                size,
                storage,
                created_at,
            });
        }

        // For BLOB storage, read back to return the full artifact
        self.get_artifact(&id)
            .and_then(|opt| opt.ok_or_else(|| rusqlite::Error::QueryReturnedNoRows))
    }

    /// Get a single artifact by ID.
    pub fn get_artifact(&self, id: &str) -> Result<Option<DeploymentArtifact>, rusqlite::Error> {
        self.db
            .query_row(
                "SELECT id, deployment_id, name, type, content_type, size, blob_data, path, created_at FROM deployment_artifacts WHERE id = ?",
                params![id],
                row_to_artifact,
            )
            .optional()
    }

    /// List all artifacts for a deployment.
    pub fn list_deployment_artifacts(
        &self,
        deployment_id: &str,
    ) -> Result<Vec<DeploymentArtifact>, rusqlite::Error> {
        let mut stmt = self.db.prepare(
            "SELECT id, deployment_id, name, type, content_type, size, blob_data, path, created_at FROM deployment_artifacts WHERE deployment_id = ? ORDER BY created_at ASC",
        )?;

        let rows = stmt.query_map(params![deployment_id], row_to_artifact)?;
        rows.collect()
    }

    /// Delete an artifact.
    pub fn delete_artifact(&self, id: &str) -> Result<(), rusqlite::Error> {
        // If it's a filesystem artifact, delete the file too
        if let Some(artifact) = self.get_artifact(id)? {
            if let ArtifactStorage::Path(path) = &artifact.storage {
                let _ = std::fs::remove_file(path);
            }
        }

        self.db.execute("DELETE FROM deployment_artifacts WHERE id = ?", params![id])?;
        Ok(())
    }

    /// Delete all artifacts for a deployment.
    pub fn delete_deployment_artifacts(&self, deployment_id: &str) -> Result<i64, rusqlite::Error> {
        let artifacts = self.list_deployment_artifacts(deployment_id)?;
        let mut deleted = 0;

        for artifact in artifacts {
            if let ArtifactStorage::Path(path) = &artifact.storage {
                let _ = std::fs::remove_file(path);
            }
            let rows = self.db.execute("DELETE FROM deployment_artifacts WHERE id = ?", params![artifact.id])?;
            deleted += rows as i64;
        }

        Ok(deleted)
    }

    /// Clean up artifacts older than the given retention period (in days).
    pub fn cleanup_old_artifacts(&self, retention_days: u32) -> Result<i64, rusqlite::Error> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        // Find completed deployments older than cutoff
        let mut stmt = self.db.prepare(
            "SELECT id FROM worker_deployments WHERE completed_at IS NOT NULL AND completed_at < ?"
        )?;

        let rows = stmt.query_map(params![cutoff_str], |row| row.get::<_, String>(0))?;
        let old_deployment_ids: Vec<String> = rows.collect::<Result<_, _>>()?;

        let mut total_deleted = 0;
        for dep_id in old_deployment_ids {
            let deleted = self.delete_deployment_artifacts(&dep_id)?;
            total_deleted += deleted;
        }

        Ok(total_deleted)
    }

    /// Get the artifacts directory path.
    fn artifacts_dir(&self) -> PathBuf {
        // Use the same directory as the database file
        let parent = self
            .db
            .path()
            .map(|p| std::path::Path::new(p).parent().unwrap_or_else(|| std::path::Path::new(".")))
            .unwrap_or_else(|| std::path::Path::new("."));
        parent.join("artifacts")
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
    fn store_and_retrieve_blob_artifact() {
        let (store, _dir) = test_store();
        let dep_id = "dep-test-001";

        // Create a deployment first
        let task_spec = serde_json::json!({"id": "test-task"});
        store.create_deployment("test-supervisor".to_string(), task_spec).unwrap();

        // Store a small artifact as BLOB
        let data = b"test log output\nline 2\n";
        let artifact = store
            .store_artifact(
                dep_id,
                "stdout.log".to_string(),
                ArtifactType::Log,
                Some("text/plain".to_string()),
                data.to_vec(),
            )
            .unwrap();

        assert_eq!(artifact.deployment_id, dep_id);
        assert_eq!(artifact.name, "stdout.log");
        assert_eq!(artifact.artifact_type, ArtifactType::Log);
        assert_eq!(artifact.size, data.len());

        // Retrieve it back
        let retrieved = store.get_artifact(&artifact.id).unwrap().unwrap();
        match &retrieved.storage {
            ArtifactStorage::Blob(blob_data) => {
                assert_eq!(blob_data, data);
            }
            ArtifactStorage::Path(_) => panic!("expected blob storage for small artifact"),
        }
    }

    #[test]
    fn store_large_artifact_on_filesystem() {
        let (store, _dir) = test_store();
        let dep_id = "dep-test-002";

        let task_spec = serde_json::json!({"id": "test-task"});
        store.create_deployment("test-supervisor".to_string(), task_spec).unwrap();

        // Create a large artifact (> 1MB) that should be stored on disk
        let large_data = vec![0u8; MAX_BLOB_SIZE + 1024];
        let artifact = store
            .store_artifact(
                dep_id,
                "large-output.bin".to_string(),
                ArtifactType::File,
                Some("application/octet-stream".to_string()),
                large_data.clone(),
            )
            .unwrap();

        assert_eq!(artifact.size, large_data.len());

        // Should be stored on filesystem
        match &artifact.storage {
            ArtifactStorage::Path(path) => {
                assert!(path.exists(), "filesystem artifact should exist at {:?}", path);
                let contents = std::fs::read(path).unwrap();
                assert_eq!(contents, large_data);
            }
            ArtifactStorage::Blob(_) => panic!("expected filesystem storage for large artifact"),
        }

        // Clean up
        store.delete_artifact(&artifact.id).unwrap();
    }

    #[test]
    fn list_deployment_artifacts() {
        let (store, _dir) = test_store();
        let dep_id = "dep-test-003";

        let task_spec = serde_json::json!({"id": "test-task"});
        store.create_deployment("test-supervisor".to_string(), task_spec).unwrap();

        // Store multiple artifacts
        store
            .store_artifact(
                dep_id,
                "stdout.log".to_string(),
                ArtifactType::Log,
                None,
                b"output".to_vec(),
            )
            .unwrap();
        store
            .store_artifact(
                dep_id,
                "stderr.log".to_string(),
                ArtifactType::Error,
                None,
                b"errors".to_vec(),
            )
            .unwrap();
        store
            .store_artifact(
                dep_id,
                "result.json".to_string(),
                ArtifactType::Result,
                Some("application/json".to_string()),
                b"{}".to_vec(),
            )
            .unwrap();

        let artifacts = store.list_deployment_artifacts(dep_id).unwrap();
        assert_eq!(artifacts.len(), 3);

        // Verify types are preserved
        assert_eq!(artifacts[0].artifact_type, ArtifactType::Log);
        assert_eq!(artifacts[1].artifact_type, ArtifactType::Error);
        assert_eq!(artifacts[2].artifact_type, ArtifactType::Result);
    }

    #[test]
    fn delete_artifact() {
        let (store, _dir) = test_store();
        let dep_id = "dep-test-004";

        let task_spec = serde_json::json!({"id": "test-task"});
        store.create_deployment("test-supervisor".to_string(), task_spec).unwrap();

        let artifact = store
            .store_artifact(
                dep_id,
                "temp.log".to_string(),
                ArtifactType::Log,
                None,
                b"temporary".to_vec(),
            )
            .unwrap();

        // Verify it exists
        assert!(store.get_artifact(&artifact.id).unwrap().is_some());

        // Delete it
        store.delete_artifact(&artifact.id).unwrap();

        // Verify it's gone
        assert!(store.get_artifact(&artifact.id).unwrap().is_none());
    }

    #[test]
    fn cleanup_old_artifacts() {
        let (store, _dir) = test_store();

        // Create a deployment that completed 10 days ago
        let old_dep_id = "dep-old";
        let task_spec = serde_json::json!({"id": "old-task"});
        let old_dep = store.create_deployment("test-supervisor".to_string(), task_spec).unwrap();

        // Manually set completed_at to 10 days ago
        let ten_days_ago = (Utc::now() - chrono::Duration::days(10)).to_rfc3339();
        store.db.execute(
            "UPDATE worker_deployments SET status = 'completed', completed_at = ? WHERE id = ?",
            params![ten_days_ago, old_dep.id],
        ).unwrap();

        // Store an artifact for the old deployment
        store
            .store_artifact(&old_dep.id, "old.log".to_string(), ArtifactType::Log, None, b"old".to_vec())
            .unwrap();

        // Create a recent deployment with artifacts
        let new_dep = store.create_deployment("test-supervisor".to_string(), serde_json::json!({"id": "new-task"})).unwrap();
        store
            .store_artifact(&new_dep.id, "new.log".to_string(), ArtifactType::Log, None, b"new".to_vec())
            .unwrap();

        // Clean up artifacts older than 7 days
        let deleted = store.cleanup_old_artifacts(7).unwrap();
        assert_eq!(deleted, 1); // Only the old one should be deleted

        // Verify old is gone, new remains
        assert!(store.list_deployment_artifacts(&old_dep.id).unwrap().is_empty());
        assert_eq!(store.list_deployment_artifacts(&new_dep.id).unwrap().len(), 1);
    }
}
