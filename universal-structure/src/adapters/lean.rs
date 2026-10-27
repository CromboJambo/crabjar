//! Lean language adapter for universal structural tokenization.
//!
//! Recognizes theorem declarations, proof terms, type annotations, and tactic blocks.

use crate::{LanguageAdapter, Span, SpanKind, SpanTree, StructuralToken};
use std::collections::HashMap;

/// Lean language adapter implementing the LanguageAdapter trait.
pub struct LeanAdapter {}

impl LeanAdapter {
    pub fn new() -> Self {
        LeanAdapter {}
    }

    /// Map a simple regex pattern to a universal SpanKind for Lean constructs.
    fn map_pattern(pattern: &str, text: &str) -> Option<SpanKind> {
        match pattern {
            "theorem" => Some(SpanKind::FnDecl),
            "lemma" => Some(SpanKind::FnDecl),
            "definition" | "def" => Some(SpanKind::LetStmt),
            "structure" | "struct" => Some(SpanKind::ClassDecl),
            "inductive" => Some(SpanKind::ClassDecl),
            "import" => Some(SpanKind::ImportStatement),
            _ => None,
        }
    }

    /// Detect Lean-specific patterns in source code.
    fn detect_patterns(source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        // Detect theorem declarations
        if source.contains("theorem ") || source.contains("lemma ") {
            tags.insert("has_theorems".to_string(), "true".to_string());
        }

        // Detect proof blocks
        if source.contains("proof") {
            tags.insert("has_proofs".to_string(), "true".to_string());
        }

        // Detect tactic usage
        if source.contains("exact ") || source.contains("apply ") || source.contains("intros ") {
            tags.insert("uses_tactics".to_string(), "true".to_string());
        }

        // Detect type annotations
        if source.contains(":") && (source.contains(" : ") || source.contains(": ")) {
            tags.insert("has_type_annotations".to_string(), "true".to_string());
        }

        tags
    }
}

impl LanguageAdapter for LeanAdapter {
    fn name(&self) -> &str {
        "lean"
    }

    fn tokenize_and_adapt(&self, source: &str) -> SpanTree {
        let mut spans = Vec::new();

        // Simple line-by-line parsing for theorem/lemma definitions
        let lines = source.lines();
        for line in lines {
            let trimmed = line.trim();

            // Detect theorem declarations
            if trimmed.starts_with("theorem ") || trimmed.starts_with("lemma ") {
                spans.push(Span {
                    kind: SpanKind::FnDecl,
                    open_token: Some(StructuralToken::new(
                        SpanKind::Keyword,
                        line.to_string(),
                        [0, 0],
                    )),
                    body: Vec::new(),
                    close_token: None,
                    source: line.to_string(),
                });
            }

            // Detect definition/structure declarations
            if trimmed.starts_with("def ") || trimmed.starts_with("definition ") {
                spans.push(Span {
                    kind: SpanKind::LetStmt,
                    open_token: Some(StructuralToken::new(
                        SpanKind::Keyword,
                        line.to_string(),
                        [0, 0],
                    )),
                    body: Vec::new(),
                    close_token: None,
                    source: line.to_string(),
                });
            }

            // Detect structure declarations
            if trimmed.starts_with("structure ") || trimmed.starts_with("struct ") {
                spans.push(Span {
                    kind: SpanKind::ClassDecl,
                    open_token: Some(StructuralToken::new(
                        SpanKind::Keyword,
                        line.to_string(),
                        [0, 0],
                    )),
                    body: Vec::new(),
                    close_token: None,
                    source: line.to_string(),
                });
            }

            // Detect inductive types (map to ClassDecl per universal schema)
            if trimmed.starts_with("inductive ") {
                spans.push(Span {
                    kind: SpanKind::ClassDecl,
                    open_token: Some(StructuralToken::new(
                        SpanKind::Keyword,
                        line.to_string(),
                        [0, 0],
                    )),
                    body: Vec::new(),
                    close_token: None,
                    source: line.to_string(),
                });
            }

            // Detect import statements
            if trimmed.starts_with("import ") {
                spans.push(Span {
                    kind: SpanKind::ImportStatement,
                    open_token: Some(StructuralToken::new(
                        SpanKind::Keyword,
                        line.to_string(),
                        [0, 0],
                    )),
                    body: Vec::new(),
                    close_token: None,
                    source: line.to_string(),
                });
            }
        }

        // Add semantic annotations at the tree level
        let semantic_tags = Self::detect_patterns(source);

        SpanTree {
            spans,
            source: source.to_string(),
            language: "lean".to_string(),
            semantic_tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lean_adapter_name() {
        let adapter = LeanAdapter::new();
        assert_eq!(adapter.name(), "lean");
    }

    #[test]
    fn test_tokenize_theorem_declaration() {
        let adapter = LeanAdapter::new();
        let tree = adapter.tokenize_and_adapt(
            "theorem add_zero (n : Nat) : n + 0 = n :=\nproof\nexact rfl\nend proof",
        );

        // Should identify a theorem declaration
        let has_fn_decl = tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        assert!(has_fn_decl);
    }

    #[test]
    fn test_tokenize_definition() {
        let adapter = LeanAdapter::new();
        let tree = adapter.tokenize_and_adapt(
            "def factorial (n : Nat) : Nat :=\nif n = 0 then 1 else n * factorial (n - 1)",
        );

        // Should identify a definition
        let has_let_stmt = tree.spans.iter().any(|s| s.kind == SpanKind::LetStmt);
        assert!(has_let_stmt);
    }

    #[test]
    fn test_tokenize_structure() {
        let adapter = LeanAdapter::new();
        let tree = adapter.tokenize_and_adapt("structure Point where\n  x : Float\n  y : Float");

        // Should identify a structure declaration
        let has_class_decl = tree.spans.iter().any(|s| s.kind == SpanKind::ClassDecl);
        assert!(has_class_decl);
    }

    #[test]
    fn test_tokenize_inductive() {
        let adapter = LeanAdapter::new();
        let tree = adapter.tokenize_and_adapt("inductive Bool\n  | true\n  | false");

        // Should identify an inductive type declaration (mapped to ClassDecl per universal schema)
        let has_class_decl = tree.spans.iter().any(|s| s.kind == SpanKind::ClassDecl);
        assert!(has_class_decl);
    }

    #[test]
    fn test_detect_theorems() {
        let adapter = LeanAdapter::new();
        let tree = adapter
            .tokenize_and_adapt("theorem add_zero (n : Nat) : n + 0 = n :=\nproof\nexact rfl");

        assert_eq!(
            tree.semantic_tags.get("has_theorems"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_detect_proofs() {
        let adapter = LeanAdapter::new();
        let tree = adapter
            .tokenize_and_adapt("theorem add_zero (n : Nat) : n + 0 = n :=\nproof\nexact rfl");

        assert_eq!(
            tree.semantic_tags.get("has_proofs"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_detect_tactics() {
        let adapter = LeanAdapter::new();
        let tree = adapter
            .tokenize_and_adapt("theorem add_zero (n : Nat) : n + 0 = n :=\nproof\nexact rfl");

        assert_eq!(
            tree.semantic_tags.get("uses_tactics"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_detect_type_annotations() {
        let adapter = LeanAdapter::new();
        let tree = adapter.tokenize_and_adapt(
            "def factorial (n : Nat) : Nat :=\nif n = 0 then 1 else n * factorial (n - 1)",
        );

        assert_eq!(
            tree.semantic_tags.get("has_type_annotations"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_cross_language_compatibility() {
        // Verify that Lean theorem declarations map to the same universal kind as Rust function declarations
        let lean_adapter = LeanAdapter::new();
        let rust_adapter = crate::adapters::rust::RustAdapter::new();

        let lean_tree = lean_adapter
            .tokenize_and_adapt("theorem add_zero (n : Nat) : n + 0 = n :=\nproof\nexact rfl");
        let rust_tree = rust_adapter.tokenize_and_adapt(
            "fn process_data(input: &[u8]) -> Result<String, Error> { Ok(String::new()) }",
        );

        // Both should identify a function/theorem declaration
        let lean_has_fn = lean_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        let rust_has_fn = rust_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        assert!(lean_has_fn && rust_has_fn);
    }
}
