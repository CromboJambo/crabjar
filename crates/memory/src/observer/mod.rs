//! Observer Worker State — Third Lifecycle Mode.
//!
//! Workers aren't binary (working/idle). There's a valuable third state where
//! a worker is actively building situational awareness about the environment
//! without modifying authoritative state. Like a new employee who can read
//! everything but isn't yet trusted to change anything.
//!
//! Observers *produce* information; workers *consume and transform* it.

pub mod models;
pub mod state_machine;

#[cfg(test)]
mod tests;

pub use models::{
    EnvironmentId, EvidenceRef, FindingCategory, FindingId, FindingRef, ObserverState,
};
pub use state_machine::{WorkerLifecycleError, WorkerLifecycleState};
