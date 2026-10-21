//! Phase 1: Structural region evaluation for JEV.
//!
//! Bridges pesti-structural-tokenizer output into the JEV decision layer
//! by defining an EvaluationTarget type that wraps structural tokens with
//! semantic annotations, then evaluates them against JEV criteria files.
//!
//! Uses universal structural types from the multi-language topological analysis
//! layer (universal-structure crate) for cross-language compatibility.

use crate::decision::condition::evaluate_condition;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub use universal_structure::{ErrorHandling, EvaluationTarget, Mutability, OwnershipMode, SpanKind};

/// Build an EvaluationTarget from a code region, populating typed semantic fields
/// by analyzing the source text for ownership, error handling, and mutability patterns.
pub fn build_target(
    structure: SpanKind,
    range: [usize; 2],
    source: &str,
    name: Option<&str>,
) -> EvaluationTarget {
    let mut target = EvaluationTarget {
        structure,
        range,
        source: source.to_string(),
        name: name.map(|s| s.to_string()),
        language: Some("rust".to_string()),
        semantic_tags: HashMap::new(),
        error_handling: None,
        mutability: None,
        ownership_mode: None,
    };

    // Phase 2: Analyze source text for typed semantic fields

    // Error handling patterns
    if source.contains("unwrap()") {
        target.error_handling = Some(ErrorHandling::Panicked);
    } else if source.contains(".map_err(") || source.contains("with_context") {
        target.error_handling = Some(ErrorHandling::Recovered);
    } else if source.contains("return Err") || source.contains("?") {
        target.error_handling = Some(ErrorHandling::Propagated);
    }

    // Ownership patterns (Rust-specific)
    if source.contains("std::rc::Rc") || source.contains("Arc<") {
        target.ownership_mode = Some(OwnershipMode::RefCounted);
    } else if source.contains("Box<") {
        target.ownership_mode = Some(OwnershipMode::Boxed);
    }

    // Mutability patterns
    if source.contains("mut ") || source.contains(".push(") || source.contains(".insert(") {
        target.mutability = Some(Mutability::Mutable);
    } else {
        target.mutability = Some(Mutability::Immutable);
    }

    // Also populate legacy string fields for backward compatibility
    if let Some(eh) = &target.error_handling {
        target
            .semantic_tags
            .insert("error_handling".to_string(), eh.as_str().to_string());
    }
    if let Some(mt) = &target.mutability {
        target
            .semantic_tags
            .insert("mutability".to_string(), mt.as_str().to_string());
    }

    target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_kind_serialization() {
        let kind = SpanKind::FnDecl;
        assert_eq!(kind.as_str(), "FnDecl");
    }

    #[test]
    fn test_evaluation_target_creation() {
        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: Some("foo".to_string()),
            language: Some("rust".to_string()),
            semantic_tags: HashMap::new(),
            error_handling: None,
            mutability: None,
            ownership_mode: None,
        };

        assert_eq!(target.structure, SpanKind::FnDecl);
        assert_eq!(target.name.as_deref(), Some("foo"));
    }

    #[test]
    fn test_observation_string() {
        let mut tags = HashMap::new();
        tags.insert("ownership".to_string(), "Owned".to_string());

        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: Some("foo".to_string()),
            language: Some("rust".to_string()),
            semantic_tags: tags,
            error_handling: None,
            mutability: None,
            ownership_mode: None,
        };

        let obs = target.to_observation();
        assert!(obs.starts_with("[FnDecl] foo"));
        assert!(obs.contains("ownership=Owned"));
        assert!(obs.contains("fn foo() {} "));
    }

    #[test]
    fn test_observation_string_with_typed_tags() {
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
        assert!(obs.contains("error_handling=Propagated"));
        assert!(obs.contains("mutability=Immutable"));
    }

    #[test]
    fn test_build_target_ownership() {
        let target = build_target(
            SpanKind::LetStmt,
            [0, 50],
            "let x: Box<i32> = Box::new(42);",
            None,
        );

        assert_eq!(target.ownership_mode, Some(OwnershipMode::Boxed));
    }

    #[test]
    fn test_build_target_ownership_rc() {
        let target = build_target(
            SpanKind::LetStmt,
            [0, 50],
            "let x = std::rc::Rc::new(42);",
            None,
        );

        assert_eq!(target.ownership_mode, Some(OwnershipMode::RefCounted));
    }

    #[test]
    fn test_build_target_error_handling() {
        let target = build_target(
            SpanKind::FnDecl,
            [0, 50],
            "fn foo() -> Result<(), Error> { return Err(e); }",
            Some("foo"),
        );

        assert_eq!(target.error_handling, Some(ErrorHandling::Propagated));
        assert_eq!(target.language, Some("rust".to_string()));
    }

    #[test]
    fn test_build_target_mutability() {
        let target = build_target(SpanKind::LetStmt, [0, 30], "let mut x = 5;", None);

        assert_eq!(target.mutability, Some(Mutability::Mutable));
    }
}
