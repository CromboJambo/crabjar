//! crabjar-scheduler — cron job scheduling, workers, and watchers with execution gating.
//!
//! Unix philosophy: declarative job specs, compose via pipes/streams, one thing well.
//! Crabjar fundamental: observation does NOT equal permission — all actions go through guard.

pub mod error;
pub mod scheduler;
pub mod worker;
pub mod watcher;
pub mod cron_parser;

pub use error::SchedulerError;
pub use scheduler::{Scheduler, JobSpec};
pub use worker::WorkerPool;
pub use watcher::Watcher;
