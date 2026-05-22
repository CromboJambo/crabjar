use candle_core::{Tensor, DType, Device};
use candle_core::op::Op;
use candle_nn::Module;
use crate::error::RunnerError;
use tracing::debug;

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
        model.forward(&input).map_err(RunnerError::Tensor)
    }

    /// Materialize lazy-loaded tensor from manifest.
    pub fn materialize_tensor(&self, file_path: &str, tensor_name: &str) -> Result<Tensor, RunnerError> {
        let data = std::fs::read(file_path).map_err(RunnerError::Asset)?;
        Tensor::from_raw_buffer(&data, self.dtype, &[1], &self.device).map_err(RunnerError::Tensor)
    }

    /// Compute tensor operations.
    pub fn compute(&self, a: Tensor, b: Tensor, op: Op) -> Result<Tensor, RunnerError> {
        op.apply(&a, &b).map_err(RunnerError::Tensor)
    }

    /// Get device info.
    pub fn device_info(&self) -> Result<String, RunnerError> {
        Ok(match self.device {
            Device::Cpu => "cpu".to_string(),
            Device::Cuda(ordinal) => format!("cuda:{ordinal}"),
        })
    }

    pub fn dtype_info(&self) -> Result<String, RunnerError> {
        Ok(match self.dtype {
            DType::F32 => "F32".to_string(),
            DType::F16 => "F16".to_string(),
            DType::I64 => "I64".to_string(),
            DType::I32 => "I32".to_string(),
            DType::I8 => "I8".to_string(),
            DType::U8 => "U8".to_string(),
            _ => "unknown".to_string(),
        })
    }

    /// Get dtype info.
    pub fn dtype_info(&self) -> Result<String, RunnerError> {
        Ok(self.dtype.to_string())
    }
}
