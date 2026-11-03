//! Rules extraction and promotion API endpoints.

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde_json::json;

/// POST /rules/extract - Extract candidate rules from decision history.
pub async fn extract_rules_handler(
    State(conductor): State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let criteria_file = body["criteria_file"]
        .as_str()
        .unwrap_or("default")
        .to_string();

    let min_occurrences = body["min_occurrences"].as_i64().unwrap_or(3);

    // Spawn blocking task to access state store and extract rules
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.extract_rules(&criteria_file, min_occurrences)
    })
    .await;

    match result {
        Ok(Ok(rules)) => {
            let rules_json: Vec<serde_json::Value> = rules
                .iter()
                .map(|r| {
                    json!({
                        "pattern_id": r.pattern_id,
                        "criteria_file": r.criteria_file,
                        "recommended_option": r.recommended_option,
                        "avg_confidence": r.avg_confidence,
                        "occurrence_count": r.occurrence_count
                    })
                })
                .collect();

            Json(json!({
                "status": "ok",
                "rules_extracted": rules.len(),
                "rules": rules_json
            }))
        }
        Ok(Err(e)) => Json(json!({
            "status": "error",
            "message": format!("Failed to extract rules: {}", e)
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("task join error: {}", e)
        })),
    }
}

/// POST /rules/promote — promote a candidate rule to persistent storage
pub async fn promote_rule_handler(
    State(conductor): State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let pattern_id = body["pattern_id"].as_str().map(std::string::ToString::to_string);
    let criteria_file = body["criteria_file"].as_str().map(std::string::ToString::to_string);
    let recommended_option = body["recommended_option"].as_str().map(std::string::ToString::to_string);

    match (pattern_id, criteria_file, recommended_option) {
        (Some(pid), Some(cf), Some(opt)) => {
            let result = tokio::task::spawn_blocking(move || {
                let c = conductor.blocking_lock();
                c.promote_rule(&pid, &cf, &opt)
            })
            .await;

            match result {
                Ok(Ok(rule_id)) => Json(json!({
                    "status": "ok",
                    "rule_id": rule_id,
                    "message": "Rule promoted successfully"
                })),
                Ok(Err(e)) => Json(json!({
                    "status": "error",
                    "message": format!("Failed to promote rule: {}", e)
                })),
                Err(e) => Json(json!({
                    "status": "error",
                    "message": format!("task join error: {}", e)
                })),
            }
        }
        _ => Json(json!({
            "status": "error",
            "message": "Missing required fields: pattern_id, criteria_file, recommended_option"
        })),
    }
}
