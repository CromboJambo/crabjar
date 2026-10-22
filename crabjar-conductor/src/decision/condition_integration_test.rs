/// Integration test: evaluate a structural target against criteria with conditions.
#[test]
fn test_structural_condition_evaluation() {
    use crate::decision::structural::{EvaluationTarget, SpanKind};
    use crate::decision::evaluate;
    use std::collections::HashMap;

    // Create a function declaration target with Propagated error handling
    let mut tags = HashMap::new();
    tags.insert("error_handling".to_string(), "Propagated".to_string());
    let target = EvaluationTarget {
        structure: SpanKind::FnDecl,
        range: [0, 100],
        source: "fn foo() -> Result<()> {} ".to_string(),
        name: Some("foo".to_string()),
        language: None,
        semantic_tags: tags,
        error_handling: None,
        mutability: None,
        ownership_mode: None,
    };

    // Evaluate against the structural-function-evaluation criteria
    let result = evaluate::evaluate_structural_conditions(
        &target,
        "specs/criteria/structural-function-evaluation.json",
    );

    assert!(result.is_some(), "Should have matched a structural condition");
    let decision = result.unwrap();
    assert_eq!(decision.option, "well-structured-function");
}

/// Phase 2 test: build target from source with typed semantic fields and evaluate.
#[test]
fn test_build_target_and_evaluate() {
    use crate::decision::structural::{build_target, SpanKind};
    use crate::decision::evaluate;

    // Build target from source code that has propagated error handling
    let source = "async fn handle_request(req) -> Result<Response, Error> {\n    let data = fetch_data(&req).await?;\n    Ok(Response::Ok(data))\n}";
    let target = build_target(SpanKind::FnDecl, [0, 150], source, Some("handle_request"));

    // Debug: print target fields
    println!("Target error_handling: {:?}", target.error_handling);
    println!("Target mutability: {:?}", target.mutability);
    println!("Target ownership_mode: {:?}", target.ownership_mode);

    // Verify typed fields were populated
    assert!(target.error_handling.is_some(), "Should detect error handling");
    assert_eq!(target.name.as_deref(), Some("handle_request"));

    // Evaluate against production criteria file
    let result = evaluate::evaluate_structural_conditions(
        &target,
        "specs/criteria/structural-function-evaluation.json",
    );

    if result.is_none() {
        println!("No condition matched");
        // Try evaluating the condition directly to see what happens
        let direct_result = crate::decision::condition::evaluate_condition(
            &target,
            "structure == FnDecl && error_handling == Propagated",
        );
        println!("Direct evaluation: {}", direct_result);
    }

    assert!(result.is_some(), "Should match a structural condition");
    let decision = result.unwrap();
    assert_eq!(decision.option, "well-structured-function");
}

/// Phase 2 test: function with panicked error handling (unwrap).
#[test]
fn test_build_target_panicked_error_handling() {
    use crate::decision::structural::{build_target, SpanKind};
    use crate::decision::evaluate;

    let source = "fn process(data) {\n    let result = parse(data).unwrap();\n    do_work(result);\n}";
    let target = build_target(SpanKind::FnDecl, [0, 100], source, Some("process"));

    // Should detect panicked error handling
    assert!(target.error_handling.is_some());

    // Evaluate — should match "simple-function" (not Propagated)
    let result = evaluate::evaluate_structural_conditions(
        &target,
        "specs/criteria/structural-function-evaluation.json",
    );

    assert!(result.is_some());
    let decision = result.unwrap();
    assert_eq!(decision.option, "simple-function");
}

/// Phase 2 test: non-function code region.
#[test]
fn test_build_target_not_a_function() {
    use crate::decision::structural::{build_target, SpanKind};
    use crate::decision::evaluate;

    let source = "let x = 42;";
    let target = build_target(SpanKind::LetStmt, [0, 10], source, None);

    // Evaluate — should match "not-a-function"
    let result = evaluate::evaluate_structural_conditions(
        &target,
        "specs/criteria/structural-function-evaluation.json",
    );

    assert!(result.is_some());
    let decision = result.unwrap();
    assert_eq!(decision.option, "not-a-function");
}

/// Phase 2.3 test: error handling adequacy criteria.
#[test]
fn test_error_handling_adequacy_criteria() {
    use crate::decision::structural::{build_target, SpanKind};
    use crate::decision::evaluate;

    // Function with propagated error handling (uses ? operator)
    let source = "async fn handle_request(req) -> Result<Response, Error> {\n    let data = fetch_data(&req).await?;\n    Ok(Response::Ok(data))\n}";
    let target = build_target(SpanKind::FnDecl, [0, 150], source, Some("handle_request"));

    // Evaluate against error handling adequacy criteria
    let result = evaluate::evaluate_structural_conditions(
        &target,
        "specs/criteria/error-handling-adequacy.json",
    );

    assert!(result.is_some(), "Should match an error handling condition");
    let decision = result.unwrap();
    // Should match "propagates-errors" since error_handling == Propagated
    assert_eq!(decision.option, "propagates-errors");
}

/// Phase 2.3 test: ownership pattern validation criteria.
#[test]
fn test_ownership_pattern_validation_criteria() {
    use crate::decision::structural::{build_target, SpanKind};
    use crate::decision::evaluate;

    // Function that takes a borrowed reference and mutates it
    let source = "fn modify_data(data: &mut Vec<i32>) {\n    data.push(42);\n}";
    let target = build_target(SpanKind::FnDecl, [0, 100], source, Some("modify_data"));

    // Evaluate against ownership pattern validation criteria
    let result = evaluate::evaluate_structural_conditions(
        &target,
        "specs/criteria/ownership-pattern-validation.json",
    );

    assert!(result.is_some(), "Should match an ownership condition");
    let decision = result.unwrap();
    // Should match "borrowed-mutable" since ownership_mode == Borrowed && mutability == Mutable
    assert_eq!(decision.option, "borrowed-mutable");
}

/// Phase 2.3 test: API surface documentation criteria.
#[test]
fn test_api_surface_documentation_criteria() {
    use crate::decision::structural::{build_target, SpanKind};
    use crate::decision::evaluate;

    // Function without documentation
    let source = "pub fn public_api(data) -> Result<String, Error> {\n    Ok(format!(\"Hello, {}\", data))\n}";
    let target = build_target(SpanKind::FnDecl, [0, 100], source, Some("public_api"));

    // Evaluate against API surface documentation criteria
    let result = evaluate::evaluate_structural_conditions(
        &target,
        "specs/criteria/api-surface-documentation.json",
    );

    assert!(result.is_some(), "Should match a documentation condition");
    let decision = result.unwrap();
    // Should match "undocumented-public-function" since documentation tag is not set
    assert_eq!(decision.option, "undocumented-public-function");
}

/// Phase 2.3 test: documented API function.
#[test]
fn test_documented_api_function_criteria() {
    use crate::decision::structural::{build_target, SpanKind};
    use crate::decision::evaluate;

    // Function with documentation (manual tag since build_target doesn't auto-detect)
    let source = "pub fn documented_api(data) -> Result<String, Error> {\n    Ok(format!(\"Hello, {}\", data))\n}";
    let mut target = build_target(SpanKind::FnDecl, [0, 100], source, Some("documented_api"));

    // Manually set documentation tag (as would be done by a documentation analyzer)
    target.semantic_tags.insert("documentation".to_string(), "Documented".to_string());

    // Evaluate against API surface documentation criteria
    let result = evaluate::evaluate_structural_conditions(
        &target,
        "specs/criteria/api-surface-documentation.json",
    );

    assert!(result.is_some(), "Should match a documentation condition");
    let decision = result.unwrap();
    // Should match "documented-public-function" since documentation tag is set to Documented
    assert_eq!(decision.option, "documented-public-function");
}
