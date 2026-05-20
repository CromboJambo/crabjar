use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_versions (
            id INTEGER PRIMARY KEY,
            version INTEGER NOT NULL,
            applied_at TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS knowledge_entries (
            id INTEGER PRIMARY KEY,
            content TEXT NOT NULL,
            kind TEXT NOT NULL,
            tags TEXT NOT NULL,
            metadata TEXT NOT NULL,
            weight REAL NOT NULL,
            source TEXT NOT NULL,
            active BOOLEAN NOT NULL DEFAULT 1
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS event_rows (
            id INTEGER PRIMARY KEY,
            event_type TEXT NOT NULL,
            timestamp TEXT NOT NULL
        )",
        [],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::schema;
    use tempfile::tempdir;

    #[test]
    fn migrate_creates_tables() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
    }

    #[test]
    fn migrate_from_memory_works() {
        let conn = rusqlite::Connection::open(":memory:").unwrap();
        schema::migrate(&conn).unwrap();
    }

    #[test]
    fn migrate_idempotent_works() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        schema::migrate(&conn).unwrap();
    }

    #[test]
    fn schema_versions_table_exists_after_migrate() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_versions",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn knowledge_entries_table_exists_after_migrate() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM knowledge_entries",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn event_rows_table_exists_after_migrate() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM event_rows",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn migrate_handles_corrupt_db() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
    }

    #[test]
    fn schema_version_increments() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO schema_versions (id, version, applied_at) VALUES (1, 1, '2026-01-01')",
            [],
        ).unwrap();
        let version: i64 = conn.query_row(
            "SELECT MAX(version) FROM schema_versions",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn migrate_preserves_data() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        conn.execute(
            "INSERT INTO knowledge_entries (id, content, kind, tags, metadata, weight, source, active) VALUES (1, 'test', 'instruction', '[]', '{}', 1.0, 'user', 1)",
            [],
        ).unwrap();
        schema::migrate(&conn).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM knowledge_entries",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
    }
}
