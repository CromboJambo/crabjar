/// Secure token cache with OS-level encryption via keyring.
///
/// Replaces the Electron `tokenCache.js` bridge that implements a
/// localStorage-compatible interface for Teams' authentication provider.
///
/// Storage strategy (mirrors tokenCache.js):
///   1. keyring (OS keychain/KWallet/SecretService) — primary
///   2. SQLite in-memory fallback — when keyring unavailable
///   3. Rust HashMap — emergency memory fallback
use crate::cookie_store::{CookieStore, SessionToken};
use chrono::Utc;
use keyring::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Authentication-related key patterns (mirrors tokenCache.js _isAuthRelatedKey).
const AUTH_PATTERNS: &[&str] = &[
    "tmp.auth.v1.",
    "refresh_token",
    "msal.token",
    "EncryptionKey",
    "authSessionId",
    "LogoutState",
    "accessToken",
    "idtoken",
    "Account",
    "Authority",
    "ClientInfo",
];

/// Prefix for keyring entries (mirrors tokenCache.js _securePrefix).
const KEYRING_PREFIX: &str = "crabjar_teams_";

/// Secure token cache backed by OS keyring with graceful fallback.
pub struct SecureTokenCache {
    cookie_store: Arc<CookieStore>,
    keyring_enabled: bool,
    memory_fallback: Arc<RwLock<HashMap<String, String>>>,
}

impl SecureTokenCache {
    /// Create a new token cache. Checks keyring availability on init.
    pub fn new(cookie_store: Arc<CookieStore>) -> Self {
        let keyring_enabled = Self::check_keyring_available();
        if keyring_enabled {
            tracing::info!("token cache: keyring available");
        } else {
            tracing::warn!("token cache: keyring unavailable, using fallback storage");
        }

        Self {
            cookie_store,
            keyring_enabled,
            memory_fallback: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if OS-level keyring is available.
    fn check_keyring_available() -> bool {
        // keyring v3: try to create an entry — if it fails, no keyring.
        // The entry itself may still work with plaintext fallback.
        // We consider it "available" if we can create the entry struct.
        Entry::new("crabjar-host-test", "test-key").is_ok()
    }

    // --- Core Storage Interface (localStorage-compatible) ---

    /// Retrieve item from cache (mirrors tokenCache.js getItem).
    pub async fn get_item(&self, key: &str) -> Option<String> {
        if let Some(value) = self.get_secure_item(key).await {
            return Some(value);
        }

        // Fallback: SQLite store
        if let Some(token) = self.cookie_store.get_token(key).await.ok().flatten() {
            return Some(token.value);
        }

        // Emergency: memory fallback
        let mem = self.memory_fallback.read().await;
        mem.get(key).cloned()
    }

    /// Store item in cache (mirrors tokenCache.js setItem).
    pub async fn set_item(&self, key: &str, value: &str) -> Result<(), TokenCacheError> {
        // Try keyring first
        if self.keyring_enabled && self.set_secure_item(key, value).await.is_ok() {
            return Ok(());
        }

        // Fallback: SQLite store
        let token = SessionToken {
            key: key.to_string(),
            value: value.to_string(),
            encrypted: false,
            created_at: Utc::now().timestamp(),
            expires_at: None,
        };
        self.cookie_store
            .save_token(&token)
            .await
            .map_err(|e| TokenCacheError::Storage(format!("SQLite save failed: {e}")))?;

        Ok(())
    }

    /// Remove item from cache (mirrors tokenCache.js removeItem).
    pub async fn remove_item(&self, key: &str) {
        // Remove from keyring
        if self.keyring_enabled {
            let _ = self.remove_secure_item(key).await;
        }

        // Remove from SQLite
        let _ = self.cookie_store.remove_token(key).await;

        // Remove from memory fallback
        let mut mem = self.memory_fallback.write().await;
        mem.remove(key);
    }

    /// Clear all authentication-related keys.
    pub async fn clear(&self) -> Result<usize, TokenCacheError> {
        let auth_keys = self.get_auth_related_keys().await;
        let mut count = 0;
        for key in &auth_keys {
            self.remove_item(key).await;
            count += 1;
        }
        Ok(count)
    }

    /// Get cache statistics (mirrors tokenCache.js getCacheStats).
    pub async fn get_stats(&self) -> TokenCacheStats {
        let auth_keys = self.get_auth_related_keys().await;
        let refresh_tokens = auth_keys
            .iter()
            .filter(|k| k.contains("refresh_token"))
            .count();
        let msal_keys = auth_keys
            .iter()
            .filter(|k| k.contains("msal.token"))
            .count();

        let storage_type = if self.keyring_enabled {
            "keyring"
        } else {
            "sqlite"
        };

        TokenCacheStats {
            total_auth_keys: auth_keys.len(),
            refresh_token_count: refresh_tokens,
            msal_token_count: msal_keys,
            storage_type: storage_type.to_string(),
            keyring_available: self.keyring_enabled,
        }
    }

    // --- Secure Storage Backend ---

    async fn get_secure_item(&self, key: &str) -> Option<String> {
        let entry_key = format!("{}{}", KEYRING_PREFIX, key);
        let entry = match Entry::new("crabjar-host", &entry_key) {
            Ok(e) => e,
            Err(_) => return None,
        };

        entry.get_password().ok()
    }

    async fn set_secure_item(&self, key: &str, value: &str) -> Result<(), TokenCacheError> {
        let entry_key = format!("{}{}", KEYRING_PREFIX, key);
        let entry = Entry::new("crabjar-host", &entry_key)
            .map_err(|e| TokenCacheError::Keyring(format!("create entry: {e}")))?;
        entry
            .set_password(value)
            .map_err(|e| TokenCacheError::Keyring(format!("set password: {e}")))?;
        Ok(())
    }

    async fn remove_secure_item(&self, key: &str) -> Result<(), keyring::Error> {
        let entry_key = format!("{}{}", KEYRING_PREFIX, key);
        let entry = Entry::new("crabjar-host", &entry_key)?;
        entry.delete_credential()
    }

    // --- Auth Key Detection (mirrors tokenCache.js _getAuthRelatedKeys) ---

    async fn get_auth_related_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();

        // Check SQLite tokens
        if let Ok(tokens) = self.cookie_store.list_tokens().await {
            for token in tokens {
                if Self::is_auth_related_key(&token.key) {
                    keys.push(token.key);
                }
            }
        }

        keys
    }

    fn is_auth_related_key(key: &str) -> bool {
        AUTH_PATTERNS.iter().any(|pattern| key.contains(pattern))
    }

    /// Sanitize a key for logging (hide UUIDs, mirroring tokenCache.js _sanitizeKey).
    pub fn sanitize_key(key: &str) -> String {
        // Simple PII masking: replace hex UUID-like patterns without regex
        let _hex_chars = "0123456789abcdef";
        let chars: Vec<char> = key.chars().collect();
        let len = chars.len();
        let mut result = String::with_capacity(len);
        let mut i = 0;
        while i < len {
            // Check for UUID pattern at position i
            if i + 35 <= len {
                let candidate: String = chars[i..i + 36].iter().collect();
                if Self::looks_like_uuid(&candidate) {
                    result.push_str(&candidate[..8]);
                    result.push_str("...");
                    i += 36;
                    continue;
                }
            }
            result.push(chars[i]);
            i += 1;
        }
        result
    }

    fn looks_like_uuid(s: &str) -> bool {
        if s.len() != 36 {
            return false;
        }
        // Format: 8-4-4-4-12 hex digits with dashes
        let chars: Vec<char> = s.chars().collect();
        for &(pos, expected_dash) in &[(8, '-'), (13, '-'), (18, '-'), (23, '-')] {
            if chars[pos] != expected_dash {
                return false;
            }
        }
        for (i, ch) in chars.iter().enumerate().take(36) {
            if i == 8 || i == 13 || i == 18 || i == 23 {
                continue;
            }
            if !"0123456789abcdef".contains(*ch) {
                return false;
            }
        }
        true
    }
}

/// Token cache statistics.
#[derive(Debug, Clone)]
pub struct TokenCacheStats {
    pub total_auth_keys: usize,
    pub refresh_token_count: usize,
    pub msal_token_count: usize,
    pub storage_type: String,
    pub keyring_available: bool,
}

/// Token cache errors.
#[derive(thiserror::Error, Debug)]
pub enum TokenCacheError {
    #[error("keyring error: {0}")]
    Keyring(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("invalid key type")]
    InvalidKeyType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_key() {
        let key = "msal.token.12345678-1234-1234-1234-123456789abc";
        let sanitized = SecureTokenCache::sanitize_key(key);
        assert!(sanitized.contains("12345678..."));
    }

    #[test]
    fn test_is_auth_related_key() {
        assert!(SecureTokenCache::is_auth_related_key("tmp.auth.v1.abc"));
        assert!(SecureTokenCache::is_auth_related_key("refresh_token_xyz"));
        assert!(SecureTokenCache::is_auth_related_key("msal.token.def"));
        assert!(!SecureTokenCache::is_auth_related_key("random_key_xyz"));
    }
}
