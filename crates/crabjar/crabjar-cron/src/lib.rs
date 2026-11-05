//! `CrabJar` Cron Job Management
//!
//! Hermes-compatible cron job scheduling with `SQLite` persistence.
//! Jobs are stored in ~/.crabjar/dbs/cron.db and executed via conductor API.

pub mod scheduler;
pub mod store;
pub mod tool;
pub mod types;

use anyhow::Result;

/// Initialize the cron job store at the default location.
pub fn init_store() -> Result<store::CronStore> {
    let db_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".crabjar/dbs/cron.db");

    store::CronStore::open(db_path)
}
