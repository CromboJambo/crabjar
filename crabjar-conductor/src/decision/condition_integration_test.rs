/// Integration test: evaluate a structural target against criteria with conditions.
#[test]
fn test_structural_condition_evaluation() {
    use crate::decision::structural::{EvaluationTarget, SpanKind};
    use std::collections::HashMap;

    // Create a function declaration target with Propagated error handling
    let mut tags = HashMap::new();
    tags.insert("error_handling".to_string(), "Propagated".to_string());
    let target = EvaluationTarget {
        structure: SpanKind::FnDecl,
        range: [0, 100],
        source: "fn foo() -> Result<()> {} ".to_string(),
        name: Some("foo".to_string()),
        semantic_tags: tags,
        error_handling: None,
        mutability: None,
        ownership_mode: None,
    };

    // Evaluate against the structural-function-evaluation criteria
    let result = evaluate_structural_conditions(
        &target,
        "specs/criteria/structural-function-evaluation.json",
    );

    assert!(result.is_some(), "Should have matched a structural condition");
    let json = result.unwrap();
    let value: serde_json::Value = json.0;
    assert_eq!(value["evaluation"]["option"], "well-structured-function");
    assert_eq!(value["evaluation"]["method"], "structural-condition");
}

/// Phase 2 test: build target from source with typed semantic fields and evaluate.
#[test]
fn test_build_target_and_evaluate() {
    use crate::decision::structural::{build_target, SpanKind};

    // Build target from source code that has propagated error handling
    let source = "async fn handle_request(req) -> Result<Response, Error> {\n    let data = fetch_data(&req).await?;\n    Ok(Response::Ok(data))\n}";
    let target = build_target(SpanKind::FnDecl, [0, 150], source, Some("handle_request"));

    // Verify typed fields were populated
    assert!(target.error_handling.is_some(), "Should detect error handling");
    assert_eq!(target.name.as_deref(), Some("handle_request"));

    // Evaluate against production criteria file
    let result = evaluate_structural_conditions(
        &target,
        "specs/criteria/structural-function-evaluation.json",
    );

    assert!(result.is_some(), "Should match a structural condition");
    let json = result.unwrap();
    let value: serde_json::Value = json.0;
    assert_eq!(value["evaluation"]["option"], "well-structured-function");
}

/// Phase 2 test: function with panicked error handling (unwrap).
#[test]
fn test_build_target_panicked_error_handling() {
    use crate::decision::structural::{build_target, SpanKind};

    let source = "fn process(data) {\n    let result = parse(data).unwrap();\n    do_work(result);\n}";
    let target = build_target(SpanKind::FnDecl, [0, 100], source, Some("process"));

    // Should detect panicked error handling
    assert!(target.error_handling.is_some());

    // Evaluate — should match "simple-function" (not Propagated)
    let result = evaluate_structural_conditions(
        &target,
        "specs/criteria/structural-function-evaluation.json",
    );

    assert!(result.is_some());
    let json = result.unwrap();
    let value: serde_json::Value = json.0;
    assert_eq!(value["evaluation"]["option"], "simple-function");
}

/// Phase 2 test: non-function code region.
#[test]
fn test_build_target_not_a_function() {
    use crate::decision::structural::{build_target, SpanKind};

    let source = "let x = 42;";
    let target = build_target(SpanKind::LetStmt, [0, 10], source, None);

    // Evaluate — should match "not-a-function"
    let result = evaluate_structural_conditions(
        &target,
        "specs/criteria/structural-function-evaluation.json",
    );

    assert!(result.is_some());
    let json = result.unwrap();
    let value: serde_json::Value = json.0;
    assert_eq!(value["evaluation"]["option"], "not-a-function");
}
