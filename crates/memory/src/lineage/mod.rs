//! Lineage & Provenance Graph — Institutional Memory for Crabjar.
//!
//! Tracks how observations become proposals, decisions, implementations, and recognitions.
//! Append-only: records are never modified or deleted, only extended.
//!
//! # Core Concepts
//!
//! - **LineageNode**: A point in the institutional history (observation → proposal → decision → implementation → recognition)
//! - **LineageEdge**: A directed connection between nodes with a type and weight
//! - **ProvenanceQuery**: BFS-based tracing forward/backward through the graph
//! - **RecognitionBuilder**: Builds callbacks when implementations trace back to original proposers
//!
//! # Storage
//!
//! Uses the existing SQLite ledger (`crabjar-state`) with dedicated tables:
//! - `lineage_nodes`: All nodes in the graph
//! - `lineage_edges`: Directed edges connecting nodes

pub mod models;
pub mod provenance;
pub mod schema;
pub mod store;

#[cfg(test)]
mod tests;

pub use models::{
    DecisionOutcome, EdgeType, LineageEdge, LineageId, LineageNode, ProposalStatus,
    RecognitionCallback, SourceRef,
};
pub use provenance::{ProvenanceQuery, RecognitionBuilder, SqlProvenanceQuery};
pub use schema::migrate as migrate_lineage;
pub use store::LineageStore;

/// Convenience: open a lineage store and run migrations on an in-memory or file-backed DB.
#[cfg(test)]
pub fn test_store() -> LineageStore {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    schema::migrate(&conn).unwrap();
    LineageStore::new(conn)
}
