use crate::GateResult;
use crate::guard_db::GuardDb;
use crate::types::TrustScore;
use tracing::{debug, warn};

/// Bounded set of reachable worst-case perturbations for an action.
///
/// Replaces single-point reversibility score with a set of perturbations
/// that a removal/execution could produce. Each perturbation has a severity
/// and an undo path. The bounded set captures all reachable states, not
/// a single worst-case point.
#[derive(Clone)]
pub struct PerturbationSet {
    perturbations: Vec<Perturbation>,
    bound: f64,
}

impl PerturbationSet {
    pub fn new(perturbations: Vec<Perturbation>, bound: f64) -> Self {
        Self {
            perturbations,
            bound: bound.clamp(0.0, 1.0),
        }
    }

    pub fn perturbations(&self) -> &[Perturbation] {
        &self.perturbations
    }

    pub fn bound(&self) -> f64 {
        self.bound
    }

    /// Compute the bounded set of perturbations for an action.
    pub fn compute_perturbations(
        _command: &str,
        _args: &[String],
        undo_paths: Vec<String>,
        checksum_targets: Vec<String>,
        checkpoint_targets: Vec<String>,
        flight_recorder_targets: Vec<String>,
        data_integrity_targets: Vec<String>,
    ) -> Self {
        let mut perturbations = Vec::new();

        for path in undo_paths {
            perturbations.push(Perturbation {
                kind: PerturbationKind::UndoPath,
                severity: 0.25,
                description: path,
                mitigable: true,
            });
        }

        for target in checksum_targets {
            perturbations.push(Perturbation {
                kind: PerturbationKind::ChecksumTarget,
                severity: 0.20,
                description: target,
                mitigable: true,
            });
        }

        for target in checkpoint_targets {
            perturbations.push(Perturbation {
                kind: PerturbationKind::CheckpointTarget,
                severity: 0.20,
                description: target,
                mitigable: true,
            });
        }

        for target in flight_recorder_targets {
            perturbations.push(Perturbation {
                kind: PerturbationKind::FlightRecorderTarget,
                severity: 0.15,
                description: target,
                mitigable: true,
            });
        }

        for target in data_integrity_targets {
            perturbations.push(Perturbation {
                kind: PerturbationKind::DataIntegrityTarget,
                severity: 0.20,
                description: target,
                mitigable: true,
            });
        }

        if perturbations.is_empty() {
            perturbations.push(Perturbation {
                kind: PerturbationKind::NoUndoPath,
                severity: 1.0,
                description: "No undo paths detected".to_string(),
                mitigable: false,
            });
        }

        Self::new(
            perturbations.clone(),
            bound_from_perturbations(&perturbations),
        )
    }

    /// Determine whether any perturbation is unmitigable.
    pub fn has_unmitigable(&self) -> bool {
        self.perturbations.iter().any(|p| !p.mitigable)
    }

    /// Determine the maximum severity across the bounded set.
    pub fn max_severity(&self) -> f64 {
        self.perturbations
            .iter()
            .max_by(|a, b| {
                a.severity
                    .partial_cmp(&b.severity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| p.severity)
            .unwrap_or(0.0)
    }

    /// Determine the number of mitigable perturbations.
    pub fn mitigable_count(&self) -> usize {
        self.perturbations.iter().filter(|p| p.mitigable).count()
    }

    /// Determine the number of unmitigable perturbations.
    pub fn unmitigable_count(&self) -> usize {
        self.perturbations.iter().filter(|p| !p.mitigable).count()
    }
}

fn bound_from_perturbations(perturbations: &[Perturbation]) -> f64 {
    let unmitigable = perturbations.iter().filter(|p| !p.mitigable).count();
    let mitigable = perturbations.iter().filter(|p| p.mitigable).count();
    if unmitigable == 0 {
        1.0
    } else if mitigable == 0 {
        0.0
    } else {
        mitigable as f64 / (mitigable + unmitigable) as f64
    }
}

/// A perturbation in the bounded set of reachable worst-case states.
#[derive(Debug, Clone, PartialEq)]
pub struct Perturbation {
    kind: PerturbationKind,
    severity: f64,
    description: String,
    mitigable: bool,
}

/// The kind of perturbation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerturbationKind {
    UndoPath,
    ChecksumTarget,
    CheckpointTarget,
    FlightRecorderTarget,
    DataIntegrityTarget,
    NoUndoPath,
    NoChecksums,
    NoCheckpoint,
    NoFlightRecorder,
    DataCorruption,
}

/// Action risk assessment that combines perturbation set with confidence decay.
///
/// Risk factors:
/// - bounded perturbation set
/// - confidence decay of the command
/// - uncertainty exposure (below threshold → surface before executing)
/// - interruptibility (allow gate to return Interrupted)
/// - additional risk factors established through testing and iteration
#[allow(dead_code)]
pub struct ActionRiskAssessment {
    perturbations: PerturbationSet,
    confidence: TrustScore,
    uncertainty_exposed: bool,
    interruptible: bool,
    risk_level: CommandRiskExtended,
}

impl ActionRiskAssessment {
    pub fn new(
        perturbations: PerturbationSet,
        confidence: TrustScore,
        uncertainty_exposed: bool,
        interruptible: bool,
    ) -> Self {
        let risk_level = Self::determine_risk_level(
            &perturbations,
            &confidence,
            uncertainty_exposed,
            interruptible,
        );

        Self {
            perturbations,
            confidence,
            uncertainty_exposed,
            interruptible,
            risk_level,
        }
    }

    /// Determine risk level based on bounded perturbation set.
    fn determine_risk_level(
        perturbations: &PerturbationSet,
        confidence: &TrustScore,
        uncertainty_exposed: bool,
        interruptible: bool,
    ) -> CommandRiskExtended {
        let max_sev = perturbations.max_severity();
        let unmitigable = perturbations.unmitigable_count();
        let conf_score = confidence.get();

        if unmitigable > 0 && max_sev > 0.8 && conf_score < 0.5 {
            CommandRiskExtended::Critical
        } else if unmitigable > 0 && max_sev > 0.5 && conf_score < 0.6 {
            CommandRiskExtended::High
        } else if conf_score < 0.6 || !uncertainty_exposed || !interruptible {
            CommandRiskExtended::Medium
        } else {
            CommandRiskExtended::Low
        }
    }

    pub fn get_risk_level(&self) -> CommandRiskExtended {
        self.risk_level
    }

    pub fn requires_permission(&self) -> bool {
        matches!(
            self.risk_level,
            CommandRiskExtended::Critical | CommandRiskExtended::High
        )
    }

    pub fn confidence_below_floor(&self, floor: f64) -> bool {
        self.confidence.get() < floor
    }

    pub fn perturbations(&self) -> &[Perturbation] {
        self.perturbations.perturbations()
    }

    pub fn bound(&self) -> f64 {
        self.perturbations.bound()
    }

    pub fn has_unmitigable(&self) -> bool {
        self.perturbations.has_unmitigable()
    }
}

/// Extended command risk levels incorporating reversibility scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRiskExtended {
    Low,
    Medium,
    High,
    Critical,
    Unauthorized,
}

/// Gate check with bounded perturbation set.
#[allow(clippy::too_many_arguments)]
pub fn gate_check_with_reversibility(
    db: &GuardDb,
    command: &str,
    args: &[String],
    confidence: TrustScore,
    trust_layer: u32,
    source_event_id: Option<String>,
    can_interrupt: bool,
    undo_paths: Vec<String>,
    checksum_targets: Vec<String>,
    checkpoint_targets: Vec<String>,
    flight_recorder_targets: Vec<String>,
    data_integrity_targets: Vec<String>,
) -> Result<(GateResult, ActionRiskAssessment), crate::guard_db::GuardDbError> {
    let perturbations = PerturbationSet::compute_perturbations(
        command,
        args,
        undo_paths,
        checksum_targets,
        checkpoint_targets,
        flight_recorder_targets,
        data_integrity_targets,
    );

    let provenance_verified = if let Some(id) = &source_event_id {
        db.verify_provenance(id)?
    } else {
        false
    };

    let assessment = ActionRiskAssessment::new(
        perturbations.clone(),
        confidence,
        provenance_verified,
        can_interrupt,
    );

    let risk_level = assessment.get_risk_level();

    match risk_level {
        CommandRiskExtended::Critical => {
            warn!(
                command = %command,
                bound = %assessment.bound(),
                unmitigable = %assessment.has_unmitigable(),
                confidence = %confidence.get(),
                "Gate: critical risk — requires explicit permission"
            );
            Ok((GateResult::Interrupted {
                reason: "Critical risk: unmitigable perturbations detected; requires explicit permission".to_string(),
            }, assessment))
        }
        CommandRiskExtended::High => {
            warn!(
                command = %command,
                bound = %assessment.bound(),
                unmitigable = %assessment.has_unmitigable(),
                confidence = %confidence.get(),
                "Gate: high risk — requires explicit permission"
            );
            Ok((GateResult::Interrupted {
                reason: "High risk: unmitigable perturbations detected; requires explicit permission".to_string(),
            }, assessment))
        }
        CommandRiskExtended::Medium => {
            debug!(
                command = %command,
                bound = %assessment.bound(),
                confidence = %confidence.get(),
                "Gate: medium risk — requires review"
            );
            Ok((GateResult::Pending, assessment))
        }
        CommandRiskExtended::Low => {
            // Apply existing gate logic from guard/src/gate.rs
            if !provenance_verified {
                Ok((
                    GateResult::Interrupted {
                        reason: "No provenance: source_event_id not found in action_requests"
                            .to_string(),
                    },
                    assessment,
                ))
            } else {
                let _can_auto = db.load_anneal_config().map_err(|_| {
                    crate::guard_db::GuardDbError::SchemaError("config load failed".to_string())
                })?;
                // Simplified: proceed if trust layer >= 3 and confidence >= 0.8
                if trust_layer >= 3 && confidence.get() >= 0.8 {
                    Ok((GateResult::Proceed, assessment))
                } else {
                    Ok((GateResult::Pending, assessment))
                }
            }
        }
        CommandRiskExtended::Unauthorized => Ok((
            GateResult::Interrupted {
                reason: "Unauthorized action: detection != authorization".to_string(),
            },
            assessment,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TrustScore;
    use tempfile::tempdir;

    #[test]
    fn test_perturbation_set_high() {
        let set = PerturbationSet::compute_perturbations(
            "echo",
            &["hello".to_string()],
            vec!["git revert".to_string()],
            vec!["checksum.db".to_string()],
            vec!["session.checkpoint".to_string()],
            vec!["flight_recorder.log".to_string()],
            vec!["data_integrity.verify".to_string()],
        );

        assert_eq!(set.bound(), 1.0);
        assert_eq!(set.mitigable_count(), 5);
        assert_eq!(set.unmitigable_count(), 0);
    }

    #[test]
    fn test_perturbation_set_low() {
        let set = PerturbationSet::compute_perturbations(
            "rm -rf",
            &["-rf".to_string(), "/tmp".to_string()],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        );

        assert_eq!(set.bound(), 0.0);
        assert_eq!(set.mitigable_count(), 0);
        assert_eq!(set.unmitigable_count(), 1);
    }

    #[test]
    fn test_perturbation_set_medium() {
        let set = PerturbationSet::compute_perturbations(
            "git commit",
            &["commit".to_string(), "-m".to_string(), "test".to_string()],
            vec!["git revert".to_string()],
            vec![],
            vec!["session.checkpoint".to_string()],
            vec![],
            vec![],
        );

        assert_eq!(set.bound(), 1.0);
        assert_eq!(set.mitigable_count(), 2);
        assert_eq!(set.unmitigable_count(), 0);
    }

    #[test]
    fn test_action_risk_critical() {
        let perturbations = PerturbationSet::compute_perturbations(
            "rm -rf",
            &["-rf".to_string(), "/tmp".to_string()],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        );

        let assessment =
            ActionRiskAssessment::new(perturbations, TrustScore::new(0.3), false, true);

        assert_eq!(assessment.get_risk_level(), CommandRiskExtended::Critical);
        assert!(assessment.requires_permission());
    }

    #[test]
    fn test_action_risk_low() {
        let perturbations = PerturbationSet::compute_perturbations(
            "cargo fmt",
            &["--check".to_string()],
            vec!["git revert".to_string()],
            vec!["checksum.db".to_string()],
            vec!["session.checkpoint".to_string()],
            vec!["flight_recorder.log".to_string()],
            vec!["data_integrity.verify".to_string()],
        );

        let assessment = ActionRiskAssessment::new(perturbations, TrustScore::new(0.9), true, true);

        assert_eq!(assessment.get_risk_level(), CommandRiskExtended::Low);
        assert!(!assessment.requires_permission());
    }

    #[test]
    fn test_gate_check_critical_risk() {
        let dir = tempdir().unwrap();
        let db = crate::guard_db::GuardDb::open(dir.path().join("guard.db")).unwrap();

        let result = gate_check_with_reversibility(
            &db,
            "rm -rf",
            &["-rf".to_string(), "/tmp".to_string()],
            TrustScore::new(0.3),
            0,
            None,
            true,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        assert!(matches!(result.0, GateResult::Interrupted { .. }));
        assert_eq!(result.1.get_risk_level(), CommandRiskExtended::Critical);
    }

    #[test]
    fn test_gate_check_low_risk() {
        let dir = tempdir().unwrap();
        let db = crate::guard_db::GuardDb::open(dir.path().join("guard.db")).unwrap();

        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO action_requests (id, source_event_id, action_type, payload, trust_layer, confidence, status)
                  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "test-action-low",
                    "evt-low",
                    "cargo fmt",
                    "--check",
                    3,
                    0.9,
                    "trust-approved",
                ],
            )
            .unwrap();
        }

        let result = gate_check_with_reversibility(
            &db,
            "cargo fmt",
            &["--check".to_string()],
            TrustScore::new(0.9),
            3,
            Some("evt-low".to_string()),
            true,
            vec!["git revert".to_string()],
            vec!["checksum.db".to_string()],
            vec!["session.checkpoint".to_string()],
            vec!["flight_recorder.log".to_string()],
            vec!["data_integrity.verify".to_string()],
        )
        .unwrap();

        assert_eq!(result.0, GateResult::Proceed);
        assert_eq!(result.1.get_risk_level(), CommandRiskExtended::Low);
    }
}
