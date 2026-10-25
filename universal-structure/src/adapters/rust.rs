//! Rust adapter: uses pesti-structural-tokenizer to build universal SpanTree.
//!
//! This is a full LanguageAdapter implementation that leverages the mature
//! pesti-structural-tokenizer for accurate Rust parsing, then maps the results
//! to the universal structural token representation with language-specific tags.

use crate::{LanguageAdapter, Span, SpanKind, SpanTree};
use pesti_structural_tokenizer::{StructuralTokenizer, TokenKind};
use std::collections::HashMap;

/// Rust adapter implementing the LanguageAdapter trait using pesti-structural-tokenizer.
pub struct RustAdapter {
    tokenizer: StructuralTokenizer,
}

impl RustAdapter {
    pub fn new() -> Self {
        RustAdapter {
            tokenizer: StructuralTokenizer::new(),
        }
    }

    /// Map a pesti TokenKind to the universal SpanKind representation.
    fn map_token_kind(&self, kind: &TokenKind) -> Option<SpanKind> {
        match kind {
            // Declarations
            TokenKind::FnDecl => Some(SpanKind::FnDecl),
            TokenKind::StructDecl | TokenKind::EnumDecl | TokenKind::TraitDecl => {
                Some(SpanKind::ClassDecl)
            }
            TokenKind::LetStmt => Some(SpanKind::LetStmt),
            TokenKind::Const => Some(SpanKind::ConstStmt),

            // Expressions
            TokenKind::CallExpr | TokenKind::MethodCall => Some(SpanKind::CallExpr),
            TokenKind::BinaryOp => Some(SpanKind::BinaryExpr),
            TokenKind::UnaryOp => Some(SpanKind::UnaryExpr),
            TokenKind::ParenExpr => Some(SpanKind::Block),

            // Literals
            TokenKind::IntLit | TokenKind::FloatLit => Some(SpanKind::LitInt),
            TokenKind::StrLit => Some(SpanKind::LitStr),
            TokenKind::BoolLit => Some(SpanKind::LitBool),
            TokenKind::CharLit => Some(SpanKind::LitChar),

            // Control flow (not in universal set, skip)
            _ => None,
        }
    }

    /// Convert a pesti StructuralToken to a universal Span with Rust-specific tags.
    fn convert_token(&self, token: &pesti_structural_tokenizer::StructuralToken) -> Option<Span> {
        let kind = self.map_token_kind(&token.kind)?;

        Some(Span {
            kind,
            open_token: None,
            body: Vec::new(),
            close_token: None,
            source: token.text.clone(),
        })
    }

    /// Detect Rust-specific ownership and lifetime patterns.
    fn detect_ownership_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        // Detect reference types
        if source.contains("&") || source.contains("&&") {
            tags.insert("ownership_pattern".to_string(), "references".to_string());
        }

        // Detect Box usage
        if source.contains("Box<") {
            tags.insert("heap_allocation".to_string(), "box".to_string());
        }

        // Detect Rc/Arc
        if source.contains("Rc<") || source.contains("Arc<") {
            tags.insert("shared_ownership".to_string(), "rc_or_arc".to_string());
        }

        // Detect lifetime annotations
        if source.contains("'a ") || source.contains("'static") {
            tags.insert("has_lifetimes".to_string(), "true".to_string());
        }

        tags
    }

    /// Detect trait usage patterns.
    fn detect_trait_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        // Detect where clauses
        if source.contains("where ") {
            tags.insert("has_where_clause".to_string(), "true".to_string());
        }

        // Detect impl blocks
        if source.contains("impl ") {
            tags.insert("has_impl_block".to_string(), "true".to_string());
        }

        // Detect trait bounds
        if source.contains(": ") && (source.contains("dyn ") || source.contains("+ ")) {
            tags.insert("has_trait_bounds".to_string(), "true".to_string());
        }

        tags
    }

    /// Detect async/await patterns.
    fn detect_async_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("async ") {
            tags.insert("has_async".to_string(), "true".to_string());
        }

        if source.contains("await") {
            tags.insert("has_await".to_string(), "true".to_string());
        }

        tags
    }
}

impl LanguageAdapter for RustAdapter {
    fn name(&self) -> &str {
        "rust"
    }

    fn tokenize_and_adapt(&self, source: &str) -> SpanTree {
        // Tokenize using pesti-structural-tokenizer
        let tokens = match self.tokenizer.tokenize(source) {
            Ok(tokens) => tokens,
            Err(_) => Vec::new(),
        };

        // Convert to universal spans
        let mut spans = Vec::new();
        for token in &tokens {
            if let Some(span) = self.convert_token(token) {
                spans.push(span);
            }
        }

        // Add semantic annotations at the tree level
        let mut semantic_tags = HashMap::new();
        semantic_tags.extend(self.detect_ownership_patterns(source));
        semantic_tags.extend(self.detect_trait_patterns(source));
        semantic_tags.extend(self.detect_async_patterns(source));

        SpanTree {
            spans,
            source: source.to_string(),
            language: "rust".to_string(),
            semantic_tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_adapter_name() {
        let adapter = RustAdapter::new();
        assert_eq!(adapter.name(), "rust");
    }

    #[test]
    fn test_tokenize_function_declaration() {
        let adapter = RustAdapter::new();
        let tree = adapter.tokenize_and_adapt(
            "fn process_data(input: &[u8]) -> Result<String, Error> { Ok(String::new()) }",
        );

        // Should identify a function declaration
        let has_fn_decl = tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        assert!(has_fn_decl);
    }

    #[test]
    fn test_tokenize_struct_declaration() {
        let adapter = RustAdapter::new();
        let tree = adapter.tokenize_and_adapt("pub struct Processor { data: Vec<u8> }");

        // Struct maps to ClassDecl in universal representation
        let has_class_decl = tree.spans.iter().any(|s| s.kind == SpanKind::ClassDecl);
        assert!(has_class_decl);
    }

    #[test]
    fn test_tokenize_enum_declaration() {
        let adapter = RustAdapter::new();
        let tree = adapter.tokenize_and_adapt("enum Status { Ok, Error }");

        let has_class_decl = tree.spans.iter().any(|s| s.kind == SpanKind::ClassDecl);
        assert!(has_class_decl);
    }

    #[test]
    fn test_tokenize_trait_declaration() {
        let adapter = RustAdapter::new();
        let tree =
            adapter.tokenize_and_adapt("pub trait Processor { fn process(&self, data: &[u8]); }");

        let has_class_decl = tree.spans.iter().any(|s| s.kind == SpanKind::ClassDecl);
        assert!(has_class_decl);
    }

    #[test]
    fn test_tokenize_let_statement() {
        let adapter = RustAdapter::new();
        let tree = adapter.tokenize_and_adapt("fn foo() { let x = 42; }");

        let has_let_stmt = tree.spans.iter().any(|s| s.kind == SpanKind::LetStmt);
        assert!(has_let_stmt);
    }

    #[test]
    fn test_detect_ownership_patterns() {
        let adapter = RustAdapter::new();
        let tree = adapter.tokenize_and_adapt(
            "fn process(data: &Vec<u8>) -> Box<String> { Box::new(String::new()) }",
        );

        assert_eq!(
            tree.semantic_tags.get("ownership_pattern"),
            Some(&"references".to_string())
        );
        assert_eq!(
            tree.semantic_tags.get("heap_allocation"),
            Some(&"box".to_string())
        );
    }

    #[test]
    fn test_detect_trait_patterns() {
        let adapter = RustAdapter::new();
        let tree = adapter.tokenize_and_adapt("fn process<T>(item: T) where T: Clone + Debug { }");

        assert_eq!(
            tree.semantic_tags.get("has_where_clause"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_detect_async_patterns() {
        let adapter = RustAdapter::new();
        let tree = adapter
            .tokenize_and_adapt("async fn fetch_data(url: &str) -> String { String::new().await }");

        assert_eq!(
            tree.semantic_tags.get("has_async"),
            Some(&"true".to_string())
        );
        assert_eq!(
            tree.semantic_tags.get("has_await"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_cross_language_compatibility() {
        // Verify that Rust tokens map to the same universal kinds as JavaScript
        let rust_adapter = RustAdapter::new();
        let js_adapter = crate::adapters::javascript::JavaScriptAdapter::new();

        let rust_tree = rust_adapter.tokenize_and_adapt("fn foo() { let x = 42; }");
        let js_tree = js_adapter.tokenize_and_adapt("function foo() { const x = 42; }");

        // Both should identify a function declaration
        let rust_has_fn = rust_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        let js_has_fn = js_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        assert!(rust_has_fn && js_has_fn);

        // Both should identify a variable binding
        let rust_has_let = rust_tree.spans.iter().any(|s| s.kind == SpanKind::LetStmt);
        let js_has_let = js_tree.spans.iter().any(|s| s.kind == SpanKind::LetStmt);
        assert!(rust_has_let && js_has_let);
    }
}
