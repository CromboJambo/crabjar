//! Capability enforcement: interface-layer checks for positive boundary model.
//!
//! Capabilities are enforced at the **interface layer**, not checked after
//! the fact. The key insight: `visible_tools()` and `visible_sources()` don't
//! return "all tools minus forbidden ones." They return **only** what's granted.
//! The worker never sees the full list.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::grants::{AccessMode, ActionGrant, DataGrant, GrantOrigin, ToolGrant};
use super::identifiers::{DataSourceId, ToolId, WorkerId};
use super::models::CapabilitySet;

/// Error for capability enforcement operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    /// Worker not found in the capability store.
    WorkerNotFound(WorkerId),
    /// Capability set has expired.
    Expired {
        worker_id: WorkerId,
        expires_at: i64,
    },
    /// Action is not permitted by any grant.
    NotPermitted { worker_id: WorkerId, action: String },
    /// Data source access denied.
    AccessDenied {
        worker_id: WorkerId,
        source_id: DataSourceId,
        required_mode: AccessMode,
    },
}

impl std::fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapabilityError::WorkerNotFound(id) => write!(f, "Worker not found: {}", id),
            CapabilityError::Expired {
                worker_id,
                expires_at,
            } => {
                write!(
                    f,
                    "Capability set expired for {} at {}",
                    worker_id, expires_at
                )
            }
            CapabilityError::NotPermitted { worker_id, action } => {
                write!(f, "Action '{}' not permitted for {}", action, worker_id)
            }
            CapabilityError::AccessDenied {
                worker_id,
                source_id,
                required_mode,
            } => {
                write!(
                    f,
                    "Access to {} denied for {} (requires {})",
                    source_id, worker_id, required_mode
                )
            }
        }
    }
}

impl std::error::Error for CapabilityError {}

/// Tool descriptor — what a worker sees when it lists available tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub tool_id: ToolId,
    pub description: String,
}

/// Source descriptor — what a worker sees when it lists available data sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDescriptor {
    pub source_id: DataSourceId,
    pub mode: AccessMode,
    pub scope_summary: String,
}

/// Capability enforcer — the interface between workers and their capabilities.
///
/// This is the **capability broker** that replaces the traditional prohibition checker.
/// Instead of "is this action forbidden?" it answers "does this worker have a grant for this action?"
pub trait CapabilityEnforcer: Send + Sync {
    /// Resolve a worker's effective capability set.
    fn resolve(&self, worker_id: &WorkerId) -> Result<CapabilitySet, CapabilityError>;

    /// Check if an action is permitted (returns the grant that permits it).
    fn check_action(
        &self,
        worker_id: &WorkerId,
        action_id: &str,
    ) -> Result<Option<ActionGrant>, CapabilityError>;

    /// Check if a tool is granted to a worker.
    fn check_tool(
        &self,
        worker_id: &WorkerId,
        tool_id: &ToolId,
    ) -> Result<Option<ToolGrant>, CapabilityError>;

    /// Check data source access with required mode.
    fn check_data(
        &self,
        worker_id: &WorkerId,
        source_id: &DataSourceId,
        required_mode: &AccessMode,
    ) -> Result<Option<DataGrant>, CapabilityError>;

    /// Get the tool interface as seen by a specific worker.
    /// Only shows tools the worker has granted access to.
    fn visible_tools(&self, worker_id: &WorkerId) -> Result<Vec<ToolDescriptor>, CapabilityError>;

    /// Get the data sources as seen by a specific worker.
    fn visible_sources(
        &self,
        worker_id: &WorkerId,
    ) -> Result<Vec<SourceDescriptor>, CapabilityError>;

    /// Grant capabilities to a worker.
    fn grant(&self, capability_set: CapabilitySet) -> Result<(), CapabilityError>;

    /// Revoke all capabilities for a worker.
    fn revoke(&self, worker_id: &WorkerId) -> Result<(), CapabilityError>;
}

/// In-memory implementation of CapabilityEnforcer.
///
/// Suitable for testing and single-process deployments. For production,
/// use a persistent backend (SQLite, etc.).
pub struct InMemoryCapabilityEnforcer {
    store: Arc<RwLock<HashMap<WorkerId, CapabilitySet>>>,
}

impl Default for InMemoryCapabilityEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryCapabilityEnforcer {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Grant capabilities to a worker.
    pub fn grant(&self, capability_set: CapabilitySet) -> Result<(), CapabilityError> {
        let mut store = self.store.write().unwrap();
        store.insert(capability_set.worker_id.clone(), capability_set);
        Ok(())
    }

    /// Revoke all capabilities for a worker.
    pub fn revoke(&self, worker_id: &WorkerId) -> Result<(), CapabilityError> {
        let mut store = self.store.write().unwrap();
        if store.remove(worker_id).is_none() {
            return Err(CapabilityError::WorkerNotFound(worker_id.clone()));
        }
        Ok(())
    }

    /// Resolve a worker's effective capability set.
    pub fn resolve(&self, worker_id: &WorkerId) -> Result<CapabilitySet, CapabilityError> {
        let store = self.store.read().unwrap();
        match store.get(worker_id) {
            Some(set) => {
                if set.is_expired() {
                    Err(CapabilityError::Expired {
                        worker_id: worker_id.clone(),
                        expires_at: set.expires_at.unwrap_or(0),
                    })
                } else {
                    Ok(set.clone())
                }
            }
            None => Err(CapabilityError::WorkerNotFound(worker_id.clone())),
        }
    }

    /// Check if an action is permitted.
    pub fn check_action(
        &self,
        worker_id: &WorkerId,
        action_id: &str,
    ) -> Result<Option<ActionGrant>, CapabilityError> {
        let set = self.resolve(worker_id)?;
        Ok(set.get_action(action_id).cloned())
    }

    /// Check if a tool is granted.
    pub fn check_tool(
        &self,
        worker_id: &WorkerId,
        tool_id: &ToolId,
    ) -> Result<Option<ToolGrant>, CapabilityError> {
        let set = self.resolve(worker_id)?;
        Ok(set.get_tool(tool_id).cloned())
    }

    /// Check data source access.
    pub fn check_data(
        &self,
        worker_id: &WorkerId,
        source_id: &DataSourceId,
        required_mode: &AccessMode,
    ) -> Result<Option<DataGrant>, CapabilityError> {
        let set = self.resolve(worker_id)?;
        if !set.has_data_source_with_mode(source_id, required_mode) {
            return Err(CapabilityError::AccessDenied {
                worker_id: worker_id.clone(),
                source_id: source_id.clone(),
                required_mode: required_mode.clone(),
            });
        }
        Ok(set
            .data_sources
            .iter()
            .find(|d| &d.source_id == source_id)
            .cloned())
    }

    /// Get visible tools for a worker.
    pub fn visible_tools(
        &self,
        worker_id: &WorkerId,
    ) -> Result<Vec<ToolDescriptor>, CapabilityError> {
        let set = self.resolve(worker_id)?;
        Ok(set
            .tools
            .iter()
            .map(|t| ToolDescriptor {
                tool_id: t.tool_id.clone(),
                description: format!("Tool {}", t.tool_id),
            })
            .collect())
    }

    /// Get visible sources for a worker.
    pub fn visible_sources(
        &self,
        worker_id: &WorkerId,
    ) -> Result<Vec<SourceDescriptor>, CapabilityError> {
        let set = self.resolve(worker_id)?;
        Ok(set
            .data_sources
            .iter()
            .map(|d| SourceDescriptor {
                source_id: d.source_id.clone(),
                mode: d.mode.clone(),
                scope_summary: format!(
                    "paths={}, tables={}",
                    d.scope.paths.len(),
                    d.scope.tables.len()
                ),
            })
            .collect())
    }

    /// Create a minimal observer capability set.
    /// Observers get read-only access to waypoints and lineage graph,
    /// ability to record observations. No write capabilities at all.
    pub fn observer_capabilities(worker_id: impl Into<String>) -> CapabilitySet {
        CapabilitySet::new(worker_id)
            .with_tool(ToolGrant::new(
                "record_observation",
                GrantOrigin::new("system", "observer role"),
            ))
            .with_tool(ToolGrant::new(
                "read_waypoints",
                GrantOrigin::new("system", "observer role"),
            ))
            .with_tool(ToolGrant::new(
                "read_lineage",
                GrantOrigin::new("system", "observer role"),
            ))
    }
}

impl CapabilityEnforcer for InMemoryCapabilityEnforcer {
    fn resolve(&self, worker_id: &WorkerId) -> Result<CapabilitySet, CapabilityError> {
        self.resolve(worker_id)
    }

    fn check_action(
        &self,
        worker_id: &WorkerId,
        action_id: &str,
    ) -> Result<Option<ActionGrant>, CapabilityError> {
        self.check_action(worker_id, action_id)
    }

    fn check_tool(
        &self,
        worker_id: &WorkerId,
        tool_id: &ToolId,
    ) -> Result<Option<ToolGrant>, CapabilityError> {
        self.check_tool(worker_id, tool_id)
    }

    fn check_data(
        &self,
        worker_id: &WorkerId,
        source_id: &DataSourceId,
        required_mode: &AccessMode,
    ) -> Result<Option<DataGrant>, CapabilityError> {
        self.check_data(worker_id, source_id, required_mode)
    }

    fn visible_tools(&self, worker_id: &WorkerId) -> Result<Vec<ToolDescriptor>, CapabilityError> {
        self.visible_tools(worker_id)
    }

    fn visible_sources(
        &self,
        worker_id: &WorkerId,
    ) -> Result<Vec<SourceDescriptor>, CapabilityError> {
        self.visible_sources(worker_id)
    }

    fn grant(&self, capability_set: CapabilitySet) -> Result<(), CapabilityError> {
        self.grant(capability_set)
    }

    fn revoke(&self, worker_id: &WorkerId) -> Result<(), CapabilityError> {
        self.revoke(worker_id)
    }
}
