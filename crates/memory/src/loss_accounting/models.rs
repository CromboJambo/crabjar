//! Core loss accounting data models.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Unix timestamp in seconds.
pub type Timestamp = i64;

/// Unique identifier for a loss profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LossProfileId(pub String);

impl LossProfileId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for LossProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Reference to the original source artifact/context that was compressed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    pub id: String,
    pub content_class: String,
    pub description: Option<String>,
}

impl SourceRef {
    pub fn new(id: impl Into<String>, content_class: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content_class: content_class.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Strategy used for compression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionStrategy {
    Summarization { model: String },
    Truncation { from: TruncationSide },
    Sampling { rate: f32 },
    Abstraction { level: u32 },
    Aggregation { group_by: String },
}

impl fmt::Display for CompressionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompressionStrategy::Summarization { model } => write!(f, "summarize({})", model),
            CompressionStrategy::Truncation { from } => write!(f, "truncate({:?})", from),
            CompressionStrategy::Sampling { rate } => write!(f, "sample({:.2})", rate),
            CompressionStrategy::Abstraction { level } => write!(f, "abstract(l{})", level),
            CompressionStrategy::Aggregation { group_by } => write!(f, "aggregate({})", group_by),
        }
    }
}

/// Side from which to truncate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TruncationSide {
    Head,
    Tail,
    Both,
}

/// Estimate of what was lost during compression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LossEstimate {
    None,
    Bounded {
        max_items: usize,
        description: String,
    },
    Estimated {
        probability: f32,
        description: String,
    },
    Unknown {
        reason: String,
    },
}

impl fmt::Display for LossEstimate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LossEstimate::None => write!(f, "none"),
            LossEstimate::Bounded {
                max_items,
                description,
            } => {
                write!(f, "bounded({} items): {}", max_items, description)
            }
            LossEstimate::Estimated {
                probability,
                description,
            } => {
                write!(f, "estimated({:.0}%): {}", probability * 100.0, description)
            }
            LossEstimate::Unknown { reason } => write!(f, "unknown: {}", reason),
        }
    }
}

/// Confidence in the loss estimate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    Verified,
    High { basis: String },
    Medium { basis: String },
    Low { basis: String },
    Unknown,
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Confidence::Verified => write!(f, "verified"),
            Confidence::High { basis } => write!(f, "high({})", basis),
            Confidence::Medium { basis } => write!(f, "medium({})", basis),
            Confidence::Low { basis } => write!(f, "low({})", basis),
            Confidence::Unknown => write!(f, "unknown"),
        }
    }
}

/// Cost of accessing the original data for recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessCost {
    Free,
    Low { seconds: u32 },
    Medium { minutes: u32 },
    High { hours: u32 },
    Prohibitive,
}

impl fmt::Display for AccessCost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessCost::Free => write!(f, "free"),
            AccessCost::Low { seconds } => write!(f, "~{}s", seconds),
            AccessCost::Medium { minutes } => write!(f, "~{}m", minutes),
            AccessCost::High { hours } => write!(f, "~{}h", hours),
            AccessCost::Prohibitive => write!(f, "prohibitive"),
        }
    }
}

/// Reference to where the original data can be recovered from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryRef {
    pub source_id: String,
    pub storage_location: String,
    pub access_cost: AccessCost,
    pub freshness_verified: bool,
}

impl RecoveryRef {
    pub fn new(source_id: impl Into<String>, storage_location: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            storage_location: storage_location.into(),
            access_cost: AccessCost::Free,
            freshness_verified: false,
        }
    }

    pub fn with_access_cost(mut self, cost: AccessCost) -> Self {
        self.access_cost = cost;
        self
    }

    pub fn with_freshness_verified(mut self) -> Self {
        self.freshness_verified = true;
        self
    }
}

/// How recoverable the original data is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recoverability {
    Lossless,
    Bounded { max_depth: u32 },
    ReferenceOnly,
    Unknown,
}

impl fmt::Display for Recoverability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Recoverability::Lossless => write!(f, "lossless"),
            Recoverability::Bounded { max_depth } => write!(f, "bounded(depth={})", max_depth),
            Recoverability::ReferenceOnly => write!(f, "reference-only"),
            Recoverability::Unknown => write!(f, "unknown"),
        }
    }
}

/// A loss profile records what was compressed, how, and what was estimated lost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LossProfile {
    pub id: LossProfileId,
    pub created_at: Timestamp,
    pub source: SourceRef,
    pub original_size: usize,
    pub target_size: Option<usize>,
    pub final_size: usize,
    pub strategy: CompressionStrategy,
    pub reduction_depth: u32,
    pub reduction_ratio: f32,
    pub estimated_loss: LossEstimate,
    pub loss_confidence: Confidence,
    pub recovery_path: Option<RecoveryRef>,
    pub recoverability: Recoverability,
}

impl LossProfile {
    pub fn new(id: impl Into<String>, source: SourceRef, original_size: usize) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            id: LossProfileId::new(id),
            created_at: now,
            source,
            original_size,
            target_size: None,
            final_size: 0,
            strategy: CompressionStrategy::Truncation {
                from: TruncationSide::Tail,
            },
            reduction_depth: 1,
            reduction_ratio: 1.0,
            estimated_loss: LossEstimate::Unknown {
                reason: "not yet estimated".to_string(),
            },
            loss_confidence: Confidence::Unknown,
            recovery_path: None,
            recoverability: Recoverability::Unknown,
        }
    }

    pub fn with_strategy(mut self, strategy: CompressionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_target_size(mut self, size: usize) -> Self {
        self.target_size = Some(size);
        self
    }

    pub fn with_final_size(mut self, size: usize) -> Self {
        self.final_size = size;
        if size > 0 {
            self.reduction_ratio = self.original_size as f32 / size as f32;
        }
        self
    }

    pub fn with_loss_estimate(mut self, estimate: LossEstimate) -> Self {
        self.estimated_loss = estimate;
        self
    }

    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.loss_confidence = confidence;
        self
    }

    pub fn with_recovery(mut self, recovery: RecoveryRef, recoverability: Recoverability) -> Self {
        self.recovery_path = Some(recovery);
        self.recoverability = recoverability;
        self
    }

    /// Check if this profile claims lossless compression.
    pub fn is_lossless(&self) -> bool {
        matches!(self.estimated_loss, LossEstimate::None)
            && matches!(self.recoverability, Recoverability::Lossless)
    }

    /// Check if recovery is possible.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self.recoverability,
            Recoverability::Lossless | Recoverability::Bounded { .. }
        )
    }
}
