//! Exact-invocation fingerprint types — prevents approval smuggling.
//!
//! IronClaw's `ironclaw_approvals` uses SHA-256 fingerprints of exact tool
//! invocations (command + args + context) for approval decisions. Crabjar
//! adopts this pattern to close the gap where pattern-based approvals create
//! a false sense of security.

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
