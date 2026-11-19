use crate::error::Result;
use rusqlite::Connection;

/// Single source of truth for waypoint DDL.
/// Migrations are append-only — never modify existing steps.
const WAYPOINT_MIGRATIONS: &[&str] = &[
    // v1 — baseline waypoint tables
    "CREATE TABLE IF NOT EXISTS waypoints (
        id TEXT PRIMARY KEY,
        parent_id TEXT REFERENCES waypoints(id),
        environment TEXT NOT NULL,
        category TEXT NOT NULL,
        description TEXT NOT NULL,
        destinations TEXT NOT NULL DEFAULT '[]',
        [references] TEXT NOT NULL DEFAULT '[]',
        relevance REAL NOT NULL DEFAULT 0.5,
        freshness INTEGER NOT NULL,
        predecessor_id TEXT REFERENCES waypoints(id),
        transformation_id TEXT,
        recoverability TEXT NOT NULL DEFAULT 'unknown',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_waypoints_parent ON waypoints(parent_id)",
    "CREATE INDEX IF NOT EXISTS idx_waypoints_environment ON waypoints(environment)",
    "CREATE INDEX IF NOT EXISTS idx_waypoints_category ON waypoints(category)",
    "CREATE INDEX IF NOT EXISTS idx_waypoints_predecessor ON waypoints(predecessor_id)",
    // Root waypoint tracking per environment
    "CREATE TABLE IF NOT EXISTS waypoint_roots (
        environment TEXT PRIMARY KEY,
        root_id TEXT NOT NULL REFERENCES waypoints(id),
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    // Traversal audit log — who traversed where and when
    "CREATE TABLE IF NOT EXISTS waypoint_traversals (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        worker_id TEXT NOT NULL,
        start_id TEXT NOT NULL REFERENCES waypoints(id),
        end_id TEXT NOT NULL REFERENCES waypoints(id),
        hops INTEGER NOT NULL,
        depth_limit INTEGER,
        timestamp INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_traversals_worker ON waypoint_traversals(worker_id)",
    "CREATE INDEX IF NOT EXISTS idx_traversals_start ON waypoint_traversals(start_id)",
];

/// Apply waypoint schema migrations to a connection.
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    for ddl in WAYPOINT_MIGRATIONS {
        conn.execute_batch(ddl)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_creates_waypoints_table() {
        let conn = Connection::open(":memory:").unwrap();
        migrate(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='waypoints'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migrate_creates_waypoint_roots_table() {
        let conn = Connection::open(":memory:").unwrap();
        migrate(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='waypoint_roots'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migrate_is_idempotent() {
        let conn = Connection::open(":memory:").unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
    }

    #[test]
    fn migrate_creates_indexes() {
        let conn = Connection::open(":memory:").unwrap();
        migrate(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name LIKE 'idx_waypoints_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(count >= 4);
    }
}
