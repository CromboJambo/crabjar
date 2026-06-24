/// `host-agent` — agent loop engine for CrabJar host.
///
/// Implements the observe → understand → plan → execute → verify → reflect → persist
/// loop with pluggable inference backends and WorkItem persistence.
#![allow(dead_code)]
pub mod loop_engine;
pub mod planner;
pub mod executor;
pub mod verifier;
pub mod reflector;
pub mod work_item_store;
pub mod inference;

pub use loop_engine::{AgentLoop, LoopResult};
pub use planner::Planner;
pub use executor::TaskExecutor;
pub use verifier::Verifier;
pub use reflector::Reflector;
pub use work_item_store::WorkItemStore;
pub use inference::{InferenceBackend, HeuristicBackend};
