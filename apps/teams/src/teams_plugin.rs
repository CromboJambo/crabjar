/// Teams plugin — the first app on the CrabJar host runtime.
///
/// Wraps the Teams-for-Linux web app with Rust-native system integration.
/// The web UI stays as-is; Rust handles tray, notifications, auth, caching.
use async_trait::async_trait;
use crabjar_host_core::{Plugin, PluginContext, plugin::PluginError};
use uuid::Uuid;

/// The Teams plugin implementation.
pub struct TeamsPlugin {
    id: String,
    name: String,
    version: String,
    session_id: Option<Uuid>,
    auth_token: Option<String>,
}

impl TeamsPlugin {
    pub fn new() -> Self {
        Self {
            id: "teams".into(),
            name: "Microsoft Teams".into(),
            version: "0.1.0".into(),
            session_id: None,
            auth_token: None,
        }
    }

    /// Initialize the Teams session (open WebView, authenticate).
    async fn initialize_session(&mut self, _ctx: &PluginContext) -> Result<(), PluginError> {
        // Teams web URL
        let teams_url = "https://teams.microsoft.com";

        // In practice, this would use host-webview to open the Teams web app
        // with the user's session cookies / SSO token.
        tracing::info!(url = teams_url, "initializing teams session");

        // For now, just store the URL as a placeholder
        // The actual WebView integration will be in host-webview
        self.session_id = Some(Uuid::new_v4());

        Ok(())
    }

    /// Handle Teams-specific actions from the tray or GUI.
    async fn handle_teams_action(
        &self,
        action: &str,
        ctx: &PluginContext,
    ) -> Result<serde_json::Value, PluginError> {
        match action {
            "show" => {
                if let Some(_sid) = self.session_id {
                    let _ = ctx.emit(crabjar_host_core::event_bus::EventType::WebView {
                        event: "show".into(),
                        url: Some("https://teams.microsoft.com".into()),
                    });
                }
                Ok(serde_json::json!({ "status": "shown" }))
            }
            "hide" => {
                if let Some(_sid) = self.session_id {
                    let _ = ctx.emit(crabjar_host_core::event_bus::EventType::WebView {
                        event: "hide".into(),
                        url: Some("https://teams.microsoft.com".into()),
                    });
                }
                Ok(serde_json::json!({ "status": "hidden" }))
            }
            "notifications" => {
                // Query Teams API for unread notifications
                // In practice: HTTP call to Teams REST API
                Ok(serde_json::json!({
                    "unread_count": 0,
                    "last_check": chrono::Utc::now().to_rfc3339(),
                }))
            }
            "sign-in" => {
                // Trigger SSO flow
                Ok(serde_json::json!({
                    "status": "sso_initiated",
                    "redirect_uri": "http://localhost:18765/callback",
                }))
            }
            _ => Err(PluginError::Execution(format!("unknown action: {action}"))),
        }
    }
}

#[async_trait]
impl Plugin for TeamsPlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn on_start(&self, _ctx: &PluginContext) -> Result<(), PluginError> {
        tracing::info!("teams plugin starting");
        Ok(())
    }

    async fn on_stop(&self, _ctx: &PluginContext) -> Result<(), PluginError> {
        tracing::info!("teams plugin stopping");
        Ok(())
    }

    async fn on_show(&self, ctx: &PluginContext) -> Result<(), PluginError> {
        self.handle_teams_action("show", ctx).await?;
        Ok(())
    }

    async fn on_hide(&self, ctx: &PluginContext) -> Result<(), PluginError> {
        self.handle_teams_action("hide", ctx).await?;
        Ok(())
    }

    async fn on_action(
        &self,
        ctx: &PluginContext,
        action: &str,
        _data: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, PluginError> {
        self.handle_teams_action(action, ctx).await
    }

    async fn health(&self, _ctx: &PluginContext) -> Result<serde_json::Value, PluginError> {
        Ok(serde_json::json!({
            "status": "healthy",
            "plugin": self.id(),
            "session_active": self.session_id.is_some(),
            "auth_token_set": self.auth_token.is_some(),
        }))
    }
}

impl Default for TeamsPlugin {
    fn default() -> Self {
        Self::new()
    }
}
