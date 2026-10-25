//! Phase 1: Structural region evaluation for JEV.
//!
//! Bridges pesti-structural-tokenizer output into the JEV decision layer
//! by defining an EvaluationTarget type that wraps structural tokens with
//! semantic annotations, then evaluates them against JEV criteria files.
//!
//! Uses universal structural types from the multi-language topological analysis
//! layer (universal-structure crate) for cross-language compatibility.

use crate::decision::condition::evaluate_condition;
use pesti_structural_tokenizer::semantic::{analyze_errors, analyze_mutability, analyze_ownership};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub use universal_structure::{
    ErrorHandling, EvaluationTarget, Mutability, OwnershipMode, SpanKind,
};

/// Run pesti semantic analyzers on source text and populate typed fields.
fn annotate_target(source: &str, target: &mut EvaluationTarget) {
    // Ownership analysis via pesti's analyzer (detects borrow patterns)
    let ownership_anns = analyze_ownership(source);
    for (_id, ann) in ownership_anns {
        match ann.mode {
            pesti_structural_tokenizer::semantic::OwnershipMode::Owned => {
                if target.ownership_mode.is_none() {
                    target.ownership_mode = Some(OwnershipMode::Owned);
                }
            }
            pesti_structural_tokenizer::semantic::OwnershipMode::Borrowed
            | pesti_structural_tokenizer::semantic::OwnershipMode::MutablyBorrowed => {
                target.ownership_mode = Some(OwnershipMode::Borrowed);
            }
        }
    }

    // Box/Arc detection (type annotation patterns not covered by ownership analyzer)
    if source.contains("Box<") || source.contains("Box::new(") {
        target.ownership_mode = Some(OwnershipMode::Boxed);
    }
    if source.contains("std::rc::Rc")
        || source.contains("Arc<")
        || source.contains("std::sync::Arc")
    {
        target.ownership_mode = Some(OwnershipMode::RefCounted);
    }

    // &mut parameter detection for function signatures (borrowed mutable reference)
    if source.contains(": &mut ") {
        target.ownership_mode = Some(OwnershipMode::Borrowed);
    } else if source.contains(": &") && target.ownership_mode.is_none() {
        target.ownership_mode = Some(OwnershipMode::Borrowed);
    }

    // Error handling analysis via pesti's analyzer
    let error_anns = analyze_errors(source);
    for (_id, ann) in error_anns {
        match ann.handling {
            pesti_structural_tokenizer::semantic::ErrorHandling::Propagated => {
                target.error_handling = Some(ErrorHandling::Propagated);
            }
            pesti_structural_tokenizer::semantic::ErrorHandling::Unwrapped
            | pesti_structural_tokenizer::semantic::ErrorHandling::Expected => {
                target.error_handling = Some(ErrorHandling::Panicked);
            }
            pesti_structural_tokenizer::semantic::ErrorHandling::Matched => {
                target.error_handling = Some(ErrorHandling::Recovered);
            }
            pesti_structural_tokenizer::semantic::ErrorHandling::Ignored => {
                target.error_handling = Some(ErrorHandling::Ignored);
            }
            _ => {}
        }
    }

    // Return Err detection (not covered by pesti's error analyzer)
    if source.contains("return Err") || source.contains("Err(") {
        target.error_handling = Some(ErrorHandling::Propagated);
    }

    // Mutability analysis via pesti's analyzer
    let mutability_anns = analyze_mutability(source);
    for (_id, ann) in mutability_anns {
        if ann.mutable {
            target.mutability = Some(Mutability::Mutable);
        } else if target.mutability.is_none() {
            target.mutability = Some(Mutability::Immutable);
        }
    }

    // &mut reference detection for function parameters (not covered by pesti)
    if source.contains(": &mut ") && target.mutability.is_none() {
        target.mutability = Some(Mutability::Mutable);
    }

    // Populate legacy string fields for backward compatibility
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
}

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

    // Phase 2: Run pesti semantic analyzers on the source text
    annotate_target(source, &mut target);

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
