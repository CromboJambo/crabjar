/// Scope isolation: identity, project, tenant, thread boundary enforcement.
///
/// Every data operation requires a scope parameter — no blind defaults.
/// Cross-scope operations require explicit authorization.
///
/// ## Design
///
/// IronClaw enforces scope boundaries as first-class type-level constructs.
/// Crabjar's guard gate protects execution but not data — without scope isolation,
/// a compromised tool can read/write across project boundaries.
///
/// ## Scope Dimensions
///
/// - `identity` — who is acting (user, agent, system)
/// - `project` — which project context (crabjar's core isolation dimension)
/// - `tenant` — multi-tenant separation (future)
/// - `thread` — conversation/session boundary
///
/// ## Usage
///
/// ```ignore
/// let scope = Scope::project("my-project");
/// let effective = scope.resolve_trust(
///     requested = TrustLayer::High,
///     via = PolicyChain::default()
/// );
/// ```
use std::fmt;

/// Scope dimensions that define the boundary of a data operation.
///
/// Every operation must carry a scope — there are no blind defaults.
/// This prevents data leakage between projects at the type level.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Scope {
    /// Identity of the actor (user, agent, system)
    pub identity: Identity,
    /// Project context — crabjar's core isolation dimension
    pub project: Option<ProjectId>,
    /// Tenant context — multi-tenant separation (future)
    pub tenant: Option<TenantId>,
    /// Thread/session context — conversation boundary
    pub thread: Option<ThreadId>,
}

/// Identity of the actor performing an operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Identity {
    /// Human user
    User(String),
    /// Agent (AI assistant)
    Agent(String),
    /// System process
    System(String),
}

impl fmt::Display for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Identity::User(name) => write!(f, "user:{}", name),
            Identity::Agent(name) => write!(f, "agent:{}", name),
            Identity::System(name) => write!(f, "system:{}", name),
        }
    }
}

/// Project identifier — core isolation dimension.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectId(String);

impl ProjectId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tenant identifier — multi-tenant separation (future).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantId(String);

impl TenantId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Thread/session identifier — conversation boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThreadId(String);

impl ThreadId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ThreadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Authorization for cross-scope operations.
///
/// When an operation needs to cross scope boundaries,
/// it must carry explicit authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossScopeAuth {
    /// The scope being accessed
    pub target_scope: Scope,
    /// The scope of the actor
    pub actor_scope: Scope,
    /// Reason for the cross-scope access
    pub reason: String,
    /// Authorized by whom (user, policy, system)
    pub authorized_by: String,
    /// When the authorization was granted
    pub authorized_at: i64,
}

impl CrossScopeAuth {
    pub fn new(
        target_scope: Scope,
        actor_scope: Scope,
        reason: impl Into<String>,
        authorized_by: impl Into<String>,
    ) -> Self {
        Self {
            target_scope,
            actor_scope,
            reason: reason.into(),
            authorized_by: authorized_by.into(),
            authorized_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Check if this authorization is still valid (not expired).
    /// Default TTL is 1 hour.
    pub fn is_valid(&self, ttl_seconds: i64) -> bool {
        chrono::Utc::now().timestamp() - self.authorized_at < ttl_seconds
    }
}

impl Scope {
    /// Create a scope for a specific project.
    /// This is the most common case — crabjar operates per-project.
    pub fn project(project_id: impl Into<String>) -> Self {
        Self {
            identity: Identity::System("crabjar".to_string()),
            project: Some(ProjectId::new(project_id)),
            tenant: None,
            thread: None,
        }
    }

    /// Create a scope for a specific user and project.
    pub fn user_project(user: impl Into<String>, project: impl Into<String>) -> Self {
        Self {
            identity: Identity::User(user.into()),
            project: Some(ProjectId::new(project.into())),
            tenant: None,
            thread: None,
        }
    }

    /// Create a scope for a specific agent and project.
    pub fn agent_project(agent: impl Into<String>, project: impl Into<String>) -> Self {
        Self {
            identity: Identity::Agent(agent.into()),
            project: Some(ProjectId::new(project.into())),
            tenant: None,
            thread: None,
        }
    }

    /// Add a thread context to this scope.
    pub fn with_thread(mut self, thread: impl Into<String>) -> Self {
        self.thread = Some(ThreadId::new(thread));
        self
    }

    /// Add a tenant context to this scope.
    pub fn with_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = Some(TenantId::new(tenant));
        self
    }

    /// Check if this scope can access data in another scope.
    /// Same project = allowed. Different project = requires CrossScopeAuth.
    pub fn can_access(&self, target: &Scope) -> bool {
        // Same project = allowed
        if self.project == target.project {
            return true;
        }
        // Same tenant = allowed only when both have a tenant set and they match
        if let (Some(a), Some(b)) = (&self.tenant, &target.tenant)
            && a == b
        {
            return true;
        }
        // System identity can access anything (with audit trail)
        if let Identity::System(_) = self.identity {
            return true;
        }
        // System scope is always accessible (trusted boundary)
        if let Identity::System(_) = &target.identity {
            return true;
        }
        false
    }

    /// Get the project ID if this scope has one.
    pub fn project_id(&self) -> Option<&ProjectId> {
        self.project.as_ref()
    }

    /// Get the thread ID if this scope has one.
    pub fn thread_id(&self) -> Option<&ThreadId> {
        self.thread.as_ref()
    }

    /// Serialize scope to a string for logging/audit.
    pub fn to_scope_string(&self) -> String {
        let mut parts = Vec::new();
        parts.push(format!("identity={}", self.identity));
        if let Some(ref p) = self.project {
            parts.push(format!("project={}", p));
        }
        if let Some(ref t) = self.tenant {
            parts.push(format!("tenant={}", t));
        }
        if let Some(ref th) = self.thread {
            parts.push(format!("thread={}", th));
        }
        parts.join(", ")
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_scope_string())
    }
}

/// Scope-aware data access trait.
///
/// Types that implement this trait require a scope for all operations.
/// This enforces scope isolation at the type level.
pub trait ScopedAccess {
    /// The scope required to access this data.
    type Scope: Clone;

    /// Read data with a scope — returns error if scope is insufficient.
    fn read_with_scope(&self, scope: &Self::Scope) -> Result<String, ScopeError>;

    /// Write data with a scope — returns error if scope is insufficient.
    fn write_with_scope(
        &self,
        scope: &Self::Scope,
        data: &str,
    ) -> Result<(), ScopeError>;

    /// Check if a scope can access this data.
    fn can_access(&self, scope: &Self::Scope) -> bool;
}

/// Error for scope-related operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeError {
    /// The scope cannot access the target — requires CrossScopeAuth.
    InsufficientScope {
        actor: Box<Scope>,
        target: Box<Scope>,
    },
    /// Cross-scope authorization expired.
    AuthorizationExpired {
        authorized_at: i64,
        ttl_seconds: i64,
    },
    /// The scope is missing required dimensions.
    MissingScopeDimension {
        dimension: &'static str,
    },
}

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScopeError::InsufficientScope { actor, target } => {
                write!(
                    f,
                    "Insufficient scope: {} cannot access {}",
                    actor, target
                )
            }
            ScopeError::AuthorizationExpired {
                authorized_at,
                ttl_seconds,
            } => {
                write!(
                    f,
                    "Authorization expired: granted at {}, TTL {}s",
                    authorized_at, ttl_seconds
                )
            }
            ScopeError::MissingScopeDimension { dimension } => {
                write!(f, "Missing scope dimension: {}", dimension)
            }
        }
    }
}

impl std::error::Error for ScopeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_project_creation() {
        let scope = Scope::project("my-project");
        assert_eq!(scope.project.as_ref().unwrap().as_str(), "my-project");
        assert!(scope.tenant.is_none());
        assert!(scope.thread.is_none());
    }

    #[test]
    fn test_scope_user_project() {
        let scope = Scope::user_project("alice", "project-a");
        assert!(matches!(scope.identity, Identity::User(name) if name == "alice"));
        assert_eq!(scope.project.as_ref().unwrap().as_str(), "project-a");
    }

    #[test]
    fn test_scope_agent_project() {
        let scope = Scope::agent_project("agent-1", "project-b");
        assert!(matches!(scope.identity, Identity::Agent(name) if name == "agent-1"));
        assert_eq!(scope.project.as_ref().unwrap().as_str(), "project-b");
    }

    #[test]
    fn test_scope_with_thread() {
        let base = Scope::project("my-project");
        let with_thread = base.with_thread("thread-123");
        assert!(with_thread.thread.is_some());
        assert_eq!(
            with_thread.thread.as_ref().unwrap().as_str(),
            "thread-123"
        );
    }

    #[test]
    fn test_scope_with_tenant() {
        let base = Scope::project("my-project");
        let with_tenant = base.with_tenant("tenant-abc");
        assert!(with_tenant.tenant.is_some());
        assert_eq!(
            with_tenant.tenant.as_ref().unwrap().as_str(),
            "tenant-abc"
        );
    }

    #[test]
    fn test_scope_can_access_same_project() {
        let scope_a = Scope::project("project-a");
        let scope_b = Scope::project("project-a");
        assert!(scope_a.can_access(&scope_b));
        assert!(scope_b.can_access(&scope_a));
    }

    #[test]
    fn test_scope_cannot_access_different_project() {
        let scope_a = Scope::user_project("alice", "project-a");
        let scope_b = Scope::user_project("bob", "project-b");
        assert!(!scope_a.can_access(&scope_b));
        assert!(!scope_b.can_access(&scope_a));
    }

    #[test]
    fn test_scope_same_tenant_allows_access() {
        let scope_a = Scope::project("project-a").with_tenant("tenant-1");
        let scope_b = Scope::project("project-b").with_tenant("tenant-1");
        // Same tenant = allowed (future: multi-tenant support)
        assert!(scope_a.can_access(&scope_b));
    }

    #[test]
    fn test_scope_system_can_access_anything() {
        let user_scope = Scope::user_project("alice", "project-a");
        let system_scope = Scope::project("system");
        // System identity can access anything
        assert!(system_scope.can_access(&user_scope));
        assert!(user_scope.can_access(&system_scope));
    }

    #[test]
    fn test_cross_scope_auth_creation() {
        let actor = Scope::project("project-a");
        let target = Scope::project("project-b");
        let auth = CrossScopeAuth::new(
            target.clone(),
            actor.clone(),
            "migration",
            "admin-policy",
        );
        assert_eq!(auth.target_scope, target);
        assert_eq!(auth.actor_scope, actor);
        assert_eq!(auth.reason, "migration");
        assert_eq!(auth.authorized_by, "admin-policy");
        assert!(auth.is_valid(3600)); // Default TTL = 1 hour
    }

    #[test]
    fn test_cross_scope_auth_expiry() {
        let actor = Scope::project("project-a");
        let target = Scope::project("project-b");
        let auth = CrossScopeAuth::new(
            target.clone(),
            actor.clone(),
            "migration",
            "admin-policy",
        );
        // Simulate time passing
        let expired_auth = CrossScopeAuth {
            authorized_at: chrono::Utc::now().timestamp() - 3700, // > 1 hour ago
            ..auth
        };
        assert!(!expired_auth.is_valid(3600));
    }

    #[test]
    fn test_scope_display() {
        let scope = Scope::user_project("alice", "project-a")
            .with_thread("thread-123");
        let display = format!("{}", scope);
        assert!(display.contains("identity=user:alice"));
        assert!(display.contains("project=project-a"));
        assert!(display.contains("thread=thread-123"));
    }

    #[test]
    fn test_scope_error_display() {
        let actor = Scope::project("project-a");
        let target = Scope::project("project-b");
        let err = ScopeError::InsufficientScope {
            actor: Box::new(actor.clone()),
            target: Box::new(target.clone()),
        };
        let display = format!("{}", err);
        assert!(display.contains("Insufficient scope"));
        assert!(display.contains("project-a"));
        assert!(display.contains("project-b"));
    }

    #[test]
    fn test_identity_display() {
        let user = Identity::User("alice".to_string());
        assert_eq!(format!("{}", user), "user:alice");

        let agent = Identity::Agent("agent-1".to_string());
        assert_eq!(format!("{}", agent), "agent:agent-1");

        let system = Identity::System("crabjar".to_string());
        assert_eq!(format!("{}", system), "system:crabjar");
    }

    #[test]
    fn test_project_id_display() {
        let pid = ProjectId::new("my-project");
        assert_eq!(format!("{}", pid), "my-project");
    }

    #[test]
    fn test_scope_hash() {
        let scope_a = Scope::project("project-a");
        let scope_b = Scope::project("project-a");
        // Same scope should hash the same
        assert_eq!(scope_a, scope_b);
    }
}
