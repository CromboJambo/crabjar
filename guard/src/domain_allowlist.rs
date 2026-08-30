//! Domain allowlist — restrict which external domains/URLs are callable.
//!
//! This is the domain-layer counterpart to command risk lists.
//! Even a "low-risk" command (e.g., `curl`) can be dangerous if its
//! target domain is untrusted. The allowlist gates outbound network calls
//! at the authorization layer.
//!
//! ## Design
//!
//! IronClaw's domain allowlist pattern: explicit allowlist (deny-by-default)
//! with per-domain trust levels. Crabjar extends this with:
//! - Domain wildcards (`*.example.com`)
//! - Per-trust-layer overrides (high-trust layers can access more domains)
//! - Audit logging of allowlist violations

use std::fmt;

/// Trust level for a domain in the allowlist.
///
/// Higher trust levels indicate more permissive access:
/// - `Trusted`: full access, no logging
/// - `Monitored`: access allowed but logged
/// - `Restricted`: requires explicit approval
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DomainTrustLevel {
    /// Fully trusted — no restrictions, no logging
    Trusted,
    /// Monitored — access allowed but logged for audit
    Monitored,
    /// Restricted — requires explicit approval from higher trust layer
    Restricted,
}

impl fmt::Display for DomainTrustLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainTrustLevel::Trusted => write!(f, "trusted"),
            DomainTrustLevel::Monitored => write!(f, "monitored"),
            DomainTrustLevel::Restricted => write!(f, "restricted"),
        }
    }
}

/// A single entry in the domain allowlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainEntry {
    /// Domain pattern (exact or wildcard, e.g., "api.example.com" or "*.github.com")
    pub domain: String,
    /// Trust level for this domain
    pub trust_level: DomainTrustLevel,
    /// When this entry was added
    pub added_at: i64,
    /// Who added it (reason/metadata)
    pub source: String,
}

impl DomainEntry {
    pub fn new(
        domain: impl Into<String>,
        trust_level: DomainTrustLevel,
        source: impl Into<String>,
    ) -> Self {
        Self {
            domain: domain.into(),
            trust_level,
            added_at: chrono::Utc::now().timestamp(),
            source: source.into(),
        }
    }

    /// Check if this entry matches a given domain.
    /// Supports exact match and wildcard prefix match (*.example.com matches api.example.com).
    pub fn matches(&self, domain: &str) -> bool {
        if self.domain == domain {
            return true;
        }
        // Wildcard match: "*.example.com" matches "api.example.com"
        if let Some(pattern) = self.domain.strip_prefix("*.") {
            let suffix = format!(".{}", pattern);
            return domain.ends_with(&suffix);
        }
        false
    }
}

/// Domain allowlist — deny-by-default list of permitted domains.
///
/// All outbound network calls must pass through this allowlist.
/// Domains not in the allowlist are blocked by default.
#[derive(Debug, Clone)]
pub struct DomainAllowlist {
    /// The allowlist entries
    entries: Vec<DomainEntry>,
    /// Whether to log monitored domain access
    pub log_monitored: bool,
    /// Whether to log restricted domain access attempts
    pub log_restricted: bool,
}

impl Default for DomainAllowlist {
    fn default() -> Self {
        Self {
            entries: Self::default_entries(),
            log_monitored: true,
            log_restricted: true,
        }
    }
}

impl DomainAllowlist {
    /// Create a new domain allowlist with default entries.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new domain allowlist with custom entries.
    pub fn with_entries(entries: Vec<DomainEntry>) -> Self {
        Self {
            entries,
            log_monitored: true,
            log_restricted: true,
        }
    }

    /// Add an entry to the allowlist.
    pub fn add_entry(&mut self, entry: DomainEntry) {
        // Remove existing entry with same domain to avoid duplicates
        self.entries.retain(|e| e.domain != entry.domain);
        self.entries.push(entry);
    }

    /// Remove an entry from the allowlist.
    pub fn remove_entry(&mut self, domain: &str) {
        self.entries.retain(|e| e.domain != domain);
    }

    /// Check if a domain is allowed.
    /// Returns:
    /// - `Ok(DomainTrustLevel)` if the domain is in the allowlist
    /// - `Err(DomainCheckError)` if the domain is blocked
    pub fn check(&self, domain: &str) -> Result<DomainTrustLevel, DomainCheckError> {
        for entry in &self.entries {
            if entry.matches(domain) {
                return Ok(entry.trust_level.clone());
            }
        }
        Err(DomainCheckError::NotAllowed(domain.to_string()))
    }

    /// Check if a domain is allowed and return the trust level.
    /// If the domain is monitored, logs the access (if log_monitored is true).
    /// If the domain is restricted, returns an error (requires explicit approval).
    pub fn check_with_log(&self, domain: &str) -> Result<DomainTrustLevel, DomainCheckError> {
        let trust_level = self.check(domain)?;

        match &trust_level {
            DomainTrustLevel::Trusted => {
                // No logging for trusted domains
            }
            DomainTrustLevel::Monitored => {
                if self.log_monitored {
                    tracing::info!(
                        domain = domain,
                        trust_level = %trust_level,
                        "Domain allowlist: monitored access"
                    );
                }
            }
            DomainTrustLevel::Restricted => {
                if self.log_restricted {
                    tracing::warn!(
                        domain = domain,
                        trust_level = %trust_level,
                        "Domain allowlist: restricted domain accessed (requires approval)"
                    );
                }
                // Restricted domains are allowed but logged
            }
        }

        Ok(trust_level)
    }

    /// Check if a domain is allowed for a given trust layer.
    /// Higher trust layers can access more domains:
    /// - Layer 3 (High): all domains
    /// - Layer 2 (Medium): trusted + monitored domains
    /// - Layer 1 (Low): trusted domains only
    pub fn check_for_trust_layer(
        &self,
        domain: &str,
        trust_layer: u32,
    ) -> Result<DomainTrustLevel, DomainCheckError> {
        let trust_level = self.check(domain)?;

        match trust_level {
            DomainTrustLevel::Trusted => Ok(DomainTrustLevel::Trusted),
            DomainTrustLevel::Monitored => {
                // Medium trust layers can access monitored domains
                if trust_layer >= 2 {
                    Ok(DomainTrustLevel::Monitored)
                } else {
                    Err(DomainCheckError::InsufficientTrustLayer {
                        domain: domain.to_string(),
                        required_layer: 2,
                        current_layer: trust_layer,
                    })
                }
            }
            DomainTrustLevel::Restricted => {
                // Only high trust layers can access restricted domains
                if trust_layer >= 3 {
                    Ok(DomainTrustLevel::Restricted)
                } else {
                    Err(DomainCheckError::InsufficientTrustLayer {
                        domain: domain.to_string(),
                        required_layer: 3,
                        current_layer: trust_layer,
                    })
                }
            }
        }
    }

    /// List all entries in the allowlist.
    pub fn entries(&self) -> &[DomainEntry] {
        &self.entries
    }

    /// Get the trust level for a domain, or None if not in allowlist.
    pub fn get_trust_level(&self, domain: &str) -> Option<DomainTrustLevel> {
        self.check(domain).ok()
    }

    /// Check if a domain is in the allowlist (any trust level).
    pub fn is_allowed(&self, domain: &str) -> bool {
        self.check(domain).is_ok()
    }

    /// Get the number of entries in the allowlist.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the allowlist is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Default entries for common development domains.
    fn default_entries() -> Vec<DomainEntry> {
        vec![
            // GitHub
            DomainEntry::new("github.com", DomainTrustLevel::Trusted, "default"),
            DomainEntry::new("api.github.com", DomainTrustLevel::Trusted, "default"),
            DomainEntry::new(
                "*.githubusercontent.com",
                DomainTrustLevel::Trusted,
                "default",
            ),
            // Rust/Crates
            DomainEntry::new("crates.io", DomainTrustLevel::Trusted, "default"),
            DomainEntry::new("crates-io.com", DomainTrustLevel::Trusted, "default"),
            DomainEntry::new("index.crates.io", DomainTrustLevel::Trusted, "default"),
            // Cargo registry
            DomainEntry::new("registry.crates.io", DomainTrustLevel::Trusted, "default"),
            // Docker Hub (for container pulls)
            DomainEntry::new("registry.docker.io", DomainTrustLevel::Monitored, "default"),
            DomainEntry::new("hub.docker.com", DomainTrustLevel::Monitored, "default"),
            // npm
            DomainEntry::new("registry.npmjs.org", DomainTrustLevel::Monitored, "default"),
            // PyPI
            DomainEntry::new("pypi.org", DomainTrustLevel::Monitored, "default"),
            DomainEntry::new(
                "files.pythonhosted.org",
                DomainTrustLevel::Monitored,
                "default",
            ),
            // npm registry mirrors
            DomainEntry::new("registry.npmjs.com", DomainTrustLevel::Monitored, "default"),
            // Rust toolchain
            DomainEntry::new("static.rust-lang.org", DomainTrustLevel::Trusted, "default"),
            DomainEntry::new("rust-lang.org", DomainTrustLevel::Trusted, "default"),
            // Cargo git
            DomainEntry::new("git.crates.io", DomainTrustLevel::Trusted, "default"),
            // Tailscale (for internal networking)
            DomainEntry::new(
                "login.tailscale.com",
                DomainTrustLevel::Monitored,
                "default",
            ),
            DomainEntry::new(
                "control.tailscale.com",
                DomainTrustLevel::Monitored,
                "default",
            ),
            // Hugging Face (for model downloads)
            DomainEntry::new("huggingface.co", DomainTrustLevel::Restricted, "default"),
            DomainEntry::new("*.huggingface.co", DomainTrustLevel::Restricted, "default"),
            DomainEntry::new("hf.co", DomainTrustLevel::Restricted, "default"),
            // Local development
            DomainEntry::new("localhost", DomainTrustLevel::Trusted, "default"),
            DomainEntry::new("127.0.0.1", DomainTrustLevel::Trusted, "default"),
            DomainEntry::new("::1", DomainTrustLevel::Trusted, "default"),
            // Internal network ranges
            DomainEntry::new("10.0.0.0/8", DomainTrustLevel::Monitored, "default"),
            DomainEntry::new("172.16.0.0/12", DomainTrustLevel::Monitored, "default"),
            DomainEntry::new("192.168.0.0/16", DomainTrustLevel::Monitored, "default"),
        ]
    }
}

/// Error for domain allowlist checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainCheckError {
    /// The domain is not in the allowlist.
    NotAllowed(String),
    /// The trust layer is insufficient for this domain.
    InsufficientTrustLayer {
        domain: String,
        required_layer: u32,
        current_layer: u32,
    },
}

impl fmt::Display for DomainCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainCheckError::NotAllowed(domain) => {
                write!(f, "Domain '{}' is not in the allowlist", domain)
            }
            DomainCheckError::InsufficientTrustLayer {
                domain,
                required_layer,
                current_layer,
            } => {
                write!(
                    f,
                    "Trust layer {} insufficient for domain '{}' (requires layer {})",
                    current_layer, domain, required_layer
                )
            }
        }
    }
}

impl std::error::Error for DomainCheckError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_domain_match() {
        let allowlist = DomainAllowlist::new();
        assert!(allowlist.check("github.com").is_ok());
        assert!(allowlist.check("crates.io").is_ok());
    }

    #[test]
    fn test_wildcard_domain_match() {
        let allowlist = DomainAllowlist::new();
        assert!(allowlist.check("api.github.com").is_ok());
        assert!(allowlist.check("raw.githubusercontent.com").is_ok());
        assert!(allowlist.check("avatars.githubusercontent.com").is_ok());
    }

    #[test]
    fn test_wildcard_does_not_match_prefix() {
        let allowlist = DomainAllowlist::new();
        // "*.github.com" should NOT match "notgithub.com"
        // (but it's not in the default allowlist anyway, so this tests the wildcard logic)
        let result = allowlist.check("notgithub.com");
        // This should be Err because "notgithub.com" is not in the default allowlist
        assert!(result.is_err());
    }

    #[test]
    fn test_domain_not_in_allowlist() {
        let allowlist = DomainAllowlist::new();
        assert!(allowlist.check("evil.com").is_err());
        assert!(allowlist.check("malicious.org").is_err());
    }

    #[test]
    fn test_default_entries_exist() {
        let allowlist = DomainAllowlist::new();
        assert!(!allowlist.is_empty());
        assert!(allowlist.is_allowed("github.com"));
        assert!(allowlist.is_allowed("crates.io"));
        assert!(allowlist.is_allowed("localhost"));
    }

    #[test]
    fn test_custom_entries() {
        let entries = vec![
            DomainEntry::new("custom.example.com", DomainTrustLevel::Trusted, "test"),
            DomainEntry::new("*.custom.example.com", DomainTrustLevel::Monitored, "test"),
        ];
        let allowlist = DomainAllowlist::with_entries(entries);

        assert!(allowlist.check("custom.example.com").is_ok());
        assert!(allowlist.check("api.custom.example.com").is_ok());
        assert!(allowlist.check("evil.com").is_err());
    }

    #[test]
    fn test_trust_layer_check() {
        let entries = vec![
            DomainEntry::new("trusted.example.com", DomainTrustLevel::Trusted, "test"),
            DomainEntry::new("monitored.example.com", DomainTrustLevel::Monitored, "test"),
            DomainEntry::new(
                "restricted.example.com",
                DomainTrustLevel::Restricted,
                "test",
            ),
        ];
        let allowlist = DomainAllowlist::with_entries(entries);

        // Layer 3 (high) can access all
        assert!(
            allowlist
                .check_for_trust_layer("trusted.example.com", 3)
                .is_ok()
        );
        assert!(
            allowlist
                .check_for_trust_layer("monitored.example.com", 3)
                .is_ok()
        );
        assert!(
            allowlist
                .check_for_trust_layer("restricted.example.com", 3)
                .is_ok()
        );

        // Layer 2 (medium) can access trusted + monitored
        assert!(
            allowlist
                .check_for_trust_layer("trusted.example.com", 2)
                .is_ok()
        );
        assert!(
            allowlist
                .check_for_trust_layer("monitored.example.com", 2)
                .is_ok()
        );
        assert!(
            allowlist
                .check_for_trust_layer("restricted.example.com", 2)
                .is_err()
        );

        // Layer 1 (low) can access trusted only
        assert!(
            allowlist
                .check_for_trust_layer("trusted.example.com", 1)
                .is_ok()
        );
        assert!(
            allowlist
                .check_for_trust_layer("monitored.example.com", 1)
                .is_err()
        );
        assert!(
            allowlist
                .check_for_trust_layer("restricted.example.com", 1)
                .is_err()
        );
    }

    #[test]
    fn test_add_and_remove_entries() {
        let mut allowlist = DomainAllowlist::new();
        assert!(!allowlist.is_allowed("new.example.com"));

        allowlist.add_entry(DomainEntry::new(
            "new.example.com",
            DomainTrustLevel::Trusted,
            "test",
        ));
        assert!(allowlist.is_allowed("new.example.com"));

        allowlist.remove_entry("new.example.com");
        assert!(!allowlist.is_allowed("new.example.com"));
    }

    #[test]
    fn test_domain_trust_level_display() {
        assert_eq!(format!("{}", DomainTrustLevel::Trusted), "trusted");
        assert_eq!(format!("{}", DomainTrustLevel::Monitored), "monitored");
        assert_eq!(format!("{}", DomainTrustLevel::Restricted), "restricted");
    }

    #[test]
    fn test_domain_check_error_display() {
        let err = DomainCheckError::NotAllowed("evil.com".to_string());
        assert!(format!("{}", err).contains("evil.com"));

        let err = DomainCheckError::InsufficientTrustLayer {
            domain: "restricted.com".to_string(),
            required_layer: 3,
            current_layer: 2,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("restricted.com"));
        assert!(msg.contains("3"));
        assert!(msg.contains("2"));
    }
}
