/// WebView session controller — manages embedded webviews with full session management.
///
/// Integrates cookie store, token cache, partition manager, and auth flow
/// into a unified session management API that replaces Electron's
/// `session` API (cookies, partitions, auth).
use crate::auth::{AuthManager, TokenResponse};
use crate::cookie_store::{CookieStore, Cookie};
use crate::partition::{PartitionManager, SessionPartition};
use crate::token_cache::SecureTokenCache;
use crabjar_host_core::event_bus::EventBus;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

/// A managed webview session with full session management.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub url: String,
    pub title: String,
    pub visible: bool,
    pub partition_name: Option<String>,
    pub created_at: i64,
}

/// WebView engine types.
#[derive(Debug, Clone, PartialEq)]
pub enum WebViewEngine {
    Webkit,
    WebView2,
}

impl std::fmt::Display for WebViewEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebViewEngine::Webkit => write!(f, "webkit"),
            WebViewEngine::WebView2 => write!(f, "webview2"),
        }
    }
}

/// Main session controller — replaces Electron's session API.
pub struct WebViewController {
    event_bus: Arc<EventBus>,
    cookie_store: Arc<CookieStore>,
    token_cache: Arc<SecureTokenCache>,
    partition_manager: Arc<PartitionManager>,
    auth_manager: Arc<AuthManager>,
    engine: WebViewEngine,
    sessions: RwLock<Vec<Session>>,
}

impl WebViewController {
    pub fn new(
        event_bus: Arc<EventBus>,
        engine: WebViewEngine,
        data_dir: PathBuf,
        client_id: String,
        redirect_uri: String,
        scopes: String,
    ) -> Self {
        let db_path = data_dir.join("webview");
        std::fs::create_dir_all(&db_path).ok();

        let cookie_db_path = db_path.join("cookies.db");
        let cookie_store = Arc::new(
            CookieStore::open(cookie_db_path).expect("Failed to open cookie store")
        );

        let token_cache = Arc::new(SecureTokenCache::new(cookie_store.clone()));
        let partition_manager = Arc::new(PartitionManager::new(cookie_store.clone()));

        let auth_manager = Arc::new(AuthManager::new(
            cookie_store.clone(),
            token_cache.clone(),
            client_id,
            redirect_uri,
            scopes,
        ));

        Self {
            event_bus,
            cookie_store,
            token_cache,
            partition_manager,
            auth_manager,
            engine,
            sessions: RwLock::new(Vec::new()),
        }
    }

    /// Initialize all subsystems (load persisted partitions, etc.).
    pub async fn initialize(&self) {
        self.partition_manager.initialize().await;
        let _ = self.cookie_store.cleanup_expired().await;
        tracing::info!(engine = %self.engine, "webview controller initialized");
    }

    // --- Session Management ---

    pub async fn open(&self, url: impl Into<String>, title: impl Into<String>) -> Result<Uuid, WebViewError> {
        let session_id = Uuid::new_v4();
        let url = url.into();
        let title = title.into();

        let session = Session {
            id: session_id,
            url: url.clone(),
            title: title.clone(),
            visible: true,
            partition_name: None,
            created_at: Utc::now().timestamp(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.push(session);

        tracing::info!(session_id = %session_id, url, title, "webview opened");

        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::WebView {
                event: "opened".into(),
                url: Some(url),
            },
            "webview-controller",
        );

        Ok(session_id)
    }

    pub async fn close(&self, session_id: Uuid) -> Result<(), WebViewError> {
        let mut sessions = self.sessions.write().await;
        let idx = sessions.iter().position(|s| s.id == session_id)
            .ok_or(WebViewError::SessionNotFound(session_id))?;

        let session = sessions.remove(idx);
        tracing::info!(session_id = %session_id, "webview closed");

        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::WebView {
                event: "closed".into(),
                url: Some(session.url),
            },
            "webview-controller",
        );

        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<Session> {
        self.sessions.read().await.clone()
    }

    // --- Cookie Management (replaces Electron session.cookies) ---

    pub async fn get_cookies(&self, url: &str) -> Result<Vec<Cookie>, WebViewError> {
        let domain = extract_domain(url)
            .ok_or_else(|| WebViewError::InvalidUrl(url.to_string()))?;
        let cookies = self.cookie_store.get_cookies_by_domain(&domain).await
            .map_err(|e| WebViewError::Storage(format!("get cookies: {e}")))?;
        Ok(cookies)
    }

    pub async fn set_cookie(&self, url: &str, name: &str, value: &str) -> Result<Uuid, WebViewError> {
        let domain = extract_domain(url)
            .ok_or_else(|| WebViewError::InvalidUrl(url.to_string()))?;

        let cookie = Cookie {
            name: name.to_string(),
            value: value.to_string(),
            domain: domain.clone(),
            path: "/".into(),
            expires: None,
            secure: url.starts_with("https"),
            http_only: false,
            same_site: "Lax".into(),
        };

        let id = self.cookie_store.save_cookie(&cookie).await
            .map_err(|e| WebViewError::Storage(format!("save cookie: {e}")))?;

        tracing::debug!(cookie_id = %id, domain, name, "cookie set");
        Ok(id)
    }

    pub async fn remove_cookie(&self, url: &str, name: &str) -> Result<bool, WebViewError> {
        let domain = extract_domain(url)
            .ok_or_else(|| WebViewError::InvalidUrl(url.to_string()))?;
        let removed = self.cookie_store.remove_cookie(&domain, name).await
            .map_err(|e| WebViewError::Storage(format!("remove cookie: {e}")))?;
        Ok(removed)
    }

    pub async fn clear_cookies(&self) -> Result<usize, WebViewError> {
        let cleared = self.cookie_store.clear_all().await
            .map_err(|e| WebViewError::Storage(format!("clear cookies: {e}")))?;
        Ok(cleared)
    }

    pub async fn list_all_cookies(&self) -> Result<Vec<Cookie>, WebViewError> {
        let cookies = self.cookie_store.list_cookies().await
            .map_err(|e| WebViewError::Storage(format!("list cookies: {e}")))?;
        Ok(cookies)
    }

    // --- Token Cache ---

    pub async fn get_token(&self, key: &str) -> Option<String> {
        self.token_cache.get_item(key).await
    }

    pub async fn set_token(&self, key: &str, value: &str) -> Result<(), WebViewError> {
        self.token_cache.set_item(key, value)
            .await
            .map_err(|e| WebViewError::Storage(format!("set token: {e}")))?;
        Ok(())
    }

    pub async fn remove_token(&self, key: &str) {
        self.token_cache.remove_item(key).await;
    }

    pub async fn get_token_stats(&self) -> crate::token_cache::TokenCacheStats {
        self.token_cache.get_stats().await
    }

    // --- Partition Management ---

    pub async fn get_partition(&self, name: &str) -> Option<SessionPartition> {
        self.partition_manager.get(name).await
    }

    pub async fn get_or_create_partition(&self, name: &str) -> SessionPartition {
        self.partition_manager.get_or_create(name).await
    }

    pub async fn list_partitions(&self) -> Vec<SessionPartition> {
        self.partition_manager.list().await
    }

    pub async fn remove_partition(&self, name: &str) -> bool {
        self.partition_manager.remove(name).await
    }

    pub async fn save_zoom_level(&self, partition_name: &str, zoom_level: f64) -> bool {
        self.partition_manager.save_zoom_level(partition_name, zoom_level).await
    }

    pub async fn get_zoom_level(&self, partition_name: &str) -> f64 {
        self.partition_manager.get_zoom_level(partition_name).await
    }

    // --- Authentication ---

    pub async fn get_auth_url(&self, extra_params: Option<&[(&str, &str)]>) -> String {
        self.auth_manager.build_auth_url(extra_params)
    }

    pub async fn handle_auth_callback(&self, code: &str, state: &str) -> Result<TokenResponse, WebViewError> {
        self.auth_manager.handle_callback(code, state)
            .await
            .map_err(WebViewError::Auth)
    }

    pub async fn refresh_auth_token(&self) -> Result<TokenResponse, WebViewError> {
        self.auth_manager.refresh_token()
            .await
            .map_err(WebViewError::Auth)
    }

    pub async fn force_refresh_auth_token(&self) -> Result<TokenResponse, WebViewError> {
        self.auth_manager.force_refresh()
            .await
            .map_err(WebViewError::Auth)
    }

    pub async fn is_token_expired(&self, buffer_seconds: i64) -> bool {
        self.auth_manager.is_token_expired(buffer_seconds).await
    }

    pub async fn get_auth_state(&self) -> crate::auth::AuthState {
        self.auth_manager.get_state().await
    }

    pub async fn is_authenticated(&self) -> bool {
        self.auth_manager.is_authenticated().await
    }

    pub async fn logout(&self) -> Result<(), WebViewError> {
        self.auth_manager.logout()
            .await
            .map_err(WebViewError::Auth)
    }

    /// Database path for external access (e.g., debugging).
    pub fn db_path(&self) -> PathBuf {
        self.cookie_store.db_path().clone()
    }

    /// Get the webview engine name.
    pub fn engine(&self) -> &str {
        match &self.engine {
            WebViewEngine::Webkit => "webkit",
            WebViewEngine::WebView2 => "webview2",
        }
    }
}

/// Extract domain from a URL string.
fn extract_domain(url: &str) -> Option<String> {
    let url = url.trim();
    let url = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let domain = url.split('/').next()?;
    Some(domain.to_string())
}

/// WebView errors.
#[derive(thiserror::Error, Debug)]
pub enum WebViewError {
    #[error("session not found: {0}")]
    SessionNotFound(Uuid),
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("auth error: {0}")]
    Auth(crate::auth::AuthError),
    #[error("webview initialization failed: {0}")]
    InitFailed(String),
    #[error("webview render failed: {0}")]
    RenderFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_data_dir() -> PathBuf {
        let dir = tempfile::tempdir().unwrap();
        dir.path().join("webview-test")
    }

    #[tokio::test]
    async fn test_controller_initialization() {
        let event_bus = Arc::new(crabjar_host_core::EventBus::new(16));
        let data_dir = temp_data_dir();
        let controller = WebViewController::new(
            event_bus,
            WebViewEngine::Webkit,
            data_dir,
            "test-client".into(),
            "http://localhost/callback".into(),
            "User.Read".into(),
        );
        controller.initialize().await;
        assert_eq!(controller.engine(), "webkit");
    }

    #[tokio::test]
    async fn test_open_and_list_sessions() {
        let event_bus = Arc::new(crabjar_host_core::EventBus::new(16));
        let data_dir = temp_data_dir();
        let controller = WebViewController::new(
            event_bus,
            WebViewEngine::Webkit,
            data_dir.clone(),
            "test-client".into(),
            "http://localhost/callback".into(),
            "User.Read".into(),
        );

        let id = controller.open("https://teams.microsoft.com", "Teams").await.unwrap();
        let sessions = controller.list_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, id);
        assert_eq!(sessions[0].url, "https://teams.microsoft.com");

        controller.close(id).await.unwrap();
        let sessions = controller.list_sessions().await;
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_cookie_crud() {
        let event_bus = Arc::new(crabjar_host_core::EventBus::new(16));
        let data_dir = temp_data_dir();
        let controller = WebViewController::new(
            event_bus,
            WebViewEngine::Webkit,
            data_dir.clone(),
            "test-client".into(),
            "http://localhost/callback".into(),
            "User.Read".into(),
        );

        let cookie_id = controller.set_cookie(
            "https://teams.microsoft.com",
            "session",
            "abc123",
        ).await.unwrap();
        assert!(!cookie_id.is_nil());

        let cookies = controller.get_cookies("https://teams.microsoft.com").await.unwrap();
        assert!(!cookies.is_empty());
        assert_eq!(cookies[0].name, "session");
        assert_eq!(cookies[0].value, "abc123");
    }

    #[tokio::test]
    async fn test_partition_crud() {
        let event_bus = Arc::new(crabjar_host_core::EventBus::new(16));
        let data_dir = temp_data_dir();
        let controller = WebViewController::new(
            event_bus,
            WebViewEngine::Webkit,
            data_dir.clone(),
            "test-client".into(),
            "http://localhost/callback".into(),
            "User.Read".into(),
        );

        let part = controller.get_or_create_partition("test-profile").await;
        assert_eq!(part.name, "test-profile");
        assert_eq!(part.cookie_jar_id, "persist:test-profile");

        controller.save_zoom_level("test-profile", 2.0).await;
        let zoom = controller.get_zoom_level("test-profile").await;
        assert!((zoom - 2.0).abs() < f64::EPSILON);

        let removed = controller.remove_partition("test-profile").await;
        assert!(removed);
        let result = controller.get_partition("test-profile").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_extract_domain() {
        assert_eq!(extract_domain("https://teams.microsoft.com/path"), Some("teams.microsoft.com".into()));
        assert_eq!(extract_domain("http://example.com"), Some("example.com".into()));
        assert_eq!(extract_domain("https://a.b.c/d/e"), Some("a.b.c".into()));
        assert!(extract_domain("not-a-url").is_none());
    }
}
