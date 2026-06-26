//! Memory graph types for the guard system.
//!
//! Types: `NodeKind`, `MemoryNode`, `EdgeRelation`, `MemoryEdge`.
//!
//! Note: This module contains *type definitions* for the memory graph.
//! The in-memory graph implementation is in the `memory` module (test-only).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Kind of memory node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Fact,
    Pattern,
    Rule,
    Reflection,
    Outcome,
    Residue,
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeKind::Fact => write!(f, "fact"),
            NodeKind::Pattern => write!(f, "pattern"),
            NodeKind::Rule => write!(f, "rule"),
            NodeKind::Reflection => write!(f, "reflection"),
            NodeKind::Outcome => write!(f, "outcome"),
            NodeKind::Residue => write!(f, "residue"),
        }
    }
}

/// A node in the memory graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: String,
    pub kind: NodeKind,
    pub content: String,
    pub trust_layer: u32,
    pub confidence: super::TrustScore,
    pub created_at: i64,
    pub last_touched: i64,
    pub anneal_count: u32,
    pub metadata: Option<String>,
}

/// Relationship between memory nodes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRelation {
    Supports,
    Contradicts,
    DerivedFrom,
    Anneals,
    DependsOn,
    EvidenceFor,
}

impl fmt::Display for EdgeRelation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EdgeRelation::Supports => write!(f, "supports"),
            EdgeRelation::Contradicts => write!(f, "contradicts"),
            EdgeRelation::DerivedFrom => write!(f, "derived_from"),
            EdgeRelation::Anneals => write!(f, "anneals"),
            EdgeRelation::DependsOn => write!(f, "depends_on"),
            EdgeRelation::EvidenceFor => write!(f, "evidence_for"),
        }
    }
}

/// A directed, weighted edge between memory nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEdge {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub relation: EdgeRelation,
    pub weight: f64,
    pub created_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TrustScore;

    #[test]
    fn node_kind_display_fact() {
        assert_eq!(format!("{}", NodeKind::Fact), "fact");
    }

    #[test]
    fn node_kind_display_pattern() {
        assert_eq!(format!("{}", NodeKind::Pattern), "pattern");
    }

    #[test]
    fn node_kind_display_rule() {
        assert_eq!(format!("{}", NodeKind::Rule), "rule");
    }

    #[test]
    fn node_kind_display_reflection() {
        assert_eq!(format!("{}", NodeKind::Reflection), "reflection");
    }

    #[test]
    fn node_kind_display_outcome() {
        assert_eq!(format!("{}", NodeKind::Outcome), "outcome");
    }

    #[test]
    fn node_kind_display_residue() {
        assert_eq!(format!("{}", NodeKind::Residue), "residue");
    }

    #[test]
    fn edge_relation_display_supports() {
        assert_eq!(format!("{}", EdgeRelation::Supports), "supports");
    }

    #[test]
    fn edge_relation_display_contradicts() {
        assert_eq!(format!("{}", EdgeRelation::Contradicts), "contradicts");
    }

    #[test]
    fn edge_relation_display_derived_from() {
        assert_eq!(format!("{}", EdgeRelation::DerivedFrom), "derived_from");
    }

    #[test]
    fn edge_relation_display_anneals() {
        assert_eq!(format!("{}", EdgeRelation::Anneals), "anneals");
    }

    #[test]
    fn edge_relation_display_depends_on() {
        assert_eq!(format!("{}", EdgeRelation::DependsOn), "depends_on");
    }

    #[test]
    fn edge_relation_display_evidence_for() {
        assert_eq!(format!("{}", EdgeRelation::EvidenceFor), "evidence_for");
    }

    #[test]
    fn node_kind_equality() {
        assert_eq!(NodeKind::Fact, NodeKind::Fact);
        assert_ne!(NodeKind::Fact, NodeKind::Rule);
    }

    #[test]
    fn edge_relation_equality() {
        assert_eq!(EdgeRelation::Supports, EdgeRelation::Supports);
        assert_ne!(EdgeRelation::Supports, EdgeRelation::Contradicts);
    }

    #[test]
    fn memory_node_clone() {
        let node = MemoryNode {
            id: "test".to_string(),
            kind: NodeKind::Fact,
            content: "content".to_string(),
            trust_layer: 2,
            confidence: TrustScore::new(0.5),
            created_at: 0,
            last_touched: 0,
            anneal_count: 0,
            metadata: None,
        };
        let cloned = node.clone();
        assert_eq!(node.id, cloned.id);
        assert_eq!(node.kind, cloned.kind);
        assert_eq!(node.content, cloned.content);
    }

    #[test]
    fn memory_edge_clone() {
        let edge = MemoryEdge {
            id: "e1".to_string(),
            from_id: "n1".to_string(),
            to_id: "n2".to_string(),
            relation: EdgeRelation::Supports,
            weight: 1.0,
            created_at: 0,
        };
        let cloned = edge.clone();
        assert_eq!(edge.id, cloned.id);
        assert_eq!(edge.relation, cloned.relation);
    }
}
