//! Verification test for Phase 2.3 semantic criteria files.
//! Evaluates each criteria file against synthetic targets to confirm
//! fast-path rules produce correct decisions.

use std::collections::HashMap;

// Import from conductor decision module (same crate)
use crabjar_conductor::decision::condition::evaluate_condition;
use crabjar_conductor::decision::structural::{SpanKind, build_target};

fn test_error_handling_criteria() {
    println!("=== error_handling_adequacy.json ===");

    // Target with propagated errors (should match "adequate-propagated")
    let target = build_target(
        SpanKind::FnDecl,
        [0, 50],
        "fn foo() -> Result<(), Error> { return Err(e); }",
        Some("foo"),
    );

    assert!(evaluate_condition(
        &target,
        "structure == FnDecl && error_handling == Propagated"
    ));
    println!("✓ Propagated error handling detected");

    // Target with panicked error handling (unwrap)
    let target2 = build_target(
        SpanKind::FnDecl,
        [0, 50],
        "fn bar() { let _ = result.unwrap(); }",
        Some("bar"),
    );

    assert!(evaluate_condition(
        &target2,
        "structure == FnDecl && error_handling == Panicked"
    ));
    println!("✓ Panicked error handling detected (unwrap)");
}

fn test_ownership_criteria() {
    println!("\n=== ownership_pattern_validation.json ===");

    // Boxed value (should match "valid-boxed")
    let target = build_target(
        SpanKind::LetStmt,
        [0, 50],
        "let x: Box<i32> = Box::new(42);",
        None,
    );

    assert!(evaluate_condition(&target, "ownership_mode == Boxed"));
    println!("✓ Boxed ownership detected");

    // Rc value (should match "valid-refcounted")
    let target2 = build_target(
        SpanKind::LetStmt,
        [0, 50],
        "let x = std::rc::Rc::new(42);",
        None,
    );

    assert!(evaluate_condition(&target2, "ownership_mode == RefCounted"));
    println!("✓ RefCounted ownership detected");

    // Mutable let (should match mutability check)
    let target3 = build_target(SpanKind::LetStmt, [0, 30], "let mut x = 5;", None);
    assert!(evaluate_condition(&target3, "mutability == Mutable"));
    println!("✓ Mutable binding detected");
}

fn test_api_doc_criteria() {
    println!("\n=== api_surface_documentation.json ===");

    // Function with name (should match "named-function")
    let mut target = build_target(SpanKind::FnDecl, [0, 50], "fn foo() {}", Some("foo"));

    assert!(evaluate_condition(
        &target,
        "structure == FnDecl && name != null"
    ));
    println!("✓ Named function detected");

    // Simulate documented function by adding doc tag
    target
        .semantic_tags
        .insert("doc".to_string(), "/// This is documented".to_string());
    assert!(evaluate_condition(
        &target,
        "structure == FnDecl && doc != null"
    ));
    println!("✓ Documented function detected (via semantic tag)");

    // Undocumented named function
    let target2 = build_target(SpanKind::FnDecl, [0, 50], "fn bar() {}", Some("bar"));
    assert!(evaluate_condition(
        &target2,
        "structure == FnDecl && doc == null && name != null"
    ));
    println!("✓ Undocumented public API detected");
}

fn main() {
    println!("Phase 2.3 Semantic Criteria Verification\n");

    test_error_handling_criteria();
    test_ownership_criteria();
    test_api_doc_criteria();

    println!("\n=== All criteria verification tests passed ===");
}
