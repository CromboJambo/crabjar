/// A persisted trust resolution record.
#[derive(Debug, Clone)]
pub struct TrustResolutionEntry {
    pub id: i64,
    pub action_id: Option<String>,
    pub requested_layer: u32,
    pub requested_confidence: f64,
    pub requested_source: String,
    pub effective_layer: u32,
    pub effective_confidence: f64,
    pub effective_by: String,
    pub scope_actor: Option<String>,
    pub scope_target: Option<String>,
    pub applied_policies: String, // JSON array of policy strings
    pub resolved_at: i64,
}
