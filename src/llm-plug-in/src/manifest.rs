use serde::{Deserialize, Serialize};

/// Weight manifest for external LLM runner consumption.
///
/// JSON schema that external runner can consume to load tensors.
/// aligns with safetensors lazy_loading=true concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightManifest {
    pub weight_id: String,
    pub model_name: String,
    pub repo_id: String,
    pub file_path: String,
    pub tensor_count: i32,
    pub dtype: String,
    pub device: String,
    pub size_bytes: i64,
    pub checksum: String,
    pub tensors: Vec<TensorMetadataRow>,
    pub metadata: serde_json::Value,
    pub lazy_loading: bool,
}

/// A single tensor metadata row for manifest output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorMetadataRow {
    pub id: String,
    pub weight_id: String,
    pub tensor_name: String,
    pub shape: String,
    pub dtype: String,
    pub size_bytes: i64,
    pub checksum: String,
}

/// A single model weight row for manifest queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelWeightRow {
    pub id: String,
    pub model_name: String,
    pub repo_id: String,
    pub file_path: String,
    pub tensor_count: i32,
    pub dtype: String,
    pub device: String,
    pub size_bytes: i64,
    pub checksum: String,
    pub metadata: serde_json::Value,
    pub loaded_at: i64,
    pub created_at: i64,
    pub active: i32,
}
