/// OAuth2 SSO authentication flow for Microsoft identity platform.
///
/// Handles the OAuth2 authorization code flow with PKCE for Microsoft
/// identity broker integration. Mirrors the Electron app's auth cookie
/// management and SSO redirect handling.
///
/// Key flows:
/// - Authorization code flow with PKCE (modern standard)
/// - Silent token refresh using refresh tokens
/// - Token caching in keyring (via SecureTokenCache)
/// - D-Bus identity broker (stubbed for Linux)
use crate::cookie_store::{CookieStore, SessionToken};
use crate::token_cache::SecureTokenCache;
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Microsoft identity platform endpoints.
pub const MICROSOFT_IDENTITY_BASE: &str = "https://login.microsoftonline.com";
pub const MICROSOFT_TOKEN_ENDPOINT: &str =
    "https://login.microsoftonline.com/common/oauth2/v2.0/token";
pub const MICROSOFT_AUTH_ENDPOINT: &str =
    "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
pub const MICROSOFT_WELL_KNOWN: &str =
    "https://login.microsoftonline.com/common/v2.0/.well-known/openid-configuration";

/// OAuth2 token response (mirrors Microsoft's token endpoint response).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Authorization URL parameters for the SSO redirect.
#[derive(Debug, Clone, Serialize)]
pub struct AuthParams {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub response_type: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub state: String,
    pub prompt: String,
    pub domain_hint: Option<String>,
}

/// Current authentication state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    pub is_authenticated: bool,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub token_expires_at: Option<i64>,
    pub last_refresh_at: Option<i64>,
    pub partition_name: Option<String>,
}

/// OAuth2 SSO auth manager.
pub struct AuthManager {
    cookie_store: Arc<CookieStore>,
    token_cache: Arc<SecureTokenCache>,
    state: RwLock<AuthState>,
    client_id: String,
    redirect_uri: String,
    scopes: String,
}

impl AuthManager {
    pub fn new(
        cookie_store: Arc<CookieStore>,
        token_cache: Arc<SecureTokenCache>,
        client_id: String,
        redirect_uri: String,
        scopes: String,
    ) -> Self {
        Self {
            cookie_store,
            token_cache,
            state: RwLock::new(AuthState {
                is_authenticated: false,
                client_id: client_id.clone(),
                redirect_uri: redirect_uri.clone(),
                scopes: Vec::new(),
                access_token: None,
                refresh_token: None,
                id_token: None,
                token_expires_at: None,
                last_refresh_at: None,
                partition_name: None,
            }),
            client_id,
            redirect_uri,
            scopes,
        }
    }

    /// Generate the authorization URL for SSO redirect.
    pub fn build_auth_url(&self, extra_params: Option<&[(&str, &str)]>) -> String {
        let code_challenge = generate_pkce_code_challenge();
        let code_challenge_method = "S256".to_string();
        let state = Uuid::new_v4().to_string();

        let params = AuthParams {
            client_id: self.client_id.clone(),
            redirect_uri: self.redirect_uri.clone(),
            scope: self.scopes.clone(),
            response_type: "code".into(),
            code_challenge,
            code_challenge_method,
            state: state.clone(),
            prompt: "none".into(),
            domain_hint: None,
        };

        let mut url = format!(
            "{}?client_id={}&redirect_uri={}&response_type={}&scope={}&code_challenge={}&code_challenge_method={}&state={}",
            MICROSOFT_AUTH_ENDPOINT,
            urlencoding(&params.client_id),
            urlencoding(&params.redirect_uri),
            urlencoding(&params.response_type),
            urlencoding(&params.scope),
            urlencoding(&params.code_challenge),
            urlencoding(&params.code_challenge_method),
            urlencoding(&params.state),
        );

        if let Some(extra) = extra_params {
            for (k, v) in extra {
                url.push_str(&format!("&{}={}", k, urlencoding(v)));
            }
        }

        url
    }

    /// Handle the OAuth2 callback — exchange authorization code for tokens.
    pub async fn handle_callback(
        &self,
        _code: &str,
        _state: &str,
    ) -> Result<TokenResponse, AuthError> {
        // In a real implementation, this would:
        // 1. Validate the state parameter (CSRF protection)
        // 2. Exchange the code for tokens via POST to MICROSOFT_TOKEN_ENDPOINT
        // 3. Store tokens in the secure token cache
        // 4. Update auth state

        // Stub: In production, this uses reqwest to POST to the token endpoint:
        // POST https://login.microsoftonline.com/common/oauth2/v2.0/token
        // Content-Type: application/x-www-form-urlencoded
        // body: grant_type=authorization_code&code={code}&client_id={id}&redirect_uri={uri}&code_verifier={verifier}&scope={scopes}

        tracing::warn!("Auth callback handler is a stub — token exchange not yet implemented");

        // For now, return a placeholder to keep the API contract
        Err(AuthError::NotImplemented("token exchange via PKCE".into()))
    }

    /// Refresh an expired access token using the stored refresh token.
    pub async fn refresh_token(&self) -> Result<TokenResponse, AuthError> {
        let state = self.state.read().await;

        let _refresh_token = state
            .refresh_token
            .as_ref()
            .ok_or(AuthError::NoRefreshToken)?;

        // In production:
        // POST https://login.microsoftonline.com/common/oauth2/v2.0/token
        // body: grant_type=refresh_token&refresh_token={rt}&client_id={id}&scope={scopes}

        tracing::warn!("Token refresh is a stub — actual HTTP exchange not yet implemented");
        Err(AuthError::NotImplemented("refresh token exchange".into()))
    }

    /// Force a fresh token request (mirrors Teams tokenCache.js forceRenew/forceRefresh/skipCache).
    /// Used for proactive token refresh at configurable intervals.
    pub async fn force_refresh(&self) -> Result<TokenResponse, AuthError> {
        let state = self.state.read().await;

        let _refresh_token = state
            .refresh_token
            .as_ref()
            .ok_or(AuthError::NoRefreshToken)?;

        // Same as refresh_token but with explicit force flags
        // In production, include forceRenew=true, forceRefresh=true, skipCache=true
        // in the token request to bypass any cached tokens.

        tracing::warn!("Force refresh is a stub — actual HTTP exchange not yet implemented");
        Err(AuthError::NotImplemented("force token refresh".into()))
    }

    /// Check if the current access token is expired or near-expiry.
    pub async fn is_token_expired(&self, buffer_seconds: i64) -> bool {
        let state = self.state.read().await;
        match state.token_expires_at {
            Some(expires_at) => Utc::now().timestamp() + buffer_seconds >= expires_at,
            None => true,
        }
    }

    /// Get the current auth state.
    pub async fn get_state(&self) -> AuthState {
        self.state.read().await.clone()
    }

    /// Check if the user is authenticated.
    pub async fn is_authenticated(&self) -> bool {
        self.state.read().await.is_authenticated
    }

    /// Save token response to persistent storage and update auth state.
    async fn save_tokens(&self, token_response: &TokenResponse) -> Result<(), AuthError> {
        let now = Utc::now().timestamp();
        let expires_at = now + token_response.expires_in as i64;

        // Save access token
        let access_key = format!("access_token.{}", Uuid::new_v4().simple());
        let access_token = SessionToken {
            key: access_key,
            value: token_response.access_token.clone(),
            encrypted: true,
            created_at: now,
            expires_at: Some(expires_at),
        };
        self.cookie_store
            .save_token(&access_token)
            .await
            .map_err(|e| AuthError::Storage(format!("save access token: {e}")))?;

        // Save refresh token
        let refresh_key = "refresh_token.primary".to_string();
        let refresh_token = SessionToken {
            key: refresh_key,
            value: token_response.refresh_token.clone(),
            encrypted: true,
            created_at: now,
            expires_at: None,
        };
        self.cookie_store
            .save_token(&refresh_token)
            .await
            .map_err(|e| AuthError::Storage(format!("save refresh token: {e}")))?;

        // Update in-memory auth state
        let mut state = self.state.write().await;
        state.access_token = Some(token_response.access_token.clone());
        state.refresh_token = Some(token_response.refresh_token.clone());
        state.id_token = token_response.id_token.clone();
        state.token_expires_at = Some(expires_at);
        state.last_refresh_at = Some(now);
        state.is_authenticated = true;

        tracing::info!("tokens saved and auth state updated");
        Ok(())
    }

    /// Clear all authentication state and tokens.
    pub async fn logout(&self) -> Result<(), AuthError> {
        let mut state = self.state.write().await;
        state.is_authenticated = false;
        state.access_token = None;
        state.refresh_token = None;
        state.id_token = None;
        state.token_expires_at = None;
        state.last_refresh_at = None;

        // Clear auth-related tokens from storage
        let _ = self.token_cache.clear().await;

        tracing::info!("auth state cleared");
        Ok(())
    }
}

/// Generate a PKCE code challenge (S256 method).
fn generate_pkce_code_challenge() -> String {
    // In production, use a proper crypto library to:
    // 1. Generate a random code verifier (43-128 chars)
    // 2. SHA256 hash it
    // 3. Base64URL encode
    // For now, return a placeholder
    let verifier = Uuid::new_v4().to_string().replace('-', "");
    // Simplified: use the verifier directly (production would SHA256 + base64url)
    let hash = sha2::Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

/// URL-encode a string for query parameters.
fn urlencoding(s: &str) -> String {
    // Simple URL encoding — production would use a proper library
    s.replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('+', "%2B")
        .replace('%', "%25")
}

/// Authentication errors.
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("no refresh token available")]
    NoRefreshToken,
    #[error("token exchange failed: {0}")]
    TokenExchange(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("invalid state parameter (possible CSRF)")]
    InvalidState,
    #[error("token expired")]
    TokenExpired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_url_generation() {
        let store = tempfile::tempdir().unwrap();
        let db_path = store.path().join("auth.db");
        std::fs::File::create(&db_path).unwrap();
        let cookie_store = Arc::new(crate::cookie_store::CookieStore::open(db_path).unwrap());
        let token_cache = Arc::new(SecureTokenCache::new(cookie_store));
        let manager = AuthManager::new(
            Arc::new(
                crate::cookie_store::CookieStore::open(
                    tempfile::tempdir().unwrap().path().join("auth2.db"),
                )
                .unwrap(),
            ),
            token_cache,
            "client-id-123".into(),
            "http://localhost:3000/callback".into(),
            "User.Read Mail.Read".into(),
        );

        let url = manager.build_auth_url(None);
        assert!(url.starts_with(MICROSOFT_AUTH_ENDPOINT));
        assert!(url.contains("client_id=client-id-123"));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state="));
    }

    #[test]
    fn test_pkce_code_challenge_generation() {
        let challenge = generate_pkce_code_challenge();
        // Base64URL encoded SHA256 of a UUID string = 43 chars
        assert!(!challenge.is_empty());
        assert!(challenge.len() > 0);
        // Should only contain URL-safe base64 characters
        for c in challenge.chars() {
            assert!(c.is_alphanumeric() || c == '-' || c == '_');
        }
    }
}
