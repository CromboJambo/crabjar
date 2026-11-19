//! Observer state data model.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::lineage::models::LineageId;
use crate::waypoints::models::WaypointId;

/// Reference to an environment the observer is exploring.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EnvironmentId(pub String);

impl EnvironmentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for EnvironmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Reference to a finding produced by an observer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FindingId(pub String);

impl FindingId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for FindingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Category of finding produced by an observer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FindingCategory {
    /// "These two crates have a circular dependency"
    StructuralInsight,
    /// "Guard is blocking 5 actions that look legitimate"
    OperationalConcern,
    /// "We could cache these lookups to save 20% time"
    ImprovementIdea,
    /// "This metric is outside normal range"
    AnomalyDetected,
    /// "There's no documentation for this module"
    KnowledgeGap,
}

impl fmt::Display for FindingCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FindingCategory::StructuralInsight => "structural-insight",
            FindingCategory::OperationalConcern => "operational-concern",
            FindingCategory::ImprovementIdea => "improvement-idea",
            FindingCategory::AnomalyDetected => "anomaly-detected",
            FindingCategory::KnowledgeGap => "knowledge-gap",
        };
        write!(f, "{}", s)
    }
}

/// Evidence supporting a finding — waypoints or artifacts the observer examined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub kind: EvidenceKind,
    pub id: String,
    pub description: Option<String>,
}

impl EvidenceRef {
    pub fn waypoint(id: WaypointId, description: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::Waypoint,
            id: id.0,
            description: Some(description.into()),
        }
    }

    pub fn artifact(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::Artifact,
            id: id.into(),
            description: Some(description.into()),
        }
    }

    pub fn lineage_node(id: LineageId) -> Self {
        Self {
            kind: EvidenceKind::LineageNode,
            id: id.0,
            description: None,
        }
    }
}

/// Type of evidence reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceKind {
    Waypoint,
    Artifact,
    LineageNode,
}

impl fmt::Display for EvidenceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EvidenceKind::Waypoint => "waypoint",
            EvidenceKind::Artifact => "artifact",
            EvidenceKind::LineageNode => "lineage-node",
        };
        write!(f, "{}", s)
    }
}

/// A structured finding produced by an observer (not yet a proposal).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindingRef {
    pub id: FindingId,
    pub category: FindingCategory,
    pub description: String,
    /// How sure is the observer about this? 0.0 to 1.0.
    pub confidence: f32,
    /// Waypoints, artifacts that support this finding.
    pub evidence: Vec<EvidenceRef>,
    pub created_at: i64,
}

impl FindingRef {
    pub fn new(
        id: impl Into<String>,
        category: FindingCategory,
        description: impl Into<String>,
        confidence: f32,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            id: FindingId::new(id),
            category,
            description: description.into(),
            confidence: confidence.clamp(0.0, 1.0),
            evidence: Vec::new(),
            created_at: now,
        }
    }

    pub fn with_evidence(mut self, evidence: EvidenceRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Check if this finding is below the speculative threshold.
    pub fn is_speculative(&self, threshold: f32) -> bool {
        self.confidence < threshold
    }
}

/// State of an observer-state worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObserverState {
    pub worker_id: String,
    pub environment: EnvironmentId,
    pub started_at: i64,

    /// Which waypoints the observer is exploring.
    pub focus_areas: Vec<WaypointId>,
    /// What the observer is trying to understand.
    pub questions: Vec<String>,

    /// Observations recorded in the lineage graph.
    pub observations: Vec<LineageId>,
    /// Proposals submitted for supervisor review.
    pub proposals: Vec<LineageId>,
    /// Structured findings (not yet proposals).
    pub findings: Vec<FindingRef>,

    /// How deep can the observer go in waypoints?
    pub max_traversal_depth: u32,
    /// When does the observer need to report back?
    pub max_findings_before_review: usize,
}

impl ObserverState {
    pub fn new(worker_id: impl Into<String>, environment: EnvironmentId) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            worker_id: worker_id.into(),
            environment,
            started_at: now,
            focus_areas: Vec::new(),
            questions: Vec::new(),
            observations: Vec::new(),
            proposals: Vec::new(),
            findings: Vec::new(),
            max_traversal_depth: 3,
            max_findings_before_review: 5,
        }
    }

    pub fn with_focus(mut self, waypoint: WaypointId) -> Self {
        self.focus_areas.push(waypoint);
        self
    }

    pub fn with_question(mut self, question: impl Into<String>) -> Self {
        self.questions.push(question.into());
        self
    }

    /// Add a finding and check if review is needed.
    pub fn add_finding(&mut self, finding: FindingRef) -> bool {
        self.findings.push(finding);
        self.needs_review()
    }

    /// Check if the observer has reached its review threshold.
    pub fn needs_review(&self) -> bool {
        self.findings.len() >= self.max_findings_before_review
    }

    /// Duration in seconds since observation started.
    pub fn duration_secs(&self, now: i64) -> i64 {
        now.saturating_sub(self.started_at)
    }

    /// Check if the observer has exceeded its maximum duration.
    pub fn is_expired(&self, now: i64, max_duration_secs: i64) -> bool {
        self.duration_secs(now) > max_duration_secs
    }
}
