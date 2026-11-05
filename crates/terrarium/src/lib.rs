//! The terrarium — an isometric hermit-crab skin over the crabjar habitat
//! contract (dagr v3).
//!
//! Architecture (spec §2): a **pure consumer** of `run.json`. The contract is
//! the sole authority on run-state; the terrarium looks in at it as data and
//! never writes it. Phase 0 is the headless behavior simulator — the scene is
//! a pure function of `(run.json, behaviors.toml)`, fully unit-testable with
//! zero TUI.

pub mod activity;
pub mod contract;
pub mod crabs;
pub mod habitat;
pub mod mapping;
pub mod sim_config;
pub mod sim_event;
pub mod sim_state;
pub mod simulator;
pub mod ui;
#[cfg(test)]
mod ui_tests;
