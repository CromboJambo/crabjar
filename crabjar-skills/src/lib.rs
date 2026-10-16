//! # crabjar-skills
//!
//! Skill loading infrastructure for `crabjar-agent` — discovers, loads, and validates
//! skills from the filesystem. Integrates with the tool registry and plugin system.
//!
//! ## Architecture
//!
//! ```text
//! crabjar-agent
//! ├── Tool Registry (tool_registry crate) — SQLite-backed tool discovery/registration
//! ├── Plugin Manager (crabjar-plugin crate) — process pool for subprocess plugins
//! └── Skill System (THIS CRATE) — skill loading, context injection, tool binding
//!     ├── Skill Loader — discovers skills from filesystem/directories
//!     ├── Skill Manifest — standard manifest format parsing and validation
//!     └── Tool Binder — connects skill tools to the registry (Phase 2)
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use crabjar_skills::{SkillLoader, LoadedSkill};
//!
//! // Discover skills from default locations
//! let loader = SkillLoader::new();
//! match loader.discover() {
//!     Ok(skills) => {
//!         for (name, skill) in &skills {
//!             println!("Found skill: {}", name);
//!             for tool in skill.tools() {
//!                 println!("  Tool: {}", tool.name);
//!             }
//!         }
//!     }
//!     Err(e) => eprintln!("Failed to discover skills: {}", e),
//! }
//! ```

pub mod error;
pub mod loader;
pub mod manifest;

pub use error::SkillError;
pub use loader::{LoadedSkill, SkillLoader};
pub use manifest::{ContextRequirements, Dependency, SkillManifest, ToolDefinition};
