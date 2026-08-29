//! Habitat — the spatial habitat layer (ADR-003).
//!
//! A coarse-geometry model of the user's lived environment (areas → grid
//! positions) over which computational state is laid out as positioned
//! entities. Presence in the model is state, not decoration.
//!
//! - `models` — the spatial nouns (area, entity, divergence, snapshot)
//! - `schema` — append-only SQLite migrations
//! - `store` — CRUD + snapshot reads
//!
//! See `specs/ADR-003_spatial_habitat_layer.md` for the decision record.

pub mod models;
pub mod schema;
pub mod store;

pub use models::{EntityKind, HabitatArea, HabitatDivergence, HabitatEntity, HabitatSnapshot};
pub use schema::migrate;
pub use store::HabitatStore;
