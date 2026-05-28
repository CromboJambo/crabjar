use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::Path;

use crabjar_gguf::parser::parse_gguf;
use crabjar_gguf::types::GgufDtype;
use safetensors::tensor::{Dtype, TensorView};
use safetensors::serialize;

/// Result of a GGUF → safetensors conversion.
pub struct GgufConversionResult {
    pub model_name: String,
    pub tensor_count: usize,
    pub total_bytes: u64,
    pub dtype: String,
    pub metadata: HashMap<String, String>,
}

/// Error type for GGUF → safetensors conversion.
#[derive(Debug, thiserror::Error)]
pub enum GgufConvertError {
    #[error("GGUF parse error: {0}")]
    GgufParse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("safetensors serialize error: {0}")]
    Serialize(String),

    #[error("unsupported dtype: {0}")]
    UnsupportedDtype(u32),

    #[error("tensor mismatch: {0}")]
    TensorMismatch(String),
}

/// Map a GGUF dtype to a safetensors Dtype.
fn gguf_dtype_to_safetensors(gguf_dtype: GgufDtype) -> Result<Dtype, GgufConvertError> {
    match gguf_dtype {
        GgufDtype::F32 => Ok(Dtype::F32),
        GgufDtype::F16 => Ok(Dtype::F16),
        GgufDtype::BF16 => Ok(Dtype::BF16),
        GgufDtype::I8 => Ok(Dtype::I8),
        GgufDtype::I16 => Ok(Dtype::I16),
        GgufDtype::I32 => Ok(Dtype::I32),
        GgufDtype::I64 => Ok(Dtype::I64),
        GgufDtype::F64 => Ok(Dtype::F64),
        GgufDtype::Q4_0
        | GgufDtype::Q4_1
        | GgufDtype::Q5_0
        | GgufDtype::Q5_1
        | GgufDtype::Q8_0
        | GgufDtype::Q8_1
        | GgufDtype::Q2_K
        | GgufDtype::Q3_K
        | GgufDtype::Q4_K
        | GgufDtype::Q5_K
        | GgufDtype::Q6_K
        | GgufDtype::Q8_K
        | GgufDtype::Q1_K
        | GgufDtype::Q4_K_M
        | GgufDtype::Q5_K_M
        | GgufDtype::Q6_K_S
        | GgufDtype::Q8_K_M
        | GgufDtype::Q2_K_S
        | GgufDtype::Q3_K_S
        | GgufDtype::Q4_K_S
        | GgufDtype::Q5_K_S
        | GgufDtype::Q2_K_M
        | GgufDtype::Unknown(_) => Err(GgufConvertError::UnsupportedDtype(gguf_dtype.to_u32())),
    }
}

/// Extract all tensors from a GGUF file and write them to a safetensors file.
///
/// Reads tensor data directly from the file at the correct data section offsets
/// (data_section_start + tensor.offset), then serializes to safetensors format.
pub fn convert_gguf_to_safetensors(
    gguf_path: &Path,
    safetensors_path: &Path,
) -> Result<GgufConversionResult, GgufConvertError> {
    let header = parse_gguf(gguf_path).map_err(|e| GgufConvertError::GgufParse(e.to_string()))?;

    let mut metadata = HashMap::new();

    // Populate metadata from KV pairs
    for kv in &header.kv_pairs {
        if let Some(s) = kv.value.as_str() {
            metadata.insert(kv.key.clone(), s.to_string());
        } else if let Some(u) = kv.value.as_u64() {
            metadata.insert(kv.key.clone(), u.to_string());
        }
    }

    let model_name = metadata.get("general.architecture").cloned().unwrap_or_default();

    // Read all tensor data into owned buffers first
    let mut tensor_data: Vec<(String, Vec<u8>, Vec<usize>, Dtype)> =
        Vec::with_capacity(header.tensors.len());
    let mut total_bytes: u64 = 0;
    let mut dtype = String::new();

    for tensor in &header.tensors {
        let safetensors_dtype = gguf_dtype_to_safetensors(GgufDtype::from_u32(tensor.dtype))?;
        let shape: Vec<usize> = tensor.shape.iter().map(|s| *s as usize).collect();
        let stored_size = tensor.stored_size() as usize;

        // Read raw bytes from the file at data_section_start + tensor.offset
        let file_offset = header.data_section_start + tensor.offset;
        let mut file = std::fs::File::open(gguf_path)?;
        let mut buffer = vec![0u8; stored_size];
        file.seek(std::io::SeekFrom::Start(file_offset))?;
        file.read_exact(&mut buffer)?;

        tensor_data.push((tensor.name.clone(), buffer, shape, safetensors_dtype));
        total_bytes += stored_size as u64;
        dtype = safetensors_dtype.to_string();
    }

    // Build TensorViews from owned data and serialize in-memory
    let tensors: Vec<(&str, TensorView)> = tensor_data
        .iter()
        .map(|(name, data, shape, dtype)| {
            let view = TensorView::new(*dtype, shape.clone(), data)
                .map_err(|e| GgufConvertError::TensorMismatch(e.to_string()))
                .unwrap();
            (name.as_str(), view)
        })
        .collect();

    let serialized = serialize(tensors, Some(metadata.clone()))
        .map_err(|e| GgufConvertError::Serialize(e.to_string()))?;

    // Write serialized data to file
    std::fs::write(safetensors_path, serialized)
        .map_err(|e| GgufConvertError::Serialize(e.to_string()))?;

    Ok(GgufConversionResult {
        model_name,
        tensor_count: tensor_data.len(),
        total_bytes,
        dtype,
        metadata,
    })
}

/// Extract a single tensor from a GGUF file and write it as a minimal safetensors file.
pub fn convert_gguf_tensor_to_safetensors(
    gguf_path: &Path,
    safetensors_path: &Path,
    tensor_name: &str,
) -> Result<GgufConversionResult, GgufConvertError> {
    let header = parse_gguf(gguf_path).map_err(|e| GgufConvertError::GgufParse(e.to_string()))?;

    let tensor = header
        .tensors
        .iter()
        .find(|t| t.name == tensor_name)
        .ok_or_else(|| GgufConvertError::TensorMismatch(format!("tensor '{tensor_name}' not found")))?;

    let safetensors_dtype = gguf_dtype_to_safetensors(GgufDtype::from_u32(tensor.dtype))?;
    let shape: Vec<usize> = tensor.shape.iter().map(|s| *s as usize).collect();
    let stored_size = tensor.stored_size() as usize;

    let file_offset = header.data_section_start + tensor.offset;
    let mut file = std::fs::File::open(gguf_path)?;
    let mut buffer = vec![0u8; stored_size];
    file.seek(std::io::SeekFrom::Start(file_offset))?;
    file.read_exact(&mut buffer)?;

    let view = TensorView::new(safetensors_dtype, shape, &buffer)
        .map_err(|e| GgufConvertError::TensorMismatch(e.to_string()))?;

    let serialized = serialize(std::iter::once((tensor_name, view)), None)
        .map_err(|e| GgufConvertError::Serialize(e.to_string()))?;

    std::fs::write(safetensors_path, serialized)
        .map_err(|e| GgufConvertError::Serialize(e.to_string()))?;

    Ok(GgufConversionResult {
        model_name: header.architecture().unwrap_or("unknown").to_string(),
        tensor_count: 1,
        total_bytes: stored_size as u64,
        dtype: safetensors_dtype.to_string(),
        metadata: HashMap::new(),
    })
}
