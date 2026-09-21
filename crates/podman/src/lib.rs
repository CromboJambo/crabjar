//! Podman container lifecycle management for CrabJar agent orchestration.
//!
//! Provides rootless container creation, execution, and supervision for
//! isolated agent workloads with least-privilege boundaries.

pub mod error;
pub mod executor;
pub mod sandbox_config;

pub use error::PodmanError;
pub use executor::PodmanExecutor;
pub use sandbox_config::{SandboxConfig, SandboxProfile};
