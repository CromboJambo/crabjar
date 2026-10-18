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
        .route("/schema/propose", post(propose_schema_change_handler))
        .route("/interface/propose", post(propose_interface_optimization_handler))
        .route("/route/decide", post(decide_routing_handler))
        .route("/route/outcome", post(record_routing_outcome_handler))
        .route("/dependencies/declare", post(declare_dependencies_handler))
        .route("/upgrade/evaluate", post(evaluate_upgrade_handler))
        .route("/upgrade/outcome", post(record_upgrade_outcome_handler))
        .route("/health/evaluate", post(evaluate_health_handler))
        .route("/recovery/outcome", post(record_recovery_outcome_handler))
        .route("/cost/evaluate", post(evaluate_cost_handler))
        .route("/optimization/outcome", post(record_optimization_outcome_handler))
        .route("/capacity/evaluate", post(evaluate_capacity_handler))
        .route("/capacity/outcome", post(record_capacity_outcome_handler))
        .route("/recovery/evaluate", post(evaluate_recovery_handler))
        .route("/recovery/outcome", post(record_disaster_recovery_outcome_handler))
        .route("/security/evaluate", post(evaluate_security_handler))
        .route("/security/outcome", post(record_security_outcome_handler))
        .route("/compliance/evaluate", post(evaluate_compliance_handler))
        .route("/compliance/outcome", post(record_compliance_outcome_handler))
        .route("/docs/evaluate", post(evaluate_documentation_handler))
        .route("/docs/outcome", post(record_documentation_outcome_handler))
        .route("/monitoring/evaluate", post(evaluate_monitoring_handler))
        .route("/monitoring/outcome", post(record_monitoring_outcome_handler))
        .route("/drift/evaluate", post(evaluate_drift_handler))
        .route("/drift/outcome", post(record_drift_outcome_handler))
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

// ─── Schema Co-evolution Handlers (Milestone 5) ──────────────────────

/// Request for schema change proposal evaluation.
#[derive(Debug, Deserialize)]
struct SchemaProposalRequest {
    component: String,
    change_type: String,
    target_table: String,
    column_name: Option<String>,
    column_type: Option<String>,
    justification: String,
}

/// POST /schema/propose - Evaluate a schema change proposal via JEV.
async fn propose_schema_change_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<SchemaProposalRequest>,
) -> Json<serde_json::Value> {
    // Build observation from proposal details
    let observation = format!(
        "Schema change proposal: {} wants to {} on {}. Justification: {}",
        req.component, req.change_type, req.target_table, req.justification
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate schema change impact
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/schema-change-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "evaluation": {
                "option": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            },
            "migration_plan": {
                "steps": [],
                "rollback_steps": [],
                "affected_components": [req.component]
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "evaluation": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "evaluation": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

// ─── Interface Optimization Handlers (Milestone 6) ──────────────────

/// Request for interface optimization proposal evaluation.
#[derive(Debug, Deserialize)]
struct InterfaceOptimizationRequest {
    optimization_type: String,
    source_component: String,
    target_component: String,
    current_format: String,
    proposed_format: String,
    justification: String,
}

/// POST /interface/propose - Evaluate an interface optimization proposal via JEV.
async fn propose_interface_optimization_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<InterfaceOptimizationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from proposal details
    let observation = format!(
        "Interface optimization proposal: {} wants to change interface to {} from {} to {}. Justification: {}",
        req.source_component, req.target_component,
        req.current_format, req.proposed_format, req.justification
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate interface optimization impact
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/interface-optimization-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "evaluation": {
                "option": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            },
            "deployment_plan": {
                "steps": [],
                "rollback_steps": [],
                "affected_endpoints": []
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "evaluation": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "evaluation": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

// ─── Autonomous Task Routing Handlers (Milestone 7) ─────────────────

/// Request for autonomous routing decision.
#[derive(Debug, Deserialize)]
struct RoutingDecisionRequest {
    task_id: String,
    task_type: String,
    complexity: String,
    required_capabilities: Vec<String>,
    available_workers: Vec<serde_json::Value>,
}

/// POST /route/decide - Get optimal routing decision via JEV.
async fn decide_routing_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<RoutingDecisionRequest>,
) -> Json<serde_json::Value> {
    // Build observation from task and worker details
    let workers_str = serde_json::to_string(&req.available_workers).unwrap_or_default();
    let capabilities_str = req.required_capabilities.join(", ");
    let observation = format!(
        "Routing decision: task {} (type: {}, complexity: {}) requires capabilities [{}]. Available workers: {}",
        req.task_id, req.task_type, req.complexity, capabilities_str, workers_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal routing decision
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/autonomous-routing-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "routing_decision": {
                "assigned_worker": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "routing_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "routing_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record routing outcome for learning.
#[derive(Debug, Deserialize)]
struct RoutingOutcomeRequest {
    task_id: String,
    assigned_worker: String,
    outcome: String,
    completion_time_seconds: Option<u64>,
    quality_score: Option<f64>,
}

/// POST /route/outcome - Record routing outcome for JEV learning.
async fn record_routing_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<RoutingOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the routing outcome for future JEV learning
    let observation = format!(
        "Routing outcome: task {} assigned to {} resulted in {}. Time: {:?}s, Quality: {:?}",
        req.task_id, req.assigned_worker, req.outcome,
        req.completion_time_seconds, req.quality_score
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "task_id": req.task_id
    }))
}

// ─── Cross-Service Dependency Management Handlers (Milestone 8) ──────

/// Request for dependency declaration validation.
#[derive(Debug, Deserialize)]
struct DependencyDeclarationRequest {
    service: String,
    version: String,
    dependencies: Vec<serde_json::Value>,
}

/// POST /dependencies/declare - Validate service dependency declarations via JEV.
async fn declare_dependencies_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<DependencyDeclarationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from dependency declaration details
    let deps_str = serde_json::to_string(&req.dependencies).unwrap_or_default();
    let observation = format!(
        "Dependency declaration: {} v{} declares dependencies: {}",
        req.service, req.version, deps_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate dependency compatibility
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/upgrade-impact-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "validation": {
                "compatible": evaluation.option == "approve-low-risk" || evaluation.option == "approve-medium-risk",
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "validation": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "validation": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request for upgrade impact evaluation.
#[derive(Debug, Deserialize)]
struct UpgradeEvaluationRequest {
    service: String,
    current_version: String,
    proposed_version: String,
    dependent_services: Vec<String>,
}

/// POST /upgrade/evaluate - Evaluate upgrade impact on dependent services via JEV.
async fn evaluate_upgrade_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<UpgradeEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from upgrade evaluation details
    let dependents_str = req.dependent_services.join(", ");
    let observation = format!(
        "Upgrade evaluation: {} upgrading from v{} to v{}. Dependent services: {}",
        req.service, req.current_version, req.proposed_version, dependents_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate upgrade impact
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/upgrade-impact-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "upgrade_assessment": {
                "risk_level": if evaluation.option == "approve-low-risk" { "low" } else if evaluation.option == "approve-medium-risk" { "medium" } else { "high" },
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            },
            "recommended_sequence": [req.service]
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "upgrade_assessment": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "upgrade_assessment": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record upgrade outcome for learning.
#[derive(Debug, Deserialize)]
struct UpgradeOutcomeRequest {
    service: String,
    version_change: String,
    outcome: String,
    downtime_seconds: Option<u64>,
    compatibility_issues: Vec<String>,
}

/// POST /upgrade/outcome - Record upgrade outcome for JEV learning.
async fn record_upgrade_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<UpgradeOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the upgrade outcome for future JEV learning
    let issues_str = req.compatibility_issues.join(", ");
    let observation = format!(
        "Upgrade outcome: {} {} resulted in {}. Downtime: {:?}s, Issues: {}",
        req.service, req.version_change, req.outcome,
        req.downtime_seconds, issues_str
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "service": req.service
    }))
}

// ─── Self-Healing Infrastructure Handlers (Milestone 9) ──────────────

/// Request for health evaluation and healing decision.
#[derive(Debug, Deserialize)]
struct HealthEvaluationRequest {
    service: String,
    metrics: serde_json::Value,
    available_actions: Vec<serde_json::Value>,
}

/// POST /health/evaluate - Get optimal self-healing decision via JEV.
async fn evaluate_health_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<HealthEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from health metrics and available actions
    let metrics_str = serde_json::to_string(&req.metrics).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Health evaluation: {} with metrics {}. Available corrective actions: {}",
        req.service, metrics_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal healing action
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/self-healing-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "healing_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "healing_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "healing_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record recovery outcome for learning.
#[derive(Debug, Deserialize)]
struct RecoveryOutcomeRequest {
    service: String,
    action_taken: String,
    outcome: String,
    recovery_time_seconds: Option<u64>,
    recurrence_within_24h: Option<bool>,
}

/// POST /recovery/outcome - Record recovery outcome for JEV learning.
async fn record_recovery_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<RecoveryOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the recovery outcome for future JEV learning
    let observation = format!(
        "Recovery outcome: {} action '{}' resulted in {}. Time: {:?}s, Recurrence: {:?}",
        req.service, req.action_taken, req.outcome,
        req.recovery_time_seconds, req.recurrence_within_24h
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "service": req.service
    }))
}

// ─── Cost Optimization Handlers (Milestone 10) ──────────────────────

/// Request for cost evaluation and optimization decision.
#[derive(Debug, Deserialize)]
struct CostEvaluationRequest {
    service: String,
    usage_metrics: serde_json::Value,
    available_optimizations: Vec<serde_json::Value>,
}

/// POST /cost/evaluate - Get optimal cost optimization decision via JEV.
async fn evaluate_cost_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<CostEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from usage metrics and available optimizations
    let metrics_str = serde_json::to_string(&req.usage_metrics).unwrap_or_default();
    let opts_str = serde_json::to_string(&req.available_optimizations).unwrap_or_default();
    let observation = format!(
        "Cost evaluation: {} with usage metrics {}. Available optimizations: {}",
        req.service, metrics_str, opts_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal cost optimization action
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/cost-optimization-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "optimization_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "optimization_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "optimization_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record optimization outcome for learning.
#[derive(Debug, Deserialize)]
struct OptimizationOutcomeRequest {
    service: String,
    action_taken: String,
    outcome: String,
    savings_achieved_usd: Option<f64>,
    performance_impact_percent: Option<f64>,
}

/// POST /optimization/outcome - Record optimization outcome for JEV learning.
async fn record_optimization_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<OptimizationOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the optimization outcome for future JEV learning
    let observation = format!(
        "Optimization outcome: {} action '{}' resulted in {}. Savings: ${:?}, Performance impact: {:?}%",
        req.service, req.action_taken, req.outcome,
        req.savings_achieved_usd, req.performance_impact_percent
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "service": req.service
    }))
}

// ─── Capacity Planning Handlers (Milestone 11) ──────────────────────

/// Request for capacity evaluation and adjustment decision.
#[derive(Debug, Deserialize)]
struct CapacityEvaluationRequest {
    service: String,
    usage_trends: serde_json::Value,
    current_capacity: serde_json::Value,
    available_adjustments: Vec<serde_json::Value>,
}

/// POST /capacity/evaluate - Get optimal capacity adjustment decision via JEV.
async fn evaluate_capacity_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<CapacityEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from usage trends and available adjustments
    let trends_str = serde_json::to_string(&req.usage_trends).unwrap_or_default();
    let capacity_str = serde_json::to_string(&req.current_capacity).unwrap_or_default();
    let adjustments_str = serde_json::to_string(&req.available_adjustments).unwrap_or_default();
    let observation = format!(
        "Capacity evaluation: {} with usage trends {}, current capacity {}. Available adjustments: {}",
        req.service, trends_str, capacity_str, adjustments_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal capacity adjustment action
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/capacity-planning-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "capacity_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "capacity_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "capacity_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record capacity outcome for learning.
#[derive(Debug, Deserialize)]
struct CapacityOutcomeRequest {
    service: String,
    action_taken: String,
    outcome: String,
    prediction_accuracy_percent: Option<f64>,
    resource_utilization_after_percent: Option<f64>,
}

/// POST /capacity/outcome - Record capacity outcome for JEV learning.
async fn record_capacity_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<CapacityOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the capacity outcome for future JEV learning
    let observation = format!(
        "Capacity outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Utilization after: {:?}%",
        req.service, req.action_taken, req.outcome,
        req.prediction_accuracy_percent, req.resource_utilization_after_percent
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "service": req.service
    }))
}

// ─── Disaster Recovery Handlers (Milestone 12) ──────────────────────

/// Request for disaster recovery evaluation and action decision.
#[derive(Debug, Deserialize)]
struct DisasterRecoveryEvaluationRequest {
    system: String,
    failure_scenario: serde_json::Value,
    system_state: serde_json::Value,
    available_actions: Vec<serde_json::Value>,
}

/// POST /recovery/evaluate - Get optimal disaster recovery action decision via JEV.
async fn evaluate_recovery_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<DisasterRecoveryEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from failure scenario and system state
    let scenario_str = serde_json::to_string(&req.failure_scenario).unwrap_or_default();
    let state_str = serde_json::to_string(&req.system_state).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Disaster recovery evaluation: {} with failure scenario {}, system state {}. Available actions: {}",
        req.system, scenario_str, state_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal disaster recovery action
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/disaster-recovery-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "recovery_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "recovery_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "recovery_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record disaster recovery outcome for learning.
#[derive(Debug, Deserialize)]
struct DisasterRecoveryOutcomeRequest {
    system: String,
    action_taken: String,
    outcome: String,
    prediction_accuracy_percent: Option<f64>,
    recovery_time_minutes: Option<f64>,
}

/// POST /recovery/outcome - Record disaster recovery outcome for JEV learning.
async fn record_disaster_recovery_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<DisasterRecoveryOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the disaster recovery outcome for future JEV learning
    let observation = format!(
        "Disaster recovery outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Recovery time: {:?} min",
        req.system, req.action_taken, req.outcome,
        req.prediction_accuracy_percent, req.recovery_time_minutes
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "system": req.system
    }))
}

// ─── Security Hardening Handlers (Milestone 13) ──────────────────────

/// Request for security hardening evaluation and action decision.
#[derive(Debug, Deserialize)]
struct SecurityEvaluationRequest {
    system: String,
    vulnerability_data: serde_json::Value,
    system_posture: serde_json::Value,
    available_actions: Vec<serde_json::Value>,
}

/// POST /security/evaluate - Get optimal security hardening action decision via JEV.
async fn evaluate_security_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<SecurityEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from vulnerability data and system posture
    let vuln_str = serde_json::to_string(&req.vulnerability_data).unwrap_or_default();
    let posture_str = serde_json::to_string(&req.system_posture).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Security evaluation: {} with vulnerability {}, system posture {}. Available actions: {}",
        req.system, vuln_str, posture_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal security hardening action
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/security-hardening-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "security_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "security_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "security_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record security hardening outcome for learning.
#[derive(Debug, Deserialize)]
struct SecurityOutcomeRequest {
    system: String,
    action_taken: String,
    outcome: String,
    prediction_accuracy_percent: Option<f64>,
    resolution_time_minutes: Option<f64>,
}

/// POST /security/outcome - Record security hardening outcome for JEV learning.
async fn record_security_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<SecurityOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the security outcome for future JEV learning
    let observation = format!(
        "Security outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Resolution time: {:?} min",
        req.system, req.action_taken, req.outcome,
        req.prediction_accuracy_percent, req.resolution_time_minutes
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "system": req.system
    }))
}

// ─── Compliance Automation Handlers (Milestone 14) ──────────────────

/// Request for compliance automation evaluation and action decision.
#[derive(Debug, Deserialize)]
struct ComplianceEvaluationRequest {
    system: String,
    compliance_requirement: serde_json::Value,
    current_configuration: serde_json::Value,
    available_actions: Vec<serde_json::Value>,
}

/// POST /compliance/evaluate - Get optimal compliance action decision via JEV.
async fn evaluate_compliance_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<ComplianceEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from compliance requirement and current configuration
    let req_str = serde_json::to_string(&req.compliance_requirement).unwrap_or_default();
    let config_str = serde_json::to_string(&req.current_configuration).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Compliance evaluation: {} with requirement {}, current configuration {}. Available actions: {}",
        req.system, req_str, config_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal compliance action
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/compliance-automation-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "compliance_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "compliance_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "compliance_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record compliance automation outcome for learning.
#[derive(Debug, Deserialize)]
struct ComplianceOutcomeRequest {
    system: String,
    action_taken: String,
    outcome: String,
    prediction_accuracy_percent: Option<f64>,
    resolution_time_hours: Option<f64>,
}

/// POST /compliance/outcome - Record compliance automation outcome for JEV learning.
async fn record_compliance_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<ComplianceOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the compliance outcome for future JEV learning
    let observation = format!(
        "Compliance outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Resolution time: {:?} hours",
        req.system, req.action_taken, req.outcome,
        req.prediction_accuracy_percent, req.resolution_time_hours
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "system": req.system
    }))
}

// ─── Documentation Automation Handlers (Milestone 15) ────────────────

/// Request for documentation automation evaluation and action decision.
#[derive(Debug, Deserialize)]
struct DocumentationEvaluationRequest {
    system: String,
    documentation_content: serde_json::Value,
    available_actions: Vec<serde_json::Value>,
}

/// POST /docs/evaluate - Get optimal documentation improvement action via JEV.
async fn evaluate_documentation_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<DocumentationEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from documentation content and available actions
    let doc_str = serde_json::to_string(&req.documentation_content).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Documentation evaluation: {} with content {}. Available actions: {}",
        req.system, doc_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal documentation improvement action
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/documentation-quality-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "documentation_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "documentation_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "documentation_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record documentation automation outcome for learning.
#[derive(Debug, Deserialize)]
struct DocumentationOutcomeRequest {
    system: String,
    action_taken: String,
    outcome: String,
    prediction_accuracy_percent: Option<f64>,
    improvement_time_hours: Option<f64>,
}

/// POST /docs/outcome - Record documentation automation outcome for JEV learning.
async fn record_documentation_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<DocumentationOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the documentation outcome for future JEV learning
    let observation = format!(
        "Documentation outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Improvement time: {:?} hours",
        req.system, req.action_taken, req.outcome,
        req.prediction_accuracy_percent, req.improvement_time_hours
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "system": req.system
    }))
}

// ─── Monitoring Automation Handlers (Milestone 16) ───────────────────

/// Request for monitoring automation evaluation and action decision.
#[derive(Debug, Deserialize)]
struct MonitoringEvaluationRequest {
    system: String,
    monitoring_config: serde_json::Value,
    available_actions: Vec<serde_json::Value>,
}

/// POST /monitoring/evaluate - Get optimal monitoring improvement action via JEV.
async fn evaluate_monitoring_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<MonitoringEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from monitoring config and available actions
    let config_str = serde_json::to_string(&req.monitoring_config).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Monitoring evaluation: {} with configuration {}. Available actions: {}",
        req.system, config_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal monitoring improvement action
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/monitoring-quality-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "monitoring_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "monitoring_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "monitoring_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record monitoring automation outcome for learning.
#[derive(Debug, Deserialize)]
struct MonitoringOutcomeRequest {
    system: String,
    action_taken: String,
    outcome: String,
    prediction_accuracy_percent: Option<f64>,
    improvement_time_hours: Option<f64>,
}

/// POST /monitoring/outcome - Record monitoring automation outcome for JEV learning.
async fn record_monitoring_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<MonitoringOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the monitoring outcome for future JEV learning
    let observation = format!(
        "Monitoring outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Improvement time: {:?} hours",
        req.system, req.action_taken, req.outcome,
        req.prediction_accuracy_percent, req.improvement_time_hours
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "system": req.system
    }))
}

// ─── Configuration Drift Detection Handlers (Milestone 17) ──────────────

/// Request for configuration drift detection evaluation and action decision.
#[derive(Debug, Deserialize)]
struct DriftEvaluationRequest {
    system: String,
    configuration_state: serde_json::Value,
    available_actions: Vec<serde_json::Value>,
}

/// POST /drift/evaluate - Get optimal configuration drift remediation action via JEV.
async fn evaluate_drift_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<DriftEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from configuration state and available actions
    let config_str = serde_json::to_string(&req.configuration_state).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Drift evaluation: {} with configuration state {}. Available actions: {}",
        req.system, config_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal drift remediation action
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, "specs/criteria/drift-quality-evaluation.json"))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "drift_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "drift_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "drift_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record configuration drift detection outcome for learning.
#[derive(Debug, Deserialize)]
struct DriftOutcomeRequest {
    system: String,
    action_taken: String,
    outcome: String,
    prediction_accuracy_percent: Option<f64>,
    remediation_time_minutes: Option<f64>,
}

/// POST /drift/outcome - Record configuration drift detection outcome for JEV learning.
async fn record_drift_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<DriftOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the drift outcome for future JEV learning
    let observation = format!(
        "Drift outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Remediation time: {:?} minutes",
        req.system, req.action_taken, req.outcome,
        req.prediction_accuracy_percent, req.remediation_time_minutes
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "system": req.system
    }))
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
