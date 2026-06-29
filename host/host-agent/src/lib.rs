//! `host-agent` — agent loop engine for CrabJar host.
//!
//! Implements the observe → understand → plan → execute → verify → reflect → persist
//! loop with pluggable inference backends and WorkItem persistence.
#![allow(dead_code)]
pub mod context_compression;
pub mod decision_gate;
pub mod executor;
pub mod inference;
pub mod loop_engine;
pub mod model_routing;
pub mod planner;
pub mod reflector;
pub mod verifier;
pub mod work_item_store;

pub use context_compression::{CompressionConfig, ContextCompressor};
pub use decision_gate::{Decision, DecisionConfig, DecisionGate};
pub use executor::TaskExecutor;
pub use inference::{HeuristicBackend, InferenceBackend};
pub use loop_engine::{AgentLoop, LoopResult};
pub use model_routing::{LoopPhase, ModelRouter, PhaseBackendKind, PhaseBuilder, PhaseConfig};
pub use planner::Planner;
pub use reflector::Reflector;
pub use verifier::Verifier;
pub use work_item_store::WorkItemStore;
