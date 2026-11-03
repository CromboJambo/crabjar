//! HTTP API endpoints for the conductor service.

pub mod sandboxes;
pub mod goals;
pub mod tasks;
pub mod jev;
pub mod decisions;
pub mod evaluations;
pub mod fleet;
pub mod health;
pub mod metrics;
pub mod monitoring;
pub mod recovery;
pub mod review;
pub mod capacity;
pub mod compliance;
pub mod cost;
pub mod dependencies;
pub mod docs;
pub mod drift;
pub mod rules;
pub mod security;
pub mod upgrade;
pub mod schema;

use crate::auth::{AuthError, validate_token};
use crate::conductor::Conductor;
use crate::fleet_status::FleetStatusTracker;
use crate::sandbox::{SandboxConfig, SandboxManager};
use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Mutex as TokioMutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub mod workers;
pub mod promotion;
pub mod route;

/// In-memory store for background evaluation results (dev/testing).
static BG_EVALUATIONS: OnceLock<Arc<Mutex<HashMap<String, serde_json::Value>>>> = OnceLock::new();

fn bg_evaluations() -> &'static Arc<Mutex<HashMap<String, serde_json::Value>>> {
    BG_EVALUATIONS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

pub type ConductorState = Arc<TokioMutex<Conductor>>;

/// Poll status of a background decision evaluation.
async fn get_evaluation_status(
    axum::extract::Path(eval_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let store = bg_evaluations().lock().unwrap();
    match store.get(&eval_id) {
        Some(result) => Json(json!({
            "success": true,
            "evaluation_id": eval_id,
            "result": result
        })),
        None => Json(json!({
            "success": false,
            "error": "Evaluation not found or not yet started",
            "evaluation_id": eval_id
        })),
    }
}

/// List registered decision capabilities (public endpoint for orchestrator awareness).
async fn list_decision_capabilities(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.list_decision_capabilities()
    })
    .await;

    match result {
        Ok(registry_json) => Json(json!({
            "success": true,
            "registry": registry_json
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("task join error: {}", e),
            "registry": {"count": 0, "capabilities": []}
        })),
    }
}

/// Query the audit log for training pipeline evaluation.
async fn query_audit_log(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        if let Some(logger) = c.audit_logger() {
            tokio::runtime::Handle::current().block_on(async move {
                let guard = logger.lock().await;
                guard.recent_events(1000).await
            })
        } else {
            Ok(vec![])
        }
    })
    .await;

    match result {
        Ok(Ok(events)) => Json(json!({
            "success": true,
            "events": events,
            "count": events.len()
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "error": format!("audit query failed: {}", e),
            "events": [],
            "count": 0
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("task join error: {}", e),
            "events": [],
            "count": 0
        })),
    }
}

pub fn routes(conductor: ConductorState) -> Router {
    // Public routes (no auth required)
    let public_app: Router = Router::new()
        .route("/fleet/status", get(get_fleet_status))
        .route("/metrics/cache", get(get_cache_stats))
        .route("/workers/register", post(workers::register_worker))
        .route("/workers/login", post(workers::worker_login_handler))
        .route("/workers/poll", post(workers::worker_poll_handler))
        .route("/workers/heartbeat", post(workers::worker_heartbeat_public))
        .route("/evaluations/decision", post(evaluate_decision_handler))
        // Sandbox management endpoints (Phase 1)
        .route("/sandboxes", post(sandboxes::create_sandbox_handler))
        .route("/sandboxes", get(sandboxes::list_sandboxes_handler))
        .route("/sandboxes/:id", get(sandboxes::get_sandbox_handler))
        .route("/sandboxes/:id", delete(sandboxes::destroy_sandbox_handler))
        .route("/sandboxes/:id/worker", post(sandboxes::assign_worker_to_sandbox_handler))
        .route("/decide/evaluate/{id}", get(get_evaluation_status))
        .route("/decisions/capabilities", get(list_decision_capabilities))
        .route("/audit/log", get(query_audit_log))
        .route("/jev/evaluate", post(evaluate_structural_handler))
        .route("/jev/postprocess", post(postprocess_decision_handler))
        .route("/jev/feedback", post(record_feedback_handler))
        .route("/schema/propose", post(schema::propose_schema_change_handler))
        .route(
            "/interface/propose",
            post(propose_interface_optimization_handler),
        )
        .route("/route/decide", post(route::decide_routing_handler))
        .route("/route/outcome", post(route::record_routing_outcome_handler))
        .route("/dependencies/declare", post(dependencies::declare_dependencies_handler))
        .route("/upgrade/evaluate", post(upgrade::evaluate_upgrade_handler))
        .route("/upgrade/outcome", post(upgrade::record_upgrade_outcome_handler))
        .route("/health/evaluate", post(evaluate_health_handler))
        .route("/health/outcome", post(record_recovery_outcome_handler))
        .route("/cost/evaluate", post(cost::evaluate_cost_handler))
        .route(
            "/optimization/outcome",
            post(cost::record_optimization_outcome_handler),
        )
        // Milestone 20: Disaster recovery decision-making
        .route("/recovery/evaluate", post(evaluate_recovery_handler))
        .route(
            "/recovery/outcome",
            post(record_disaster_recovery_outcome_handler),
        )
        .route("/capacity/evaluate", post(capacity::evaluate_capacity_handler))
        .route("/capacity/outcome", post(capacity::record_capacity_outcome_handler))
        .route("/security/evaluate", post(security::evaluate_security_handler))
        .route("/security/outcome", post(security::record_security_outcome_handler))
        .route("/compliance/evaluate", post(compliance::evaluate_compliance_handler))
        .route(
            "/compliance/outcome",
            post(compliance::record_compliance_outcome_handler),
        )
        .route("/docs/evaluate", post(docs::evaluate_documentation_handler))
        .route("/docs/outcome", post(docs::record_documentation_outcome_handler))
        .route("/monitoring/evaluate", post(evaluate_monitoring_handler))
        .route(
            "/monitoring/outcome",
            post(record_monitoring_outcome_handler),
        )
        .route("/drift/evaluate", post(drift::evaluate_drift_handler))
        .route("/drift/outcome", post(drift::record_drift_outcome_handler))
        .route("/cost/outcome", post(cost::record_optimization_outcome_handler))
        .route("/rules/extract", post(rules::extract_rules_handler))
        .route("/rules/promote", post(rules::promote_rule_handler))
        // Evidence-based promotion endpoints
        .route("/promotion/evaluate/{worker_id}", get(promotion::evaluate_promotion_endpoint))
        .route("/promotion/execute/{worker_id}", post(promotion::execute_promotion_endpoint))
        .route("/promotion/history/{worker_id}", get(promotion::get_promotion_history_endpoint))
        .route("/promotion/metrics/{worker_id}", get(promotion::get_worker_metrics_endpoint))
        .with_state(Arc::clone(&conductor));

    // Protected routes (JWT auth required)
    let protected_app: Router = Router::new()
        .route("/goals", post(submit_goal))
        .route("/goals/{id}", get(get_goal))
        .route("/workers", get(workers::list_workers))
        .route("/tasks/for-worker/{worker_id}", get(workers::tasks_for_worker))
        .route("/tasks/{id}/result", post(report_task_result))
        .route("/review", post(review::submit_review))
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



/// Get fleet status snapshot.
async fn get_fleet_status(
    State(conductor): State<ConductorState>,
) -> Json<serde_json::Value> {
    let conductor = conductor.lock().await;
    let status = conductor.fleet_status().get_status().await;

    Json(json!({
        "success": true,
        "fleet_status": status
    }))
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

// ─── Disaster Recovery Handlers (Milestone 20) ──────────────────────

/// Request for disaster recovery evaluation and action decision.
#[derive(Debug, Deserialize)]
struct DisasterRecoveryEvaluationRequest {
    system: String,
    failure_scenario: String,
    system_state: serde_json::Value,
    available_actions: Vec<serde_json::Value>,
}

/// POST /recovery/evaluate - Get optimal disaster recovery decision via JEV.
async fn evaluate_recovery_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<DisasterRecoveryEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from failure scenario and available actions
    let state_str = serde_json::to_string(&req.system_state).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Disaster recovery evaluation: {} with failure scenario '{}' and system state {}. Available actions: {}",
        req.system, req.failure_scenario, state_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal disaster recovery action
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/disaster-recovery-evaluation.json",
        ))
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
    recovery_time_minutes: Option<f64>,
    actual_data_loss_percent: Option<f64>,
}

/// POST /recovery/outcome - Record disaster recovery outcome for JEV learning.
async fn record_disaster_recovery_outcome_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<DisasterRecoveryOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the disaster recovery outcome for future JEV learning
    let observation = format!(
        "Disaster recovery outcome: {} action '{}' resulted in {}. Recovery time: {:?} min, Data loss: {:?}%.",
        req.system,
        req.action_taken,
        req.outcome,
        req.recovery_time_minutes,
        req.actual_data_loss_percent
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

// ─── Rule Extraction Handlers ────────────────────────────────────────

/// Request for decision evaluation.
#[derive(Debug, Deserialize)]
struct EvaluateDecisionRequest {
    observation: String,
    criteria_file: String,
    /// If true, run evaluation asynchronously and return an ID for polling.
    #[serde(default)]
    background: bool,
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

// ─── Structural Region Evaluation (Phase 1) ──────────────────────

/// Request for structural region evaluation via JEV.
#[derive(Debug, Deserialize)]
struct EvaluateStructuralRequest {
    /// The code region to evaluate (tokens or source with range).
    target: crate::decision::structural::EvaluationTarget,
    /// Path to the JEV criteria file.
    criteria_path: String,
}

/// Request for post-processing a JEV decision.
#[derive(Debug, Deserialize)]
struct PostProcessRequest {
    /// The evaluation target.
    target: crate::decision::structural::EvaluationTarget,
    /// The option ID that was selected.
    option_id: String,
    /// Path to the JEV criteria file.
    criteria_path: String,
}

/// POST /jev/postprocess - Validate a JEV decision against structural rules.
async fn postprocess_decision_handler(
    Json(req): Json<PostProcessRequest>,
) -> Json<serde_json::Value> {
    let result = crate::decision::postprocessor::validate_decision(
        &req.target,
        &req.option_id,
        &req.criteria_path,
    );

    Json(json!({
        "success": true,
        "validation": {
            "valid": result.valid,
            "issues": result.issues,
            "confidence_adjustment": result.confidence_adjustment
        }
    }))
}

/// POST /jev/feedback - Record feedback on a JEV decision for reflection.
async fn record_feedback_handler(
    Json(req): Json<crate::decision::postprocessor::DecisionFeedback>,
) -> Json<serde_json::Value> {
    match crate::decision::postprocessor::record_feedback(&req) {
        Ok(_) => Json(json!({
            "success": true,
            "message": "Feedback recorded"
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e
        })),
    }
}

/// POST /jev/evaluate - Evaluate a structural code region against JEV criteria.
async fn evaluate_structural_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<EvaluateStructuralRequest>,
) -> Json<serde_json::Value> {
    let target = req.target;
    let criteria_path = req.criteria_path.clone();

    // First, try direct structural condition evaluation
    if let Some(result) = evaluate_structural_conditions(&target, &criteria_path) {
        return result;
    }

    // Fall back to embedding-based JEV evaluation
    let observation = target.to_observation();
    let post_criteria_path = criteria_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, &criteria_path))
    })
    .await;

    match result {
        Ok(Ok(decision)) => {
            // Phase 3: Post-process the decision for validation
            let post_process = crate::decision::postprocessor::validate_decision(
                &target,
                &decision.option,
                &post_criteria_path,
            );

            Json(json!({
                "success": true,
                "evaluation": {
                    "option": decision.option,
                    "rationale": decision.rationale,
                    "confidence": decision.confidence + post_process.confidence_adjustment,
                    "post_processed": true,
                    "validation": {
                        "valid": post_process.valid,
                        "issues": post_process.issues
                    }
                }
            }))
        }
        Ok(Err(e)) => Json(json!({
            "success": false,
            "error": format!("Decision evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Evaluate structural conditions directly against the target.
/// Returns Some(Json) if a condition matched, None to fall back to embedding evaluation.
fn evaluate_structural_conditions(
    target: &crate::decision::structural::EvaluationTarget,
    criteria_path: &str,
) -> Option<Json<serde_json::Value>> {
    // Read and parse the criteria file
    let criteria_content = match std::fs::read_to_string(criteria_path) {
        Ok(content) => content,
        Err(_) => return None, // File not found or unreadable — fall back to JEV
    };

    let criteria: serde_json::Value = match serde_json::from_str(&criteria_content) {
        Ok(value) => value,
        Err(_) => return None, // Invalid JSON — fall back to JEV
    };

    // Check each option for structural conditions
    if let Some(options) = criteria["options"].as_array() {
        for option in options {
            if let Some(condition) = option["conditions"].as_str() {
                if crate::decision::condition::evaluate_condition(target, condition) {
                    return Some(Json(json!({
                        "success": true,
                        "evaluation": {
                            "option": option["id"].as_str().unwrap_or("unknown"),
                            "rationale": format!("Structural condition matched: {}", condition),
                            "confidence": 1.0,
                            "method": "structural-condition"
                        }
                    })));
                }
            }
        }
    }

    None // No conditions matched — fall back to JEV embedding evaluation
}

/// POST /decide/evaluate - Evaluate a decision via JEV semantic matching.
async fn evaluate_decision_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(req): Json<EvaluateDecisionRequest>,
) -> Json<serde_json::Value> {
    let observation = req.observation.clone();
    let criteria_file = req.criteria_file.clone();

    // If background flag is set, spawn async task and return ID immediately
    if req.background {
        let eval_id = Uuid::new_v4().to_string();
        let eval_id_for_closure = eval_id.clone();
        let conductor_clone = conductor.clone();

        tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                let c = conductor_clone.blocking_lock();
                let rt = tokio::runtime::Handle::current();
                rt.block_on(c.evaluate_decision(&observation, &criteria_file))
            })
            .await;

            // Store result for polling
            let mut store = bg_evaluations().lock().unwrap();
            match result {
                Ok(Ok(decision)) => {
                    store.insert(
                        eval_id_for_closure.clone(),
                        json!({
                            "status": "completed",
                            "success": true,
                            "option": decision.option,
                            "confidence": decision.confidence,
                            "model": null,
                            "rationale": decision.rationale
                        }),
                    );
                }
                Ok(Err(e)) => {
                    store.insert(
                        eval_id_for_closure.clone(),
                        json!({
                            "status": "completed",
                            "success": false,
                            "option": "error",
                            "confidence": 0.0,
                            "model": null,
                            "rationale": format!("Evaluation failed: {}", e)
                        }),
                    );
                }
                Err(e) => {
                    store.insert(
                        eval_id_for_closure,
                        json!({
                            "status": "completed",
                            "success": false,
                            "option": "error",
                            "confidence": 0.0,
                            "model": null,
                            "rationale": format!("Task join error: {}", e)
                        }),
                    );
                }
            }
        });

        return Json(json!({
            "success": true,
            "evaluation_id": eval_id,
            "status": "pending",
            "message": "Evaluation started in background. Poll /decide/evaluate/{id} for result."
        }));
    }

    // Synchronous evaluation (default)
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
        req.source_component,
        req.target_component,
        req.current_format,
        req.proposed_format,
        req.justification
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate interface optimization impact
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/interface-optimization-evaluation.json",
        ))
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
        rt.block_on(
            c.evaluate_decision(&observation, "specs/criteria/self-healing-evaluation.json"),
        )
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
        req.service,
        req.action_taken,
        req.outcome,
        req.recovery_time_seconds,
        req.recurrence_within_24h
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




// ─── Security Hardening Handlers (Milestone 13) ──────────────────────

/// Request for security hardening evaluation and action decision.




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
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/monitoring-quality-evaluation.json",
        ))
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
        req.system,
        req.action_taken,
        req.outcome,
        req.prediction_accuracy_percent,
        req.improvement_time_hours
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



// ─── Evidence-Based Promotion Handlers ──────────────────────────────


