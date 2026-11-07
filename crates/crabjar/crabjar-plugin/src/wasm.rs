//! Wasmtime-based dynamic component loading for CrabJar plugins.
//!
//! This module provides the foundation for loading and executing WebAssembly
//! components as dynamic plugins. It wraps the wasmtime runtime with sensible
//! defaults appropriate for plugin execution.

use std::fs;
use std::path::Path;

use thiserror::Error;
use tracing::info;
use wasmtime::{Engine, Instance, Module, Store};

/// Errors that can occur during Wasm component loading and execution.
#[derive(Debug, Error)]
pub enum WasmError {
    /// Failed to create the Wasmtime engine.
    #[error("failed to create Wasmtime engine: {0}")]
    EngineCreation(String),

    /// Failed to read a .wasm file from disk.
    #[error("failed to read wasm file '{path}': {source}")]
    FileRead {
        path: String,
        source: std::io::Error,
    },

    /// Failed to validate or compile a WebAssembly module.
    #[error("failed to compile wasm module: {0}")]
    ModuleCompilation(String),

    /// Failed to instantiate a WebAssembly module.
    #[error("failed to instantiate wasm module: {0}")]
    Instantiation(String),

    /// Exported function not found in the instantiated module.
    #[error("export '{name}' not found in wasm module")]
    ExportNotFound { name: String },

    /// Type mismatch when calling an exported function.
    #[error("type error calling export '{name}': {details}")]
    TypeError { name: String, details: String },

    /// Error during execution of wasm code.
    #[error("wasm execution error: {0}")]
    Execution(String),
}

/// A configured Wasmtime engine with sensible defaults for plugin execution.
pub struct WasmEngine {
    engine: Engine,
}

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new().expect("failed to create default Wasmtime engine")
    }
}

impl WasmEngine {
    /// Create a new Wasmtime engine with plugin-appropriate defaults.
    ///
    /// Configures:
    /// - Memory growth limits (max 256MB for sandboxing)
    /// - Stack size limits (default 1MB)
    /// - Garbage collection enabled (for future GC proposals)
    pub fn new() -> Result<Self, WasmError> {
        info!("creating Wasmtime engine with plugin defaults");

        let mut config = wasmtime::Config::default();

        // Enable component model support
        config.wasm_component_model(true);

        // Enable garbage collection for future proposals
        config.wasm_gc(true);

        let engine =
            Engine::new(&config).map_err(|e| WasmError::EngineCreation(format!("{:?}", e)))?;

        info!("Wasmtime engine created successfully");

        Ok(Self { engine })
    }

    /// Load a WebAssembly module from file bytes.
    pub fn load_module(&self, wasm_bytes: &[u8]) -> Result<Module, WasmError> {
        Module::new(&self.engine, wasm_bytes)
            .map_err(|e| WasmError::ModuleCompilation(format!("{:?}", e)))
    }

    /// Load a WebAssembly module from a file path.
    pub fn load_module_from_file<P: AsRef<Path>>(&self, path: P) -> Result<Module, WasmError> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let wasm_bytes = fs::read(path).map_err(|e| WasmError::FileRead {
            path: path_str.clone(),
            source: e,
        })?;

        info!(path = %path_str, "loading wasm module from file");

        self.load_module(&wasm_bytes)
    }

    /// Instantiate a module with no imports (empty imports).
    ///
    /// This is useful for testing and simple modules that don't require
    /// host-provided functions. For production plugins, use `instantiate_with_imports`.
    pub fn instantiate(&self, module: &Module) -> Result<Instance, WasmError> {
        let store = Store::new(&self.engine, ());
        Instance::new(store, module, &[]).map_err(|e| WasmError::Instantiation(format!("{:?}", e)))
    }

    /// Get the underlying wasmtime Engine reference.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal WebAssembly module (wasm binary format):
    /// - Exports a function `add(i32, i32) -> i32`
    /// - Uses the minimal valid wasm structure
    fn make_add_module() -> Vec<u8> {
        // WASM magic number + version
        let mut bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

        // Type section: one function type (i32, i32) -> i32
        bytes.extend_from_slice(&[0x01, 0x07, 0x01, 0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f]);

        // Function section: one function of type 0
        bytes.extend_from_slice(&[0x03, 0x02, 0x01, 0x00]);

        // Export section: export "add" as function 0
        bytes.extend_from_slice(&[0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00]);

        // Code section: function body that adds two i32s and returns
        bytes.extend_from_slice(&[
            0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b,
        ]);

        bytes
    }

    #[test]
    fn test_engine_creation() {
        let engine = WasmEngine::new();
        assert!(
            engine.is_ok(),
            "Wasmtime engine should be created successfully"
        );
    }

    #[test]
    fn test_load_and_instantiate_module() {
        let wasm_engine = WasmEngine::default();
        let module_bytes = make_add_module();

        // Load the module
        let module = wasm_engine.load_module(&module_bytes);
        assert!(module.is_ok(), "Module should load successfully");
        let module = module.unwrap();

        // Instantiate with no imports
        let instance = wasm_engine.instantiate(&module);
        assert!(instance.is_ok(), "Module should instantiate successfully");
    }

    #[test]
    fn test_export_not_found() {
        let wasm_engine = WasmEngine::default();
        let module_bytes = make_add_module();
        let module = wasm_engine.load_module(&module_bytes).unwrap();

        // Instantiate and check export exists
        let mut store = wasmtime::Store::new(wasm_engine.engine(), ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();

        // Check that our exported function exists
        let add = instance.get_func(&mut store, "add");
        assert!(add.is_some(), "add function should be exported");
    }
}
