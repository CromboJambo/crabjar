use rusqlite::{Connection, Result, params};

/// Apply lineage graph migrations to the database connection.
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS lineage_nodes (
            id TEXT PRIMARY KEY,
            node_type TEXT NOT NULL,
            source_ref TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            description TEXT NOT NULL,
            payload TEXT NOT NULL DEFAULT '{}',
            status TEXT NOT NULL DEFAULT 'active',
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS lineage_edges (
            from_id TEXT NOT NULL REFERENCES lineage_nodes(id),
            to_id TEXT NOT NULL REFERENCES lineage_nodes(id),
            edge_type TEXT NOT NULL,
            weight REAL DEFAULT 1.0,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (from_id, to_id, edge_type)
        );

        CREATE INDEX IF NOT EXISTS idx_lineage_source ON lineage_nodes(source_ref);
        CREATE INDEX IF NOT EXISTS idx_lineage_type ON lineage_nodes(node_type);
        CREATE INDEX IF NOT EXISTS idx_lineage_edges_from ON lineage_edges(from_id);
        CREATE INDEX IF NOT EXISTS idx_lineage_edges_to ON lineage_edges(to_id);
        ",
    )?;

    // Track migration version
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations (name TEXT PRIMARY KEY, applied_at INTEGER NOT NULL);")?;
    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (name, applied_at) VALUES ('lineage_graph', ?)",
        params![
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ],
    )?;

    Ok(())
}
