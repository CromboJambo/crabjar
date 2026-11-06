//! Seed container environment reporting.
//! Receives hardware/environment reports from deployed seed containers.

use axum::extract::Json;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, OnceLock};

/// Report data from a seed container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedReport {
    pub hostname: String,
    pub timestamp: i64,
    pub cpu_count: usize,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub has_gpu: bool,
    pub gpu_model: Option<String>,
    pub growth_capabilities: Vec<String>,
}

/// In-memory store for seed container reports.
static SEED_REPORTS: OnceLock<Arc<Mutex<Vec<SeedReport>>>> = OnceLock::new();

/// POST /api/report - Receive environment report from seed container.
pub async fn seed_report_handler(
    Json(report): Json<SeedReport>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Store report in global state (production would use persistent storage)
    let store = SEED_REPORTS.get_or_init(|| Arc::new(Mutex::new(vec![])));
    let mut reports = store.lock().unwrap();
    reports.push(report.clone());

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": "Environment report received",
            "hostname": report.hostname
        })),
    )
}

/// GET /api/reports - List all environment reports from seed containers.
pub async fn list_seed_reports_handler() -> (StatusCode, Json<serde_json::Value>) {
    let store = SEED_REPORTS.get_or_init(|| Arc::new(Mutex::new(vec![])));
    let guard = store.lock().unwrap();
    let reports = guard.clone();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "reports": reports,
            "count": reports.len()
        })),
    )
}
