use chrono::{DateTime, Utc};
use rusqlite::{Connection, Result, params};
use serde_json;

use super::models::{EdgeType, LineageEdge, LineageId, LineageNode, SourceRef};

/// Storage backend for lineage graph nodes and edges.
pub struct LineageStore {
    conn: Connection,
}

impl LineageStore {
    /// Open or create a lineage store on the given connection.
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Expose the underlying connection for direct queries.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Insert a new lineage node. Append-only: panics if ID already exists.
    pub fn insert_node(&self, node: &LineageNode) -> Result<()> {
        let now = Utc::now().timestamp();
        let payload = serde_json::to_string(node).unwrap_or_else(|_| "{}".into());

        self.conn.execute(
            "INSERT INTO lineage_nodes (id, node_type, source_ref, timestamp, description, payload, status, created_at)
             VALUES (?, ?, ?, ?, ?, ?, 'active', ?)",
            params![
                node.id().as_str(),
                node.node_type(),
                node.source().to_string(),
                node.timestamp().timestamp(),
                node.description(),
                payload,
                now,
            ],
        )?;

        Ok(())
    }

    /// Insert a new edge between two lineage nodes.
    pub fn insert_edge(&self, edge: &LineageEdge) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO lineage_edges (from_id, to_id, edge_type, weight, created_at)
             VALUES (?, ?, ?, ?, ?)",
            params![
                edge.from_id.as_str(),
                edge.to_id.as_str(),
                edge.edge_type.as_str(),
                edge.weight,
                edge.created_at.timestamp(),
            ],
        )?;

        Ok(())
    }

    /// Fetch a single node by ID.
    pub fn get_node(&self, id: &LineageId) -> Result<Option<LineageNode>> {
        let mut stmt = self
            .conn
            .prepare("SELECT payload FROM lineage_nodes WHERE id = ?")?;
        let mut rows = stmt.query_map(params![id.as_str()], |row| row.get::<_, String>(0))?;

        match rows.next() {
            Some(Ok(payload)) => serde_json::from_str(&payload)
                .map(Some)
                .map_err(|e| rusqlite::Error::InvalidColumnName(e.to_string())),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /// Fetch all outgoing edges from a node.
    pub fn outgoing_edges(&self, id: &LineageId) -> Result<Vec<LineageEdge>> {
        let mut stmt = self
            .conn
            .prepare("SELECT from_id, to_id, edge_type, weight, created_at FROM lineage_edges WHERE from_id = ?")?;
        let edges = stmt
            .query_map(params![id.as_str()], Self::row_to_edge)?
            .collect::<Result<Vec<_>>>()?;
        Ok(edges)
    }

    /// Fetch all incoming edges to a node.
    pub fn incoming_edges(&self, id: &LineageId) -> Result<Vec<LineageEdge>> {
        let mut stmt = self
            .conn
            .prepare("SELECT from_id, to_id, edge_type, weight, created_at FROM lineage_edges WHERE to_id = ?")?;
        let edges = stmt
            .query_map(params![id.as_str()], Self::row_to_edge)?
            .collect::<Result<Vec<_>>>()?;
        Ok(edges)
    }

    /// Find all proposals based on observations from a specific source.
    pub fn proposals_from_source(&self, source: &SourceRef) -> Result<Vec<LineageNode>> {
        let source_str = source.to_string();
        let mut stmt = self.conn.prepare(
            "SELECT n.payload FROM lineage_nodes n
             JOIN lineage_edges e ON n.id = e.to_id
             WHERE e.edge_type = 'based_on'
               AND n.node_type = 'proposal'
               AND EXISTS (
                   SELECT 1 FROM lineage_nodes obs
                   WHERE obs.id = e.from_id
                     AND obs.source_ref = ?
                     AND obs.node_type = 'observation'
               )",
        )?;

        let nodes = stmt
            .query_map(params![source_str], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok().and_then(|p| serde_json::from_str(&p).ok()))
            .collect();

        Ok(nodes)
    }

    /// Find implementations that resulted from a worker's proposals.
    pub fn implementations_from_worker(&self, worker: &SourceRef) -> Result<Vec<LineageNode>> {
        let worker_str = worker.to_string();
        let mut stmt = self.conn.prepare(
            "SELECT imp.payload FROM lineage_nodes imp
             JOIN lineage_edges e1 ON imp.id = e1.to_id AND e1.edge_type = 'implements'
             JOIN lineage_nodes dec ON dec.id = e1.from_id AND dec.node_type = 'decision'
             JOIN lineage_edges e2 ON dec.id = e2.to_id AND e2.edge_type = 'decides'
             JOIN lineage_nodes prop ON prop.id = e2.from_id AND prop.node_type = 'proposal'
             WHERE prop.source_ref = ?",
        )?;

        let nodes = stmt
            .query_map(params![worker_str], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok().and_then(|p| serde_json::from_str(&p).ok()))
            .collect();

        Ok(nodes)
    }

    fn row_to_edge(row: &rusqlite::Row<'_>) -> Result<LineageEdge> {
        let created_at_ts: i64 = row.get(4)?;
        Ok(LineageEdge {
            from_id: LineageId(row.get(0)?),
            to_id: LineageId(row.get(1)?),
            edge_type: EdgeType::parse(&row.get::<_, String>(2)?).unwrap_or(EdgeType::BasedOn),
            weight: row.get(3)?,
            created_at: DateTime::from_timestamp(created_at_ts, 0).unwrap_or_else(Utc::now),
        })
    }
}
