//! Structural condition evaluator for JEV decision rules.
//!
//! Evaluates structural predicates against EvaluationTarget fields using
//! a simple expression parser supporting:
//! - Equality: `structure == FnDecl`, `error_handling == Propagated`
//! - Comparison: `<`, `>`, `<=`, `>=`
//! - Boolean: `&&`, `||`, `!`
//! - Null checks: `!= null`
//!
//! Supported field names: structure, name, ownership_mode, error_handling, mutability

use std::collections::HashMap;

use crate::decision::structural::{EvaluationTarget, SpanKind};

/// Evaluate a condition expression against an evaluation target.
pub fn evaluate_condition(target: &EvaluationTarget, condition: &str) -> bool {
    let expr = condition.trim();

    // Handle negation of parenthesized expressions: !(expr) or ! (expr)
    if expr.starts_with("!(") || expr.starts_with("! (") {
        let inner_start = if expr.starts_with("!(") { 2 } else { 3 };
        let inner_end = expr.len() - 1; // Remove closing ')'
        let inner = &expr[inner_start..inner_end];
        return !evaluate_condition(target, inner);
    }

    // Handle negation of simple comparisons: !field == value
    if expr.starts_with("!") {
        let rest = expr[1..].trim();
        if rest.contains("==") || rest.contains("!=") {
            return !evaluate_condition(target, rest);
        }
    }

    // Handle boolean AND at top level
    if let Some(and_pos) = find_top_level_operator(expr, "&&") {
        let left = expr[..and_pos].trim();
        let right = expr[and_pos + 2..].trim();
        return evaluate_condition(target, left) && evaluate_condition(target, right);
    }

    // Handle boolean OR at top level
    if let Some(or_pos) = find_top_level_operator(expr, "||") {
        let left = expr[..or_pos].trim();
        let right = expr[or_pos + 2..].trim();
        return evaluate_condition(target, left) || evaluate_condition(target, right);
    }

    // Handle parenthesized expressions: (expr)
    if expr.starts_with('(') && expr.ends_with(')') {
        let inner = &expr[1..expr.len() - 1];
        return evaluate_condition(target, inner);
    }

    // Parse comparison: field op value
    parse_comparison(target, expr)
}

/// Find the position of a boolean operator at the top level (not inside parentheses).
fn find_top_level_operator(expr: &str, op: &str) -> Option<usize> {
    let mut depth = 0;
    let bytes = expr.as_bytes();
    let op_bytes = op.as_bytes();

    for i in 0..bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ if depth == 0 && i + op_bytes.len() <= bytes.len() => {
                if &bytes[i..i + op_bytes.len()] == op_bytes {
                    // Check that this isn't part of a longer token
                    let before_ok = i == 0 || bytes[i - 1] == b' ';
                    let after_ok =
                        i + op_bytes.len() >= bytes.len() || bytes[i + op_bytes.len()] == b' ';
                    if before_ok && after_ok {
                        return Some(i);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Parse and evaluate a comparison expression.
fn parse_comparison(target: &EvaluationTarget, expr: &str) -> bool {
    let expr = expr.trim();

    // Try to find the comparison operator
    for op in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some(pos) = expr.find(op) {
            let left = expr[..pos].trim();
            let right = expr[pos + op.len()..].trim();

            let left_value = resolve_value(target, left);
            let right_value = resolve_value(target, right);

            return match op {
                "==" => left_value == right_value,
                "!=" => left_value != right_value,
                "<" if is_numeric(&left_value) && is_numeric(&right_value) => {
                    left_value.parse::<f64>().unwrap() < right_value.parse::<f64>().unwrap()
                }
                ">" if is_numeric(&left_value) && is_numeric(&right_value) => {
                    left_value.parse::<f64>().unwrap() > right_value.parse::<f64>().unwrap()
                }
                "<=" if is_numeric(&left_value) && is_numeric(&right_value) => {
                    left_value.parse::<f64>().unwrap() <= right_value.parse::<f64>().unwrap()
                }
                ">=" if is_numeric(&left_value) && is_numeric(&right_value) => {
                    left_value.parse::<f64>().unwrap() >= right_value.parse::<f64>().unwrap()
                }
                _ => false,
            };
        }
    }

    // No comparison operator found — treat as boolean literal
    expr == "true"
}

/// Resolve a value (field name or literal) to its string representation.
fn resolve_value(target: &EvaluationTarget, value: &str) -> String {
    let value = value.trim();

    // Quoted string literal
    if value.starts_with('"') && value.ends_with('"') {
        return value[1..value.len() - 1].to_string();
    }

    // Null literal
    if value == "null" {
        return "null".to_string();
    }

    // Numeric literal
    if is_numeric(value) {
        return value.to_string();
    }

    // Enum variant (starts with capital letter)
    if let Some(first) = value.chars().next() {
        if first.is_uppercase() && value.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return value.to_string();
        }
    }

    // Field name — resolve against target (typed fields first, then semantic_tags)
    match value {
        "structure" => target.structure.as_str().to_string(),
        "name" => target.name.clone().unwrap_or("null".to_string()),
        "error_handling" => target
            .error_handling
            .as_ref()
            .map(|eh| eh.as_str().to_string())
            .unwrap_or_else(|| resolve_semantic_tag(target, "error_handling")),
        "mutability" => target
            .mutability
            .as_ref()
            .map(|mt| mt.as_str().to_string())
            .unwrap_or_else(|| resolve_semantic_tag(target, "mutability")),
        // Ownership is stored in semantic_tags (language-specific)
        _ => resolve_semantic_tag(target, value),
    }
}

fn resolve_semantic_tag(target: &EvaluationTarget, tag_name: &str) -> String {
    target
        .semantic_tags
        .get(tag_name)
        .cloned()
        .unwrap_or("null".to_string())
}

fn is_numeric(s: &str) -> bool {
    s.parse::<f64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::structural::{ErrorHandling, Mutability};

    fn make_target(structure: SpanKind, name: Option<&str>) -> EvaluationTarget {
        let mut tags = HashMap::new();
        tags.insert("error_handling".to_string(), "Propagated".to_string());
        EvaluationTarget {
            structure,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: name.map(|s| s.to_string()),
            language: Some("rust".to_string()),
            semantic_tags: tags,
            error_handling: None,
            mutability: None,
        }
    }

    fn make_target_with_tags(
        structure: SpanKind,
        name: Option<&str>,
        error_handling: Option<ErrorHandling>,
        mutability: Option<Mutability>,
    ) -> EvaluationTarget {
        EvaluationTarget {
            structure,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: name.map(|s| s.to_string()),
            language: Some("rust".to_string()),
            semantic_tags: HashMap::new(),
            error_handling,
            mutability,
        }
    }

    #[test]
    fn test_simple_equality() {
        let target = make_target(SpanKind::FnDecl, Some("foo"));
        assert!(evaluate_condition(&target, "structure == FnDecl"));
        assert!(!evaluate_condition(&target, "structure == StructDecl"));
    }

    #[test]
    fn test_boolean_and() {
        let target = make_target(SpanKind::FnDecl, Some("foo"));
        assert!(evaluate_condition(
            &target,
            "structure == FnDecl && error_handling == Propagated"
        ));
        assert!(!evaluate_condition(
            &target,
            "structure == FnDecl && error_handling == Ignored"
        ));
    }

    #[test]
    fn test_boolean_or() {
        let target = make_target(SpanKind::FnDecl, Some("foo"));
        assert!(evaluate_condition(
            &target,
            "structure == StructDecl || structure == FnDecl"
        ));
    }

    #[test]
    fn test_negation_parens() {
        let target = make_target(SpanKind::FnDecl, Some("foo"));
        assert!(evaluate_condition(&target, "!(structure == StructDecl)"));
    }

    #[test]
    fn test_negation_simple() {
        let target = make_target(SpanKind::FnDecl, Some("foo"));
        assert!(evaluate_condition(&target, "! structure == StructDecl"));
    }

    #[test]
    fn test_null_check() {
        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: None,
            language: Some("rust".to_string()),
            semantic_tags: HashMap::new(),
            error_handling: None,
            mutability: None,
        };
        assert!(evaluate_condition(&target, "name == null"));
        assert!(evaluate_condition(&target, "error_handling == null"));
        assert!(evaluate_condition(&target, "mutability == null"));
    }

    #[test]
    fn test_numeric_comparison() {
        let target = make_target(SpanKind::FnDecl, Some("foo"));
        assert!(evaluate_condition(&target, "10 > 5"));
        assert!(!evaluate_condition(&target, "10 < 5"));
    }

    #[test]
    fn test_typed_field_access() {
        // Test that typed fields take precedence over semantic_tags map
        let target = make_target_with_tags(
            SpanKind::FnDecl,
            Some("foo"),
            Some(ErrorHandling::Propagated),
            Some(Mutability::Immutable),
        );

        assert!(evaluate_condition(&target, "error_handling == Propagated"));
        assert!(evaluate_condition(&target, "mutability == Immutable"));

        // Test complex condition with typed fields
        assert!(evaluate_condition(
            &target,
            "structure == FnDecl && error_handling == Propagated"
        ));
    }

    #[test]
    fn test_typed_field_fallback_to_map() {
        // When typed field is None, fall back to semantic_tags map
        let mut tags = HashMap::new();
        tags.insert("error_handling".to_string(), "Handled".to_string());
        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: Some("foo".to_string()),
            language: Some("rust".to_string()),
            semantic_tags: tags,
            error_handling: None, // Typed field is None
            mutability: None,
        };

        // Should fall back to map value "Handled"
        assert!(evaluate_condition(&target, "error_handling == Handled"));
    }

    #[test]
    fn test_integration_with_criteria_file() {
        // Integration test: evaluate against criteria with structural conditions
        let target = make_target(SpanKind::FnDecl, Some("foo"));

        // Test the condition directly (same as what would be in a criteria file)
        assert!(evaluate_condition(
            &target,
            "structure == FnDecl && error_handling == Propagated"
        ));

        // Verify different conditions match correctly for this target
        assert!(!evaluate_condition(&target, "structure == StructDecl"));
        assert!(!evaluate_condition(&target, "error_handling != Propagated"));
        assert!(evaluate_condition(&target, "! structure == StructDecl"));

        // Test a different target (struct declaration)
        let struct_target = make_target(SpanKind::ClassDecl, Some("MyStruct"));
        assert!(evaluate_condition(&struct_target, "structure == ClassDecl"));
        assert!(!evaluate_condition(&struct_target, "structure == FnDecl"));
    }
}
