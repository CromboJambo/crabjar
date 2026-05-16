use crate::GateResult;
use crate::guard_db::GuardDb;
use crate::types::TrustScore;
use tracing::{debug, warn};

/// Reversibility score for an action. Higher score means easier to undo.
///
/// Scoring factors:
/// - undo path availability (explicit rollback commands)
/// - data integrity preservation (checksums, hashes)
/// - state preservation (session checkpoints, flight recorder)
/// - threshold established through testing and iteration
#[derive(Clone)]
pub struct ReversibilityScore {
    score: f64,
    factors: Vec<ReversibilityFactor>,
}

impl ReversibilityScore {
    pub fn new(score: f64, factors: Vec<ReversibilityFactor>) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
            factors,
        }
    }

    pub fn get(&self) -> f64 {
        self.score
    }

    pub fn factors(&self) -> &[ReversibilityFactor] {
        &self.factors
    }

    /// Score an action based on its properties.
    pub fn score_action(
        _command: &str,
        _args: &[String],
        has_undo_path: bool,
        has_checksums: bool,
        has_checkpoint: bool,
        has_flight_recorder: bool,
        data_integrity: bool,
    ) -> Self {
        let mut score = 0.0;
        let mut factors = Vec::new();

        if has_undo_path {
            score += 0.25;
            factors.push(ReversibilityFactor::UndoPath);
        }

        if has_checksums {
            score += 0.20;
            factors.push(ReversibilityFactor::Checksums);
        }

        if has_checkpoint {
            score += 0.20;
            factors.push(ReversibilityFactor::Checkpoint);
        }

        if has_flight_recorder {
            score += 0.15;
            factors.push(ReversibilityFactor::FlightRecorder);
        }

        if data_integrity {
            score += 0.20;
            factors.push(ReversibilityFactor::DataIntegrity);
        }

        Self::new(score, factors)
    }
}

/// A factor contributing to reversibility scoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReversibilityFactor {
    UndoPath,
    Checksums,
    Checkpoint,
    FlightRecorder,
    DataIntegrity,
    NoUndoPath,
    NoChecksums,
    NoCheckpoint,
    NoFlightRecorder,
    DataCorruption,
}

/// Action risk assessment that combines reversibility scoring with confidence decay.
///
/// Risk factors:
/// - reversibility score
/// - confidence decay of the command
/// - uncertainty exposure (below threshold → surface before executing)
/// - interruptibility (allow gate to return Interrupted)
/// - additional risk factors established through testing and iteration
#[allow(dead_code)]
pub struct ActionRiskAssessment {
    reversibility: ReversibilityScore,
    confidence: TrustScore,
    uncertainty_exposed: bool,
    interruptible: bool,
    risk_level: CommandRiskExtended,
}

impl ActionRiskAssessment {
    pub fn new(
        reversibility: ReversibilityScore,
        confidence: TrustScore,
        uncertainty_exposed: bool,
        interruptible: bool,
    ) -> Self {
        let risk_level = Self::determine_risk_level(
            &reversibility,
            &confidence,
            uncertainty_exposed,
            interruptible,
        );

        Self {
            reversibility,
            confidence,
            uncertainty_exposed,
            interruptible,
            risk_level,
        }
    }

    /// Determine risk level based on scoring factors.
    fn determine_risk_level(
        reversibility: &ReversibilityScore,
        confidence: &TrustScore,
        uncertainty_exposed: bool,
        interruptible: bool,
    ) -> CommandRiskExtended {
        let rev_score = reversibility.get();
        let conf_score = confidence.get();

        if rev_score < 0.3 && conf_score < 0.5 {
            CommandRiskExtended::Critical
        } else if rev_score < 0.5 && conf_score < 0.6 {
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

/// Gate check with reversibility scoring.
#[allow(clippy::too_many_arguments)]
pub fn gate_check_with_reversibility(
    db: &GuardDb,
    command: &str,
    args: &[String],
    confidence: TrustScore,
    trust_layer: u32,
    has_raw_data: bool,
    has_uncertainty: bool,
    can_interrupt: bool,
    has_undo_path: bool,
    has_checksums: bool,
    has_checkpoint: bool,
    has_flight_recorder: bool,
    data_integrity: bool,
) -> Result<(GateResult, ActionRiskAssessment), crate::guard_db::GuardDbError> {
    let reversibility = ReversibilityScore::score_action(
        command,
        args,
        has_undo_path,
        has_checksums,
        has_checkpoint,
        has_flight_recorder,
        data_integrity,
    );

    let assessment = ActionRiskAssessment::new(
        reversibility.clone(),
        confidence,
        has_uncertainty,
        can_interrupt,
    );

    let risk_level = assessment.get_risk_level();

    match risk_level {
        CommandRiskExtended::Critical => {
            warn!(
                command = %command,
                reversibility = %reversibility.get(),
                confidence = %confidence.get(),
                "Gate: critical risk — requires explicit permission"
            );
            Ok((GateResult::Interrupted {
                reason: "Critical risk: reversibility score below threshold; requires explicit permission".to_string(),
            }, assessment))
        }
        CommandRiskExtended::High => {
            warn!(
                command = %command,
                reversibility = %reversibility.get(),
                confidence = %confidence.get(),
                "Gate: high risk — requires explicit permission"
            );
            Ok((GateResult::Interrupted {
                reason: "High risk: reversibility score below threshold; requires explicit permission".to_string(),
            }, assessment))
        }
        CommandRiskExtended::Medium => {
            debug!(
                command = %command,
                reversibility = %reversibility.get(),
                confidence = %confidence.get(),
                "Gate: medium risk — requires review"
            );
            Ok((GateResult::Pending, assessment))
        }
        CommandRiskExtended::Low => {
            // Apply existing gate logic from guard/src/gate.rs
            if !has_raw_data {
                Ok((
                    GateResult::Interrupted {
                        reason: "Action triggered without raw data reference".to_string(),
                    },
                    assessment,
                ))
            } else if !has_uncertainty {
                Ok((
                    GateResult::Interrupted {
                        reason: "Action triggered without uncertainty exposure".to_string(),
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
    fn test_reversibility_score_high() {
        let score = ReversibilityScore::score_action(
            "echo",
            &["hello".to_string()],
            true,
            true,
            true,
            true,
            true,
        );

        assert_eq!(score.get(), 1.0);
        assert_eq!(score.factors().len(), 5);
    }

    #[test]
    fn test_reversibility_score_low() {
        let score = ReversibilityScore::score_action(
            "rm -rf",
            &["-rf".to_string(), "/tmp".to_string()],
            false,
            false,
            false,
            false,
            false,
        );

        assert_eq!(score.get(), 0.0);
        assert_eq!(score.factors().len(), 0);
    }

    #[test]
    fn test_reversibility_score_medium() {
        let score = ReversibilityScore::score_action(
            "git commit",
            &["commit".to_string(), "-m".to_string(), "test".to_string()],
            true,
            false,
            true,
            false,
            false,
        );

        assert_eq!(score.get(), 0.45);
        assert_eq!(score.factors().len(), 2);
    }

    #[test]
    fn test_action_risk_critical() {
        let reversibility = ReversibilityScore::score_action(
            "rm -rf",
            &["-rf".to_string(), "/tmp".to_string()],
            false,
            false,
            false,
            false,
            false,
        );

        let assessment =
            ActionRiskAssessment::new(reversibility, TrustScore::new(0.3), false, true);

        assert_eq!(assessment.get_risk_level(), CommandRiskExtended::Critical);
        assert!(assessment.requires_permission());
    }

    #[test]
    fn test_action_risk_low() {
        let reversibility = ReversibilityScore::score_action(
            "cargo fmt",
            &["--check".to_string()],
            true,
            true,
            true,
            true,
            true,
        );

        let assessment = ActionRiskAssessment::new(reversibility, TrustScore::new(0.9), true, true);

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
            true,
            true,
            true,
            false,
            false,
            false,
            false,
            false,
        )
        .unwrap();

        assert!(matches!(result.0, GateResult::Interrupted { .. }));
        assert_eq!(result.1.get_risk_level(), CommandRiskExtended::Critical);
    }

    #[test]
    fn test_gate_check_low_risk() {
        let dir = tempdir().unwrap();
        let db = crate::guard_db::GuardDb::open(dir.path().join("guard.db")).unwrap();

        let result = gate_check_with_reversibility(
            &db,
            "cargo fmt",
            &["--check".to_string()],
            TrustScore::new(0.9),
            3,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
        )
        .unwrap();

        assert_eq!(result.0, GateResult::Proceed);
        assert_eq!(result.1.get_risk_level(), CommandRiskExtended::Low);
    }
}
