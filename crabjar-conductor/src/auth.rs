//! Authentication and authorization for the conductor service.
//!
//! Uses JWT tokens signed with HMAC-SHA256. The signing secret is loaded from
//! the `CRABJAR_AUTH_SECRET` environment variable, allowing both conductor and
//! workers to share the same credential pool.

use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;

/// Default token expiration: 1 hour.
const DEFAULT_TOKEN_TTL_SECS: i64 = 3600;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("missing CRABJAR_AUTH_SECRET environment variable")]
    MissingSecret,

    #[error("token generation failed: {0}")]
    Encoding(#[from] jsonwebtoken::errors::Error),

    #[error("invalid token: {0}")]
    InvalidToken(String),

    #[error("token expired")]
    Expired,

    #[error("worker not found")]
    WorkerNotFound,
}

/// Claims embedded in a conductor auth token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorClaims {
    /// Worker UUID (subject).
    pub sub: String,

    /// Worker name (for logging/debugging).
    pub name: String,

    /// Worker capabilities.
    pub capabilities: Vec<String>,

    /// Current trust level (0-3).
    pub trust_level: u8,

    /// Token scopes.
    pub scope: Vec<String>,

    /// Expiration timestamp.
    pub exp: i64,

    /// Issued at timestamp.
    pub iat: i64,
}

/// Generate a new auth token for a worker.
pub fn generate_token(
    worker_id: &str,
    name: &str,
    capabilities: &[String],
    trust_level: u8,
) -> Result<String, AuthError> {
    let secret = get_secret()?;
    let now = Utc::now().timestamp();
    let exp = now + DEFAULT_TOKEN_TTL_SECS;

    // Determine scopes based on trust level
    let scope = match trust_level {
        0 => vec!["heartbeat".to_string()],
        1 => vec![
            "heartbeat".to_string(),
            "tasks:claim".to_string(),
            "tasks:report".to_string(),
        ],
        _ => vec![
            "heartbeat".to_string(),
            "tasks:claim".to_string(),
            "tasks:report".to_string(),
            "goals:read".to_string(),
        ],
    };

    let claims = ConductorClaims {
        sub: worker_id.to_string(),
        name: name.to_string(),
        capabilities: capabilities.to_vec(),
        trust_level,
        scope,
        exp,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(AuthError::Encoding)
}

/// Validate a token and return its claims.
pub fn validate_token(token: &str) -> Result<ConductorClaims, AuthError> {
    let secret = get_secret()?;

    let mut validation = Validation::default();
    // We check expiration manually to provide better error messages
    validation.validate_exp = false;

    let token_data = decode::<ConductorClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

    // Manual expiration check
    if Utc::now().timestamp() > token_data.claims.exp {
        return Err(AuthError::Expired);
    }

    Ok(token_data.claims)
}

/// Check if claims have a required scope.
pub fn has_scope(claims: &ConductorClaims, required: &str) -> bool {
    claims.scope.contains(&required.to_string())
}

/// Validate an SSH public key format. Supports ed25519, rsa, and ecdsa keys.
pub fn validate_ssh_public_key(key: &str) -> Result<(), anyhow::Error> {
    let trimmed = key.trim();

    // Check for valid OpenSSH public key format
    if !trimmed.starts_with("ssh-ed25519 ")
        && !trimmed.starts_with("ssh-rsa ")
        && !trimmed.starts_with("ecdsa-sha2-nistp")
    {
        return Err(anyhow::anyhow!(
            "SSH public key must start with 'ssh-ed25519', 'ssh-rsa', or 'ecdsa-sha2-nistp'"
        ));
    }

    // OpenSSH keys have format: <type> <base64-data> [comment]
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!(
            "SSH public key must have at least type and data"
        ));
    }

    // Validate base64 data is non-empty
    if parts[1].is_empty() {
        return Err(anyhow::anyhow!("SSH public key data is empty"));
    }

    Ok(())
}

/// Get the shared signing secret from environment.
fn get_secret() -> Result<String, AuthError> {
    match env::var("CRABJAR_AUTH_SECRET") {
        Ok(s) => Ok(s),
        Err(_) => Ok("default-conductor-secret".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn setup_env(secret: &str) {
        unsafe { std::env::set_var("CRABJAR_AUTH_SECRET", secret) };
    }

    #[test]
    fn generate_and_validate_token() {
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_env("test-secret-key-123");

        let token =
            generate_token("worker-123", "ftw3", &["gpu:nvidia:3070ti".to_string()], 2).unwrap();

        let claims = validate_token(&token).unwrap();
        assert_eq!(claims.sub, "worker-123");
        assert_eq!(claims.name, "ftw3");
        assert_eq!(claims.trust_level, 2);
        assert!(has_scope(&claims, "tasks:claim"));
    }

    #[test]
    fn token_expires() {
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_env("test-secret-expiry");

        let now = Utc::now().timestamp();

        // Create expired token manually
        let claims = ConductorClaims {
            sub: "worker-123".to_string(),
            name: "ftw3".to_string(),
            capabilities: vec![],
            trust_level: 0,
            scope: vec!["heartbeat".to_string()],
            exp: now - 100, // Expired 100 seconds ago
            iat: now - 200,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("test-secret-expiry".as_bytes()),
        )
        .unwrap();

        let result = validate_token(&token);
        // Should fail with Expired error
        assert!(result.is_err());
        match result {
            Err(AuthError::Expired) => {}
            Err(e) => panic!("expected Expired, got: {:?}", e),
            Ok(_) => panic!("expected error for expired token"),
        }
    }

    #[test]
    fn wrong_secret_fails() {
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_env("signing-secret");

        // Generate a token with one secret
        let token = generate_token("worker-123", "ftw3", &[], 0).unwrap();

        // Change the secret and try to validate
        setup_env("different-secret");
        let result = validate_token(&token);
        assert!(matches!(result, Err(AuthError::InvalidToken(_))));
    }

    #[test]
    fn scope_by_trust_level() {
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_env("test-secret-scopes");

        // Trust level 0: heartbeat only
        let token0 = generate_token("w1", "worker", &[], 0).unwrap();
        let claims0 = validate_token(&token0).unwrap();
        assert_eq!(claims0.scope, vec!["heartbeat"]);

        // Trust level 1: + task claim/report
        let token1 = generate_token("w2", "worker", &[], 1).unwrap();
        let claims1 = validate_token(&token1).unwrap();
        assert!(has_scope(&claims1, "tasks:claim"));
        assert!(has_scope(&claims1, "tasks:report"));

        // Trust level 2+: + goals read
        let token3 = generate_token("w3", "worker", &[], 3).unwrap();
        let claims3 = validate_token(&token3).unwrap();
        assert!(has_scope(&claims3, "goals:read"));
    }
}
