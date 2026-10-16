//! # crabjar-trust-manager
//!
//! Trust tier management and skill cache for CrabJar remote skills.
//! Maps community/verified/official tiers to execution policies,
//! manages version-aware skill caching in ~/.crabjar/skill-cache/,
//! and provides update detection for installed skills.

pub mod error;
pub mod models;
pub mod trust;
pub mod cache;
pub mod updates;

pub use error::TrustManagerError;
pub use models::{TrustTier, SkillCacheEntry, UpdateCheckResult};
pub use trust::TrustManager;
pub use cache::SkillCache;
pub use updates::UpdateDetector;
