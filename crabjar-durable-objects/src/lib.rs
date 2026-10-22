//! Durable Object Checkpointing Infrastructure
//!
//! Provides disk-backed checkpointing for worker execution contexts, enabling
//! resume from last checkpoint on failure or restart. Uses SQLite for metadata
//! storage and file-based serialization for checkpoint payloads.
//!
//! # Architecture
//!
//! - **CheckpointStore**: SQLite-backed metadata store tracking checkpoints
//! - **CheckpointWriter**: Writes serialized state to disk with atomicity guarantees
//! - **CheckpointReader**: Reads and validates checkpoint data
//! - **CheckpointConfig**: Configuration options for frequency and retention
//!
//! # Usage
//!
//! ```rust
//! use crabjar_durable_objects::{CheckpointConfig, CheckpointStore};
//!
//! let config = CheckpointConfig::new("/tmp/checkpoints.db");
//! let store = CheckpointStore::open(&config.db_path).unwrap();
//!
//! // Create a checkpoint
//! let state = serde_json::json!({"step": 42, "data": vec![1, 2, 3]});
//! let id = store.create_checkpoint("task-1", state.clone(), None).unwrap();
//!
//! // Later, resume from last checkpoint
//! if let Some(cp) = store.get_latest_checkpoint("task-1").unwrap() {
//!     println!("Latest checkpoint: {}", cp.id);
//! }
//! ```

pub mod config;
pub mod store;
pub mod writer;
pub mod reader;
pub mod error;

pub use config::CheckpointConfig;
pub use store::CheckpointStore;
pub use writer::CheckpointWriter;
pub use reader::CheckpointReader;
pub use error::{CheckpointError, CheckpointResult};