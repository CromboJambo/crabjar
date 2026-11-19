//! Core capability model types.
//!
//! Workers receive a `CapabilitySet` — an explicitly granted collection of
//! tools, data sources, and actions. Anything not in the set simply doesn't
//! exist from the worker's perspective.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::grants::{ActionGrant, DataGrant, NetworkGrant, ToolGrant};
use super::identifiers::{DataSourceId, ToolId, WorkerId};

/// A capability set for a worker — everything the worker can do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub worker_id: WorkerId,
    pub granted_at: i64,
    pub expires_at: Option<i64>,
    pub tools: Vec<ToolGrant>,
    pub data_sources: Vec<DataGrant>,
    pub actions: Vec<ActionGrant>,
    pub network: Option<NetworkGrant>,
}

impl CapabilitySet {
    /// Create an empty capability set (least privilege default).
    pub fn new(worker_id: impl Into<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            worker_id: WorkerId::new(worker_id),
            granted_at: now,
            expires_at: None,
            tools: vec![],
            data_sources: vec![],
            actions: vec![],
            network: None,
        }
    }

    /// Check if this capability set has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            now > expires_at
        } else {
            false
        }
    }

    /// Add a tool grant.
    pub fn with_tool(mut self, grant: ToolGrant) -> Self {
        self.tools.push(grant);
        self
    }

    /// Add a data source grant.
    pub fn with_data_source(mut self, grant: DataGrant) -> Self {
        self.data_sources.push(grant);
        self
    }

    /// Add an action grant.
    pub fn with_action(mut self, grant: ActionGrant) -> Self {
        self.actions.push(grant);
        self
    }

    /// Set network access.
    pub fn with_network(mut self, grant: NetworkGrant) -> Self {
        self.network = Some(grant);
        self
    }

    /// Set expiration time.
    pub fn expires_in(mut self, seconds: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.expires_at = Some(now as i64 + seconds as i64);
        self
    }

    /// Check if a tool is granted.
    pub fn has_tool(&self, tool_id: &ToolId) -> bool {
        self.tools.iter().any(|t| &t.tool_id == tool_id)
    }

    /// Get the tool grant for a specific tool.
    pub fn get_tool(&self, tool_id: &ToolId) -> Option<&ToolGrant> {
        self.tools.iter().find(|t| &t.tool_id == tool_id)
    }

    /// Check if an action is granted.
    pub fn has_action(&self, action_id: &str) -> bool {
        self.actions.iter().any(|a| a.action_id == action_id)
    }

    /// Get the action grant for a specific action.
    pub fn get_action(&self, action_id: &str) -> Option<&ActionGrant> {
        self.actions.iter().find(|a| a.action_id == action_id)
    }

    /// Check if a data source is granted with at least the specified access mode.
    pub fn has_data_source_with_mode(
        &self,
        source_id: &DataSourceId,
        required_mode: &super::grants::AccessMode,
    ) -> bool {
        self.data_sources
            .iter()
            .any(|d| &d.source_id == source_id && &d.mode >= required_mode)
    }

    /// Intersect two capability sets — the result contains only what's in both.
    /// Composition through intersection: combining grants never increases access.
    pub fn intersect(&self, other: &CapabilitySet) -> CapabilitySet {
        // Tools: intersection of granted tools
        let tools: Vec<ToolGrant> = self
            .tools
            .iter()
            .filter(|t| other.tools.iter().any(|ot| ot.tool_id == t.tool_id))
            .cloned()
            .collect();

        // Data sources: intersection with most restrictive mode and scope
        let data_sources: Vec<DataGrant> = self
            .data_sources
            .iter()
            .filter_map(|d| {
                other
                    .data_sources
                    .iter()
                    .find(|od| od.source_id == d.source_id)
                    .map(|od| DataGrant {
                        source_id: d.source_id.clone(),
                        scope: d.scope.intersect(&od.scope),
                        mode: if d.mode <= od.mode {
                            d.mode.clone()
                        } else {
                            od.mode.clone()
                        },
                        filter: None, // Filters would need conjunction logic
                        origin: d.origin.clone(),
                    })
            })
            .collect();

        // Actions: intersection
        let actions: Vec<ActionGrant> = self
            .actions
            .iter()
            .filter(|a| other.actions.iter().any(|oa| oa.action_id == a.action_id))
            .cloned()
            .collect();

        CapabilitySet {
            worker_id: self.worker_id.clone(),
            granted_at: std::cmp::max(self.granted_at, other.granted_at),
            expires_at: if let (Some(a), Some(b)) = (self.expires_at, other.expires_at) {
                Some(std::cmp::min(a, b))
            } else {
                self.expires_at.or(other.expires_at)
            },
            tools,
            data_sources,
            actions,
            network: None, // Network intersection not implemented yet
        }
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
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        now > self.expires_at
    }
}

impl<T> std::fmt::Display for ExpiringGrant<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExpiringGrant(expires={})", self.expires_at)
    }
}
