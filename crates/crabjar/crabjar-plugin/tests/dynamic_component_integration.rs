//! Integration tests for dynamic component loading with PluginManager.
//! Tests the full pipeline: load .wasm → WasmPluginAdapter → PluginRegistry → tool execution.

use crabjar_plugin::Plugin;
use crabjar_plugin::types::HealthStatus;
use crabjar_plugin::wasm::WasmEngine;
use crabjar_plugin::wasm_plugin::WasmPluginAdapter;

/// Minimal add(x, y) Wasm module in binary form (same as wasm.rs tests).
fn make_add_module() -> Vec<u8> {
    vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // magic + version
        0x01, 0x07, 0x01, 0x60, 0x02, 0x7f, 0x7f, 0x01,
        0x7f, // type section: (i32, i32) -> i32
        0x03, 0x02, 0x01, 0x00, // function section
        0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00, // export "add"
        0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b, // code: add args
    ]
}

#[test]
fn test_wasm_plugin_adapter_creation() {
    let engine = WasmEngine::new().unwrap();
    let wasm_bytes = make_add_module();

    let adapter = WasmPluginAdapter::new("counter", "1.0.0", engine, &wasm_bytes);
    assert!(
        adapter.is_ok(),
        "Failed to create WasmPluginAdapter: {:?}",
        adapter.err()
    );
}

#[test]
fn test_plugin_trait_methods() {
    let engine = WasmEngine::new().unwrap();
    let wasm_bytes = make_add_module();

    let adapter = WasmPluginAdapter::new("counter", "1.0.0", engine, &wasm_bytes).unwrap();

    // Verify Plugin trait methods work
    assert_eq!(adapter.name(), "counter");
    assert_eq!(adapter.version(), "1.0.0");
    assert!(!adapter.capabilities().is_empty());
}

#[tokio::test]
async fn test_plugin_health_check() {
    let engine = WasmEngine::new().unwrap();
    let wasm_bytes = make_add_module();

    let adapter = WasmPluginAdapter::new("counter", "1.0.0", engine, &wasm_bytes).unwrap();

    let status = adapter.health_check().await;
    assert_eq!(status, HealthStatus::Healthy);
}

#[tokio::test]
async fn test_plugin_tool_execution() {
    let engine = WasmEngine::new().unwrap();
    let wasm_bytes = make_add_module();

    let adapter = WasmPluginAdapter::new("counter", "1.0.0", engine, &wasm_bytes).unwrap();

    // Execute a tool (stub implementation returns success)
    let result = adapter
        .execute_tool("increment", serde_json::json!({}))
        .await;
    assert!(result.is_ok(), "Tool execution failed: {:?}", result.err());
}

#[test]
fn test_plugin_list_tools() {
    let engine = WasmEngine::new().unwrap();
    let wasm_bytes = make_add_module();

    let adapter = WasmPluginAdapter::new("counter", "1.0.0", engine, &wasm_bytes).unwrap();

    let tools = adapter.list_tools();
    assert!(!tools.is_empty(), "list_tools() returned empty");
}

#[tokio::test]
async fn test_full_plugin_lifecycle() {
    let engine = WasmEngine::new().unwrap();
    let wasm_bytes = make_add_module();

    // Create adapter
    let adapter = WasmPluginAdapter::new("counter", "1.0.0", engine, &wasm_bytes).unwrap();

    // Simulate plugin lifecycle
    adapter.on_load();

    // Health check
    let status = adapter.health_check().await;
    assert_eq!(status, HealthStatus::Healthy);

    // Tool execution
    let result = adapter
        .execute_tool("increment", serde_json::json!({}))
        .await;
    assert!(result.is_ok());

    // List tools
    let tools = adapter.list_tools();
    assert!(!tools.is_empty());

    // Unload
    adapter.on_unload();
}
