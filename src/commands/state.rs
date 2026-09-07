//! State-docs CLI commands (SQLite-backed via agent_context::state_docs)

use serde_json::json;
use crabjar_lib::StateCommand;

pub fn handle(command: StateCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        StateCommand::Index { docs_dir, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let count = agent_context::state_docs::indexer::index_all_docs(
                &conn,
                std::path::Path::new(&docs_dir),
            )?;
            Ok(json!({
                "success": true,
                "message": format!("indexed {} state-docs", count),
                "payload": {
                    "count": count,
                    "docs_dir": docs_dir,
                    "db_path": db_path,
                }
            }))
        }
        StateCommand::Show { doc_name, zoom } => {
            let conn = rusqlite::Connection::open("state-docs.db")?;
            agent_context::state_docs::migrate(&conn)?;
            // Strip .md extension to match how the indexer stores doc names (from frontmatter `name:`)
            let lookup_name = doc_name.strip_suffix(".md").unwrap_or(&doc_name);
            let renderer = agent_context::state_docs::Renderer::new(&conn);
            let (markdown, metadata) = renderer.render_doc(lookup_name, zoom)?;
            Ok(json!({
                "success": true,
                "message": format!("rendered {} at zoom level {}", doc_name, zoom),
                "payload": {
                    "doc": doc_name,
                    "zoom": zoom,
                    "markdown": markdown,
                    "metadata": metadata,
                }
            }))
        }
        StateCommand::Query {
            doc_name,
            section,
            keyword,
            db_path,
        } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            let querier = agent_context::state_docs::StateDocQuerier::new(
                conn,
                std::path::PathBuf::from(&db_path),
            );
            if let Some(section) = section {
                let result = querier.query_by_section(&doc_name, &section);
                Ok(json!({
                    "success": true,
                    "message": format!("queried section '{}' in {}", section, doc_name),
                    "payload": result,
                }))
            } else if let Some(keyword) = keyword {
                let result = querier.query_by_keyword(&doc_name, &keyword);
                Ok(json!({
                    "success": true,
                    "message": format!("searched keyword '{}' in {}", keyword, doc_name),
                    "payload": result,
                }))
            } else {
                Ok(json!({
                    "success": false,
                    "error": "must provide --section or --keyword",
                }))
            }
        }
        StateCommand::List { db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let mut stmt = conn.prepare(
                "SELECT doc_name, description, last_modified, line_count, checksum FROM doc_metadata ORDER BY last_modified DESC"
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(json!({
                    "name": row.get::<_, String>(0)?,
                    "description": row.get::<_, String>(1)?,
                    "last_modified": row.get::<_, String>(2)?,
                    "line_count": row.get::<_, i64>(3)?,
                    "checksum": row.get::<_, String>(4)?,
                }))
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(json!({
                "success": true,
                "message": format!("listed {} state-docs", results.len()),
                "payload": {
                    "docs": results,
                }
            }))
        }
        StateCommand::Confidence { doc_name, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let mut stmt = conn.prepare(
                "SELECT what_captured, what_missed, assumptions, blind_spots, stale_after FROM confidence WHERE doc_id = ?1"
            )?;
            let row = stmt
                .query_row(rusqlite::params![doc_name], |row| {
                    Ok(json!({
                        "what_captured": row.get::<_, String>(0)?,
                        "what_missed": row.get::<_, String>(1)?,
                        "assumptions": row.get::<_, String>(2)?,
                        "blind_spots": row.get::<_, String>(3)?,
                        "stale_after": row.get::<_, String>(4)?,
                    }))
                })
                .map_err(|e| {
                    if e == rusqlite::Error::QueryReturnedNoRows {
                        format!("no confidence assessment for {}", doc_name)
                    } else {
                        e.to_string()
                    }
                })?;
            Ok(json!({
                "success": true,
                "message": format!("retrieved confidence for {}", doc_name),
                "payload": {
                    "doc": doc_name,
                    "confidence": row,
                }
            }))
        }
        StateCommand::Annotations { doc_name, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let mut stmt = conn.prepare(
                "SELECT line, kind, message, author, status, created_at FROM annotations WHERE doc_id = ?1 ORDER BY line ASC"
            )?;
            let rows = stmt.query_map(rusqlite::params![doc_name], |row| {
                Ok(json!({
                    "line": row.get::<_, i64>(0)?,
                    "kind": row.get::<_, String>(1)?,
                    "message": row.get::<_, String>(2)?,
                    "author": row.get::<_, String>(3)?,
                    "status": row.get::<_, String>(4)?,
                    "created_at": row.get::<_, String>(5)?,
                }))
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            let open_count = results
                .iter()
                .filter(|a| a.get("status").and_then(|s| s.as_str()) == Some("open"))
                .count();
            Ok(json!({
                "success": true,
                "message": format!("retrieved {} annotations for {}", results.len(), doc_name),
                "payload": {
                    "doc": doc_name,
                    "annotations": results,
                    "open_count": open_count,
                }
            }))
        }
        StateCommand::Tables { doc_name, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let mut stmt = conn.prepare(
                "SELECT start_line, end_line, headers, rows FROM tables WHERE doc_id = ?1 ORDER BY start_line ASC"
            )?;
            let rows = stmt.query_map(rusqlite::params![doc_name], |row| {
                Ok(json!({
                    "start_line": row.get::<_, i64>(0)?,
                    "end_line": row.get::<_, i64>(1)?,
                    "headers": row.get::<_, String>(2)?,
                    "rows": row.get::<_, String>(3)?,
                }))
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(json!({
                "success": true,
                "message": format!("retrieved {} tables for {}", results.len(), doc_name),
                "payload": {
                    "doc": doc_name,
                    "tables": results,
                }
            }))
        }
        StateCommand::CodeBlocks { doc_name, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let mut stmt = conn.prepare(
                "SELECT start_line, end_line, language, content_hash FROM code_blocks WHERE doc_id = ?1 ORDER BY start_line ASC"
            )?;
            let rows = stmt.query_map(rusqlite::params![doc_name], |row| {
                Ok(json!({
                    "start_line": row.get::<_, i64>(0)?,
                    "end_line": row.get::<_, i64>(1)?,
                    "language": row.get::<_, String>(2)?,
                    "line_count": row.get::<_, i64>(3)?,
                }))
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(json!({
                "success": true,
                "message": format!("retrieved {} code blocks for {}", results.len(), doc_name),
                "payload": {
                    "doc": doc_name,
                    "code_blocks": results,
                }
            }))
        }
        StateCommand::Staleness { doc_name, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let querier = agent_context::state_docs::StateDocQuerier::new(
                conn,
                std::path::PathBuf::from(&db_path),
            );
            let result = querier.staleness_status(&doc_name);
            Ok(json!({
                "success": true,
                "message": format!("staleness check for {}", doc_name),
                "payload": result,
            }))
        }
    }
}
