//! Positive Boundary Model — capabilities over prohibitions.
//!
//! Workers receive an explicitly granted collection of interfaces, tools,
//! and data sources. Anything not in the set simply doesn't exist from the
//! worker's perspective. No prohibition needed because there's nothing to
//! prohibit against.
//!
//! ## Core Principle
//!
//! The negative model tells the worker about the boundary ("don't look here").
//! The positive model *is* the boundary — the worker only sees what's granted.
//!
//! ## Key Properties
//!
//! - **Least privilege default**: Workers start with an empty capability set
//! - **Composition through intersection**: Combining grants never increases access
//! - **Expiring capabilities**: Grants can have optional expiration
//! - **Interface-layer enforcement**: O(1) lookup, checked before execution
//! - **Full auditability**: Every grant records who, why, when, what

pub mod enforcer;
pub mod grants;
pub mod identifiers;
pub mod models;
pub mod patterns;

#[cfg(test)]
mod tests;

pub use enforcer::{CapabilityEnforcer, InMemoryCapabilityEnforcer};
pub use grants::{
    AccessMode, ActionGrant, DataFilter, DataGrant, GrantOrigin, NetworkGrant,
    ParameterConstraints, Precondition, RateLimit, SideEffect, ToolGrant,
};
pub use identifiers::{DataSourceId, ToolId, WorkerId};
pub use models::{CapabilitySet, ExpiringGrant};
pub use patterns::{CapabilityScope, EndpointPattern, KeyPattern, PathPattern, TablePattern};
