//! Screen capture and sharing infrastructure.
//!
//! Provides VNC frame capture, thumbnail generation, and HTTP endpoints
//! for serving VM display previews. This is the foundation for Phase 4
//! screen sharing (PipeWire source, XDG-Portal, audio capture build on top).

pub mod thumbnail;
pub mod vnc_client;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::manifest::Vm;
use vnc_client::VncCapture;

/// Shared state for the screen capture service.
#[derive(Clone)]
pub struct ScreenState {
    pub vm: Vm,
    /// Lazily-connected VNC capture session. Reconnected on failure.
    pub capture: Arc<RwLock<Option<VncCapture>>>,
}

impl ScreenState {
    pub fn new(vm: Vm) -> Self {
        Self {
            vm,
            capture: Arc::new(RwLock::new(None)),
        }
    }

    /// Get or create a VNC capture connection.
    async fn get_capture(&self) -> Result<Arc<RwLock<Option<VncCapture>>>> {
        let mut guard = self.capture.write().await;
        if guard.as_ref().is_none() {
            tracing::info!(vm = %self.vm.name, target = %self.vm.target, "connecting VNC capture");
            let cap = VncCapture::connect(&self.vm.target)
                .await
                .with_context(|| format!("VNC connect for {}", self.vm.name))?;
            *guard = Some(cap);
        }
        Ok(Arc::clone(&self.capture))
    }

    /// Capture a frame and generate a JPEG thumbnail.
    pub async fn capture_thumbnail(&self, max_dim: u32, quality: u8) -> Result<Vec<u8>> {
        let capture = self.get_capture().await?;
        let mut guard = capture.write().await;
        let cap = guard.as_mut().context("no VNC connection")?;

        match cap.capture_frame().await {
            Ok(frame) => {
                thumbnail::generate_thumbnail(&frame, max_dim, quality)
            }
            Err(e) => {
                // Connection likely died — drop it so next call reconnects.
                tracing::warn!(vm = %self.vm.name, error = %e, "VNC capture failed, will reconnect");
                *guard = None;
                Err(e)
            }
        }
    }
}

/// Auth check for screen endpoints (same pattern as proxy).
fn check_auth(
    vm: &Vm,
    headers: &HeaderMap,
    query: &std::collections::HashMap<String, String>,
) -> Result<(), (StatusCode, String)> {
    if let Some(expected) = &vm.token {
        let provided = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|auth| auth.strip_prefix("Bearer "))
            .map(str::to_string)
            .or_else(|| query.get("token").cloned());

        match provided {
            Some(t) if t == *expected => Ok(()),
            _ => Err((StatusCode::UNAUTHORIZED, "invalid or missing token".into())),
        }
    } else {
        Ok(())
    }
}

/// Handler: GET /screen/thumbnail?max_dim=320&quality=75
async fn thumbnail_handler(
    State(state): State<ScreenState>,
    headers: HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    if let Err(e) = check_auth(&state.vm, &headers, &query.0) {
        return e.into_response();
    }

    let max_dim = query
        .get("max_dim")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(320)
        .clamp(64, 1920);
    let quality = query
        .get("quality")
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(75)
        .clamp(1, 100);

    match state.capture_thumbnail(max_dim, quality).await {
        Ok(jpeg) => (
            StatusCode::OK,
            [("Content-Type", "image/jpeg")],
            jpeg,
        )
            .into_response(),
        Err(e) => {
            tracing::error!(vm = %state.vm.name, error = %e, "thumbnail capture failed");
            (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response()
        }
    }
}

/// Build the screen sharing router for a VM worker.
pub fn routes(state: ScreenState) -> Router {
    Router::new()
        .route("/screen/thumbnail", get(thumbnail_handler))
        .with_state(state)
}
