//! Grant types for the capability model.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::identifiers::{DataSourceId, ToolId};
use super::patterns::CapabilityScope;

/// Access mode for a data grant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessMode {
    ReadOnly,
    ReadWrite,
    Execute,
}

impl fmt::Display for AccessMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessMode::ReadOnly => write!(f, "read-only"),
            AccessMode::ReadWrite => write!(f, "read-write"),
            AccessMode::Execute => write!(f, "execute"),
        }
    }
}

impl PartialOrd for AccessMode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AccessMode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // ReadOnly < ReadWrite < Execute (more restrictive first)
        let rank = |m: &AccessMode| match m {
            AccessMode::ReadOnly => 0,
            AccessMode::ReadWrite => 1,
            AccessMode::Execute => 2,
        };
        rank(self).cmp(&rank(other))
    }
}

/// Rate limit for a tool grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimit {
    pub max_calls: u32,
    pub window_seconds: u64,
}

impl fmt::Display for RateLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} calls / {}s", self.max_calls, self.window_seconds)
    }
}

/// Parameter constraints for a tool grant — what arguments are allowed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterConstraints {
    /// Allowed parameter names (empty = all allowed)
    pub allowed_params: Vec<String>,
    /// Required parameters
    pub required_params: Vec<String>,
}

/// Grant origin — who granted this capability and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantOrigin {
    /// Who granted it (supervisor ID, policy engine ID, human user)
    pub granted_by: String,
    /// Why it was granted (task reference, lineage ID, etc.)
    pub reason: String,
    /// When it was granted (Unix timestamp)
    pub granted_at: i64,
}

impl GrantOrigin {
    pub fn new(granted_by: impl Into<String>, reason: impl Into<String>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        Self {
            granted_by: granted_by.into(),
            reason: reason.into(),
            granted_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        }
    }
}

/// A grant for a specific tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolGrant {
    pub tool_id: ToolId,
    pub parameters: ParameterConstraints,
    pub rate_limit: Option<RateLimit>,
    pub origin: GrantOrigin,
}

impl ToolGrant {
    pub fn new(tool_id: impl Into<String>, origin: GrantOrigin) -> Self {
        Self {
            tool_id: ToolId::new(tool_id),
            parameters: ParameterConstraints::default(),
            rate_limit: None,
            origin,
        }
    }

    pub fn with_rate_limit(mut self, limit: RateLimit) -> Self {
        self.rate_limit = Some(limit);
        self
    }

    pub fn with_parameters(mut self, params: ParameterConstraints) -> Self {
        self.parameters = params;
        self
    }
}

/// Data filter for row/column-level filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataFilter {
    /// SQL-like WHERE clause for row filtering
    pub where_clause: Option<String>,
    /// Allowed columns (empty = all allowed)
    pub allowed_columns: Vec<String>,
}

/// A grant for a specific data source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataGrant {
    pub source_id: DataSourceId,
    pub scope: CapabilityScope,
    pub mode: AccessMode,
    pub filter: Option<DataFilter>,
    pub origin: GrantOrigin,
}

impl DataGrant {
    pub fn new(
        source_id: impl Into<String>,
        mode: AccessMode,
        scope: CapabilityScope,
        origin: GrantOrigin,
    ) -> Self {
        Self {
            source_id: DataSourceId::new(source_id),
            scope,
            mode,
            filter: None,
            origin,
        }
    }

    pub fn with_filter(mut self, filter: DataFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Check if a path is accessible under this grant.
    pub fn can_access_path(&self, path: &str) -> bool {
        self.scope.path_in_scope(path)
    }

    /// Check if a table is accessible under this grant.
    pub fn can_access_table(&self, table: &str) -> bool {
        self.scope.table_in_scope(table)
    }
}

/// Precondition for an action grant — what must be true before the action can execute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Precondition {
    pub description: String,
    /// Expression or condition that must evaluate to true
    pub condition: String,
}

/// Side effect of an action — what will change when the action executes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideEffect {
    pub description: String,
    /// Resource being modified
    pub resource: String,
}

/// A grant for a specific action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionGrant {
    pub action_id: String,
    pub preconditions: Vec<Precondition>,
    pub side_effects: Vec<SideEffect>,
    pub origin: GrantOrigin,
}

impl ActionGrant {
    pub fn new(action_id: impl Into<String>, origin: GrantOrigin) -> Self {
        Self {
            action_id: action_id.into(),
            preconditions: vec![],
            side_effects: vec![],
            origin,
        }
    }

    pub fn with_precondition(mut self, precondition: Precondition) -> Self {
        self.preconditions.push(precondition);
        self
    }

    pub fn with_side_effect(mut self, effect: SideEffect) -> Self {
        self.side_effects.push(effect);
        self
    }
}

/// Network grant — which network endpoints are accessible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkGrant {
    pub allowed_hosts: Vec<String>,
    pub allowed_ports: Vec<u16>,
    pub origin: GrantOrigin,
}

impl NetworkGrant {
    pub fn new(allowed_hosts: impl IntoIterator<Item = String>, origin: GrantOrigin) -> Self {
        Self {
            allowed_hosts: allowed_hosts.into_iter().collect(),
            allowed_ports: vec![],
            origin,
        }
    }

    pub fn with_ports(mut self, ports: impl IntoIterator<Item = u16>) -> Self {
        self.allowed_ports = ports.into_iter().collect();
        self
    }

    /// Check if a host is allowed.
    pub fn host_allowed(&self, host: &str) -> bool {
        self.allowed_hosts.iter().any(|h| h == host)
    }

    /// Check if a port is allowed (empty list = all ports allowed).
    pub fn port_allowed(&self, port: u16) -> bool {
        self.allowed_ports.is_empty() || self.allowed_ports.contains(&port)
    }
}

/// A grant that can expire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpiringGrant<T> {
    pub grant: T,
    pub expires_at: i64,
    pub auto_renew: bool,
}

impl<T> ExpiringGrant<T> {
    pub fn new(grant: T, expires_at: i64) -> Self {
        Self {
            grant,
            expires_at,
            auto_renew: false,
        }
    }

    pub fn with_auto_renew(mut self) -> Self {
        self.auto_renew = true;
        self
    }

    pub fn is_expired(&self) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        now > self.expires_at
    }
}

impl<T> fmt::Display for ExpiringGrant<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExpiringGrant(expires={})", self.expires_at)
    }
}
