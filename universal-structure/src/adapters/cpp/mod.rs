//! C++ adapter for universal structural token representation.
//!
//! Uses regex-based pattern matching to identify key C++ constructs and map them
//! to universal `SpanKind` variants. This follows the same approach as the JavaScript,
//! Python, and C adapters, demonstrating the pluggable adapter architecture without
//! requiring a full C++ parser.

mod adapter;
mod patterns;
mod tokenizer;

/// Simplified C++ token kind for internal use.
#[derive(Debug, Clone, PartialEq)]
pub enum CppTok {
    Keyword(String),
    Identifier(String),
    Punctuation(String),
    StringLiteral(String),
    CharLiteral(char),
    NumberLiteral(String),
    Preprocessor(String),
}

// Re-export the adapter for use via adapters::cpp::CppAdapter
pub use adapter::CppAdapter;
