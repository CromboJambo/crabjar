//! Metrics routes: /metrics/cache

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde_json::json;

/// GET /metrics/cache - Get JEV decision cache statistics (public metrics endpoint).
pub async fn get_cache_stats(
    State(conductor): State<ConductorState>,
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
