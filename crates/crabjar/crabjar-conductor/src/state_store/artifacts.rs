//! Deployment artifact management.
//!
//! Captures, stores, and retrieves output artifacts from completed deployments
//! (logs, results, generated files). Supports both `SQLite` BLOB storage for small
//! artifacts (< 1MB) and filesystem paths for large files.

use chrono::Utc;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use super::StateStore;

/// Maximum size for `SQLite` BLOB storage (1MB). Larger artifacts are stored on disk.
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

    #[must_use]
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
    /// Stored as BLOB in `SQLite` (small artifacts).
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
        let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

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
                    Box::new(std::io::Error::other(e)),
                )
            })?;
            let file_path = artifacts_dir.join(format!("{id}.artifact"));

            std::fs::write(&file_path, &data).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::other(e)),
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
                content_type: Some(content_type),
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
        if let Some(artifact) = self.get_artifact(id)?
            && let ArtifactStorage::Path(path) = &artifact.storage
        {
            let _ = std::fs::remove_file(path);
        }

        self.db
            .execute("DELETE FROM deployment_artifacts WHERE id = ?", params![id])?;
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
            let rows = self.db.execute(
                "DELETE FROM deployment_artifacts WHERE id = ?",
                params![artifact.id],
            )?;
            deleted += rows as i64;
        }

        Ok(deleted)
    }

    /// Clean up artifacts older than the given retention period (in days).
    pub fn cleanup_old_artifacts(&self, retention_days: u32) -> Result<i64, rusqlite::Error> {
        let cutoff = Utc::now() - chrono::Duration::days(i64::from(retention_days));
        let cutoff_str = cutoff.to_rfc3339();

        // Find completed deployments older than cutoff
        let mut stmt = self.db.prepare(
            "SELECT id FROM worker_deployments WHERE completed_at IS NOT NULL AND completed_at < ?",
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
        let parent = self.db.path().map_or_else(
            || std::path::Path::new("."),
            |p| {
                std::path::Path::new(p)
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
            },
        );
        parent.join("artifacts")
    }
}

#[cfg(test)]
#[path = "artifacts_tests.rs"]
mod artifacts_tests;
