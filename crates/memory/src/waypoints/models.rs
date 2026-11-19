use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a waypoint.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WaypointId(pub String);

impl WaypointId {
    /// Generate a new unique waypoint ID.
    pub fn generate() -> Self {
        Self(format!("wp-{}", Uuid::new_v4().simple()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WaypointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Reference to an artifact or document associated with a waypoint.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(pub String);

impl ArtifactId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Reference to a transformation (fold) applied to produce a compressed waypoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoldRef(pub String);

impl FoldRef {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Unique identifier for an environment (repo, project, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EnvironmentId(pub String);

impl EnvironmentId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EnvironmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Category of a waypoint — what kind of navigational information it provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointCategory {
    /// Understand where you are in the environment.
    Orientation,
    /// Understand how components relate to each other.
    Structural,
    /// Understand how you got here (history, ADRs).
    Historical,
    /// Understand current conditions and state.
    Operational,
    /// Understand the current objective.
    Task,
    /// Find last known-good state for recovery.
    Recovery,
}

impl WaypointCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Orientation => "orientation",
            Self::Structural => "structural",
            Self::Historical => "historical",
            Self::Operational => "operational",
            Self::Task => "task",
            Self::Recovery => "recovery",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "orientation" => Some(Self::Orientation),
            "structural" => Some(Self::Structural),
            "historical" => Some(Self::Historical),
            "operational" => Some(Self::Operational),
            "task" => Some(Self::Task),
            "recovery" => Some(Self::Recovery),
            _ => None,
        }
    }
}

impl fmt::Display for WaypointCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Recoverability of a (possibly compressed) waypoint — can we get back to source?
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recoverability {
    /// Full source is accessible.
    Lossless,
    /// Can traverse N levels back through predecessor links.
    Bounded { max_depth: u32 },
    /// Has pointers but not full content.
    ReferenceOnly,
    /// Don't know yet — a valid state.
    #[default]
    Unknown,
}

impl fmt::Display for Recoverability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lossless => write!(f, "lossless"),
            Self::Bounded { max_depth } => write!(f, "bounded({})", max_depth),
            Self::ReferenceOnly => write!(f, "reference_only"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl Recoverability {
    pub fn parse(s: &str) -> Option<Self> {
        if s == "lossless" {
            Some(Self::Lossless)
        } else if let Some(depth) = s.strip_prefix("bounded(").and_then(|s| s.strip_suffix(')')) {
            depth
                .parse::<u32>()
                .ok()
                .map(|d| Recoverability::Bounded { max_depth: d })
        } else if s == "reference_only" {
            Some(Self::ReferenceOnly)
        } else if s == "unknown" {
            Some(Self::Unknown)
        } else {
            None
        }
    }
}

/// Compression depth policy for waypoint traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionDepthPolicy {
    /// 1 fold — status checks, simple queries.
    QuickContext,
    /// 3 folds — normal task execution.
    WorkingContext,
    /// 6 folds — debugging, root cause analysis.
    DeepInvestigation,
    /// Unbounded — audits, compliance.
    FullProvenance,
}

impl CompressionDepthPolicy {
    pub fn max_depth(&self) -> Option<u32> {
        match self {
            Self::QuickContext => Some(1),
            Self::WorkingContext => Some(3),
            Self::DeepInvestigation => Some(6),
            Self::FullProvenance => None,
        }
    }
}

/// A durable navigational landmark through the information environment.
///
/// Not an instruction, not a permission, not documentation — a pointer with context.
/// Workers traverse waypoints progressively rather than receiving the entire environment at once.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waypoint {
    pub id: WaypointId,
    /// Parent waypoint for hierarchy traversal. None for root waypoints.
    pub parent_id: Option<WaypointId>,
    /// Environment this waypoint belongs to.
    pub environment: EnvironmentId,
    /// What kind of navigational information this provides.
    pub category: WaypointCategory,
    /// What this waypoint tells you.
    pub description: String,
    /// Where to go next from this waypoint.
    pub destinations: Vec<WaypointId>,
    /// Related artifacts/docs referenced by this waypoint.
    pub references: Vec<ArtifactId>,
    /// How important is this? 0.0 = irrelevant, 1.0 = critical.
    pub relevance: f32,
    /// When was this last verified as accurate?
    pub freshness: DateTime<Utc>,
    /// Which waypoint produced this one (for compressed waypoints).
    pub predecessor: Option<WaypointId>,
    /// What transformation was applied to produce this from its predecessor.
    pub transformation: Option<FoldRef>,
    /// Can we get back to source?
    pub recoverability: Recoverability,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Waypoint {
    /// Create a new waypoint with sensible defaults.
    pub fn new(
        environment: EnvironmentId,
        category: WaypointCategory,
        description: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: WaypointId::generate(),
            parent_id: None,
            environment,
            category,
            description: description.into(),
            destinations: Vec::new(),
            references: Vec::new(),
            relevance: 0.5,
            freshness: now,
            predecessor: None,
            transformation: None,
            recoverability: Recoverability::Unknown,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the parent of this waypoint.
    pub fn with_parent(mut self, parent: WaypointId) -> Self {
        self.parent_id = Some(parent);
        self
    }

    /// Add a destination (where to go next).
    pub fn with_destination(mut self, dest: WaypointId) -> Self {
        self.destinations.push(dest);
        self
    }

    /// Add a reference artifact.
    pub fn with_reference(mut self, ref_id: ArtifactId) -> Self {
        self.references.push(ref_id);
        self
    }

    /// Set relevance score (0.0–1.0).
    pub fn with_relevance(mut self, relevance: f32) -> Self {
        self.relevance = relevance.clamp(0.0, 1.0);
        self
    }

    /// Check if this waypoint is stale relative to a threshold.
    pub fn is_stale(&self, threshold_hours: i64) -> bool {
        Utc::now().signed_duration_since(self.freshness).num_hours() > threshold_hours
    }

    /// Check if this waypoint respects the given compression depth policy.
    pub fn respects_depth_policy(&self, policy: CompressionDepthPolicy) -> bool {
        let max_depth = match policy.max_depth() {
            Some(d) => d as i64,
            None => return true, // FullProvenance allows anything
        };
        self.ancestry_depth() <= max_depth
    }

    /// Compute ancestry depth by counting predecessor links (requires external lookup).
    /// This is a placeholder — actual implementation needs DB access.
    pub fn ancestry_depth(&self) -> i64 {
        // Without DB access, we can only report direct predecessor existence.
        if self.predecessor.is_some() { 1 } else { 0 }
    }
}

/// Result of a freshness verification check on a waypoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessCheck {
    pub waypoint_id: WaypointId,
    pub is_fresh: bool,
    pub age_hours: i64,
    pub threshold_hours: i64,
    pub message: String,
}

/// Traversal result — a path of waypoints from start to current position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalPath {
    pub waypoints: Vec<Waypoint>,
    pub total_hops: usize,
    pub reached_target: bool,
}

impl TraversalPath {
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }
}
