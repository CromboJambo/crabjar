/// Microsoft Graph API client for CrabJar host.
///
/// Provides typed access to Microsoft Graph endpoints:
/// - User profile (`/me`)
/// - Calendar events and views
/// - Mail messages
/// - People search
/// - Chat messages
///
/// Token acquisition is delegated to a `TokenProvider` trait
/// because the actual token comes from the WebView session manager
/// (which communicates with the Electron Chromium session).

pub mod client;
pub mod config;
pub mod types;

pub use client::GraphApiClient;
pub use config::GraphApiConfig;
pub use types::*;
