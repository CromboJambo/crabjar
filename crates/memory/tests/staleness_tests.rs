use agent_context::state_docs::indexer;
use agent_context::state_docs::querier::StateDocQuerier;
use agent_context::state_docs::schema::migrate;
use rusqlite::Connection;
use std::path::PathBuf;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();
    conn
}

fn index_content(conn: &Connection, doc_name: &str, content: &str) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("{}.md", doc_name));
    std::fs::write(&path, content).unwrap();
    indexer::index_doc(conn, &path, &[]).unwrap();
}

#[test]
fn staleness_fresh_doc() {
    let conn = setup_db();
    index_content(&conn, "fresh-doc", "# Fresh\n\nContent.\n");

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let status = querier.staleness_status("fresh-doc");
    assert_eq!(status["status"], "fresh");
    assert_eq!(status["is_trustworthy"], true);
}

#[test]
fn staleness_stale_doc() {
    let conn = setup_db();
    index_content(&conn, "stale-doc", "# Stale\n\nContent.\n");

    // Backdate the doc to 10 days ago
    conn.execute(
        "UPDATE doc_metadata SET last_modified = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-10 days') WHERE doc_name = ?",
        ("stale-doc",),
    )
    .unwrap();

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let status = querier.staleness_status("stale-doc");
    assert_eq!(status["status"], "stale");
    assert_eq!(status["is_trustworthy"], true);
}

#[test]
fn staleness_expired_doc() {
    let conn = setup_db();
    index_content(&conn, "expired-doc", "# Expired\n\nContent.\n");

    // Backdate the doc to 20 days ago
    conn.execute(
        "UPDATE doc_metadata SET last_modified = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-20 days') WHERE doc_name = ?",
        ("expired-doc",),
    )
    .unwrap();

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let status = querier.staleness_status("expired-doc");
    assert_eq!(status["status"], "expired");
    assert_eq!(status["is_trustworthy"], false);
}

#[test]
fn staleness_moldy_doc() {
    let conn = setup_db();
    index_content(&conn, "moldy-doc", "# Moldy\n\nContent.\n");

    // Backdate the doc to 40 days ago
    conn.execute(
        "UPDATE doc_metadata SET last_modified = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-40 days') WHERE doc_name = ?",
        ("moldy-doc",),
    )
    .unwrap();

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let status = querier.staleness_status("moldy-doc");
    assert_eq!(status["status"], "moldy");
    assert_eq!(status["is_trustworthy"], false);
}

#[test]
fn staleness_unknown_doc() {
    let conn = setup_db();
    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));

    // Non-existent docs are treated as fresh (no metadata)
    let status = querier.staleness_status("nonexistent");
    assert_eq!(status["status"], "fresh");
}

#[test]
fn staleness_boundary_fresh_to_stale() {
    let conn = setup_db();
    index_content(&conn, "boundary-doc", "# Boundary\n\nContent.\n");

    // Backdate to exactly 7 days (should be stale)
    conn.execute(
        "UPDATE doc_metadata SET last_modified = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-7 days') WHERE doc_name = ?",
        ("boundary-doc",),
    )
    .unwrap();

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let status = querier.staleness_status("boundary-doc");
    assert_eq!(status["status"], "stale");
}

#[test]
fn staleness_boundary_stale_to_expired() {
    let conn = setup_db();
    index_content(&conn, "boundary-doc", "# Boundary\n\nContent.\n");

    // Backdate to exactly 14 days (should be expired)
    conn.execute(
        "UPDATE doc_metadata SET last_modified = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-14 days') WHERE doc_name = ?",
        ("boundary-doc",),
    )
    .unwrap();

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let status = querier.staleness_status("boundary-doc");
    assert_eq!(status["status"], "expired");
}

#[test]
fn staleness_boundary_expired_to_moldy() {
    let conn = setup_db();
    index_content(&conn, "boundary-doc", "# Boundary\n\nContent.\n");

    // Backdate to exactly 30 days (should be moldy)
    conn.execute(
        "UPDATE doc_metadata SET last_modified = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-30 days') WHERE doc_name = ?",
        ("boundary-doc",),
    )
    .unwrap();

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let status = querier.staleness_status("boundary-doc");
    assert_eq!(status["status"], "moldy");
}

#[test]
fn staleness_json_output() {
    let conn = setup_db();
    index_content(&conn, "json-doc", "# JSON Doc\n\nContent.\n");

    // Backdate to 10 days ago (stale)
    conn.execute(
        "UPDATE doc_metadata SET last_modified = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-10 days') WHERE doc_name = ?",
        ("json-doc",),
    )
    .unwrap();

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let json = querier.staleness_status("json-doc");

    // Verify JSON structure
    assert!(json["status"] == "stale" || json["status"] == "fresh");
    assert!(json["days_old"].is_i64());
}
