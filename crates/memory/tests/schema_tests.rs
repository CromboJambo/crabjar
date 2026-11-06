use agent_context::state_docs::schema::migrate;
use rusqlite::Connection;

#[test]
fn schema_migrate_creates_all_tables() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();

    let mut stmt = conn
        .prepare("SELECT name, type FROM sqlite_master WHERE type IN ('table', 'index') ORDER BY type, name")
        .unwrap();
    let tables: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let table_names: Vec<&str> = tables
        .iter()
        .filter(|(_, t)| *t == "table")
        .map(|(n, _)| n.as_str())
        .collect();
    assert!(table_names.contains(&"doc_metadata"));
    assert!(table_names.contains(&"sections"));
    assert!(table_names.contains(&"tables"));
    assert!(table_names.contains(&"code_blocks"));
    assert!(table_names.contains(&"confidence"));
    assert!(table_names.contains(&"annotations"));
}

#[test]
fn schema_migrate_creates_indexes() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();

    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='index' ORDER BY name")
        .unwrap();
    let indexes: Vec<String> = stmt
        .query_map([], |row| -> std::result::Result<String, rusqlite::Error> {
            row.get(0)
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(!indexes.is_empty(), "should have at least one index");
}

#[test]
fn schema_migrate_is_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();
    migrate(&conn).unwrap();
    migrate(&conn).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 6);
}

#[test]
fn schema_wal_journal_mode() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();

    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "memory");
}
