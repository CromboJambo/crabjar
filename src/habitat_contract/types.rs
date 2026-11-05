//! Habitat contract data types.

/// A pending guard action, projected from the `pending_queue` table.
/// Intermediate form so [`build_contract`] stays pure (no SQLite).
#[derive(Debug, Clone)]
pub struct PendingAction {
    pub id: String,
    pub action_type: String,
    pub command: String,
    pub trust_layer: u32,
    pub confidence: f64,
    pub queued_at: i64,
    pub reason: String,
}

/// The theory state-doc's staleness, projected from the state-docs querier.
#[derive(Debug, Clone)]
pub struct TheoryStatus {
    /// Whether the doc is indexed at all (false → not created yet).
    pub indexed: bool,
    /// `fresh | stale | expired | moldy`.
    pub status: String,
    pub days_old: f64,
    pub is_trustworthy: bool,
    pub warning: Option<String>,
}

impl Default for TheoryStatus {
    fn default() -> Self {
        Self {
            indexed: false,
            status: String::new(),
            days_old: 0.0,
            is_trustworthy: false,
            warning: None,
        }
    }
}
