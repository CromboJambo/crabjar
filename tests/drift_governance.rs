#[tokio::test]
async fn drift_governance_loop() {
    // This test validates the complete drift governance infrastructure:
    // 1. Index a state-doc (baseline)
    // 2. Modify it (create drift)
    // 3. Detect drift via checksum comparison
    // 4. Reindex to resolve drift
    
    let temp = tempfile::tempdir().unwrap();
    let docs_dir = temp.path().join("docs");
    std::fs::create_dir(&docs_dir).unwrap();
    let db_path = temp.path().join("drift.db");
    
    // Step 1: Create and index a state-doc
    let doc_path = docs_dir.join("test-drift.md");
    std::fs::write(&doc_path, "---\nname: test-drift\n---\n# Test\nOriginal content.").unwrap();
    
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        agent_context::state_docs::migrate(&conn).unwrap();
        agent_context::state_docs::indexer::index_all_docs(&conn, &docs_dir).unwrap();
    }
    
    // Step 2: Modify the doc to create drift
    std::fs::write(&doc_path, "---\nname: test-drift\n---\n# Test\nModified content.").unwrap();
    
    // Step 3: Detect drift via checksum comparison
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        agent_context::state_docs::migrate(&conn).unwrap();
        let querier = agent_context::state_docs::StateDocQuerier::new(conn, docs_dir.clone());
        let drift = querier.drift_status("test-drift");
        assert_eq!(drift["drift"], true, "Should detect drift after modification");
        assert_ne!(drift["indexed_checksum"], drift["current_checksum"], "Checksums should differ when drifted");
    }
    
    // Step 4: Reindex to resolve drift
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        agent_context::state_docs::migrate(&conn).unwrap();
        agent_context::state_docs::indexer::index_all_docs(&conn, &docs_dir.clone()).unwrap();
    }
    
    // Verify drift is resolved
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        agent_context::state_docs::migrate(&conn).unwrap();
        let querier2 = agent_context::state_docs::StateDocQuerier::new(conn, docs_dir.clone());
        let drift2 = querier2.drift_status("test-drift");
        assert_eq!(drift2["drift"], false, "Drift should be resolved after reindex");
        assert_eq!(drift2["indexed_checksum"], drift2["current_checksum"], "Checksums should match after reindex");
        
        // Also verify staleness reporting works
        let staleness = querier2.staleness_status("test-drift");
        assert_eq!(staleness["status"], "fresh", "Freshly indexed doc should be fresh");
    }
}

#[tokio::test]
async fn drift_nonexistent_doc() {
    // Verify drift detection handles non-indexed docs gracefully
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("drift.db");
    
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        agent_context::state_docs::migrate(&conn).unwrap();
        
        let querier = agent_context::state_docs::StateDocQuerier::new(conn, temp.path().to_path_buf());
        let drift = querier.drift_status("does-not-exist");
        
        assert_eq!(drift["exists"], false);
        // Non-indexed doc shows as "not drifted" (no baseline to compare against)
        assert_eq!(drift["drift"], false);
    }
}
