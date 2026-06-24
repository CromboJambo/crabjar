//! Exact-invocation fingerprint approvals — prevents approval smuggling.
//!
//! IronClaw's `ironclaw_approvals` uses SHA-256 fingerprints of exact tool
//! invocations (command + args + context) for approval decisions. Crabjar
//! adopts this pattern to close the gap where pattern-based approvals create
//! a false sense of security.
//!
//! ## Design
//!
//! - `InvocationFingerprint` = SHA-256(command + args)
//! - `ApprovalLease` = approved fingerprint + TTL + scope
//! - Approving `cp src dst` does NOT approve `cp src malicious`
//! - Lease-based: approvals expire after a time window
//! - Scope-isolated: fingerprint only matches within same scope
//!
//! ## Usage
//!
//! ```ignore
//! let fp = InvocationFingerprint::from_command("rm", &["-rf".to_string(), "/tmp/test".to_string()]);
//! let lease = ApprovalLease::new(fp, Duration::from_secs(3600));
//! ```

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// SHA-256 fingerprint of an exact command invocation.
///
/// Computed from the command basename + all arguments joined with spaces.
/// This ensures that approving `cp src dst` does NOT approve `cp src malicious`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InvocationFingerprint(String);

impl InvocationFingerprint {
    /// Compute fingerprint from command and arguments.
    ///
    /// The fingerprint is SHA-256(command_basename + " " + args.join(" ")).
    /// If no args are provided, fingerprints just the command.
    pub fn from_command(command: &str, args: &[String]) -> Self {
        let basename = command.split('/').next_back().unwrap_or(command);
        let input = if args.is_empty() {
            basename.to_string()
        } else {
            format!("{} {}", basename, args.join(" "))
        };
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        Self(format!("{:x}", result))
    }

    /// Get the hex-encoded fingerprint string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this fingerprint matches another.
    pub fn matches(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    /// Check if this fingerprint matches a raw hex string.
    pub fn matches_hex(&self, hex: &str) -> bool {
        self.0 == hex
    }
}

impl fmt::Display for InvocationFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fp:{:.16}...", &self.0)
    }
}

/// Scope for fingerprint-based approvals.
///
/// Fingerprint matching is scope-isolated: a fingerprint approved in one
/// project scope does NOT match in another.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApprovalScope {
    /// Project context — core isolation dimension
    pub project: Option<String>,
    /// User context — who requested the action
    pub user: Option<String>,
    /// Source event ID — provenance of the triggering observation
    pub source_event_id: Option<String>,
}

impl ApprovalScope {
    pub fn new(
        project: Option<String>,
        user: Option<String>,
        source_event_id: Option<String>,
    ) -> Self {
        Self {
            project,
            user,
            source_event_id,
        }
    }

    /// Check if two scopes are compatible for fingerprint matching.
    pub fn is_compatible(&self, other: &Self) -> bool {
        // Must share the same project (if both have one) and same user
        let project_match = match (&self.project, &other.project) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        };
        let user_match = match (&self.user, &other.user) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        };
        project_match && user_match
    }
}

/// A lease granted for an exact invocation fingerprint.
///
/// IronClaw's `ironclaw_approvals` uses lease-based approvals with TTL.
/// Crabjar adapts this: an approval is a lease that expires after a time
/// window, preventing indefinite reuse of the same approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalLease {
    /// Unique ID for this lease
    pub id: String,
    /// The approved invocation fingerprint
    pub fingerprint: InvocationFingerprint,
    /// Scope this lease applies to
    pub scope: ApprovalScope,
    /// When the lease was granted
    pub granted_at: i64,
    /// When the lease expires (Unix timestamp)
    pub expires_at: i64,
    /// Who granted the approval
    pub granted_by: String,
    /// Whether this is a persistent (never-expiring) approval
    pub persistent: bool,
}

impl ApprovalLease {
    /// Create a new approval lease with the given TTL.
    pub fn new(
        fingerprint: InvocationFingerprint,
        scope: ApprovalScope,
        ttl_seconds: u64,
        granted_by: String,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            id: Uuid::new_v4().to_string(),
            fingerprint,
            scope,
            granted_at: now,
            expires_at: now + ttl_seconds as i64,
            granted_by,
            persistent: false,
        }
    }

    /// Create a persistent (never-expiring) approval lease.
    pub fn persistent(
        fingerprint: InvocationFingerprint,
        scope: ApprovalScope,
        granted_by: String,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            id: Uuid::new_v4().to_string(),
            fingerprint,
            scope,
            granted_at: now,
            expires_at: i64::MAX,
            granted_by,
            persistent: true,
        }
    }

    /// Check if this lease is still valid.
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        now < self.expires_at
    }

    /// Check if this lease matches a given fingerprint and scope.
    pub fn matches(&self, fingerprint: &InvocationFingerprint, scope: &ApprovalScope) -> bool {
        self.fingerprint.matches(fingerprint) && self.scope.is_compatible(scope) && self.is_valid()
    }
}

impl fmt::Display for ApprovalLease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "lease:{} fp:{:.16} expires:{} persistent:{}",
            &self.id[..8],
            self.fingerprint.as_str(),
            self.expires_at,
            self.persistent
        )
    }
}

/// In-memory store for approval leases.
///
/// IronClaw has both in-memory and filesystem-backed stores. Crabjar
/// starts with in-memory for simplicity; the schema is ready for
/// persistence later.
#[derive(Debug, Default)]
pub struct InMemoryApprovalStore {
    leases: std::sync::RwLock<Vec<ApprovalLease>>,
}

impl InMemoryApprovalStore {
    pub fn new() -> Self {
        Self {
            leases: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Insert a new approval lease.
    pub fn insert(&self, lease: ApprovalLease) {
        let mut leases = self.leases.write().unwrap();
        leases.push(lease);
    }

    /// Find a valid matching lease for the given fingerprint and scope.
    pub fn find_matching(
        &self,
        fingerprint: &InvocationFingerprint,
        scope: &ApprovalScope,
    ) -> Option<ApprovalLease> {
        let leases = self.leases.read().unwrap();
        leases
            .iter()
            .find(|l| l.matches(fingerprint, scope))
            .cloned()
    }

    /// List all valid leases (for audit/debug).
    pub fn list_valid(&self) -> Vec<ApprovalLease> {
        let leases = self.leases.read().unwrap();
        leases.iter().filter(|l| l.is_valid()).cloned().collect()
    }

    /// List all leases (including expired, for audit/debug).
    pub fn list_all(&self) -> Vec<ApprovalLease> {
        let leases = self.leases.read().unwrap();
        leases.clone()
    }

    /// Remove expired leases.
    pub fn cleanup_expired(&self) {
        let mut leases = self.leases.write().unwrap();
        leases.retain(|l| l.is_valid());
    }

    /// Remove all leases for a given scope.
    pub fn revoke_scope(&self, scope: &ApprovalScope) {
        let mut leases = self.leases.write().unwrap();
        leases.retain(|l| !l.scope.is_compatible(scope));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_from_command() {
        let fp = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let fp2 = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        assert_eq!(fp, fp2);
        assert_eq!(fp.as_str().len(), 64);
    }

    #[test]
    fn test_fingerprint_prevents_approval_smuggling() {
        let fp1 =
            InvocationFingerprint::from_command("cp", &["src".to_string(), "dst".to_string()]);
        let fp2 = InvocationFingerprint::from_command(
            "cp",
            &["src".to_string(), "malicious".to_string()],
        );
        assert!(!fp1.matches(&fp2));
        assert!(!fp2.matches(&fp1));
    }

    #[test]
    fn test_fingerprint_different_commands() {
        let fp1 = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let fp2 = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string(), "-v".to_string()],
        );
        assert!(!fp1.matches(&fp2));
    }

    #[test]
    fn test_fingerprint_matches_hex() {
        let fp = InvocationFingerprint::from_command("echo", &["hello".to_string()]);
        assert!(fp.matches_hex(fp.as_str()));
        assert!(
            !fp.matches_hex("0000000000000000000000000000000000000000000000000000000000000000")
        );
    }

    #[test]
    fn test_approval_lease_new() {
        let fp = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let scope = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-1".to_string()),
        );
        let lease = ApprovalLease::new(fp.clone(), scope, 3600, "user".to_string());
        assert!(lease.is_valid());
        assert!(!lease.persistent);
    }

    #[test]
    fn test_approval_lease_expires() {
        let fp = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let scope = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-1".to_string()),
        );
        let lease = ApprovalLease::new(fp.clone(), scope, 0, "user".to_string());
        // TTL of 0 means it expires immediately
        assert!(!lease.is_valid());
    }

    #[test]
    fn test_approval_lease_persistent() {
        let fp = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let scope = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-1".to_string()),
        );
        let lease = ApprovalLease::persistent(fp.clone(), scope, "user".to_string());
        assert!(lease.is_valid());
        assert!(lease.persistent);
    }

    #[test]
    fn test_approval_lease_matches_fingerprint() {
        let fp = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let scope = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-1".to_string()),
        );
        let lease = ApprovalLease::new(fp.clone(), scope.clone(), 3600, "user".to_string());
        assert!(lease.matches(&fp, &scope));
    }

    #[test]
    fn test_approval_lease_scope_mismatch() {
        let fp = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let scope_a = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-1".to_string()),
        );
        let scope_b = ApprovalScope::new(
            Some("project-b".to_string()),
            Some("alice".to_string()),
            Some("evt-2".to_string()),
        );
        let lease = ApprovalLease::new(fp.clone(), scope_a.clone(), 3600, "user".to_string());
        // Different project = no match
        assert!(!lease.matches(&fp, &scope_b));
    }

    #[test]
    fn test_approval_lease_user_mismatch() {
        let fp = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let scope_a = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-1".to_string()),
        );
        let scope_b = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("bob".to_string()),
            Some("evt-2".to_string()),
        );
        let lease = ApprovalLease::new(fp.clone(), scope_a.clone(), 3600, "user".to_string());
        // Different user = no match
        assert!(!lease.matches(&fp, &scope_b));
    }

    #[test]
    fn test_scope_compatibility() {
        let scope_a = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-1".to_string()),
        );
        let scope_b = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-2".to_string()),
        );
        assert!(scope_a.is_compatible(&scope_b));

        let scope_c = ApprovalScope::new(
            Some("project-b".to_string()),
            Some("alice".to_string()),
            Some("evt-3".to_string()),
        );
        assert!(!scope_a.is_compatible(&scope_c));
    }

    #[test]
    fn test_scope_compatible_none_values() {
        let scope_a = ApprovalScope::new(None, None, None);
        let scope_b = ApprovalScope::new(None, None, Some("evt-1".to_string()));
        assert!(scope_a.is_compatible(&scope_b));
    }

    #[test]
    fn test_approval_store_insert_and_find() {
        let store = InMemoryApprovalStore::new();
        let fp = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let scope = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-1".to_string()),
        );
        let lease = ApprovalLease::new(fp.clone(), scope.clone(), 3600, "user".to_string());
        store.insert(lease);

        let found = store.find_matching(&fp, &scope);
        assert!(found.is_some());
        assert_eq!(found.unwrap().fingerprint, fp);
    }

    #[test]
    fn test_approval_store_no_match() {
        let store = InMemoryApprovalStore::new();
        let fp1 = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let fp2 =
            InvocationFingerprint::from_command("cp", &["src".to_string(), "dst".to_string()]);
        let scope = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-1".to_string()),
        );
        let lease = ApprovalLease::new(fp1.clone(), scope.clone(), 3600, "user".to_string());
        store.insert(lease);

        let found = store.find_matching(&fp2, &scope);
        assert!(found.is_none());
    }

    #[test]
    fn test_approval_store_cleanup_expired() {
        let store = InMemoryApprovalStore::new();
        let fp = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let scope = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-1".to_string()),
        );
        // Add a valid lease
        store.insert(ApprovalLease::new(
            fp.clone(),
            scope.clone(),
            3600,
            "user".to_string(),
        ));
        // Add an expired lease
        store.insert(ApprovalLease::new(fp.clone(), scope, 0, "user".to_string()));

        store.cleanup_expired();
        assert_eq!(store.list_valid().len(), 1);
    }

    #[test]
    fn test_approval_store_revoke_scope() {
        let store = InMemoryApprovalStore::new();
        let fp = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let scope = ApprovalScope::new(
            Some("project-a".to_string()),
            Some("alice".to_string()),
            Some("evt-1".to_string()),
        );
        let lease = ApprovalLease::new(fp.clone(), scope.clone(), 3600, "user".to_string());
        store.insert(lease);

        store.revoke_scope(&scope);
        assert_eq!(store.list_valid().len(), 0);
    }

    #[test]
    fn test_fingerprint_from_path_command() {
        // Command with path should use basename only
        let fp1 = InvocationFingerprint::from_command(
            "/usr/bin/rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        let fp2 = InvocationFingerprint::from_command(
            "rm",
            &["-rf".to_string(), "/tmp/test".to_string()],
        );
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_empty_args() {
        let fp1 = InvocationFingerprint::from_command("ls", &[]);
        let fp2 = InvocationFingerprint::from_command("ls", &[]);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_display() {
        let fp = InvocationFingerprint::from_command("echo", &["hello".to_string()]);
        let display = format!("{}", fp);
        assert!(display.starts_with("fp:"));
        assert!(display.ends_with("..."));
    }

    #[test]
    fn test_approval_lease_display() {
        let fp = InvocationFingerprint::from_command("echo", &["hello".to_string()]);
        let scope = ApprovalScope::new(None, None, None);
        let lease = ApprovalLease::new(fp.clone(), scope, 3600, "user".to_string());
        let display = format!("{}", lease);
        assert!(display.starts_with("lease:"));
    }
}
