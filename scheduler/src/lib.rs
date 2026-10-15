//! crabjar-scheduler — cron job scheduling, workers, and watchers with execution gating.
//!
//! Unix philosophy: declarative job specs, compose via pipes/streams, one thing well.
//! Crabjar fundamental: observation does NOT equal permission — all actions go through guard.

pub mod cron_parser;
pub mod error;
pub mod scheduler;
pub mod watcher;
pub mod worker;

pub use error::SchedulerError;
pub use scheduler::{JobSpec, Scheduler};
pub use watcher::Watcher;
pub use worker::WorkerPool;
