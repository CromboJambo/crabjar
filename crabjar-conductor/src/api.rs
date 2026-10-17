//! HTTP API endpoints for the conductor service.

use crate::auth::{AuthError, validate_token};
use crate::conductor::Conductor;
use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Json as JsonResponse},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub type ConductorState = Arc<Mutex<Conductor>>;

pub fn routes(conductor: ConductorState) -> Router {
    // Public routes (no auth required)
    let public_app: Router = Router::new()
        .route("/metrics/cache", get(get_cache_stats))
        .route("/decide/evaluate", post(evaluate_decision_handler))
        .route("/rules/extract", post(extract_rules_handler))
        .route("/rules/promote", post(promote_rule_handler))
        .with_state(Arc::clone(&conductor));

    // Protected routes (JWT auth required)
    let protected_app: Router = Router::new()
        .route("/goals", post(submit_goal))
        .route("/goals/{id}", get(get_goal))
        .route("/workers", get(list_workers))
        .route("/workers/register", post(register_worker))
        .route("/workers/{id}/heartbeat", post(worker_heartbeat))
        .route("/tasks/for-worker/{worker_id}", get(tasks_for_worker))
        .route("/tasks/{id}/result", post(report_task_result))
        .route("/review", post(submit_review))
        .layer(middleware::from_fn(jwt_auth_middleware))
        .with_state(Arc::clone(&conductor));

    // Merge public and protected routes
    let app = public_app.merge(protected_app);

    // Add request logging middleware to all routes
    app.layer(middleware::from_fn(log_requests))
}

/// JWT authentication middleware.
async fn jwt_auth_middleware(request: Request, next: Next) -> axum::response::Response {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Parse Bearer token
    let token = if auth_header.starts_with("Bearer ") {
        &auth_header[7..]
    } else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "error",
                "message": "missing or invalid Authorization header"
            })),
        )
            .into_response();
    };

    // Validate token
    match validate_token(token) {
        Ok(_claims) => {
            // Token is valid — continue to handler
            next.run(request).await
        }
        Err(AuthError::Expired) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "error",
                "message": "token expired"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "error",
                "message": format!("invalid token: {}", e)
            })),
        )
            .into_response(),
    }
}

async fn log_requests(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = std::time::Instant::now();

    debug!(method = %method, path = %path, "Incoming request");

    let resp = next.run(req).await;

    let elapsed = start.elapsed();
    info!(method = %method, path = %path, status = resp.status().as_u16(), elapsed_ms = elapsed.as_millis(), "Request completed");

    resp
}

async fn submit_goal(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let description = body["description"]
        .as_str()
        .unwrap_or("unnamed goal")
        .to_string();

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.submit_goal(description)
    })
    .await;

    match result {
        Ok(Ok(goal)) => Json(json!({
            "status": "accepted",
            "goal_id": goal.id,
            "description": goal.description
        })),
        Ok(Err(e)) => Json(json!({
            "status": "error",
            "message": e.to_string()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("task join error: {}", e)
        })),
    }
}

async fn get_goal(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let goal_id = id.0;
    let goal_id_for_response = goal_id.clone();

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.get_goal(&goal_id)
    })
    .await;

    match result {
        Ok(Ok(Some(goal))) => Json(json!({
            "goal_id": goal.id,
            "description": goal.description,
            "status": goal.status,
            "created_at": goal.created_at.to_rfc3339()
        })),
        Ok(Ok(None)) => Json(json!({
            "goal_id": goal_id_for_response,
            "status": "not-found"
        })),
        Ok(Err(e)) => Json(json!({
            "goal_id": goal_id_for_response,
            "status": "error",
            "message": e.to_string()
        })),
        Err(e) => Json(json!({
            "goal_id": goal_id_for_response,
            "status": "error",
            "message": format!("task join error: {}", e)
        })),
    }
}

async fn list_workers(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.list_workers()
    })
    .await;

    match result {
        Ok(workers) => Json(json!({
            "workers": workers.iter().map(|w| json!({
                "id": w.id,
                "name": w.name,
                "status": w.status,
                "capabilities": w.capabilities
            })).collect::<Vec<_>>()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("task join error: {}", e)
        })),
    }
}

async fn register_worker(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("unknown").to_string();
    let capabilities: Vec<String> = body["capabilities"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let ssh_key = body["ssh_key"].as_str().map(|s| s.to_string());

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.register_worker(name, capabilities, ssh_key)
    })
    .await;

    match result {
        Ok(Ok((worker, token))) => Json(json!({
            "status": "registered",
            "worker_id": worker.id,
            "name": worker.name,
            "auth_token": token
        })),
        Ok(Err(e)) => Json(json!({
            "status": "error",
            "message": e.to_string()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("task join error: {}", e)
        })),
    }
}

async fn worker_heartbeat(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let worker_id = id.0;

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.worker_heartbeat(&worker_id)
    })
    .await;

    match result {
        Ok(Ok(())) => Json(json!({ "status": "ok" })),
        Ok(Err(e)) => Json(json!({
            "status": "error",
            "message": e.to_string()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("task join error: {}", e)
        })),
    }
}

async fn tasks_for_worker(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let worker_id = id.0;

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.tasks_for_worker(&worker_id)
    })
    .await;

    match result {
        Ok(Ok(tasks)) => Json(json!({
            "tasks": tasks.iter().map(|t| json!({
                "id": t.id,
                "goal_id": t.goal_id,
                "command": t.command,
                "status": t.status
            })).collect::<Vec<_>>()
        })),
        Ok(Err(e)) => Json(json!({
            "status": "error",
            "message": e.to_string()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("task join error: {}", e)
        })),
    }
}

async fn report_task_result(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let task_id = id.0;
    let status = body["status"].as_str().unwrap_or("completed").to_string();
    let result_text = body["result"].as_str().map(|s| s.to_string());

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        match status.as_str() {
            "failed" => {
                let error = result_text.unwrap_or_default();
                c.report_task_failed(&task_id, error)
            }
            _ => c.report_task_complete(&task_id, result_text),
        }
    })
    .await;

    match result {
        Ok(Ok(Some(new_token))) => Json(json!({
            "status": "recorded",
            "trust_level_changed": true,
            "auth_token": new_token
        })),
        Ok(Ok(None)) => Json(json!({ "status": "recorded" })),
        Ok(Err(e)) => Json(json!({
            "status": "error",
            "message": e.to_string()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("task join error: {}", e)
        })),
    }
}

async fn submit_review(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let repo_url = body["repo_url"].as_str().unwrap_or("").to_string();
    let pr_number = body["pr_number"].as_i64();
    let commit_sha = body["commit_sha"].as_str().unwrap_or("").to_string();
    let diff = body["diff"].as_str().unwrap_or("").to_string();

    info!(repo = %repo_url, pr = ?pr_number, sha = %commit_sha, "Submitting code review");

    // Run the deterministic pre-processor using git2 directly for the demo
    // In production, this would clone and inspect the actual repo
    let changed_files = parse_diff_to_changed_files(&diff);

    if changed_files.is_empty() {
        return Json(json!({
            "status": "completed",
            "findings": [],
            "message": "No relevant changed files detected"
        }));
    }

    info!(
        files = changed_files.len(),
        "Changed files selected for review"
    );

    // Smart bundling
    let bundles = crabjar_review::bundling::create_bundles(changed_files);
    info!(bundles = bundles.len(), "Files bundled for review");

    // Load default rules
    let default_rules = r#"
[[rules]]
id = "check_errors"
applies_to = ["source"]
not_applies_to = ["tests", "generated"]
severity = "high"
description = "Errors must be propagated or explicitly handled"

[[rules]]
id = "check_unreachable"
applies_to = ["source"]
severity = "medium"
description = "Check for unreachable code after return/break/continue"

[[rules]]
id = "check_style"
applies_to = ["source", "tests"]
severity = "low"
description = "Style and formatting conventions"
"#;

    let rules = match crabjar_review::rules::load_rules_from_str(default_rules) {
        Ok(r) => r,
        Err(e) => {
            return Json(json!({
                "status": "error",
                "message": format!("Failed to load review rules: {}", e)
            }));
        }
    };

    // Agent review phase (per bundle)
    let mut all_findings = Vec::new();

    for bundle in &bundles {
        // Match rules per file within the bundle
        for file in &bundle.files {
            let matched_rules = crabjar_review::rules::match_rules(file, &rules);

            if matched_rules.is_empty() {
                continue;
            }

            // Generate review prompt
            let scenario = "general".to_string();
            let owned_rules: Vec<crabjar_review::types::ReviewRule> =
                matched_rules.iter().map(|r| (*r).clone()).collect();
            let prompt =
                crabjar_review::prompts::generate_review_prompt(bundle, &owned_rules, &scenario);

            debug!(bundle_files = bundle.files.len(), "Reviewing bundle");

            // Simulated agent output (replace with real pesti-server call)
            let mock_findings = simulate_bundle_review(bundle, &owned_rules);
            all_findings.extend(mock_findings);
        }
    }

    info!(
        findings = all_findings.len(),
        "Raw findings collected from review"
    );

    // Post-processing: deduplication and reflection
    let processed_findings = crabjar_review::reflection::process_findings(&all_findings);
    info!(
        processed_findings = processed_findings.len(),
        "Findings after post-processing"
    );

    // Generate summary
    let summary = crabjar_review::reflection::generate_reflection(&processed_findings);

    Json(json!({
        "status": "completed",
        "repo_url": repo_url,
        "pr_number": pr_number,
        "commit_sha": commit_sha,
        "bundles_reviewed": bundles.len(),
        "findings_count": processed_findings.len(),
        "findings": processed_findings,
        "summary": summary
    }))
}

/// Parse a unified diff into ChangedFile structs for review.
fn parse_diff_to_changed_files(diff: &str) -> Vec<crabjar_review::types::ChangedFile> {
    let mut files = Vec::new();
    let mut current_file = None;

    for line in diff.lines() {
        if line.starts_with("diff --git") {
            if let Some(f) = current_file.take() {
                files.push(f);
            }
            // Extract filename from "diff --git a/path b/path"
            if let Some(parts) = line.split_whitespace().nth(3) {
                let path = parts.strip_prefix("b/").unwrap_or(parts).to_string();
                current_file = Some(crabjar_review::types::ChangedFile {
                    path: std::path::PathBuf::from(&path),
                    file_type: classify_file_type(&path),
                    status: crabjar_review::types::FileStatus::Modified,
                    additions: 0,
                    deletions: 0,
                });
            }
        } else if line.starts_with("+++") && current_file.is_some() {
            // Count additions
            if let Some(f) = current_file.as_mut() {
                f.additions += 1;
            }
        } else if line.starts_with("---") && current_file.is_some() {
            // Count deletions
            if let Some(f) = current_file.as_mut() {
                f.deletions += 1;
            }
        }
    }

    if let Some(f) = current_file.take() {
        files.push(f);
    }

    files
}

/// Classify file type based on extension and path.
fn classify_file_type(path: &str) -> crabjar_review::types::FileType {
    if path.ends_with(".rs")
        || path.ends_with(".ts")
        || path.ends_with(".js")
        || path.ends_with(".py")
        || path.ends_with(".go")
    {
        crabjar_review::types::FileType::Source
    } else if path.contains("test") || path.ends_with("_test.rs") {
        crabjar_review::types::FileType::Test
    } else if path.ends_with(".json") || path.ends_with(".yaml") || path.ends_with(".toml") {
        crabjar_review::types::FileType::Config
    } else if path.ends_with(".md") {
        crabjar_review::types::FileType::Docs
    } else {
        crabjar_review::types::FileType::Unknown
    }
}

/// Simulate agent review for a bundle (placeholder until pesti-server integration).
fn simulate_bundle_review(
    bundle: &crabjar_review::types::ReviewBundle,
    matched_rules: &[crabjar_review::types::ReviewRule],
) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();

    for file in &bundle.files {
        if let Ok(content) = std::fs::read_to_string(&file.path) {
            for rule in matched_rules {
                match rule.id.as_str() {
                    "check_errors" => {
                        if content.contains("unwrap()")
                            && !file.path.to_string_lossy().contains("test")
                        {
                            findings.push(serde_json::json!({
                                "file": file.path.to_string_lossy(),
                                "line": 1,
                                "severity": "high",
                                "type": "bug",
                                "rule_id": rule.id,
                                "description": "unwrap() called without error handling"
                            }));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    findings
}

/// Get JEV decision cache statistics (public metrics endpoint).
async fn get_cache_stats(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.cache_stats()
    })
    .await;

    match result {
        Ok(Some(stats)) => Json(json!({
            "status": "ok",
            "cache": {
                "entries": stats.entries,
                "max_entries": stats.max_entries,
                "hits": stats.hits,
                "misses": stats.misses,
                "evictions": stats.evictions,
                "hit_rate_percent": stats.hit_rate
            }
        })),
        Ok(None) => Json(json!({
            "status": "ok",
            "cache": null,
            "message": "JEV decision gate not configured"
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("task join error: {}", e)
        })),
    }
}

// ─── Rule Extraction Handlers ────────────────────────────────────────

/// Request for decision evaluation.
#[derive(Debug, Deserialize)]
struct EvaluateDecisionRequest {
    observation: String,
    criteria_file: String,
}

/// Response for decision evaluation.
#[derive(Debug, Serialize)]
struct EvaluateDecisionResponse {
    success: bool,
    option: String,
    confidence: f64,
    model: Option<String>,
    rationale: String,
}

/// POST /decide/evaluate - Evaluate a decision via JEV semantic matching.
async fn evaluate_decision_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<EvaluateDecisionRequest>,
) -> Json<serde_json::Value> {
    let observation = req.observation;
    let criteria_file = req.criteria_file;

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use block_on to run the async evaluate_decision method within spawn_blocking
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, &criteria_file))
    })
    .await;

    match result {
        Ok(Ok(decision)) => Json(json!({
            "success": true,
            "option": decision.option,
            "confidence": decision.confidence,
            "model": null,
            "rationale": decision.rationale
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "option": "error",
            "confidence": 0.0,
            "model": null,
            "rationale": format!("Evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "option": "error",
            "confidence": 0.0,
            "model": null,
            "rationale": format!("Task join error: {}", e)
        })),
    }
}

/// POST /rules/extract - Extract candidate rules from decision history.
async fn extract_rules_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let criteria_file = body["criteria_file"]
        .as_str()
        .unwrap_or("default")
        .to_string();

    let min_occurrences = body["min_occurrences"]
        .as_i64()
        .unwrap_or(3);

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
async fn promote_rule_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let pattern_id = body["pattern_id"]
        .as_str()
        .map(|s| s.to_string());
    let criteria_file = body["criteria_file"]
        .as_str()
        .map(|s| s.to_string());
    let recommended_option = body["recommended_option"]
        .as_str()
        .map(|s| s.to_string());

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
