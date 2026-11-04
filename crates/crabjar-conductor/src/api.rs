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
use axum::{
    Json, Router,
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::IntoResponse,
    routing::{delete, get, post},
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex as TokioMutex;
use tracing::{debug, info};

pub mod workers;
pub mod promotion;
pub mod route;

// Split out modules for 500 LoC governance
pub mod optimization;

pub type ConductorState = Arc<TokioMutex<Conductor>>;

/// In-memory store for background evaluation results (dev/testing).
static BG_EVALUATIONS: OnceLock<Arc<TokioMutex<HashMap<String, serde_json::Value>>>> =
    OnceLock::new();

fn bg_evaluations() -> &'static Arc<TokioMutex<HashMap<String, serde_json::Value>>> {
    BG_EVALUATIONS.get_or_init(|| Arc::new(TokioMutex::new(HashMap::new())))
}

/// Re-export handlers from split modules for use in routes.
use evaluations::{
    evaluate_decision_handler, get_evaluation_status, list_decision_capabilities,
    query_audit_log,
};
use goals::report_task_result;
use health::{evaluate_health_handler, record_recovery_outcome_handler};
use monitoring::{evaluate_monitoring_handler, record_monitoring_outcome_handler};
use optimization::propose_interface_optimization_handler;
use recovery::{evaluate_recovery_handler, record_disaster_recovery_outcome_handler};

/// Build the complete API router with all routes.
pub fn build_routes(conductor: ConductorState) -> Router {
    // Public routes (no auth required)
    let public_app: Router = Router::new()
        .route("/fleet/status", get(fleet::get_fleet_status))
        .route("/metrics/cache", get(metrics::get_cache_stats))
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
        .route(
            "/sandboxes/:id/worker",
            post(sandboxes::assign_worker_to_sandbox_handler),
        )
        .route("/decide/evaluate/{id}", get(get_evaluation_status))
        .route("/decisions/capabilities", get(list_decision_capabilities))
        .route("/audit/log", get(query_audit_log))
        .route("/jev/evaluate", post(jev::evaluate_structural_handler))
        .route("/jev/postprocess", post(jev::postprocess_decision_handler))
        .route("/jev/feedback", post(jev::record_feedback_handler))
        .route("/schema/propose", post(schema::propose_schema_change_handler))
        .route(
            "/interface/propose",
            post(propose_interface_optimization_handler),
        )
        .route("/route/decide", post(route::decide_routing_handler))
        .route("/route/outcome", post(route::record_routing_outcome_handler))
        .route(
            "/dependencies/declare",
            post(dependencies::declare_dependencies_handler),
        )
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
