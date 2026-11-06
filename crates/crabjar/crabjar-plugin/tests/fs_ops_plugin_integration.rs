//! Integration tests for the file operations Wasm plugin.
//! Validates loading and executing the compiled fs_ops_plugin.wasm through
//! the full PluginManager → WasmPluginAdapter pipeline.

use std::path::Path;

use crabjar_plugin::Plugin;
use crabjar_plugin::types::HealthStatus;
use crabjar_plugin::wasm::WasmEngine;
use crabjar_plugin::wasm_plugin::WasmPluginAdapter;

const FS_OPS_WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/fs_ops/target/wasm32-unknown-unknown/release/fs_ops_plugin.wasm"
);

#[test]
fn test_fs_ops_plugin_loads() {
    let engine = WasmEngine::new().unwrap();
    
    // Load the compiled fs_ops plugin from file
    let wasm_path = Path::new(FS_OPS_WASM);
    let adapter = WasmPluginAdapter::from_file("fs_ops", "0.1.0", engine, wasm_path);
    assert!(adapter.is_ok(), "Failed to load fs_ops plugin: {:?}", adapter.err());
}

#[test]
fn test_fs_ops_list_tools() {
    let engine = WasmEngine::new().unwrap();
    let wasm_path = Path::new(FS_OPS_WASM);
    let adapter = WasmPluginAdapter::from_file("fs_ops", "0.1.0", engine, wasm_path).unwrap();
    
    let tools = adapter.list_tools();
    assert!(!tools.is_empty(), "list_tools() returned empty");
}

#[tokio::test]
async fn test_fs_ops_exists_tool() {
    let engine = WasmEngine::new().unwrap();
    let wasm_path = Path::new(FS_OPS_WASM);
    let adapter = WasmPluginAdapter::from_file("fs_ops", "0.1.0", engine, wasm_path).unwrap();
    
    // Test exists tool with a known file (the wasm file itself)
    let result = adapter.execute_tool("exists", serde_json::json!({
        "path": FS_OPS_WASM
    })).await;
    
    assert!(result.is_ok(), "execute_tool failed: {:?}", result.err());
}

#[tokio::test]
async fn test_fs_ops_read_file_tool() {
    let engine = WasmEngine::new().unwrap();
    let wasm_path = Path::new(FS_OPS_WASM);
    let adapter = WasmPluginAdapter::from_file("fs_ops", "0.1.0", engine, wasm_path).unwrap();
    
    // Test read_file with the WIT interface file
    let wit_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/fs_ops/plugin.wit");
    let result = adapter.execute_tool("read_file", serde_json::json!({
        "path": wit_path
    })).await;
    
    assert!(result.is_ok(), "execute_tool failed: {:?}", result.err());
}

#[tokio::test]
async fn test_fs_ops_health_check() {
    let engine = WasmEngine::new().unwrap();
    let wasm_path = Path::new(FS_OPS_WASM);
    let adapter = WasmPluginAdapter::from_file("fs_ops", "0.1.0", engine, wasm_path).unwrap();
    
    let status = adapter.health_check().await;
    assert_eq!(status, HealthStatus::Healthy);
}

#[tokio::test]
async fn test_fs_ops_full_lifecycle() {
    let engine = WasmEngine::new().unwrap();
    let wasm_path = Path::new(FS_OPS_WASM);
    let adapter = WasmPluginAdapter::from_file("fs_ops", "0.1.0", engine, wasm_path).unwrap();
    
    // Full lifecycle: load → health check → list tools → execute tool → unload
    adapter.on_load();
    
    let status = adapter.health_check().await;
    assert_eq!(status, HealthStatus::Healthy);
    
    let tools = adapter.list_tools();
    assert!(!tools.is_empty());
    
    let result = adapter.execute_tool("exists", serde_json::json!({
        "path": FS_OPS_WASM
    })).await;
    assert!(result.is_ok());
    
    adapter.on_unload();
}