//! The Slow Friend: an autonomous daemon that sleeps most of the time and only
//! wakes when something noteworthy happens. Implements the tiered attention
//! architecture from EXPL-005 with escalating computational cost at each tier.
//!
//! Architecture:
//!   LOW COST
//!            │
//!     deterministic sensors (cheap, constant)
//!            │
//!      something odd?
//!       /          \
//!     no            yes
//!     │              │
//!   sleep        structural interpretation (medium)
//!                  │
//!             meaningful?
//!              /       \
//!            no         yes
//!            │           │
//!          sleep       Jev decision layer (expensive)
//!                        │
//!                   worth acting?
//!                    /          \
//!                   no            yes
//!                   │              │
//!                 sleep       CrabJar proposal/execution
//!
//! Each tier is more expensive but only triggered when cheaper checks detect
//! something interesting. Attention itself becomes a scarce resource.

pub mod sensor;
pub mod interpretation;
pub mod decision;
pub mod action;
pub mod autonomy;
pub mod lattice;
pub mod daemon;
pub mod crabjar_bridge;
pub mod testing;

// Re-export key public types
pub use sensor::{Sensor, SensorResult};
pub use interpretation::{Interpretation, InterpretationEngine};
pub use decision::{JevDecision, JevEngine};
pub use action::{Action, ActionClass, PermissionLevel};
pub use autonomy::{AutonomyTracker, AutonomyLevel};
pub use lattice::SparseLattice;
pub use daemon::SlowFriendDaemon;
