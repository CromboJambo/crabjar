//! Waypoint System — Durable Navigational Objects
//!
//! Workers should be able to orient themselves in an environment without being
//! given the entire universe upfront. Waypoints are navigation hints, not security
//! boundaries. They form a hierarchy that workers traverse progressively rather
//! than receiving the entire environment at once.
//!
//! ## Hierarchy
//!
//! ```text
//! Root waypoint (environment orientation)
//!   └── Project waypoint (project context)
//!        └── System waypoint (available interfaces)
//!             └── Task waypoint (current objective + relevant artifacts)
//! ```
//!
//! ## Usage
//!
//! ```no_run
//! use agent_context::waypoints::{
//!     SqliteWaypointNavigator, WaypointNavigator, Waypoint, WaypointCategory, EnvironmentId,
//! };
//! use rusqlite::Connection;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let conn = Connection::open(":memory:")?;
//!     agent_context::waypoints::schema::migrate(&conn)?;
//!     let nav = SqliteWaypointNavigator::new(conn);
//!
//!     let env = EnvironmentId("crabjar-repo".to_string());
//!     let root = Waypoint::new(env.clone(), WaypointCategory::Orientation,
//!         "This is the crabjar repo — agent orchestration framework");
//!     let root = nav.create(&root)?;
//!     nav.set_root(&env, &root.id)?;
//!
//!     // Worker orients itself by traversing from root
//!     let path = nav.traverse("worker-1", &root.id, 3)?;
//!     println!("Worker oriented via {} hops", path.total_hops);
//!     Ok(())
//! }
//! ```

pub mod models;
pub mod navigator;
pub mod schema;

pub use models::{
    ArtifactId, CompressionDepthPolicy, EnvironmentId, FoldRef, FreshnessCheck, Recoverability,
    TraversalPath, Waypoint, WaypointCategory, WaypointId,
};
pub use navigator::{SqliteWaypointNavigator, WaypointNavigator};
pub use schema::migrate;
