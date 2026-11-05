//! The taste layer (spec §5) — parsed `behaviors.toml`.
//!
//! This is the "abstraction of the abstraction": a declarative assignment of
//! contract state to crab behavior. The renderer and the Phase-0 simulator
//! both consume a `Behaviors` value, so taste and code can never drift apart.
//! Unknown keys degrade gracefully (default behavior `wait`, default tint) —
//! a taste change never crashes the terrarium.

use serde::Deserialize;
use std::collections::HashMap;

/// Parsed `behaviors.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct Behaviors {
    #[serde(default)]
    pub schema_version: u32,
    /// effective_state → behavior name.
    #[serde(default)]
    pub behavior: HashMap<String, String>,
    /// evidence tier → certainty signal.
    #[serde(default)]
    pub evidence: HashMap<String, EvidenceSignal>,
    /// project id → habitat zone.
    #[serde(default)]
    pub zone: HashMap<String, Zone>,
    /// behavior → station marker (floor glyph + label).
    #[serde(default)]
    pub station: HashMap<String, Station>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EvidenceSignal {
    pub tint: String,
    #[serde(default = "default_posture")]
    pub posture: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Zone {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Station {
    #[serde(default = "default_glyph")]
    pub glyph: String,
    #[serde(default)]
    pub label: Option<String>,
}

fn default_posture() -> String {
    "neutral".to_string()
}
fn default_glyph() -> String {
    "·".to_string()
}

impl Default for Behaviors {
    fn default() -> Self {
        // The built-in taste. Used when no file is given, so the terrarium
        // always has a working mapping even with zero configuration.
        Self {
            schema_version: 1,
            behavior: [
                ("queued".into(), "wait".into()),
                ("running".into(), "work".into()),
                ("review".into(), "inspect".into()),
                ("done".into(), "deliver".into()),
                ("failed".into(), "dig".into()),
                ("blocked".into(), "stuck".into()),
                // Exhaust behaviors — AI agent token/cost signals
                ("burning".into(), "smoke".into()),
                ("spinning".into(), "flicker".into()),
                ("stalled".into(), "idle".into()),
                ("switching".into(), "glitch".into()),
                ("pressured".into(), "puff".into()),
            ]
            .into_iter()
            .collect(),
            evidence: [
                (
                    "verified".into(),
                    EvidenceSignal {
                        tint: "green".into(),
                        posture: "upright".into(),
                    },
                ),
                (
                    "reported".into(),
                    EvidenceSignal {
                        tint: "amber".into(),
                        posture: "neutral".into(),
                    },
                ),
                (
                    "heuristic".into(),
                    EvidenceSignal {
                        tint: "orange".into(),
                        posture: "hunched".into(),
                    },
                ),
                (
                    "asserted".into(),
                    EvidenceSignal {
                        tint: "gray".into(),
                        posture: "shell-up".into(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            zone: [
                (
                    "triage".into(),
                    Zone {
                        label: Some("Triage Burrow".into()),
                        x: 0,
                        y: 0,
                    },
                ),
                (
                    "guard".into(),
                    Zone {
                        label: Some("Guard Reef".into()),
                        x: 6,
                        y: 2,
                    },
                ),
                (
                    "theory".into(),
                    Zone {
                        label: Some("Theory Grotto".into()),
                        x: 12,
                        y: 4,
                    },
                ),
                (
                    "root".into(),
                    Zone {
                        label: Some("Open Floor".into()),
                        x: 0,
                        y: 8,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            station: HashMap::new(),
        }
    }
}

impl Behaviors {
    /// Load a taste file. Missing file → the built-in default (the terrarium
    /// must never blank because the user hasn't tuned their taste yet).
    pub fn load(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(raw) => toml::from_str(&raw).unwrap_or_else(|e| {
                eprintln!("warning: bad behaviors.toml ({e}); using built-in taste");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Map an effective state to a behavior. Unknown states → `wait` (never
    /// crash on a taste change, spec §5).
    pub fn behavior_for(&self, effective_state: &str) -> String {
        self.behavior
            .get(effective_state)
            .cloned()
            .unwrap_or_else(|| "wait".to_string())
    }

    /// Map an evidence tier to its certainty signal. Unknown tiers → `reported`.
    pub fn evidence_for(&self, tier: &str) -> EvidenceSignal {
        self.evidence
            .get(tier)
            .or_else(|| self.evidence.get("reported"))
            .cloned()
            .unwrap_or(EvidenceSignal {
                tint: "amber".to_string(),
                posture: "neutral".to_string(),
            })
    }

    /// Resolve a project id to its zone. Unmapped projects (and run-root tasks
    /// with no project) fall into the `root` zone, else a derived default.
    pub fn zone_for(&self, project: Option<&str>) -> (String, Zone) {
        let key = project.unwrap_or("root").to_string();
        if let Some(z) = self.zone.get(&key) {
            return (key, z.clone());
        }
        // Unmapped project: synthesize a zone from its id so it still renders.
        (
            key,
            Zone {
                label: None,
                x: 0,
                y: 0,
            },
        )
    }

    /// Station glyph for a behavior, with a derived default.
    pub fn station_glyph(&self, behavior: &str) -> String {
        self.station
            .get(behavior)
            .map(|s| s.glyph.clone())
            .unwrap_or_else(|| "·".to_string())
    }

    /// Cost tier tint based on cost rate (cents per minute).
    pub fn cost_tint_for(&self, cents_per_min: f64) -> String {
        if cents_per_min > 0.10 {
            "expensive".to_string()
        } else if cents_per_min > 0.01 {
            "moderate".to_string()
        } else {
            "cheap".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_maps_all_six_states() {
        let b = Behaviors::default();
        assert_eq!(b.behavior_for("queued"), "wait");
        assert_eq!(b.behavior_for("running"), "work");
        assert_eq!(b.behavior_for("review"), "inspect");
        assert_eq!(b.behavior_for("done"), "deliver");
        assert_eq!(b.behavior_for("failed"), "dig");
        assert_eq!(b.behavior_for("blocked"), "stuck");
    }

    #[test]
    fn default_maps_exhaust_states() {
        let b = Behaviors::default();
        assert_eq!(b.behavior_for("burning"), "smoke");
        assert_eq!(b.behavior_for("spinning"), "flicker");
        assert_eq!(b.behavior_for("stalled"), "idle");
        assert_eq!(b.behavior_for("switching"), "glitch");
        assert_eq!(b.behavior_for("pressured"), "puff");
    }

    #[test]
    fn unknown_state_falls_back_to_wait() {
        let b = Behaviors::default();
        assert_eq!(b.behavior_for("nonsense"), "wait");
    }

    #[test]
    fn unknown_evidence_falls_back_to_reported() {
        let b = Behaviors::default();
        let sig = b.evidence_for("bogus");
        assert_eq!(sig.tint, "amber");
    }

    #[test]
    fn unmapped_project_falls_to_root() {
        let b = Behaviors::default();
        let (key, zone) = b.zone_for(None);
        assert_eq!(key, "root");
        assert!(zone.label.is_some());
    }

    #[test]
    fn taste_file_overrides_behavior() {
        let toml = r#"
            schema_version = 1
            [behavior]
            failed = "sew"
        "#;
        let b: Behaviors = toml::from_str(toml).unwrap();
        // Overridden entry wins…
        assert_eq!(b.behavior_for("failed"), "sew");
        // …but an entry absent from the file is simply missing (→ wait),
        // because a partial taste file is a partial override, not a merge.
        assert_eq!(b.behavior_for("done"), "wait");
    }

    #[test]
    fn missing_file_yields_default() {
        let b = Behaviors::load(std::path::Path::new("/nonexistent/behaviors.toml"));
        assert_eq!(b.behavior_for("failed"), "dig");
    }

    #[test]
    fn cost_tint_tiers() {
        let b = Behaviors::default();
        assert_eq!(b.cost_tint_for(0.005), "cheap");
        assert_eq!(b.cost_tint_for(0.05), "moderate");
        assert_eq!(b.cost_tint_for(0.25), "expensive");
    }
}
