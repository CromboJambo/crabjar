#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(count > 0);
    }

    #[test]
    fn knowledge_table_exists_after_migrate() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM knowledge",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn events_table_exists_after_migrate() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM events",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn indexes_exist_after_migrate() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(count > 0);
    }

    #[test]
    fn wal_mode_set_after_migrate() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        let mode: String = conn.query_row(
            "PRAGMA journal_mode",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(mode, "WAL");
    }

    #[test]
    fn foreign_keys_enabled_after_migrate() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        let enabled: i64 = conn.query_row(
            "PRAGMA foreign_keys",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(enabled, 1);
    }

    #[test]
    fn knowledge_default_values() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO knowledge (content, kind) VALUES ('test', 'instruction')",
            [],
        ).unwrap();
        let weight: f64 = conn.query_row(
            "SELECT weight FROM knowledge WHERE content = 'test'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(weight, 1.0);
    }

    #[test]
    fn knowledge_default_tags_empty() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO knowledge (content, kind) VALUES ('test', 'instruction')",
            [],
        ).unwrap();
        let tags: String = conn.query_row(
            "SELECT tags FROM knowledge WHERE content = 'test'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(tags, "[]");
    }

    #[test]
    fn knowledge_default_meta_empty() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO knowledge (content, kind) VALUES ('test', 'instruction')",
            [],
        ).unwrap();
        let meta: String = conn.query_row(
            "SELECT meta FROM knowledge WHERE content = 'test'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(meta, "{}");
    }

    #[test]
    fn knowledge_default_active_one() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO knowledge (content, kind) VALUES ('test', 'instruction')",
            [],
        ).unwrap();
        let active: i64 = conn.query_row(
            "SELECT active FROM knowledge WHERE content = 'test'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(active, 1);
    }

    #[test]
    fn knowledge_checksum_default() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO knowledge (content, kind, checksum) VALUES ('test', 'instruction', 'initial')",
            [],
        ).unwrap();
        let checksum: String = conn.query_row(
            "SELECT checksum FROM knowledge WHERE content = 'test'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(checksum, "initial");
    }

    #[test]
    fn events_default_values() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO events (kind, source) VALUES ('insert', 'user')",
            [],
        ).unwrap();
        let ts: String = conn.query_row(
            "SELECT ts FROM events WHERE kind = 'insert'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(ts.contains("datetime"));
    }

    #[test]
    fn schema_versions_default_values() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO schema_versions (version) VALUES (0)",
            [],
        ).unwrap();
        let applied: String = conn.query_row(
            "SELECT applied FROM schema_versions WHERE version = 0",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(applied.contains("datetime"));
    }
}
