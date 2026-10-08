//! VM lifecycle control plane — HTTP API for starting, stopping, and
//! managing VMs via vm-core's LibvirtBackend.

use anyhow::Result;
use serde_json::json;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use vm_core::{
    backend::LibvirtBackend,
    manager::{ManagerConfig, VmManager},
    types::{SnapshotRequest, VmId, VmSpec},
};

/// Shared state for the lifecycle API.
#[derive(Clone)]
pub struct LifecycleState {
    manager: Arc<VmManager>,
}

impl LifecycleState {
    pub fn new(manager: Arc<VmManager>) -> Self {
        Self { manager }
    }
}

/// Request body for creating a VM.
#[derive(Debug, Deserialize)]
pub struct CreateVmRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_template")]
    pub template: String,
    #[serde(default = "default_vcpus")]
    pub vcpus: u32,
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u64,
    #[serde(default)]
    pub disk_image: Option<String>,
}

fn default_template() -> String {
    "ubuntu-24.04".to_string()
}

fn default_vcpus() -> u32 {
    1
}

fn default_memory_mb() -> u64 {
    512
}

/// Request body for snapshot operations.
#[derive(Debug, Deserialize)]
pub struct SnapshotRequestBody {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Response envelope for lifecycle API endpoints.
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }

    fn err(msg: impl Into<String>) -> Self {
        Self { success: false, data: None, error: Some(msg.into()) }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = if self.success { StatusCode::OK } else { StatusCode::BAD_REQUEST };
        (status, Json(self)).into_response()
    }
}

/// Build the lifecycle API router.
pub fn routes(state: LifecycleState) -> Router {
    Router::new()
        .route("/vms", post(create_vm))
        .route("/vms/{id}", get(get_vm))
        .route("/vms/{id}/start", post(start_vm))
        .route("/vms/{id}/stop", post(stop_vm))
        .route("/vms/{id}/destroy", post(destroy_vm))
        .route("/vms/{id}/snapshots", post(create_snapshot))
        .with_state(state)
}

async fn create_vm(
    State(state): State<LifecycleState>,
    Json(req): Json<CreateVmRequest>,
) -> impl IntoResponse {
    let spec = VmSpec {
        name: req.name,
        id: None,
        vcpus: req.vcpus,
        memory_mb: req.memory_mb,
        disk_image: req.disk_image.unwrap_or_default(),
        boot_order: None,
    };

    match state.manager.create(spec).await {
        Ok(vm_id) => ApiResponse::ok(serde_json::json!({ "vm_id": vm_id.to_string() })),
        Err(e) => ApiResponse::err(format!("failed to create VM: {}", e)),
    }
}

async fn get_vm(
    State(state): State<LifecycleState>,
    Path(id_str): Path<String>,
) -> impl IntoResponse {
    let vm_id = match VmId::try_from(id_str.as_str()) {
        Ok(id) => id,
        Err(_) => return ApiResponse::err("invalid VM ID format"),
    };

    match state.manager.state(vm_id).await {
        Ok(state) => ApiResponse::ok(json!({ "vm_id": vm_id.to_string(), "state": state })),
        Err(e) => ApiResponse::err(format!("failed to get VM state: {}", e)),
    }
}

async fn start_vm(
    State(state): State<LifecycleState>,
    Path(id_str): Path<String>,
) -> impl IntoResponse {
    let vm_id = match VmId::try_from(id_str.as_str()) {
        Ok(id) => id,
        Err(_) => return ApiResponse::err("invalid VM ID format"),
    };

    match state.manager.start(vm_id).await {
        Ok(()) => ApiResponse::ok(json!({ "vm_id": vm_id.to_string(), "action": "started" })),
        Err(e) => ApiResponse::err(format!("failed to start VM: {}", e)),
    }
}

async fn stop_vm(
    State(state): State<LifecycleState>,
    Path(id_str): Path<String>,
) -> impl IntoResponse {
    let vm_id = match VmId::try_from(id_str.as_str()) {
        Ok(id) => id,
        Err(_) => return ApiResponse::err("invalid VM ID format"),
    };

    match state.manager.stop(vm_id).await {
        Ok(()) => ApiResponse::ok(json!({ "vm_id": vm_id.to_string(), "action": "stopped" })),
        Err(e) => ApiResponse::err(format!("failed to stop VM: {}", e)),
    }
}

async fn destroy_vm(
    State(state): State<LifecycleState>,
    Path(id_str): Path<String>,
) -> impl IntoResponse {
    let vm_id = match VmId::try_from(id_str.as_str()) {
        Ok(id) => id,
        Err(_) => return ApiResponse::err("invalid VM ID format"),
    };

    match state.manager.destroy(vm_id).await {
        Ok(()) => ApiResponse::ok(json!({ "vm_id": vm_id.to_string(), "action": "destroyed" })),
        Err(e) => ApiResponse::err(format!("failed to destroy VM: {}", e)),
    }
}

async fn create_snapshot(
    State(state): State<LifecycleState>,
    Path(id_str): Path<String>,
    Json(req): Json<SnapshotRequestBody>,
) -> impl IntoResponse {
    let vm_id = match VmId::try_from(id_str.as_str()) {
        Ok(id) => id,
        Err(_) => return ApiResponse::err("invalid VM ID format"),
    };

    match state.manager.snapshot(vm_id, req.name).await {
        Ok(info) => ApiResponse::ok(json!({ "snapshot": info })),
        Err(e) => ApiResponse::err(format!("failed to create snapshot: {}", e)),
    }
}