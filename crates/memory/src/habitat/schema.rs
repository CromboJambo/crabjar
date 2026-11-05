//! Habitat schema — single source of truth for the spatial habitat DDL.
//!
//! Migrations are append-only — never modify existing steps.
//! The habitat is a coarse-geometry model of the user's lived environment
//! (areas → positions) over which computational state is laid out as
//! positioned entities. See `specs/ADR-003_spatial_habitat_layer.md`.
use crate::error::Result;
use rusqlite::Connection;

const HABITAT_MIGRATIONS: &[&str] = &[
    // v1 — baseline: coarse geometry (areas) + positioned entities + divergence records
    "CREATE TABLE IF NOT EXISTS habitat_areas (
        id          INTEGER PRIMARY KEY,
        name        TEXT NOT NULL UNIQUE,
        grid_w      INTEGER NOT NULL DEFAULT 16,
        grid_h      INTEGER NOT NULL DEFAULT 4,
        created_at  TEXT NOT NULL DEFAULT (datetime('now'))
    )",
    "CREATE TABLE IF NOT EXISTS habitat_entities (
        id          TEXT NOT NULL UNIQUE,
        area_id     INTEGER NOT NULL,
        kind        TEXT NOT NULL,
        state       TEXT NOT NULL DEFAULT 'idle',
        label       TEXT NOT NULL DEFAULT '',
        x           INTEGER NOT NULL DEFAULT 0,
        y           INTEGER NOT NULL DEFAULT 0,
        created_at  TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (area_id) REFERENCES habitat_areas(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS habitat_divergences (
        id          INTEGER PRIMARY KEY,
        area_id     INTEGER NOT NULL,
        description TEXT NOT NULL,
        status      TEXT NOT NULL DEFAULT 'open',
        created_at  TEXT NOT NULL DEFAULT (datetime('now')),
        resolved_at TEXT,
        FOREIGN KEY (area_id) REFERENCES habitat_areas(id) ON DELETE CASCADE
    )",
    // Indexes for fast query
    "CREATE INDEX IF NOT EXISTS idx_habitat_entities_area ON habitat_entities(area_id)",
    "CREATE INDEX IF NOT EXISTS idx_habitat_entities_kind ON habitat_entities(kind)",
    "CREATE INDEX IF NOT EXISTS idx_habitat_divergences_area ON habitat_divergences(area_id)",
];

/// Apply habitat schema migrations to a connection.
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    for ddl in HABITAT_MIGRATIONS {
        conn.execute_batch(ddl)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_creates_habitat_tables() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'habitat_%'")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert!(tables.contains(&"habitat_areas".to_string()));
        assert!(tables.contains(&"habitat_entities".to_string()));
        assert!(tables.contains(&"habitat_divergences".to_string()));
    }

    #[test]
    fn migrate_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
    }
}
