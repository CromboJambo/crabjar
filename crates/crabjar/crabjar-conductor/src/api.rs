//! HTTP API endpoints for the conductor service.

pub mod capacity;
pub mod compliance;
pub mod cost;
pub mod decisions;
pub mod dependencies;
pub mod docs;
pub mod drift;
pub mod evaluations;
pub mod fleet;
pub mod goals;
pub mod health;
pub mod jev;
pub mod metrics;
pub mod monitoring;
pub mod promotion;
pub mod recovery;
pub mod review;
pub mod route;
pub mod rules;
pub mod sandboxes;
pub mod schema;
pub mod security;
pub mod seed_reports;
pub mod tasks;
pub mod upgrade;
pub mod workers;

use crate::auth::{AuthError, validate_token};
use crate::conductor::Conductor;
use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::IntoResponse,
    routing::{delete, get, post},
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Mutex as TokioMutex;
use tracing::{debug, info};

pub mod optimization;

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
        .route("/fleet/status", get(fleet::get_fleet_status))
        .route("/metrics/cache", get(metrics::get_cache_stats))
        .route("/workers/register", post(workers::register_worker))
        .route("/workers/login", post(workers::worker_login_handler))
        .route("/workers/poll", post(workers::worker_poll_handler))
        .route("/workers/heartbeat", post(workers::worker_heartbeat_public))
        // Seed container environment reporting
        .route("/api/report", post(seed_reports::seed_report_handler))
        .route("/api/reports", get(seed_reports::list_seed_reports_handler))
        .route(
            "/evaluations/decision",
            post(evaluations::evaluate_decision_handler),
        )
        // Sandbox management endpoints (Phase 1)
        .route("/sandboxes", post(sandboxes::create_sandbox_handler))
        .route("/sandboxes", get(sandboxes::list_sandboxes_handler))
        .route("/sandboxes/{id}", get(sandboxes::get_sandbox_handler))
        .route(
            "/sandboxes/{id}",
            delete(sandboxes::destroy_sandbox_handler),
        )
        .route(
            "/sandboxes/{id}/worker",
            post(sandboxes::assign_worker_to_sandbox_handler),
        )
        .route("/decide/evaluate/{id}", get(get_evaluation_status))
        .route("/decisions/capabilities", get(list_decision_capabilities))
        .route("/audit/log", get(query_audit_log))
        .route("/jev/evaluate", post(jev::evaluate_structural_handler))
        .route("/jev/postprocess", post(jev::postprocess_decision_handler))
        .route("/jev/feedback", post(jev::record_feedback_handler))
        .route(
            "/schema/propose",
            post(schema::propose_schema_change_handler),
        )
        .route(
            "/interface/propose",
            post(optimization::propose_interface_optimization_handler),
        )
        .route("/route/decide", post(route::decide_routing_handler))
        .route(
            "/route/outcome",
            post(route::record_routing_outcome_handler),
        )
        .route(
            "/dependencies/declare",
            post(dependencies::declare_dependencies_handler),
        )
        .route("/upgrade/evaluate", post(upgrade::evaluate_upgrade_handler))
        .route(
            "/upgrade/outcome",
            post(upgrade::record_upgrade_outcome_handler),
        )
        .route("/health/evaluate", post(health::evaluate_health_handler))
        .route(
            "/health/outcome",
            post(health::record_recovery_outcome_handler),
        )
        .route("/cost/evaluate", post(cost::evaluate_cost_handler))
        .route(
            "/optimization/outcome",
            post(cost::record_optimization_outcome_handler),
        )
        // Milestone 20: Disaster recovery decision-making
        .route(
            "/recovery/evaluate",
            post(recovery::evaluate_recovery_handler),
        )
        .route(
            "/recovery/outcome",
            post(recovery::record_disaster_recovery_outcome_handler),
        )
        .route(
            "/capacity/evaluate",
            post(capacity::evaluate_capacity_handler),
        )
        .route(
            "/capacity/outcome",
            post(capacity::record_capacity_outcome_handler),
        )
        .route(
            "/security/evaluate",
            post(security::evaluate_security_handler),
        )
        .route(
            "/security/outcome",
            post(security::record_security_outcome_handler),
        )
        .route(
            "/compliance/evaluate",
            post(compliance::evaluate_compliance_handler),
        )
        .route(
            "/compliance/outcome",
            post(compliance::record_compliance_outcome_handler),
        )
        .route("/docs/evaluate", post(docs::evaluate_documentation_handler))
        .route(
            "/docs/outcome",
            post(docs::record_documentation_outcome_handler),
        )
        .route(
            "/monitoring/evaluate",
            post(monitoring::evaluate_monitoring_handler),
        )
        .route(
            "/monitoring/outcome",
            post(monitoring::record_monitoring_outcome_handler),
        )
        .route("/drift/evaluate", post(drift::evaluate_drift_handler))
        .route("/drift/outcome", post(drift::record_drift_outcome_handler))
        .route(
            "/cost/outcome",
            post(cost::record_optimization_outcome_handler),
        )
        .route("/rules/extract", post(rules::extract_rules_handler))
        .route("/rules/promote", post(rules::promote_rule_handler))
        // Evidence-based promotion endpoints
        .route(
            "/promotion/evaluate/{worker_id}",
            get(promotion::evaluate_promotion_endpoint),
        )
        .route(
            "/promotion/execute/{worker_id}",
            post(promotion::execute_promotion_endpoint),
        )
        .route(
            "/promotion/history/{worker_id}",
            get(promotion::get_promotion_history_endpoint),
        )
        .route(
            "/promotion/metrics/{worker_id}",
            get(promotion::get_worker_metrics_endpoint),
        )
        .with_state(Arc::clone(&conductor));

    // Protected routes (JWT auth required)
    let protected_app: Router = Router::new()
        .route("/goals", post(goals::submit_goal))
        .route("/goals/{id}", get(goals::get_goal))
        .route("/workers", get(workers::list_workers))
        .route(
            "/tasks/for-worker/{worker_id}",
            get(workers::tasks_for_worker),
        )
        .route("/tasks/{id}/result", post(tasks::report_task_result))
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
