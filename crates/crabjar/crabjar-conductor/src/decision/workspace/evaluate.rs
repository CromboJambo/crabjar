//! Crate evaluation using structural conditions.

use std::fs;
use std::path::Path;

use super::super::evaluate::evaluate_structural_conditions;
use super::super::structural::build_target;
use super::types::{CrateInfo, EvaluationResult};

/// Evaluate a single crate using structural conditions.
#[must_use]
pub fn evaluate_crate(crate_info: &CrateInfo, criteria_path: &str) -> Option<EvaluationResult> {
    // Build evaluation target from crate's source
    let src_dir = crate_info.path.join("src");
    if !src_dir.exists() {
        return None;
    }

    // For workspace analysis, evaluate the entire crate as a single region
    let lib_path = src_dir.join("lib.rs");
    let main_path = src_dir.join("main.rs");

    let target_path = if lib_path.exists() {
        lib_path
    } else if main_path.exists() {
        main_path
    } else {
        return None;
    };

    let source = match fs::read_to_string(&target_path) {
        Ok(content) => content,
        Err(_) => return None,
    };

    let target = build_target(
        super::super::structural::SpanKind::Item,
        [0, source.len()],
        &source,
        Some(&crate_info.name),
    );

    // Try structural condition evaluation first
    if let Some(result) = evaluate_structural_conditions(&target, criteria_path) {
        return Some(EvaluationResult {
            option: result.option.clone(),
            confidence: result.confidence,
            rationale: result.rationale.clone(),
            method: "structural-condition".to_string(),
        });
    }

    None
}
