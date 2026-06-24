//! `host-webview` — WebView session management for CrabJar host.
//!
//! Provides embedded WebView lifecycle, OAuth2 token acquisition,
//! cookie store, partition management, and secure token caching.
#![allow(dead_code)]
pub mod auth;
pub mod controller;
pub mod cookie_store;
pub mod partition;
pub mod token_cache;

pub use auth::{AuthError, AuthManager, AuthState, TokenResponse};
pub use controller::{Session, WebViewController, WebViewEngine};
pub use cookie_store::{Cookie, CookieStore, Partition, SessionToken};
pub use partition::{PartitionManager, SessionPartition};
pub use token_cache::SecureTokenCache;
