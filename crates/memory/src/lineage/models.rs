use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a lineage node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineageId(pub String);

impl LineageId {
    /// Generate a new unique lineage ID.
    pub fn generate() -> Self {
        Self(format!("ln-{}", Uuid::new_v4().simple()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LineageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Reference to the source that created a lineage node (worker, sensor, human, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceRef {
    /// Type of source: "worker", "human", "sensor", "policy_engine", etc.
    pub kind: String,
    /// Identifier for the specific source instance.
    pub id: String,
    /// Optional role or capability description (for matching equivalent workers).
    pub role: Option<String>,
}

impl SourceRef {
    pub fn worker(id: impl Into<String>) -> Self {
        Self {
            kind: "worker".into(),
            id: id.into(),
            role: None,
        }
    }

    pub fn human(id: impl Into<String>) -> Self {
        Self {
            kind: "human".into(),
            id: id.into(),
            role: None,
        }
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }
}

impl fmt::Display for SourceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.role {
            Some(role) => write!(f, "{}:{} ({})", self.kind, self.id, role),
            None => write!(f, "{}:{}", self.kind, self.id),
        }
    }
}

/// Status of a proposal in the lineage graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Pending,
    UnderReview,
    Accepted,
    Rejected { reason: String },
    Superseded { by: LineageId },
}

impl fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::UnderReview => write!(f, "under_review"),
            Self::Accepted => write!(f, "accepted"),
            Self::Rejected { reason } => write!(f, "rejected: {}", reason),
            Self::Superseded { by } => write!(f, "superseded_by: {}", by),
        }
    }
}

/// Outcome of a decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionOutcome {
    Approved,
    Rejected { reason: String },
    Modified { changes: String },
}

impl fmt::Display for DecisionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Approved => write!(f, "approved"),
            Self::Rejected { reason } => write!(f, "rejected: {}", reason),
            Self::Modified { changes } => write!(f, "modified: {}", changes),
        }
    }
}

/// Type of edge connecting lineage nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    /// A proposal is based on one or more observations.
    BasedOn,
    /// A decision responds to a proposal.
    Proposes,
    /// An implementation realizes a decision.
    Decides,
    /// A recognition acknowledges an implementation's origin.
    Implements,
    /// Alternative edge type for recognition links.
    Recognizes,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BasedOn => "based_on",
            Self::Proposes => "proposes",
            Self::Decides => "decides",
            Self::Implements => "implements",
            Self::Recognizes => "recognizes",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "based_on" => Some(Self::BasedOn),
            "proposes" => Some(Self::Proposes),
            "decides" => Some(Self::Decides),
            "implements" => Some(Self::Implements),
            "recognizes" => Some(Self::Recognizes),
            _ => None,
        }
    }
}

impl fmt::Display for EdgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A node in the lineage graph. Append-only: once created, never modified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineageNode {
    Observation {
        id: LineageId,
        source: SourceRef,
        timestamp: DateTime<Utc>,
        description: String,
        /// Optional waypoint where this was observed in the environment.
        context: Option<String>,
    },
    Proposal {
        id: LineageId,
        based_on: Vec<LineageId>,
        proposer: SourceRef,
        timestamp: DateTime<Utc>,
        description: String,
        status: ProposalStatus,
    },
    Decision {
        id: LineageId,
        proposal_id: LineageId,
        decision_maker: SourceRef,
        timestamp: DateTime<Utc>,
        outcome: DecisionOutcome,
        rationale: String,
    },
    Implementation {
        id: LineageId,
        decision_id: LineageId,
        implementer: SourceRef,
        timestamp: DateTime<Utc>,
        artifacts: Vec<String>,
    },
    Recognition {
        id: LineageId,
        implementation_id: LineageId,
        recognized_by: SourceRef,
        timestamp: DateTime<Utc>,
        note: String,
    },
}

impl LineageNode {
    pub fn id(&self) -> &LineageId {
        match self {
            Self::Observation { id, .. }
            | Self::Proposal { id, .. }
            | Self::Decision { id, .. }
            | Self::Implementation { id, .. }
            | Self::Recognition { id, .. } => id,
        }
    }

    pub fn source(&self) -> &SourceRef {
        match self {
            Self::Observation { source, .. } => source,
            Self::Proposal { proposer, .. } => proposer,
            Self::Decision { decision_maker, .. } => decision_maker,
            Self::Implementation { implementer, .. } => implementer,
            Self::Recognition { recognized_by, .. } => recognized_by,
        }
    }

    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            Self::Observation { timestamp, .. }
            | Self::Proposal { timestamp, .. }
            | Self::Decision { timestamp, .. }
            | Self::Implementation { timestamp, .. }
            | Self::Recognition { timestamp, .. } => timestamp,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Observation { description, .. } | Self::Proposal { description, .. } => {
                description.clone()
            }
            Self::Decision { rationale, .. } => rationale.clone(),
            Self::Implementation { artifacts, .. } => artifacts.join(", "),
            Self::Recognition { note, .. } => note.clone(),
        }
    }

    pub fn node_type(&self) -> &'static str {
        match self {
            Self::Observation { .. } => "observation",
            Self::Proposal { .. } => "proposal",
            Self::Decision { .. } => "decision",
            Self::Implementation { .. } => "implementation",
            Self::Recognition { .. } => "recognition",
        }
    }
}

/// An edge connecting two lineage nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    pub from_id: LineageId,
    pub to_id: LineageId,
    pub edge_type: EdgeType,
    /// Strength of connection for relevance scoring.
    pub weight: f64,
    pub created_at: DateTime<Utc>,
}

/// Callback payload when an implementation is recognized back to its origin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionCallback {
    pub original_proposal_id: LineageId,
    pub implementation_id: LineageId,
    pub original_source: SourceRef,
    /// Workers who can receive this recognition (same role/capabilities).
    pub equivalent_workers: Vec<SourceRef>,
    pub message: String,
}
