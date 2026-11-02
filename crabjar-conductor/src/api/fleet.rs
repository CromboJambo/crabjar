//! Fleet status routes: /fleet/status

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde_json::json;

/// GET /fleet/status - Get current fleet status.
pub async fn get_fleet_status(
    State(conductor): State<ConductorState>,
) -> Json<serde_json::Value> {
    let conductor = conductor.lock().await;
    let status = conductor.fleet_status().get_status().await;

    Json(json!({
        "success": true,
        "fleet_status": status
    }))
}
