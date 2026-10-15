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

pub mod action;
pub mod autonomy;
pub mod crabjar_bridge;
pub mod daemon;
pub mod decision;
pub mod interpretation;
pub mod lattice;
pub mod sensor;
pub mod testing;

// Re-export key public types
pub use action::{Action, ActionClass, PermissionLevel};
pub use autonomy::{AutonomyLevel, AutonomyTracker};
pub use daemon::SlowFriendDaemon;
pub use decision::{JevDecision, JevEngine};
pub use interpretation::{Interpretation, InterpretationEngine};
pub use lattice::SparseLattice;
pub use sensor::{Sensor, SensorResult};
