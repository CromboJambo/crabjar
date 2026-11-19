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
        store
            .create_deployment("test-supervisor".to_string(), task_spec)
            .unwrap();

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
        store
            .create_deployment("test-supervisor".to_string(), task_spec)
            .unwrap();

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
                assert!(
                    path.exists(),
                    "filesystem artifact should exist at {:?}",
                    path
                );
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
        store
            .create_deployment("test-supervisor".to_string(), task_spec)
            .unwrap();

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
        store
            .create_deployment("test-supervisor".to_string(), task_spec)
            .unwrap();

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
        let old_dep = store
            .create_deployment("test-supervisor".to_string(), task_spec)
            .unwrap();

        // Manually set completed_at to 10 days ago
        let ten_days_ago = (Utc::now() - chrono::Duration::days(10)).to_rfc3339();
        store
            .db
            .execute(
                "UPDATE worker_deployments SET status = 'completed', completed_at = ? WHERE id = ?",
                params![ten_days_ago, old_dep.id],
            )
            .unwrap();

        // Store an artifact for the old deployment
        store
            .store_artifact(
                &old_dep.id,
                "old.log".to_string(),
                ArtifactType::Log,
                None,
                b"old".to_vec(),
            )
            .unwrap();

        // Create a recent deployment with artifacts
        let new_dep = store
            .create_deployment(
                "test-supervisor".to_string(),
                serde_json::json!({"id": "new-task"}),
            )
            .unwrap();
        store
            .store_artifact(
                &new_dep.id,
                "new.log".to_string(),
                ArtifactType::Log,
                None,
                b"new".to_vec(),
            )
            .unwrap();

        // Clean up artifacts older than 7 days
        let deleted = store.cleanup_old_artifacts(7).unwrap();
        assert_eq!(deleted, 1); // Only the old one should be deleted

        // Verify old is gone, new remains
        assert!(
            store
                .list_deployment_artifacts(&old_dep.id)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            store.list_deployment_artifacts(&new_dep.id).unwrap().len(),
            1
        );
    }
}
