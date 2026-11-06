//! Plugin system for CrabJar — trait definitions, process pool, and lifecycle management.
//!
//! ## Three-Tier Model
//!
//! 1. **ToolServer** (subprocess) — heavy workloads, external binaries
//! 2. **Script** (in-process) — lightweight automation via Rhai/Lua
//! 3. **Reserved** (WASM) — sandboxed execution outside editor
//!
//! This crate implements Tier 1 (ToolServer). Tiers 2-3 are stubbed for future implementation.

mod context;
pub mod plugin_trait;
pub mod process_pool;
mod process_types;
pub mod registry;
pub mod types;
pub mod wasm;
pub mod bindings;
pub mod wasm_plugin;

// Re-export main types at top level for backward compatibility
pub use plugin_trait::*;
pub use process_pool::*;
pub use registry::*;
pub use types::*;
pub use wasm::WasmEngine;
pub use wasm_plugin::WasmPluginAdapter;
