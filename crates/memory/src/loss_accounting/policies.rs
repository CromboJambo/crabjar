//! Compression policies and fidelity estimation.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::models::{Confidence, Recoverability};

/// Type of artifact being compressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactType {
    RawObservation,
    WorkerScratchpad,
    ContextSummary,
    TaskHistory,
    InstitutionalMemory,
}

impl fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactType::RawObservation => write!(f, "raw-observation"),
            ArtifactType::WorkerScratchpad => write!(f, "worker-scratchpad"),
            ArtifactType::ContextSummary => write!(f, "context-summary"),
            ArtifactType::TaskHistory => write!(f, "task-history"),
            ArtifactType::InstitutionalMemory => write!(f, "institutional-memory"),
        }
    }
}

/// Type of task being performed (for fidelity estimation).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    Architecture,
    Debugging,
    Implementation,
    Review,
    Planning,
    Other(String),
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskType::Architecture => write!(f, "architecture"),
            TaskType::Debugging => write!(f, "debugging"),
            TaskType::Implementation => write!(f, "implementation"),
            TaskType::Review => write!(f, "review"),
            TaskType::Planning => write!(f, "planning"),
            TaskType::Other(s) => write!(f, "{}", s),
        }
    }
}

impl TaskType {
    pub fn other(s: impl Into<String>) -> Self {
        TaskType::Other(s.into())
    }
}

/// Confidence level required for loss estimates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    VerifiedOnly,
    HighOrBetter,
    MediumOrBetter,
    Any,
}

impl fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfidenceLevel::VerifiedOnly => write!(f, "verified-only"),
            ConfidenceLevel::HighOrBetter => write!(f, "high-or-better"),
            ConfidenceLevel::MediumOrBetter => write!(f, "medium-or-better"),
            ConfidenceLevel::Any => write!(f, "any"),
        }
    }
}

/// Check if a confidence meets or exceeds the required level.
pub fn confidence_meets_requirement(actual: &Confidence, required: &ConfidenceLevel) -> bool {
    let actual_rank = match actual {
        Confidence::Verified => 4,
        Confidence::High { .. } => 3,
        Confidence::Medium { .. } => 2,
        Confidence::Low { .. } => 1,
        Confidence::Unknown => 0,
    };
    let required_rank = match required {
        ConfidenceLevel::VerifiedOnly => 4,
        ConfidenceLevel::HighOrBetter => 3,
        ConfidenceLevel::MediumOrBetter => 2,
        ConfidenceLevel::Any => 0,
    };
    actual_rank >= required_rank
}

/// Compression policy for a specific artifact type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressionPolicy {
    pub artifact_type: ArtifactType,
    pub max_reduction_depth: u32,
    pub required_recoverability: Recoverability,
    pub min_fidelity_estimate: f32,
    pub loss_confidence_required: ConfidenceLevel,
}

impl CompressionPolicy {
    /// Default policies from the spec.
    pub fn for_artifact(artifact_type: ArtifactType) -> Self {
        match artifact_type {
            ArtifactType::RawObservation => Self {
                artifact_type,
                max_reduction_depth: 0,
                required_recoverability: Recoverability::Lossless,
                min_fidelity_estimate: 1.0,
                loss_confidence_required: ConfidenceLevel::VerifiedOnly,
            },
            ArtifactType::WorkerScratchpad => Self {
                artifact_type,
                max_reduction_depth: 2,
                required_recoverability: Recoverability::ReferenceOnly,
                min_fidelity_estimate: 0.7,
                loss_confidence_required: ConfidenceLevel::MediumOrBetter,
            },
            ArtifactType::ContextSummary => Self {
                artifact_type,
                max_reduction_depth: 3,
                required_recoverability: Recoverability::Lossless,
                min_fidelity_estimate: 0.85,
                loss_confidence_required: ConfidenceLevel::HighOrBetter,
            },
            ArtifactType::TaskHistory => Self {
                artifact_type,
                max_reduction_depth: 4,
                required_recoverability: Recoverability::Bounded { max_depth: 2 },
                min_fidelity_estimate: 0.8,
                loss_confidence_required: ConfidenceLevel::MediumOrBetter,
            },
            ArtifactType::InstitutionalMemory => Self {
                artifact_type,
                max_reduction_depth: 1,
                required_recoverability: Recoverability::Lossless,
                min_fidelity_estimate: 0.95,
                loss_confidence_required: ConfidenceLevel::VerifiedOnly,
            },
        }
    }

    /// Check if a reduction depth is allowed by this policy.
    pub fn allows_depth(&self, depth: u32) -> bool {
        depth <= self.max_reduction_depth
    }

    /// Check if a recoverability meets the policy requirement.
    pub fn allows_recoverability(&self, actual: &Recoverability) -> bool {
        match (&self.required_recoverability, actual) {
            (Recoverability::Lossless, Recoverability::Lossless) => true,
            (Recoverability::Bounded { max_depth }, Recoverability::Bounded { .. }) => {
                *max_depth > 0
            }
            (_, Recoverability::Lossless) => true,
            _ => false,
        }
    }
}

/// Basis for fidelity measurement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeasurementBasis {
    Empirical { samples: usize },
    Analogical { similar_task: TaskType },
    Theoretical { model: String },
    Unknown,
}

impl fmt::Display for MeasurementBasis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeasurementBasis::Empirical { samples } => write!(f, "empirical(n={})", samples),
            MeasurementBasis::Analogical { similar_task } => {
                write!(f, "analogical({})", similar_task)
            }
            MeasurementBasis::Theoretical { model } => write!(f, "theoretical({})", model),
            MeasurementBasis::Unknown => write!(f, "unknown"),
        }
    }
}

/// Fidelity profile — estimated fidelity for a task type at a given fold depth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FidelityProfile {
    pub task_type: TaskType,
    pub fold_depth: u32,
    pub estimated_fidelity: f32,
    pub measurement_basis: MeasurementBasis,
}

impl FidelityProfile {
    pub fn new(
        task_type: TaskType,
        fold_depth: u32,
        estimated_fidelity: f32,
        basis: MeasurementBasis,
    ) -> Self {
        Self {
            task_type,
            fold_depth,
            estimated_fidelity,
            measurement_basis: basis,
        }
    }

    /// Check if the fidelity meets a minimum threshold.
    pub fn meets_threshold(&self, min_fidelity: f32) -> bool {
        self.estimated_fidelity >= min_fidelity
    }
}
