use crate::error::SafetensorsError;
use crate::schema::{
    deactivate_weight, init_db, insert_model_weights, insert_tensor_metadata, list_active_weights,
    query_model_weights, query_tensor_metadata, verify_weight_checksum,
};
use path_absolutize::Absolutize;
use rusqlite::Connection;
use sha2::Digest;
use std::path::Path;
use tracing::debug;

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
    #[allow(clippy::too_many_arguments)]
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

    /// Parse a safetensors file and extract tensor metadata.
    pub fn parse_weights(
        &self,
        file_path: &str,
        model_name: &str,
        repo_id: &str,
    ) -> Result<(String, Vec<crate::schema::TensorMetadataRow>), SafetensorsError> {
        let abs_path = Path::new(file_path).absolutize()?;
        if !abs_path.exists() {
            return Err(SafetensorsError::NotFound(file_path.to_string()));
        }

        let data = std::fs::read(&abs_path)?;
        if data.len() < 8 {
            return Err(SafetensorsError::Internal(
                "too short for safetensors header".to_string(),
            ));
        }

        let header_len =
            u64::from_le_bytes(data[0..8].try_into().map_err(|_| {
                SafetensorsError::Internal("header length not 8 bytes".to_string())
            })?);
        if header_len as usize > data.len() {
            return Err(SafetensorsError::Internal(
                "header exceeds file".to_string(),
            ));
        }

        let header_bytes = &data[8..8 + header_len as usize];
        let header_str = String::from_utf8(header_bytes.to_vec())
            .map_err(|_| SafetensorsError::Internal("header not valid UTF-8".to_string()))?;
        let header: serde_json::Value = serde_json::from_str(&header_str)
            .map_err(|_| SafetensorsError::Internal("header not valid JSON".to_string()))?;

        let tensors = header
            .get("tensors")
            .and_then(|t| t.as_object())
            .ok_or_else(|| SafetensorsError::Internal("no tensors in header".to_string()))?;

        let mut tensor_rows = Vec::new();
        let mut total_tensors: i32 = 0;
        let mut total_bytes = 0i64;
        let mut dtype = String::new();

        for (tensor_name, tensor_info) in tensors {
            let shape: Vec<i64> = tensor_info
                .get("shape")
                .and_then(|s| s.as_array())
                .ok_or_else(|| SafetensorsError::Internal("tensor shape missing".to_string()))?
                .iter()
                .filter_map(|v| v.as_i64())
                .collect();

            let dtype_str = tensor_info
                .get("dtype")
                .and_then(|d| d.as_str())
                .ok_or_else(|| SafetensorsError::Internal("tensor dtype missing".to_string()))?
                .to_string();

            let _data_type = tensor_info
                .get("data_type")
                .and_then(|d| d.as_str())
                .unwrap_or(dtype_str.as_str());

            let shape_str = format!(
                "({})",
                shape
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            tensor_rows.push(crate::schema::TensorMetadataRow {
                id: uuid::Uuid::new_v4().to_string(),
                weight_id: String::new(),
                tensor_name: tensor_name.clone(),
                shape: shape_str,
                dtype: dtype_str.clone(),
                size_bytes: tensor_info
                    .get("data_type")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .len() as i64,
                checksum: String::new(),
            });

            total_tensors += 1;
            total_bytes += tensor_info
                .get("data_type")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .len() as i64;
            dtype = dtype_str;
        }

        let weight_id = self.insert_weights(
            model_name,
            repo_id,
            file_path,
            total_tensors,
            dtype.as_str(),
            "CPU",
            total_bytes,
            &hex::encode(sha2::Sha256::new().finalize().as_slice()),
            "{}",
        )?;

        for row in &mut tensor_rows {
            row.weight_id = weight_id.clone();
        }

        debug!(
            model_name = %model_name,
            tensor_count = total_tensors,
            "Safetensors store: weights parsed from file"
        );

        Ok((weight_id, tensor_rows))
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
