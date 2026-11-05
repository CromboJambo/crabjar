use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Staleness status for a state-doc based on three-tier thresholds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StalenessStatus {
    /// Document is fresh — captured within 7 days and content unchanged.
    Fresh,
    /// Document is stale — past 7 days but within 14 days; warning level.
    Stale { days: i64 },
    /// Document is expired — past 14 days; treated as untrustworthy without re-index.
    Expired { days: i64 },
    /// Document is moldy — past 30 days; discarded unless additional context added since last modification.
    Moldy { days: i64, has_recent_context: bool },
}

impl StalenessStatus {
    pub fn is_fresh(&self) -> bool {
        matches!(self, Self::Fresh)
    }

    pub fn is_trustworthy(&self) -> bool {
        matches!(self, Self::Fresh | Self::Stale { .. })
    }

    /// Compute staleness status from the last modified timestamp.
    /// Uses three-tier thresholds: 7d stale, 14d expired, 30d moldy.
    pub fn compute(last_modified: &DateTime<Utc>) -> Self {
        let now = Utc::now();
        let duration = now.signed_duration_since(*last_modified);
        let days = duration.num_days().max(0);

        if days < 7 {
            Self::Fresh
        } else if days < 14 {
            Self::Stale { days }
        } else if days < 30 {
            Self::Expired { days }
        } else {
            // Moldy: check if there's been any annotation activity since last modification.
            // If annotations were added after the doc was last modified, it has recent context.
            // This is a conservative heuristic — actual check requires querying annotations table.
            Self::Moldy {
                days,
                has_recent_context: false,
            }
        }
    }

    /// Compute staleness status with knowledge of whether there's been annotation activity after the doc was last modified.
    pub fn compute_with_context(
        last_modified: &DateTime<Utc>,
        latest_annotation_at: Option<DateTime<Utc>>,
    ) -> Self {
        let now = Utc::now();
        let duration = now.signed_duration_since(*last_modified);
        let days = duration.num_days().max(0);

        // Check if there's been any annotation activity after the doc was last modified.
        let has_recent_context = latest_annotation_at.is_some_and(|ann_time| {
            ann_time > *last_modified && (now.signed_duration_since(ann_time).num_days() < 30)
        });

        if days < 7 {
            Self::Fresh
        } else if days < 14 {
            Self::Stale { days }
        } else if days < 30 {
            Self::Expired { days }
        } else {
            // Moldy: reset to expired if there's recent context (user added value).
            if has_recent_context {
                Self::Expired { days }
            } else {
                Self::Moldy {
                    days,
                    has_recent_context: false,
                }
            }
        }
    }

    /// Return the number of days since last modification.
    pub fn age_days(&self) -> i64 {
        match self {
            Self::Fresh => 0,
            Self::Stale { days } | Self::Expired { days } | Self::Moldy { days, .. } => *days,
        }
    }

    /// Human-readable label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Stale { .. } => "stale",
            Self::Expired { .. } => "expired",
            Self::Moldy { .. } => "moldy",
        }
    }

    /// Warning message for the user/agent.
    pub fn warning(&self) -> Option<String> {
        match self {
            Self::Fresh => None,
            Self::Stale { days } => Some(format!(
                "State-doc is {} days old (stale threshold: 7d). Content may have drifted from indexed state.",
                days
            )),
            Self::Expired { days } => Some(format!(
                "State-doc is {} days old (expired at 14d). Treat as untrustworthy — re-index for authoritative use.",
                days
            )),
            Self::Moldy { days, .. } => Some(format!(
                "State-doc is {} days old (moldy at 30d). Corroded beyond useful provenance; discarded unless additional context added relative to reconstruction cost.",
                days
            )),
        }
    }
}

/// Metadata about a state-doc as a whole.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocMetadata {
    pub doc_name: String,
    pub display_name: String,
    pub description: String,
    pub path: String,
    pub last_modified: DateTime<Utc>,
    pub line_count: usize,
    pub section_count: usize,
    pub table_count: usize,
    pub code_block_count: usize,
    pub annotation_count: usize,
    pub open_annotation_count: usize,
    pub checksum: String,
}

/// A section in the state-doc hierarchy. 3 levels: h1 → h2 → h3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub id: i64,
    pub doc_name: String,
    pub level: u8, // 1, 2, or 3
    pub title: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parent_id: Option<i64>,
    pub child_count: usize,
    pub content_hash: String,
    pub is_confidence_section: bool,
}

/// An extracted table from a state-doc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub id: i64,
    pub doc_name: String,
    pub section_id: i64,
    pub start_line: usize,
    pub end_line: usize,
    pub headers: Vec<String>,
    pub row_count: usize,
    pub content_hash: String,
}

/// A code block extracted from a state-doc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub id: i64,
    pub doc_name: String,
    pub section_id: i64,
    pub start_line: usize,
    pub end_line: usize,
    pub language: String,
    pub content_hash: String,
    pub line_count: usize,
}

/// The confidence assessment (doubt block) from a state-doc.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfidenceAssessment {
    pub doc_name: String,
    pub section_id: i64,
    pub what_captured: String,
    pub what_missed: String,
    pub assumptions: Vec<String>,
    pub blind_spots: Vec<String>,
    pub stale_after: String,
    pub captured_at: DateTime<Utc>,
}

/// An overlay annotation linked to a specific line in a state-doc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: i64,
    pub doc_name: String,
    pub section_id: Option<i64>,
    pub line: usize,
    pub kind: String, // "note" or "question"
    pub message: String,
    pub author: String,
    pub status: String, // "open" or "resolved"
    pub created_at: DateTime<Utc>,
}

/// A query result row that combines section content with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionRow {
    pub id: i64,
    pub doc_name: String,
    pub level: u8,
    pub title: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parent_id: Option<i64>,
    pub child_count: usize,
    pub content_hash: String,
    pub is_confidence_section: bool,
    pub open_annotations: usize,
}

/// A table query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub id: i64,
    pub doc_name: String,
    pub section_id: i64,
    pub section_title: String,
    pub start_line: usize,
    pub end_line: usize,
    pub headers: Vec<String>,
    pub row_count: usize,
}

/// A code block query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlockRow {
    pub id: i64,
    pub doc_name: String,
    pub section_id: i64,
    pub section_title: String,
    pub start_line: usize,
    pub end_line: usize,
    pub language: String,
    pub line_count: usize,
}

/// A confidence assessment query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceRow {
    pub doc_name: String,
    pub section_id: i64,
    pub what_captured: String,
    pub what_missed: String,
    pub assumptions: Vec<String>,
    pub blind_spots: Vec<String>,
    pub stale_after: String,
    pub captured_at: DateTime<Utc>,
}

/// An annotation query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationRow {
    pub id: i64,
    pub doc_name: String,
    pub section_id: Option<i64>,
    pub section_title: Option<String>,
    pub line: usize,
    pub kind: String,
    pub message: String,
    pub author: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}
