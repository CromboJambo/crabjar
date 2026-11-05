//! Habitat domain models — the spatial nouns from ADR-003.
//!
//! The habitat is a coarse-geometry model of the user's lived environment
//! over which computational state is laid out. Entities *occupy* positions;
//! their presence is state, not decoration.
use serde::{Deserialize, Serialize};

/// A coarse-geometry region of the lived environment (e.g. "desk", "kitchen").
///
/// Not a scan — just a named grid that entities are placed on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitatArea {
    pub id: i64,
    pub name: String,
    /// Grid width in cells (coarse geometry).
    pub grid_w: i64,
    /// Grid height in cells (coarse geometry).
    pub grid_h: i64,
}

/// The kind of computational state an entity represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    /// An agent, with a `working`/`blocked`/`idle` state.
    Agent,
    /// A produced artifact left behind.
    Artifact,
    /// A pending guard action awaiting review.
    PendingGuardAction,
    /// A suspended runtime (agents keep running, state persists).
    SuspendedRuntime,
    /// An unresolved decision occupying space.
    UnresolvedDecision,
}

impl EntityKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Artifact => "artifact",
            Self::PendingGuardAction => "pending_guard_action",
            Self::SuspendedRuntime => "suspended_runtime",
            Self::UnresolvedDecision => "unresolved_decision",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "agent" => Some(Self::Agent),
            "artifact" => Some(Self::Artifact),
            "pending_guard_action" => Some(Self::PendingGuardAction),
            "suspended_runtime" => Some(Self::SuspendedRuntime),
            "unresolved_decision" => Some(Self::UnresolvedDecision),
            _ => None,
        }
    }
}

/// A positioned entity — computational state laid out over the habitat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitatEntity {
    /// Stable entity id (e.g. a guard pending-entry id or agent id).
    pub id: String,
    pub area_id: i64,
    pub kind: EntityKind,
    /// State string — for agents: `working`/`blocked`/`idle`; else free-form.
    pub state: String,
    /// Human-readable label shown in the render.
    pub label: String,
    /// Coarse grid position within the area.
    pub x: i64,
    pub y: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// A divergence record — the physical environment and the spatial model
/// disagree. Exposed, never auto-corrected (ADR-003 decision 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitatDivergence {
    pub id: i64,
    pub area_id: i64,
    pub description: String,
    /// `open` or `resolved`.
    pub status: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

impl HabitatDivergence {
    pub fn is_open(&self) -> bool {
        self.status == "open"
    }
}

/// A full habitat snapshot: areas, entities, and divergences (open and
/// resolved — resolved records are kept; the model is append-only history).
///
/// This is the render-ready shape the TUI panel and CLI consume.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HabitatSnapshot {
    pub areas: Vec<HabitatArea>,
    pub entities: Vec<HabitatEntity>,
    pub divergences: Vec<HabitatDivergence>,
}

impl HabitatSnapshot {
    /// Entities occupying a given area.
    pub fn entities_in(&self, area_id: i64) -> Vec<&HabitatEntity> {
        self.entities
            .iter()
            .filter(|e| e.area_id == area_id)
            .collect()
    }

    /// Count of unresolved state (entities) — the "clutter" metric.
    pub fn clutter(&self) -> usize {
        self.entities.len()
    }

    /// Count of open divergences.
    pub fn open_divergences(&self) -> usize {
        self.divergences.iter().filter(|d| d.is_open()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_kind_roundtrips_through_str() {
        for kind in [
            EntityKind::Agent,
            EntityKind::Artifact,
            EntityKind::PendingGuardAction,
            EntityKind::SuspendedRuntime,
            EntityKind::UnresolvedDecision,
        ] {
            let s = kind.as_str();
            assert_eq!(EntityKind::parse(s), Some(kind));
        }
        assert_eq!(EntityKind::parse("nonsense"), None);
    }

    #[test]
    fn snapshot_clutter_counts_entities() {
        let snap = HabitatSnapshot {
            areas: vec![],
            entities: vec![
                HabitatEntity {
                    id: "a".into(),
                    area_id: 1,
                    kind: EntityKind::Agent,
                    state: "working".into(),
                    label: "agent".into(),
                    x: 0,
                    y: 0,
                    created_at: "".into(),
                    updated_at: "".into(),
                },
                HabitatEntity {
                    id: "b".into(),
                    area_id: 1,
                    kind: EntityKind::PendingGuardAction,
                    state: "pending".into(),
                    label: "guard".into(),
                    x: 1,
                    y: 0,
                    created_at: "".into(),
                    updated_at: "".into(),
                },
            ],
            divergences: vec![],
        };
        assert_eq!(snap.clutter(), 2);
        assert_eq!(snap.entities_in(1).len(), 2);
        assert_eq!(snap.entities_in(2).len(), 0);
    }

    #[test]
    fn divergence_open_status() {
        let d = HabitatDivergence {
            id: 1,
            area_id: 1,
            description: "desk diverged".into(),
            status: "open".into(),
            created_at: "".into(),
            resolved_at: None,
        };
        assert!(d.is_open());
    }
}
