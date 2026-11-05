//! Universal structural token representation for multi-language code analysis.
//!
//! All languages map to a common set of structural tokens regardless of syntax
//! differences. Language-specific tags are preserved alongside universal tokens
//! for nuanced analysis while maintaining cross-language compatibility.

pub mod adapters;
pub mod comparison;
mod evaluation;
mod semantic;
mod span_kind;
mod types;

// Re-export key types for convenience
pub use self::adapters::{
    BinaryAdapter, CAdapter, CppAdapter, GoAdapter, JavaScriptAdapter, PythonAdapter, RustAdapter,
};
pub use self::comparison::compare_trees;
pub use self::evaluation::EvaluationTarget;
pub use self::semantic::{ErrorHandling, Mutability, OwnershipMode};
pub use self::span_kind::SpanKind;
pub use self::types::{Budget, Span, SpanTree, StructuralToken};

/// Trait for language-specific adapters that convert native tokenizer output
/// to universal structural tokens.
pub trait LanguageAdapter {
    /// Return the name of this adapter (e.g., "rust", "javascript").
    fn name(&self) -> &str;

    /// Tokenize source code and adapt to universal `SpanTree` representation.
    fn tokenize_and_adapt(&self, source: &str) -> SpanTree;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_span_kind_as_str() {
        assert_eq!(SpanKind::FnDecl.as_str(), "FnDecl");
        assert_eq!(SpanKind::ClassDecl.as_str(), "ClassDecl");
        assert_eq!(SpanKind::ImportStatement.as_str(), "ImportStatement");
        assert_eq!(SpanKind::TryBlock.as_str(), "TryBlock");
    }

    #[test]
    fn test_span_kind_classification() {
        assert!(SpanKind::FnDecl.is_declaration());
        assert!(!SpanKind::CallExpr.is_declaration());

        assert!(SpanKind::CallExpr.is_expression());
        assert!(!SpanKind::FnDecl.is_expression());

        assert!(SpanKind::IfStmt.is_control_flow());
        assert!(!SpanKind::LetStmt.is_control_flow());

        assert!(SpanKind::TryBlock.is_error_handling());
        assert!(!SpanKind::ReturnStmt.is_error_handling());

        assert!(SpanKind::ImportStatement.is_module_boundary());
        assert!(!SpanKind::Expr.is_module_boundary());
    }

    #[test]
    fn test_structural_token_creation() {
        let token = StructuralToken::new(SpanKind::FnDecl, "fn foo()", [0, 10])
            .with_line(1)
            .with_tag("access", "public");

        assert_eq!(token.kind, SpanKind::FnDecl);
        assert_eq!(token.text, "fn foo()");
        assert_eq!(token.range, [0, 10]);
        assert_eq!(token.line, Some(1));
        assert_eq!(token.tags.get("access").unwrap(), &"public");
    }

    #[test]
    fn test_span_tree_operations() {
        let inner = Span::new(SpanKind::LetStmt);
        let outer = Span::new(SpanKind::FnDecl);
        let mut span = Span::new(SpanKind::ModuleDecl);
        span.body.push(outer);
        span.body[0].body.push(inner);

        assert_eq!(span.depth(), 3);
        assert_eq!(span.span_count(), 3);
        assert_eq!(span.find_all(SpanKind::LetStmt).len(), 1);
    }

    #[test]
    fn test_evaluation_target_observation() {
        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: Some("foo".to_string()),
            language: Some("rust".to_string()),
            semantic_tags: HashMap::new(),
            error_handling: Some(ErrorHandling::Propagated),
            mutability: Some(Mutability::Immutable),
            ownership_mode: None,
        };

        let obs = target.to_observation();
        assert!(obs.starts_with("[FnDecl] foo"));
        assert!(obs.contains("error_handling=Propagated"));
        assert!(obs.contains("mutability=Immutable"));
        assert!(obs.contains("fn foo() {} "));
    }

    #[test]
    fn test_evaluation_target_observation_ownership() {
        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: Some("foo".to_string()),
            language: Some("rust".to_string()),
            semantic_tags: HashMap::new(),
            error_handling: None,
            mutability: None,
            ownership_mode: Some(OwnershipMode::Owned),
        };

        let obs = target.to_observation();
        assert!(obs.contains("ownership_mode=Owned"));
    }

    #[test]
    fn test_evaluation_target_parse_value() {
        let target = EvaluationTarget {
            structure: SpanKind::ClassDecl,
            range: [0, 100],
            source: "".to_string(),
            name: Some("MyClass".to_string()),
            language: Some("javascript".to_string()),
            semantic_tags: HashMap::new(),
            error_handling: None,
            mutability: None,
            ownership_mode: Some(OwnershipMode::Owned),
        };

        assert_eq!(
            target.parse_value("structure"),
            Some("ClassDecl".to_string())
        );
        assert_eq!(target.parse_value("name"), Some("MyClass".to_string()));
        assert_eq!(
            target.parse_value("language"),
            Some("javascript".to_string())
        );
        assert_eq!(target.parse_value("unknown"), None);
        assert_eq!(
            target.parse_value("ownership_mode"),
            Some("Owned".to_string())
        );
    }

    #[test]
    fn test_budget_defaults() {
        let budget = Budget::default();
        assert_eq!(budget.max_tokens, 10000);
        assert_eq!(budget.min_node_bytes, 50);
    }
}
