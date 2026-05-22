//! crabjar-llm-runner: LLM inference engine for tensor computation and model loading.
//!
//! Separate workspace member that eventually becomes independent.
//! Interface boundary: consumes WeightManifest from safetensors, emits InferenceResponse to guard.
//!
//! ## Modules
//!
//! - `model-loader`: consumes WeightManifest JSON → loads tensors
//! - `inference-engine`: actual tensor computation
//! - `tokenizer`: prompt encoding
//! - `device`: CUDA/CPU/MKL backend selection
//! - `runner`: external runner bridge (endpoint/protocol)
//! - `plug-in`: implements InferenceRequest/Response protocol

pub mod model_loader;
pub mod inference_engine;
pub mod tokenizer;
pub mod device;
pub mod runner;
pub mod plug_in;
pub mod error;

pub use error::{RunnerError, Result};
pub use model_loader::ModelLoader;
pub use inference_engine::InferenceEngine;
pub use tokenizer::Tokenizer;
pub use device::DeviceBackend;
pub use runner::RunnerBridge;
pub use plug_in::PlugInProtocol;
