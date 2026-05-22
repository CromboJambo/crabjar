use crabjar_llm_plug_in::manifest::{WeightManifest, ModelWeightRow, TensorMetadataRow};
use crabjar_safetensors::schema::{query_model_weights, query_tensor_metadata};
use crabjar_safetensors::error::SafetensorsSchemaError;
use crate::error::RunnerError;
use tracing::debug;

/// Model loader that consumes WeightManifest from safetensors DB.
///
/// loads tensors for inference engine consumption.
pub struct ModelLoader {
    pub conn: rusqlite::Connection,
}

impl ModelLoader {
    pub fn new(conn: rusqlite::Connection) -> Self {
        Self { conn }
    }

    /// Load weight manifest for a model from safetensors DB.
    pub fn load_manifest(&self, model_name: &str) -> Result<WeightManifest, RunnerError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, model_name, repo_id, file_path, tensor_count, dtype, device, size_bytes, checksum, metadata, loaded_at, created_at, active FROM model_weights
             WHERE model_name = ?1 AND active = 1
             ORDER BY loaded_at DESC LIMIT 1",
        )?;

        let row = stmt.query_row(rusqlite::params![model_name], |row| {
            let metadata_str: String = row.get(9)?;
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str).unwrap_or_default();

            Ok(crate::plug_in::ModelWeightRow {
                id: row.get(0)?,
                model_name: row.get(1)?,
                repo_id: row.get(2)?,
                file_path: row.get(3)?,
                tensor_count: row.get(4)?,
                dtype: row.get(5)?,
                device: row.get(6)?,
                size_bytes: row.get(7)?,
                checksum: row.get(8)?,
                metadata,
                loaded_at: row.get(10)?,
                created_at: row.get(11)?,
                active: row.get(12)?,
            })
        }).map_err(RunnerError::Sqlite)?;

        let mut tensor_stmt = self.conn.prepare(
            "SELECT id, weight_id, tensor_name, shape, dtype, size_bytes, checksum FROM tensor_metadata
             WHERE weight_id = ?1",
        )?;

        let tensors: Vec(crate::plug_in::TensorMetadataRow) = tensor_stmt
            .query_map(rusqlite::params![row.id], |row| {
                Ok(crate::plug_in::TensorMetadataRow {
                    id: row.get(0)?,
                    weight_id: row.get(1)?,
                    tensor_name: row.get(2)?,
                    shape: row.get(3)?,
                    dtype: row.get(4)?,
                    size_bytes: row.get(5)?,
                    checksum: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let manifest = WeightManifest {
            weight_id: row.id,
            model_name: row.model_name,
            repo_id: row.repo_id,
            file_path: row.file_path,
            tensor_count: row.tensor_count,
            dtype: row.dtype,
            device: row.device,
            size_bytes: row.size_bytes,
            checksum: row.checksum,
            tensors,
            metadata: row.metadata,
            lazy_loading: true,
        };

        debug!(
            model_name = %model_name,
            tensor_count = manifest.tensor_count,
            "Model loader: manifest loaded from safetensors DB"
        );

        Ok(manifest)
    }

    /// Verify weight checksum integrity.
    pub fn verify_checksum(&self, weight_id: &str, expected: &str) -> Result<bool, RunnerError> {
        crabjar_safetensors::schema::verify_weight_checksum(&self.conn, weight_id, expected)
            .map_err(|e: SafetensorsSchemaError| RunnerError::Sqlite(e.into()))
    }

    /// List active weights for model selection.
    pub fn list_active(&self, limit: usize) -> Result<Vec<ModelWeightRow>, RunnerError> {
        crabjar_safetensors::schema::list_active_weights(&self.conn, limit)
            .map_err(|e: SafetensorsSchemaError| RunnerError::Sqlite(e.into()))
    }
}
