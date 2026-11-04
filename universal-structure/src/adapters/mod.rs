//! Language-specific adapters for converting native tokenizer output
//! to universal structural tokens.

pub mod binary;
pub mod c;
pub mod cpp;
pub mod go;
pub mod javascript;
pub mod lean;
pub mod python;
pub mod rust;

// Internal pattern detection and keyword lists (not public API)
mod c_keywords;
mod c_patterns;
mod cpp_keywords;
mod cpp_patterns;
mod go_keywords;
mod go_patterns;

// Re-export adapter types for convenience
pub use self::binary::BinaryAdapter;
pub use self::c::CAdapter;
pub use self::cpp::CppAdapter;
pub use self::go::GoAdapter;
pub use self::javascript::JavaScriptAdapter;
pub use self::lean::LeanAdapter;
pub use self::python::PythonAdapter;
pub use self::rust::RustAdapter;
