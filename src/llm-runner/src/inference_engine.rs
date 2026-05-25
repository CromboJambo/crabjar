use crate::error::RunnerError;
use candle_core::{DType, Device, Tensor};
use candle_nn::Module;

/// Inference engine for tensor computation.
///
/// actual tensor computation layer. separate from crabjar host.
pub struct InferenceEngine {
    pub device: candle_core::Device,
    pub dtype: DType,
}

impl InferenceEngine {
    pub fn new(device: Device, dtype: DType) -> Self {
        Self { device, dtype }
    }

    /// Run inference on a loaded model.
    pub fn infer(&self, model: &impl Module, input: Tensor) -> Result<Tensor, RunnerError> {
        model
            .forward(&input)
            .map_err(|e: candle_core::Error| RunnerError::Tensor(e.to_string()))
    }

    /// Materialize lazy-loaded tensor from manifest.
    pub fn materialize_tensor(
        &self,
        file_path: &str,
        _tensor_name: &str,
    ) -> Result<Tensor, RunnerError> {
        let data = std::fs::read(file_path)
            .map_err(|e: std::io::Error| RunnerError::Asset(e.to_string()))?;
        Tensor::from_raw_buffer(&data, self.dtype, &[1], &self.device)
            .map_err(|e: candle_core::Error| RunnerError::Tensor(e.to_string()))
    }

    /// Get device info.
    pub fn device_info(&self) -> Result<String, RunnerError> {
        Ok(match &self.device {
            Device::Cpu => "cpu".to_string(),
            Device::Cuda(ordinal) => format!("cuda:{ordinal:?}"),
            Device::Metal(_) => "metal".to_string(),
        })
    }

    /// Get dtype info.
    pub fn dtype_info(&self) -> Result<String, RunnerError> {
        Ok(match self.dtype {
            DType::F32 => "F32".to_string(),
            DType::F16 => "F16".to_string(),
            DType::I64 => "I64".to_string(),
            DType::I32 => "I32".to_string(),
            DType::U8 => "U8".to_string(),
            _ => "unknown".to_string(),
        })
    }
}
