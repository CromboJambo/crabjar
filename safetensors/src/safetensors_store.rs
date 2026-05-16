use crate::error::SafetensorsError;
use crate::schema::{
    deactivate_weight, init_db, insert_model_weights, insert_tensor_metadata, list_active_weights,
    query_model_weights, query_tensor_metadata, verify_weight_checksum,
};
use path_absolutize::Absolutize;
use rusqlite::Connection;
use std::path::Path;
use tracing::{debug, info, warn};

/// Safetensors model weight storage for SQLite-backed storage.
///
/// Provides safe weight loading, zero-copy/lazy loading, and avoiding pickle-style code execution.
/// Uses safetensors under the PyTorch Foundation for safe model serialization.
pub struct SafetensorsStore<'a> {
    conn: &'a Connection,
}

impl<'a> SafetensorsStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Initialize the safetensors database.
    pub fn init(&self) -> Result<(), SafetensorsError> {
        init_db(self.conn).map_err(SafetensorsError::Schema)
    }

    /// Insert model weights metadata.
    pub fn insert_weights(
        &self,
        model_name: &str,
        repo_id: &str,
        file_path: &str,
        tensor_count: i32,
        dtype: &str,
        device: &str,
        size_bytes: i64,
        checksum: &str,
        metadata: &str,
    ) -> Result<String, SafetensorsError> {
        insert_model_weights(
            self.conn,
            model_name,
            repo_id,
            file_path,
            tensor_count,
            dtype,
            device,
            size_bytes,
            checksum,
            metadata,
        )
        .map_err(SafetensorsError::Schema)
    }

    /// Insert tensor metadata for a weight.
    pub fn insert_tensor_metadata(
        &self,
        weight_id: &str,
        tensor_name: &str,
        shape: &str,
        dtype: &str,
        size_bytes: i64,
        checksum: &str,
    ) -> Result<(), SafetensorsError> {
        insert_tensor_metadata(
            self.conn,
            weight_id,
            tensor_name,
            shape,
            dtype,
            size_bytes,
            checksum,
        )
        .map_err(SafetensorsError::Schema)
    }

    /// Query model weights by name.
    pub fn query_weights(
        &self,
        model_name: &str,
        limit: usize,
    ) -> Result<Vec<crate::schema::ModelWeightRow>, SafetensorsError> {
        query_model_weights(self.conn, model_name, limit).map_err(SafetensorsError::Schema)
    }

    /// Query tensor metadata for a weight.
    pub fn query_tensors(
        &self,
        weight_id: &str,
    ) -> Result<Vec<crate::schema::TensorMetadataRow>, SafetensorsError> {
        query_tensor_metadata(self.conn, weight_id).map_err(SafetensorsError::Schema)
    }

    /// Verify weight checksum integrity.
    pub fn verify_checksum(
        &self,
        weight_id: &str,
        expected: &str,
    ) -> Result<bool, SafetensorsError> {
        verify_weight_checksum(self.conn, weight_id, expected).map_err(SafetensorsError::Schema)
    }

    /// Deactivate a model weight.
    pub fn deactivate(&self, weight_id: &str) -> Result<usize, SafetensorsError> {
        deactivate_weight(self.conn, weight_id).map_err(SafetensorsError::Schema)
    }

    /// List all active model weights.
    pub fn list_active(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::schema::ModelWeightRow>, SafetensorsError> {
        list_active_weights(self.conn, limit).map_err(SafetensorsError::Schema)
    }

    /// Verify safetensors file path existence.
    pub fn verify_file_path(&self, file_path: &str) -> Result<bool, SafetensorsError> {
        let abs_path = Path::new(file_path).absolutize()?;
        Ok(abs_path.exists())
    }

    /// Generate safetensors load configuration.
    pub fn generate_load_config(
        &self,
        model_name: &str,
        dtype: &str,
        device: &str,
    ) -> Result<String, SafetensorsError> {
        let mut config = String::new();
        config.push_str(&format!("model = {}\n", model_name));
        config.push_str(&format!("dtype = {}\n", dtype));
        config.push_str(&format!("device = {}\n", device));
        config.push_str("format = safetensors\n");
        config.push_str("lazy_loading = true\n");

        debug!(
            model_name = %model_name,
            "Safetensors store: load config generated"
        );

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_safetensors_store_init() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("safetensors.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let store = SafetensorsStore::new(&conn);
        store.init().unwrap();

        let rows = store.list_active(10).unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn test_insert_and_query_weights() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("safetensors.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let store = SafetensorsStore::new(&conn);
        store.init().unwrap();

        let id = store
            .insert_weights(
                "qwen3-4b",
                "Qwen/Qwen3-4B",
                "/tmp/model.safetensors",
                150,
                "F32",
                "CPU",
                2000000000,
                "abc123",
                "{}",
            )
            .unwrap();

        let rows = store.query_weights("qwen3-4b", 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].model_name, "qwen3-4b");
    }

    #[test]
    fn test_verify_checksum() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("safetensors.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let store = SafetensorsStore::new(&conn);
        store.init().unwrap();

        let id = store
            .insert_weights(
                "qwen3-4b",
                "Qwen/Qwen3-4B",
                "/tmp/model.safetensors",
                150,
                "F32",
                "CPU",
                2000000000,
                "abc123",
                "{}",
            )
            .unwrap();

        let verified = store.verify_checksum(&id, "abc123").unwrap();
        assert!(verified);

        let verified = store.verify_checksum(&id, "wrong").unwrap();
        assert!(!verified);
    }

    #[test]
    fn test_deactivate_weight() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("safetensors.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let store = SafetensorsStore::new(&conn);
        store.init().unwrap();

        let id = store
            .insert_weights(
                "qwen3-4b",
                "Qwen/Qwen3-4B",
                "/tmp/model.safetensors",
                150,
                "F32",
                "CPU",
                2000000000,
                "abc123",
                "{}",
            )
            .unwrap();

        let affected = store.deactivate(&id).unwrap();
        assert_eq!(affected, 1);

        let rows = store.query_weights("qwen3-4b", 10).unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn test_generate_load_config() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("safetensors.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let store = SafetensorsStore::new(&conn);
        store.init().unwrap();

        let config = store
            .generate_load_config("qwen3-4b", "F32", "CPU")
            .unwrap();

        assert!(config.contains("model = qwen3-4b"));
        assert!(config.contains("format = safetensors"));
        assert!(config.contains("lazy_loading = true"));
    }
}
