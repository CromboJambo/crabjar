//! Go adapter for universal structural token representation.
//!
//! Uses regex-based pattern matching to identify key Go constructs and map them
//! to universal `SpanKind` variants. This follows the same approach as the JavaScript,
//! Python, C, and C++ adapters, demonstrating the pluggable adapter architecture without
//! requiring a full Go parser.

mod adapter;
mod patterns;
mod tokenizer;

/// Simplified Go token kind for internal use.
#[derive(Debug, Clone, PartialEq)]
pub enum GoTok {
    Keyword(String),
    Identifier(String),
    Punctuation(String),
    StringLiteral(String),
    CharLiteral(char),
    NumberLiteral(String),
    Comment(String),
}

// Re-export the adapter for use via adapters::go::GoAdapter
pub use adapter::GoAdapter;

#[cfg(test)]
mod tests {
    use crate::{LanguageAdapter, SpanKind};

    use super::GoAdapter;

    #[test]
    fn test_go_adapter_name() {
        let adapter = GoAdapter::new();
        assert_eq!(adapter.name(), "go");
    }

    #[test]
    fn test_tokenize_package_declaration() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("package main");

        let has_module = tree.spans.iter().any(|s| s.kind == SpanKind::ModuleDecl);
        assert!(has_module);
    }

    #[test]
    fn test_tokenize_import_statement() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("import \"fmt\"");

        let has_import = tree
            .spans
            .iter()
            .any(|s| s.kind == SpanKind::ImportStatement);
        assert!(has_import);
    }

    #[test]
    fn test_tokenize_function_definition() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("func main() {}");

        let has_func = tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        assert!(has_func);
    }

    #[test]
    fn test_tokenize_struct_definition() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("type Person struct { Name string }");

        let has_type = tree.spans.iter().any(|s| s.kind == SpanKind::ClassDecl);
        assert!(has_type);
    }

    #[test]
    fn test_tokenize_interface_definition() {
        let adapter = GoAdapter::new();
        let tree =
            adapter.tokenize_and_adapt("type Reader interface { Read([]byte) (int, error) }");

        let has_interface = tree.spans.iter().any(|s| s.kind == SpanKind::TraitItem);
        assert!(has_interface);
    }

    #[test]
    fn test_tokenize_control_flow() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("if x > 0 { return true } else { return false }");

        let has_if = tree.spans.iter().any(|s| s.kind == SpanKind::IfStmt);
        assert!(has_if);
    }

    #[test]
    fn test_tokenize_for_loop() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("for i := 0; i < 10; i++ {}");

        let has_for = tree.spans.iter().any(|s| s.kind == SpanKind::ForLoop);
        assert!(has_for);
    }

    #[test]
    fn test_tokenize_range_loop() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("for _, item := range items {}");

        let has_range = tree.spans.iter().any(|s| s.kind == SpanKind::ForLoop);
        assert!(has_range);
    }

    #[test]
    fn test_cross_language_compatibility() {
        let go_adapter = GoAdapter::new();
        let js_adapter = crate::adapters::javascript::JavaScriptAdapter::new();

        // Both should detect functions
        let go_tree = go_adapter.tokenize_and_adapt("func hello() {}");
        let js_tree = js_adapter.tokenize_and_adapt("function hello() {}");

        assert!(go_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
        assert!(js_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
    }
}
