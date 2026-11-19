use std::collections::{HashSet, VecDeque};

use rusqlite::Result;

use super::models::{LineageId, LineageNode, SourceRef};
use super::store::LineageStore;

/// Query interface for tracing provenance through the lineage graph.
pub trait ProvenanceQuery {
    /// Trace forward: what happened after this node? (BFS along outgoing edges)
    fn trace_forward(&self, id: &LineageId) -> Result<Vec<LineageNode>>;

    /// Trace backward: what led to this node? (BFS along incoming edges)
    fn trace_backward(&self, id: &LineageId) -> Result<Vec<LineageNode>>;

    /// Find all proposals based on observations from a specific source.
    fn proposals_from_source(&self, source: &SourceRef) -> Result<Vec<LineageNode>>;

    /// Find implementations that resulted from a worker's proposals.
    fn implementations_from_worker(&self, worker: &SourceRef) -> Result<Vec<LineageNode>>;

    /// Check if a specific rule/policy has a traceable origin in the lineage graph.
    fn rule_origin(&self, rule_id: &str) -> Result<Option<LineageNode>>;
}

/// SQLite-backed implementation of ProvenanceQuery.
pub struct SqlProvenanceQuery {
    pub store: LineageStore,
}

impl SqlProvenanceQuery {
    pub fn new(store: LineageStore) -> Self {
        Self { store }
    }

    /// BFS traversal from a starting node along outgoing edges.
    fn bfs_forward(&self, start: &LineageId) -> Result<Vec<LineageNode>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut results = Vec::new();

        visited.insert(start.as_str().to_string());
        queue.push_back(start.clone());

        while let Some(current) = queue.pop_front() {
            for edge in self.store.outgoing_edges(&current)? {
                if !visited.contains(edge.to_id.as_str()) {
                    visited.insert(edge.to_id.as_str().to_string());
                    if let Some(node) = self.store.get_node(&edge.to_id)? {
                        results.push(node);
                        queue.push_back(edge.to_id);
                    }
                }
            }
        }

        Ok(results)
    }

    /// BFS traversal from a starting node along incoming edges.
    fn bfs_backward(&self, start: &LineageId) -> Result<Vec<LineageNode>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut results = Vec::new();

        visited.insert(start.as_str().to_string());
        queue.push_back(start.clone());

        while let Some(current) = queue.pop_front() {
            for edge in self.store.incoming_edges(&current)? {
                if !visited.contains(edge.from_id.as_str()) {
                    visited.insert(edge.from_id.as_str().to_string());
                    if let Some(node) = self.store.get_node(&edge.from_id)? {
                        results.push(node);
                        queue.push_back(edge.from_id);
                    }
                }
            }
        }

        Ok(results)
    }
}

impl ProvenanceQuery for SqlProvenanceQuery {
    fn trace_forward(&self, id: &LineageId) -> Result<Vec<LineageNode>> {
        self.bfs_forward(id)
    }

    fn trace_backward(&self, id: &LineageId) -> Result<Vec<LineageNode>> {
        self.bfs_backward(id)
    }

    fn proposals_from_source(&self, source: &SourceRef) -> Result<Vec<LineageNode>> {
        self.store.proposals_from_source(source)
    }

    fn implementations_from_worker(&self, worker: &SourceRef) -> Result<Vec<LineageNode>> {
        self.store.implementations_from_worker(worker)
    }

    fn rule_origin(&self, rule_id: &str) -> Result<Option<LineageNode>> {
        // Search for a decision or implementation node whose description mentions the rule ID
        let mut stmt = self.store.conn().prepare(
            "SELECT payload FROM lineage_nodes
             WHERE node_type IN ('decision', 'implementation')
               AND (description LIKE ? OR payload LIKE ?)
             ORDER BY timestamp DESC LIMIT 1",
        )?;

        let pattern = format!("%{}%", rule_id);
        let mut rows = stmt.query_map(rusqlite::params![pattern, pattern], |row| {
            row.get::<_, String>(0)
        })?;

        match rows.next() {
            Some(Ok(payload)) => Ok(serde_json::from_str(&payload).ok()),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }
}

/// Build a recognition callback when an implementation is created.
pub struct RecognitionBuilder {
    pub query: SqlProvenanceQuery,
}

impl RecognitionBuilder {
    pub fn new(query: SqlProvenanceQuery) -> Self {
        Self { query }
    }

    /// Given an implementation ID, trace back to find the original proposer
    /// and build a recognition callback.
    pub fn build_callback(
        &self,
        implementation_id: &LineageId,
    ) -> Result<Option<super::models::RecognitionCallback>> {
        // Trace backward from implementation to find the decision
        let ancestors = self.query.trace_backward(implementation_id)?;

        // Find the decision node
        let decision = ancestors.iter().find(|n| n.node_type() == "decision");
        let Some(decision) = decision else {
            return Ok(None);
        };

        // Trace backward from decision to find the proposal
        let decision_ancestors = self.query.trace_backward(decision.id())?;
        let proposal = decision_ancestors
            .iter()
            .find(|n| n.node_type() == "proposal");
        let Some(proposal) = proposal else {
            return Ok(None);
        };

        // Extract proposer from the proposal
        let original_source = proposal.source().clone();

        // Build the callback
        Ok(Some(super::models::RecognitionCallback {
            original_proposal_id: proposal.id().clone(),
            implementation_id: implementation_id.clone(),
            original_source: original_source.clone(),
            equivalent_workers: vec![original_source],
            message: format!(
                "Your observation led to policy implementation {}",
                implementation_id
            ),
        }))
    }
}
