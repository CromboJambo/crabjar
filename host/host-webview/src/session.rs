/// Session model for webview sessions.
///
/// Replaced by the unified Session type in controller.rs.
/// Kept for backward compatibility — re-exported from controller.

#[deprecated(since = "0.2.0", note = "Use crate::Session from controller.rs instead")]
pub use crate::controller::Session;
