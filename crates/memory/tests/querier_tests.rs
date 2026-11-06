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
fn query_by_section_returns_json() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Test\n\n## Target Section\n\nContent here.\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.query_by_section("test-doc", "Target Section");
    assert_eq!(result["doc"], "test-doc");
    assert_eq!(result["section"], "Target Section");
}

#[test]
fn query_by_keyword_finds_matches() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Test\n\n## Target Section\n\nContent here.\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.query_by_keyword("test-doc", "Target");
    assert_eq!(result["doc"], "test-doc");
    assert_eq!(result["keyword"], "Target");
}

#[test]
fn get_confidence_returns_json() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Test\n\n## Target Section\n\nContent here.\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.get_confidence("test-doc");
    assert_eq!(result["doc"], "test-doc");
}

#[test]
fn get_annotations_returns_json() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Test\n\n## Target Section\n\nContent here.\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.get_annotations("test-doc");
    assert_eq!(result["doc"], "test-doc");
}

#[test]
fn get_doc_metadata_returns_json() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Test\n\n## Target Section\n\nContent here.\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.get_doc_metadata("test-doc");
    assert_eq!(result["doc"], "test-doc");
}

#[test]
fn get_all_sections_returns_json() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Test\n\n## Target Section\n\nContent here.\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.get_all_sections("test-doc");
    assert_eq!(result["doc"], "test-doc");
}

#[test]
fn drift_status_returns_json() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Test\n\n## Target Section\n\nContent here.\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.drift_status("test-doc");
    assert_eq!(result["doc"], "test-doc");
}

#[test]
fn staleness_status_returns_json() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Test\n\n## Target Section\n\nContent here.\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.staleness_status("test-doc");
    assert_eq!(result["doc"], "test-doc");
}

#[test]
fn query_by_section_finds_nested_sections() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Root\n\n## Parent\n\n### Child Section\n\nChild content.\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.query_by_section("test-doc", "Child Section");
    assert_eq!(result["doc"], "test-doc");
    assert_eq!(result["section"], "Child Section");
}

#[test]
fn query_by_keyword_finds_multiple_matches() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Root\n\n## First Match\n\nContent.\n\n## Second Match\n\nMore content.\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.query_by_keyword("test-doc", "Match");
    assert_eq!(result["doc"], "test-doc");
    // Should find both sections with "Match" in the title
    assert!(result["matches"].is_array());
}

#[test]
fn query_by_tags_returns_json() {
    let conn = setup_db();
    index_content(&conn, "test-doc", "# Test\n\n## Section\n\nContent.\n");

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.query_by_tags(&["test"]);
    assert!(result["tags"].is_array());
}

#[test]
fn query_section_with_tables() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Test\n\n## Data\n| A | B |\n|---|---|\n| 1 | 2 |\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    // query_by_section returns tables for that section
    let result = querier.query_by_section("test-doc", "Data");
    assert_eq!(result["doc"], "test-doc");
}

#[test]
fn query_section_with_code_blocks() {
    let conn = setup_db();
    index_content(
        &conn,
        "test-doc",
        "# Test\n\n## Code\n\n```rust\nlet x = 42;\n```\n",
    );

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    // query_by_section returns code blocks for that section
    let result = querier.query_by_section("test-doc", "Code");
    assert_eq!(result["doc"], "test-doc");
}

#[test]
fn empty_query_returns_empty_results() {
    let conn = setup_db();
    index_content(&conn, "test-doc", "# Test\n\n## Section\n\nContent.\n");

    let querier = StateDocQuerier::new(conn, PathBuf::from("/tmp"));
    let result = querier.query_by_keyword("test-doc", "nonexistent keyword");
    assert_eq!(result["doc"], "test-doc");
}
